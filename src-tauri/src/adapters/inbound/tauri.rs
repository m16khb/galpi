use crate::application::error::AppError;
use crate::application::model::{
    EnvironmentStatus, RecordingFailure, RecordingResult, RecordingStatus, SetupResult,
    TranscriptionResult,
};
use crate::application::ports::{JobEvents, RecordingEvents};
use crate::application::use_cases::Application;
use crate::domain::artifact::ArtifactKind;
use crate::domain::job::{SetupRequest, TranscriptionRequest};
use crate::domain::worker::WorkerEvent;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

#[tauri::command]
pub async fn diagnose_environment(
    application: State<'_, Application>,
) -> Result<EnvironmentStatus, AppError> {
    application.diagnose().await
}

#[tauri::command]
pub async fn prepare_environment(
    application: State<'_, Application>,
    request: SetupRequest,
) -> Result<SetupResult, AppError> {
    application.prepare(request).await
}

#[tauri::command]
pub async fn load_hugging_face_token(
    application: State<'_, Application>,
) -> Result<Option<String>, AppError> {
    application.load_hugging_face_token().await
}

#[tauri::command]
pub async fn save_hugging_face_token(
    application: State<'_, Application>,
    token: String,
) -> Result<(), AppError> {
    application.save_hugging_face_token(token).await
}

#[tauri::command]
pub async fn start_transcription(
    application: State<'_, Application>,
    request: TranscriptionRequest,
) -> Result<TranscriptionResult, AppError> {
    application.transcribe(request).await
}

#[tauri::command]
pub async fn cancel_job(application: State<'_, Application>, job_id: Uuid) -> Result<(), AppError> {
    tokio::task::yield_now().await;
    application.cancel(job_id)
}

#[tauri::command]
pub async fn open_artifact(
    application: State<'_, Application>,
    job_id: Uuid,
    kind: ArtifactKind,
) -> Result<(), AppError> {
    tokio::task::yield_now().await;
    application.open_artifact(job_id, kind)
}

#[tauri::command]
pub async fn reveal_output_directory(
    application: State<'_, Application>,
    job_id: Uuid,
) -> Result<(), AppError> {
    tokio::task::yield_now().await;
    application.reveal_output(job_id)
}

#[tauri::command]
pub async fn start_recording(
    application: State<'_, Application>,
    output_root: String,
) -> Result<RecordingStatus, AppError> {
    application.start_recording(output_root).await
}

#[tauri::command]
pub async fn stop_recording(
    application: State<'_, Application>,
    recording_id: Uuid,
) -> Result<RecordingResult, AppError> {
    application.stop_recording(recording_id).await
}

#[tauri::command]
pub async fn cancel_recording(
    application: State<'_, Application>,
    recording_id: Uuid,
) -> Result<(), AppError> {
    application.cancel_recording(recording_id).await
}

#[derive(Debug, Clone)]
pub struct TauriEvents {
    app: AppHandle,
}

impl TauriEvents {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JobEvent {
    job_id: Uuid,
    #[serde(flatten)]
    event: WorkerEvent,
}

impl JobEvents for TauriEvents {
    fn emit(&self, job_id: Uuid, event: WorkerEvent) -> Result<(), AppError> {
        self.app
            .emit("job-event", JobEvent { job_id, event })
            .map_err(|error| AppError::new("EVENT_ERROR", error.to_string()))
    }
}

impl RecordingEvents for TauriEvents {
    fn emit_failure(&self, failure: RecordingFailure) -> Result<(), AppError> {
        self.app
            .emit("recording-event", failure)
            .map_err(|error| AppError::new("EVENT_ERROR", error.to_string()))
    }
}
