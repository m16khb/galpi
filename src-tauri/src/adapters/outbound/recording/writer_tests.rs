use super::{WriterCommand, spawn};
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
    writer.sender().send(WriterCommand::Samples {
        samples: vec![-32_768, 0, 32_767, 1],
        dropped_before: 0,
    })?;
    let summary = writer.finish(0)?;
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

#[test]
fn fills_dropped_samples_with_silence_and_reports_them() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let path = std::env::temp_dir().join(format!("galpi-writer-{}.wav", Uuid::now_v7()));
    let failure = failure::new(Uuid::now_v7(), Arc::new(NoopEvents));
    let writer = spawn(&path, 48_000, 1, failure)?;

    // When
    writer.sender().send(WriterCommand::Samples {
        samples: vec![1, 2],
        dropped_before: 2,
    })?;
    let summary = writer.finish(2)?;
    let mut reader = WavReader::open(&path)?;
    let samples = reader.samples::<i16>().collect::<Result<Vec<_>, _>>()?;
    std::fs::remove_file(path)?;

    // Then
    assert_eq!(summary.samples, 6);
    assert_eq!(summary.dropped_samples, 4);
    assert_eq!(samples, [0, 0, 1, 2, 0, 0]);
    Ok(())
}
