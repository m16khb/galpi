use super::failure::{SharedFailure, set_failure};
use crate::application::error::AppError;
use hound::{SampleFormat, WavSpec, WavWriter};
use std::fs::{File, OpenOptions};
use std::io::BufWriter;
use std::path::Path;
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::thread::JoinHandle;

const QUEUE_CAPACITY: usize = 32;
const RIFF_DATA_LIMIT: u64 = u32::MAX as u64 - 44;

pub enum WriterCommand {
    Samples(Vec<i16>),
    Finish(SyncSender<Result<WriterSummary, String>>),
    Cancel(SyncSender<()>),
}

pub struct WriterSummary {
    pub samples: u64,
}

pub struct WriterHandle {
    sender: SyncSender<WriterCommand>,
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
    let (sender, receiver) = sync_channel(QUEUE_CAPACITY);
    let thread = match std::thread::Builder::new()
        .name("galpi-wav-writer".to_owned())
        .spawn(move || run_writer(writer, &receiver, sample_rate, channels, &failure))
    {
        Ok(thread) => thread,
        Err(error) => {
            return Err(remove_after_spawn_failure(
                path,
                AppError::new("WAV_WRITER_FAILED", error.to_string()),
            ));
        }
    };
    Ok(WriterHandle { sender, thread })
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

    pub fn finish(self) -> Result<WriterSummary, AppError> {
        let (reply, receiver) = sync_channel(1);
        self.sender
            .send(WriterCommand::Finish(reply))
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

fn run_writer(
    mut writer: WavWriter<BufWriter<File>>,
    receiver: &Receiver<WriterCommand>,
    sample_rate: u32,
    channels: u16,
    failure: &SharedFailure,
) {
    let flush_every = u64::from(sample_rate) * u64::from(channels) * 5;
    let mut samples = 0_u64;
    let mut next_flush = flush_every;
    while let Ok(command) = receiver.recv() {
        match command {
            WriterCommand::Samples(chunk) => {
                if samples.saturating_add(chunk.len() as u64) * 2 > RIFF_DATA_LIMIT {
                    set_failure(
                        failure,
                        "WAV_TOO_LARGE",
                        "WAV 파일이 4GB 한도에 도달했습니다.".to_owned(),
                    );
                    continue;
                }
                for sample in chunk {
                    if let Err(error) = writer.write_sample(sample) {
                        set_failure(failure, "WAV_WRITE_FAILED", error.to_string());
                        break;
                    }
                    samples += 1;
                }
                if samples >= next_flush {
                    if let Err(error) = writer.flush() {
                        set_failure(failure, "WAV_WRITE_FAILED", error.to_string());
                    }
                    next_flush = samples.saturating_add(flush_every);
                }
            }
            WriterCommand::Finish(reply) => {
                let result = writer
                    .finalize()
                    .map(|()| WriterSummary { samples })
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
mod tests {
    use super::spawn;
    use crate::adapters::outbound::recording::failure;
    use crate::application::error::AppError;
    use crate::application::model::RecordingFailure;
    use crate::application::ports::RecordingEvents;
    use hound::WavReader;
    use std::sync::Arc;
    use uuid::Uuid;

    struct NoopEvents;

    impl RecordingEvents for NoopEvents {
        fn emit_failure(&self, _failure: RecordingFailure) -> Result<(), AppError> {
            Ok(())
        }
    }

    #[test]
    fn writes_exact_pcm_samples_and_header() -> Result<(), Box<dyn std::error::Error>> {
        let path = std::env::temp_dir().join(format!("galpi-writer-{}.wav", Uuid::now_v7()));
        let failure = failure::new(Uuid::now_v7(), Arc::new(NoopEvents));
        let writer = spawn(&path, 48_000, 2, failure)?;
        writer
            .sender()
            .send(super::WriterCommand::Samples(vec![-32_768, 0, 32_767, 1]))?;
        let summary = writer.finish()?;
        let mut reader = WavReader::open(&path)?;
        let spec = reader.spec();
        let samples = reader.samples::<i16>().collect::<Result<Vec<_>, _>>()?;
        std::fs::remove_file(path)?;

        assert_eq!(summary.samples, 4);
        assert_eq!(spec.channels, 2);
        assert_eq!(spec.sample_rate, 48_000);
        assert_eq!(samples, [-32_768, 0, 32_767, 1]);
        Ok(())
    }
}
