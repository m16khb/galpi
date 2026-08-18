use super::failure::{SharedFailure, has_failure, set_failure};
use super::writer::WriterCommand;
use crate::application::error::AppError;
use cpal::traits::DeviceTrait;
use cpal::{
    Device, ErrorKind, FromSample, I24, Sample, SampleFormat, SizedSample, Stream, StreamConfig,
    U24,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{SyncSender, TrySendError};

const MAX_FRAMES_PER_CHUNK: usize = 4_096;

pub fn build(
    device: &Device,
    config: &StreamConfig,
    format: SampleFormat,
    sender: SyncSender<WriterCommand>,
    failure: SharedFailure,
    samples: Arc<AtomicU64>,
    channels: u16,
) -> Result<Stream, AppError> {
    let capture = Capture {
        device,
        config,
        sender,
        failure,
        samples,
        channels,
    };
    match format {
        SampleFormat::I8 => capture.typed::<i8>(convert::<i8>),
        SampleFormat::I16 => capture.typed::<i16>(|sample| sample),
        SampleFormat::I24 => capture.typed::<I24>(convert::<I24>),
        SampleFormat::I32 => capture.typed::<i32>(convert::<i32>),
        SampleFormat::I64 => capture.typed::<i64>(convert::<i64>),
        SampleFormat::U8 => capture.typed::<u8>(convert::<u8>),
        SampleFormat::U16 => capture.typed::<u16>(convert::<u16>),
        SampleFormat::U24 => capture.typed::<U24>(convert::<U24>),
        SampleFormat::U32 => capture.typed::<u32>(convert::<u32>),
        SampleFormat::U64 => capture.typed::<u64>(convert::<u64>),
        SampleFormat::F32 => capture.typed::<f32>(f32_to_i16),
        SampleFormat::F64 => capture.typed::<f64>(f64_to_i16),
        unsupported => Err(AppError::new(
            "UNSUPPORTED_AUDIO_CONFIG",
            format!("지원하지 않는 마이크 샘플 형식입니다: {unsupported}"),
        )),
    }
}

struct Capture<'a> {
    device: &'a Device,
    config: &'a StreamConfig,
    sender: SyncSender<WriterCommand>,
    failure: SharedFailure,
    samples: Arc<AtomicU64>,
    channels: u16,
}

impl Capture<'_> {
    fn typed<T>(self, convert: fn(T) -> i16) -> Result<Stream, AppError>
    where
        T: SizedSample + Copy + Send + 'static,
    {
        let callback_failure = self.failure.clone();
        self.device
            .build_input_stream(
                *self.config,
                move |input: &[T], _| {
                    enqueue(
                        input,
                        &self.sender,
                        &self.failure,
                        &self.samples,
                        self.channels,
                        convert,
                    );
                },
                move |error| {
                    if is_recoverable_stream_error(error.kind()) {
                        return;
                    }
                    set_failure(
                        &callback_failure,
                        "MICROPHONE_DISCONNECTED",
                        error.to_string(),
                    );
                },
                None,
            )
            .map_err(|error| AppError::new("MICROPHONE_ERROR", error.to_string()))
    }
}

pub(super) fn is_recoverable_stream_error(kind: ErrorKind) -> bool {
    matches!(kind, ErrorKind::Xrun)
}

fn enqueue<T: Copy>(
    input: &[T],
    sender: &SyncSender<WriterCommand>,
    failure: &SharedFailure,
    samples: &AtomicU64,
    channels: u16,
    convert: fn(T) -> i16,
) {
    if has_failure(failure) {
        return;
    }
    let chunk_samples = MAX_FRAMES_PER_CHUNK * usize::from(channels);
    for input_chunk in input.chunks(chunk_samples) {
        let chunk = input_chunk.iter().copied().map(convert).collect();
        match sender.try_send(WriterCommand::Samples(chunk)) {
            Ok(()) => {
                samples.fetch_add(input_chunk.len() as u64, Ordering::Relaxed);
            }
            Err(TrySendError::Full(_)) => {
                set_failure(
                    failure,
                    "AUDIO_OVERRUN",
                    "디스크 기록이 마이크 입력을 따라가지 못했습니다.".to_owned(),
                );
                return;
            }
            Err(TrySendError::Disconnected(_)) => {
                set_failure(
                    failure,
                    "WAV_WRITER_FAILED",
                    "녹음 파일 writer가 예기치 않게 종료되었습니다.".to_owned(),
                );
                return;
            }
        }
    }
}

fn convert<T>(sample: T) -> i16
where
    i16: FromSample<T>,
{
    i16::from_sample(sample)
}

pub(super) fn f32_to_i16(sample: f32) -> i16 {
    float_to_i16(f64::from(sample))
}

pub(super) fn f64_to_i16(sample: f64) -> i16 {
    float_to_i16(sample)
}

fn float_to_i16(sample: f64) -> i16 {
    if !sample.is_finite() {
        0
    } else if sample <= -1.0 {
        i16::MIN
    } else if sample >= 1.0 {
        i16::MAX
    } else {
        i16::from_sample(sample)
    }
}

fn _assert_sample_types(_: I24, _: U24) {}
