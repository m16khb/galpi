use super::error::AppError;
use super::model::{CompletedTranscription, EnvironmentStatus};
use super::ports::{ArtifactPort, EnginePort, RecordingPort, SettingsPort, TranscriptionPort};
use super::use_cases::Application;
use crate::domain::artifact::{ArtifactKind, Artifacts};
use crate::domain::job::{SetupRequest, SpeakerHint, TranscriptionRequest};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

enum TranscriptionBehavior {
    Success,
    Failure,
    Blocking(Mutex<Option<mpsc::UnboundedSender<Uuid>>>),
}

struct FakePort {
    prepare_calls: AtomicUsize,
    opened_paths: Mutex<Vec<PathBuf>>,
    prepared_token: Mutex<Option<String>>,
    settings_token: Mutex<Option<String>>,
    behavior: TranscriptionBehavior,
}

impl FakePort {
    fn new(behavior: TranscriptionBehavior) -> Self {
        Self {
            prepare_calls: AtomicUsize::new(0),
            opened_paths: Mutex::new(Vec::new()),
            prepared_token: Mutex::new(None),
            settings_token: Mutex::new(None),
            behavior,
        }
    }

    fn application(self: &Arc<Self>) -> Application {
        let engine: Arc<dyn EnginePort> = self.clone();
        let transcription: Arc<dyn TranscriptionPort> = self.clone();
        let artifacts: Arc<dyn ArtifactPort> = self.clone();
        let recording: Arc<dyn RecordingPort> = self.clone();
        let settings: Arc<dyn SettingsPort> = self.clone();
        Application::new(engine, transcription, artifacts, recording, settings)
    }
}

#[async_trait]
impl EnginePort for FakePort {
    async fn diagnose(&self) -> Result<EnvironmentStatus, AppError> {
        Ok(EnvironmentStatus {
            engine_ready: true,
            models_ready: true,
            ffmpeg_ready: true,
            data_directory: "/tmp/galpi-test".to_owned(),
            default_output_directory: "/tmp/output".to_owned(),
            engine_version: "test".to_owned(),
        })
    }

    async fn prepare(
        &self,
        job_id: Uuid,
        _cancel: &mut oneshot::Receiver<()>,
        request: &SetupRequest,
    ) -> Result<EnvironmentStatus, AppError> {
        let _ = job_id;
        *self
            .prepared_token
            .lock()
            .map_err(|_| AppError::new("TEST_ERROR", "prepared token lock poisoned"))? =
            request.hugging_face_token.clone();
        self.diagnose().await
    }
}

#[async_trait]
impl SettingsPort for FakePort {
    async fn load_hugging_face_token(&self) -> Result<Option<String>, AppError> {
        self.settings_token
            .lock()
            .map(|token| token.clone())
            .map_err(|_| AppError::new("TEST_ERROR", "settings token lock poisoned"))
    }

    async fn save_hugging_face_token(&self, token: Option<String>) -> Result<(), AppError> {
        *self
            .settings_token
            .lock()
            .map_err(|_| AppError::new("TEST_ERROR", "settings token lock poisoned"))? = token;
        Ok(())
    }
}

#[async_trait]
impl TranscriptionPort for FakePort {
    async fn prepare_job(
        &self,
        input: &Path,
        output_root: &Path,
    ) -> Result<(PathBuf, PathBuf), AppError> {
        self.prepare_calls.fetch_add(1, Ordering::SeqCst);
        Ok((input.to_owned(), output_root.join("job")))
    }

    async fn transcribe(
        &self,
        job_id: Uuid,
        cancel: &mut oneshot::Receiver<()>,
        _input: &Path,
        output: &Path,
        _hint: &SpeakerHint,
    ) -> Result<CompletedTranscription, AppError> {
        match &self.behavior {
            TranscriptionBehavior::Success => Ok(completion(output)),
            TranscriptionBehavior::Failure => {
                Err(AppError::new("TRANSCRIBE_FAILED", "expected test failure"))
            }
            TranscriptionBehavior::Blocking(started) => {
                let sender = started
                    .lock()
                    .map_err(|_| AppError::new("TEST_ERROR", "sender lock poisoned"))?
                    .take()
                    .ok_or_else(|| AppError::new("TEST_ERROR", "sender missing"))?;
                sender
                    .send(job_id)
                    .map_err(|_| AppError::new("TEST_ERROR", "receiver closed"))?;
                let _cancelled = cancel.await;
                Err(AppError::new("CANCELLED", "cancelled by test"))
            }
        }
    }
}

impl ArtifactPort for FakePort {
    fn open_file(&self, path: &Path, _trusted_root: &Path) -> Result<(), AppError> {
        self.opened_paths
            .lock()
            .map_err(|_| AppError::new("TEST_ERROR", "opener lock poisoned"))?
            .push(path.to_owned());
        Ok(())
    }

