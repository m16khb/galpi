use super::failure::{SharedFailure, set_failure};
use crate::application::error::AppError;
use hound::{SampleFormat, WavSpec, WavWriter};
use std::fs::{File, OpenOptions};
use std::io::BufWriter;
use std::path::Path;
use std::sync::mpsc::{Receiver, Sender, SyncSender, channel, sync_channel};
use std::thread::JoinHandle;

pub(super) const FRAMES_PER_CHUNK: usize = 4_096;
const QUEUE_SECONDS: usize = 30;
const RIFF_DATA_LIMIT: u64 = u32::MAX as u64 - 44;

pub enum WriterCommand {
    Samples {
        samples: Vec<i16>,
        dropped_before: u64,
    },
    Finish {
        trailing_dropped: u64,
        reply: SyncSender<Result<WriterSummary, String>>,
    },
    Cancel(SyncSender<()>),
}

pub struct WriterSummary {
    pub samples: u64,
    pub dropped_samples: u64,
}

pub struct WriterHandle {
    sender: SyncSender<WriterCommand>,
    /// Emptied buffers travelling back to the capture callback for refilling.
    recycled: Option<Receiver<Vec<i16>>>,
    thread: JoinHandle<()>,
}

pub fn spawn(
    path: &Path,
    sample_rate: u32,
    channels: u16,
    failure: SharedFailure,
) -> Result<WriterHandle, AppError> {
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| AppError::io("녹음 파일을 만들지 못했습니다", &error))?;
    let writer = match WavWriter::new(
        BufWriter::new(file),
        WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        },
    ) {
        Ok(writer) => writer,
        Err(error) => {
            return Err(remove_after_spawn_failure(
                path,
                AppError::new("WAV_WRITE_FAILED", error.to_string()),
            ));
        }
    };
    let (sender, receiver) = sync_channel(queue_capacity(sample_rate));
    let (recycle_sender, recycled) = channel();
    let thread = match std::thread::Builder::new()
        .name("galpi-wav-writer".to_owned())
        .spawn(move || {
            run_writer(
                writer,
                &receiver,
                &recycle_sender,
                sample_rate,
                channels,
                &failure,
            );
        }) {
        Ok(thread) => thread,
        Err(error) => {
            return Err(remove_after_spawn_failure(
                path,
                AppError::new("WAV_WRITER_FAILED", error.to_string()),
            ));
        }
    };
    Ok(WriterHandle {
        sender,
        recycled: Some(recycled),
        thread,
    })
}

fn remove_after_spawn_failure(path: &Path, error: AppError) -> AppError {
    match std::fs::remove_file(path) {
        Ok(()) => error,
        Err(remove_error) => AppError::new(
            "RECORDING_CLEANUP_FAILED",
            format!("{error}; cleanup: {remove_error}"),
        ),
    }
}

impl WriterHandle {
    pub fn sender(&self) -> SyncSender<WriterCommand> {
        self.sender.clone()
    }

    /// Hand the buffer pool to the capture stream; only one owner can hold it.
    pub fn take_recycled(&mut self) -> Receiver<Vec<i16>> {
        self.recycled
            .take()
            .unwrap_or_else(|| channel::<Vec<i16>>().1)
    }

    pub fn finish(self, trailing_dropped: u64) -> Result<WriterSummary, AppError> {
        let (reply, receiver) = sync_channel(1);
        self.sender
            .send(WriterCommand::Finish {
                trailing_dropped,
                reply,
            })
            .map_err(|_| AppError::new("WAV_WRITER_FAILED", "WAV writer가 종료되었습니다."))?;
        let result = receiver
            .recv()
            .map_err(|_| AppError::new("WAV_WRITER_FAILED", "WAV 완료 응답이 없습니다."))?;
        self.join()?;
        result.map_err(|message| AppError::new("WAV_WRITE_FAILED", message))
    }

    pub fn cancel(self) -> Result<(), AppError> {
        let (reply, receiver) = sync_channel(1);
        self.sender
            .send(WriterCommand::Cancel(reply))
            .map_err(|_| AppError::new("WAV_WRITER_FAILED", "WAV writer가 종료되었습니다."))?;
        receiver
            .recv()
            .map_err(|_| AppError::new("WAV_WRITER_FAILED", "WAV 취소 응답이 없습니다."))?;
        self.join()
    }

    fn join(self) -> Result<(), AppError> {
        self.thread
            .join()
            .map_err(|_| AppError::new("WAV_WRITER_FAILED", "WAV writer thread가 중단됐습니다."))
    }
}

const fn queue_capacity(sample_rate: u32) -> usize {
    let frames = sample_rate as usize * QUEUE_SECONDS;
    frames.div_ceil(FRAMES_PER_CHUNK)
}

