use super::{FakePort, TranscriptionBehavior};
use crate::application::error::AppError;
use crate::application::model::{RecordingResult, RecordingStatus};
use crate::application::ports::RecordingPort;
use async_trait::async_trait;
use std::path::Path;
use uuid::Uuid;

#[async_trait]
impl RecordingPort for FakePort {
    async fn start(
        &self,
        recording_id: Uuid,
        output_root: &Path,
    ) -> Result<RecordingStatus, AppError> {
        Ok(RecordingStatus {
            recording_id,
            path: output_root
                .join("recording.wav")
                .to_string_lossy()
                .into_owned(),
            sample_rate: 48_000,
            channels: 1,
        })
    }

    async fn stop(&self, recording_id: Uuid) -> Result<RecordingResult, AppError> {
        Ok(RecordingResult {
            recording_id,
            path: "/tmp/recording.wav".to_owned(),
            sample_rate: 48_000,
            channels: 1,
            frames: 48_000,
            duration_seconds: 1.0,
        })
    }

    async fn cancel(&self, _recording_id: Uuid) -> Result<(), AppError> {
        Ok(())
    }
}

#[tokio::test]
async fn recording_lifecycle_rejects_reentry_and_wrong_session() -> Result<(), AppError> {
    let app = std::sync::Arc::new(FakePort::new(TranscriptionBehavior::Success)).application();
    let started = app.start_recording("/tmp".to_owned()).await?;

    let busy = app.start_recording("/tmp".to_owned()).await;
    let Err(busy) = busy else {
        return Err(AppError::new(
            "TEST_ERROR",
            "second recording unexpectedly started",
        ));
    };
    assert_eq!(busy.code, "RECORDING_BUSY");

    let mismatch = app.stop_recording(Uuid::now_v7()).await;
    let Err(mismatch) = mismatch else {
        return Err(AppError::new(
            "TEST_ERROR",
            "wrong recording id unexpectedly stopped",
        ));
    };
    assert_eq!(mismatch.code, "RECORDING_ID_MISMATCH");

    let stopped = app.stop_recording(started.recording_id).await?;
    assert_eq!(stopped.recording_id, started.recording_id);
    let restarted = app.start_recording("/tmp".to_owned()).await?;
    app.cancel_recording(restarted.recording_id).await
}