    fn open_directory(&self, path: &Path) -> Result<(), AppError> {
        self.opened_paths
            .lock()
            .map_err(|_| AppError::new("TEST_ERROR", "opener lock poisoned"))?
            .push(path.to_owned());
        Ok(())
    }
}

#[tokio::test]
async fn saved_hugging_face_token_is_trimmed_and_can_be_cleared() -> Result<(), AppError> {
    let port = Arc::new(FakePort::new(TranscriptionBehavior::Success));
    let app = port.application();

    app.save_hugging_face_token("  hf_saved  ".to_owned())
        .await?;
    assert_eq!(
        app.load_hugging_face_token().await?,
        Some("hf_saved".to_owned())
    );

    app.save_hugging_face_token("   ".to_owned()).await?;
    assert_eq!(app.load_hugging_face_token().await?, None);
    Ok(())
}

#[tokio::test]
async fn prepare_uses_saved_hugging_face_token() -> Result<(), AppError> {
    let port = Arc::new(FakePort::new(TranscriptionBehavior::Success));
    let app = port.application();
    app.save_hugging_face_token("hf_saved".to_owned()).await?;

    app.prepare(SetupRequest {
        hugging_face_token: None,
    })
    .await?;

    let token = port
        .prepared_token
        .lock()
        .map_err(|_| AppError::new("TEST_ERROR", "prepared token lock poisoned"))?;
    assert_eq!(token.as_deref(), Some("hf_saved"));
    Ok(())
}

#[tokio::test]
async fn invalid_hint_is_rejected_before_workspace_access() -> Result<(), AppError> {
    let port = Arc::new(FakePort::new(TranscriptionBehavior::Success));
    let app = port.application();

    let result = app
        .transcribe(request(SpeakerHint::Exact { count: 0 }))
        .await;

    let Err(error) = result else {
        return Err(AppError::new("TEST_ERROR", "invalid hint was accepted"));
    };
    assert_eq!(error.code, "INVALID_SPEAKER_HINT");
    assert_eq!(port.prepare_calls.load(Ordering::SeqCst), 0);
    Ok(())
}

#[tokio::test]
async fn failed_transcription_releases_active_job() -> Result<(), AppError> {
    let port = Arc::new(FakePort::new(TranscriptionBehavior::Failure));
    let app = port.application();

    for _attempt in 0..2 {
        let result = app.transcribe(request(SpeakerHint::Auto)).await;
        let Err(error) = result else {
            return Err(AppError::new(
                "TEST_ERROR",
                "transcription unexpectedly passed",
            ));
        };
        assert_eq!(error.code, "TRANSCRIBE_FAILED");
    }
    Ok(())
}

#[tokio::test]
async fn completed_artifact_is_opened_from_registry() -> Result<(), AppError> {
    let port = Arc::new(FakePort::new(TranscriptionBehavior::Success));
    let app = port.application();
    let result = app.transcribe(request(SpeakerHint::Auto)).await?;

    app.open_artifact(result.job_id, ArtifactKind::SpeakerText)?;

    let opened = port
        .opened_paths
        .lock()
        .map_err(|_| AppError::new("TEST_ERROR", "opener lock poisoned"))?;
    assert_eq!(
        opened.as_slice(),
        [PathBuf::from("/tmp/output/job/meeting_화자별.txt")]
    );
    Ok(())
}

#[tokio::test]
async fn cancellation_reaches_running_port_without_timing_waits()
-> Result<(), Box<dyn std::error::Error>> {
    let (started_sender, mut started_receiver) = mpsc::unbounded_channel();
    let port = Arc::new(FakePort::new(TranscriptionBehavior::Blocking(Mutex::new(
        Some(started_sender),
    ))));
    let app = Arc::new(port.application());
    let running = {
        let app = app.clone();
        tokio::spawn(async move { app.transcribe(request(SpeakerHint::Auto)).await })
    };
    let job_id = started_receiver
        .recv()
        .await
        .ok_or("worker did not announce start")?;

    app.cancel(job_id)?;
    let result = running.await?;

    let Err(error) = result else {
        return Err("cancelled transcription unexpectedly passed".into());
    };
    assert_eq!(error.code, "CANCELLED");
    Ok(())
}

fn request(speaker_hint: SpeakerHint) -> TranscriptionRequest {
    TranscriptionRequest {
        job_id: Uuid::now_v7(),
        input_path: "/tmp/input.m4a".to_owned(),
        output_root: "/tmp/output".to_owned(),
        speaker_hint,
    }
}

fn completion(output: &Path) -> CompletedTranscription {
    CompletedTranscription {
        artifacts: Artifacts {
            srt: output.join("meeting.srt"),
            txt: output.join("meeting_화자별.txt"),
            checkpoint: output.join("meeting.aligned.v2.json"),
            output_directory: output.to_owned(),
        },
        segments: 8,
        filtered: 1,
    }
}

mod recording;