fn append_silence(
    writer: &mut WavWriter<BufWriter<File>>,
    samples: &mut u64,
    count: u64,
) -> Result<(), String> {
    for _ in 0..count {
        writer
            .write_sample(0_i16)
            .map_err(|error| error.to_string())?;
        *samples += 1;
    }
    Ok(())
}

fn append_samples(
    writer: &mut WavWriter<BufWriter<File>>,
    samples: &mut u64,
    mut chunk: Vec<i16>,
    recycle: &Sender<Vec<i16>>,
) -> Result<(), String> {
    let result = write_chunk(writer, samples, &chunk);
    // The buffer goes back to the callback either way; a write failure ends
    // the recording, and an unreturned buffer would just make it allocate.
    chunk.clear();
    let _returned = recycle.send(chunk);
    result
}

fn write_chunk(
    writer: &mut WavWriter<BufWriter<File>>,
    samples: &mut u64,
    chunk: &[i16],
) -> Result<(), String> {
    // hound's bulk writer skips the per-sample bounds and format checks that
    // `write_sample` repeats for every one of 48,000 samples a second.
    let count = u32::try_from(chunk.len()).map_err(|error| error.to_string())?;
    let mut bulk = writer.get_i16_writer(count);
    for sample in chunk {
        bulk.write_sample(*sample);
    }
    bulk.flush().map_err(|error| error.to_string())?;
    *samples += u64::from(count);
    Ok(())
}

fn run_writer(
    mut writer: WavWriter<BufWriter<File>>,
    receiver: &Receiver<WriterCommand>,
    recycle: &Sender<Vec<i16>>,
    sample_rate: u32,
    channels: u16,
    failure: &SharedFailure,
) {
    let flush_every = u64::from(sample_rate) * u64::from(channels) * 5;
    let mut samples = 0_u64;
    let mut dropped_samples = 0_u64;
    let mut next_flush = flush_every;
    while let Ok(command) = receiver.recv() {
        match command {
            WriterCommand::Samples {
                samples: chunk,
                dropped_before,
            } => {
                let additional = dropped_before.saturating_add(chunk.len() as u64);
                if samples.saturating_add(additional) * 2 > RIFF_DATA_LIMIT {
                    set_failure(
                        failure,
                        "WAV_TOO_LARGE",
                        "WAV 파일이 4GB 한도에 도달했습니다.".to_owned(),
                    );
                    continue;
                }
                if let Err(error) = append_silence(&mut writer, &mut samples, dropped_before) {
                    set_failure(failure, "WAV_WRITE_FAILED", error);
                    continue;
                }
                dropped_samples += dropped_before;
                let outcome = append_samples(&mut writer, &mut samples, chunk, recycle);
                if let Err(error) = outcome {
                    set_failure(failure, "WAV_WRITE_FAILED", error);
                    continue;
                }
                if samples >= next_flush {
                    if let Err(error) = writer.flush() {
                        set_failure(failure, "WAV_WRITE_FAILED", error.to_string());
                    }
                    next_flush = samples.saturating_add(flush_every);
                }
            }
            WriterCommand::Finish {
                trailing_dropped,
                reply,
            } => {
                if samples.saturating_add(trailing_dropped) * 2 > RIFF_DATA_LIMIT {
                    set_failure(
                        failure,
                        "WAV_TOO_LARGE",
                        "WAV 파일이 4GB 한도에 도달했습니다.".to_owned(),
                    );
                } else if let Err(error) =
                    append_silence(&mut writer, &mut samples, trailing_dropped)
                {
                    set_failure(failure, "WAV_WRITE_FAILED", error);
                } else {
                    dropped_samples += trailing_dropped;
                }
                let result = writer
                    .finalize()
                    .map(|()| WriterSummary {
                        samples,
                        dropped_samples,
                    })
                    .map_err(|error| error.to_string());
                if reply.send(result).is_err() {
                    set_failure(
                        failure,
                        "WAV_WRITER_FAILED",
                        "WAV 완료 응답 수신자가 종료되었습니다.".to_owned(),
                    );
                }
                return;
            }
            WriterCommand::Cancel(reply) => {
                drop(writer);
                if reply.send(()).is_err() {
                    set_failure(
                        failure,
                        "WAV_WRITER_FAILED",
                        "WAV 취소 응답 수신자가 종료되었습니다.".to_owned(),
                    );
                }
                return;
            }
        }
    }
    set_failure(
        failure,
        "WAV_WRITER_FAILED",
        "WAV 명령 채널이 예기치 않게 종료되었습니다.".to_owned(),
    );
}

#[cfg(test)]
#[path = "writer_tests.rs"]
mod tests;
