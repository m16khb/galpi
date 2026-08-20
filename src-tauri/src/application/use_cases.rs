use crate::application::error::AppError;
use crate::application::jobs::JobRegistry;
use crate::application::model::{
    AssistantSettings, EnvironmentStatus, Participant, RefinementResult, SetupResult,
    TranscriptionResult,
};
use crate::application::model::{RecordingResult, RecordingStatus};
use crate::application::ports::{
    ArtifactPort, EnginePort, RecordingPort, RefinementJob, RefinementPort, SettingsPort,
    TranscriptionPort,
};
use crate::domain::artifact::{ArtifactKind, minutes_path};
use crate::domain::job::{SetupRequest, TranscriptionRequest, validate_speaker_hint};
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

pub struct Application {
    engine: Arc<dyn EnginePort>,
    transcription: Arc<dyn TranscriptionPort>,
    artifacts: Arc<dyn ArtifactPort>,
    recording: Arc<dyn RecordingPort>,
    settings: Arc<dyn SettingsPort>,
    refinement: Arc<dyn RefinementPort>,
    jobs: JobRegistry,
    active_recording: tokio::sync::Mutex<Option<Uuid>>,
}

impl Application {
    pub fn new(
        engine: Arc<dyn EnginePort>,
        transcription: Arc<dyn TranscriptionPort>,
        artifacts: Arc<dyn ArtifactPort>,
        recording: Arc<dyn RecordingPort>,
        settings: Arc<dyn SettingsPort>,
        refinement: Arc<dyn RefinementPort>,
    ) -> Self {
        Self {
            engine,
            transcription,
            artifacts,
            recording,
            settings,
            refinement,
            jobs: JobRegistry::default(),
            active_recording: tokio::sync::Mutex::new(None),
        }
    }

    pub async fn diagnose(&self) -> Result<EnvironmentStatus, AppError> {
        self.engine.diagnose().await
    }

    pub async fn prepare(&self, request: SetupRequest) -> Result<SetupResult, AppError> {
        let request = if request.hugging_face_token.is_some() {
            request
        } else {
            SetupRequest {
                hugging_face_token: self.settings.load_hugging_face_token().await?,
            }
        };
        let (job_id, mut cancel) = self.jobs.claim()?;
        let result = self.engine.prepare(job_id, &mut cancel, &request).await;
        self.jobs.finish(job_id)?;
        result.map(|status| SetupResult { job_id, status })
    }

    pub async fn load_hugging_face_token(&self) -> Result<Option<String>, AppError> {
        self.settings.load_hugging_face_token().await
    }

    pub async fn save_hugging_face_token(&self, token: String) -> Result<(), AppError> {
        let token = token.trim();
        self.settings
            .save_hugging_face_token((!token.is_empty()).then(|| token.to_owned()))
            .await
    }

    pub async fn load_assistant_settings(&self) -> Result<AssistantSettings, AppError> {
        self.settings.load_assistant().await
    }

    pub async fn save_assistant_settings(
        &self,
        settings: AssistantSettings,
    ) -> Result<(), AppError> {
        self.settings.save_assistant(settings.trimmed()).await
    }

    /// Turn a completed transcript into meeting minutes with the configured assistant.
    ///
    /// `attendees` holds the roster ids selected for this meeting; an empty selection
    /// sends no participant context at all.
    pub async fn refine_transcript(
        &self,
        target: Uuid,
        attendees: &[String],
    ) -> Result<RefinementResult, AppError> {
        let artifacts = self.jobs.artifacts(target)?;
        let assistant = self.settings.load_assistant().await?.trimmed();
        let participants: Vec<Participant> = assistant
            .participants
            .iter()
            .filter(|participant| attendees.contains(&participant.id))
            .cloned()
            .collect();
        let api_key = assistant.api_key.ok_or_else(|| {
            AppError::new(
                "ASSISTANT_KEY_MISSING",
                "설정에서 z.ai 코딩 플랜 토큰을 먼저 저장해 주세요.",
            )
        })?;
        let output = minutes_path(&artifacts.txt);
        let (job_id, mut cancel) = self.jobs.claim()?;
        let result = self
            .refinement
            .refine(
                job_id,
                &mut cancel,
                RefinementJob {
                    transcript: &artifacts.txt,
                    output: &output,
                    background: assistant.background.as_deref(),
                    participants: &participants,
                    model: assistant.model.as_deref(),
                    api_key: &api_key,
                },
            )
            .await;
        self.jobs.finish(job_id)?;
        let minutes = result?;
        self.jobs.register_minutes(target, minutes.clone())?;
        Ok(RefinementResult {
            job_id,
            minutes: minutes.to_string_lossy().into_owned(),
        })
    }

    pub async fn transcribe(
        &self,
        request: TranscriptionRequest,
    ) -> Result<TranscriptionResult, AppError> {
        validate_speaker_hint(&request.speaker_hint)
            .map_err(|error| AppError::new("INVALID_SPEAKER_HINT", error.to_string()))?;
        let (job_id, mut cancel) = self.jobs.claim_with_id(request.job_id)?;
        let result = self.run_transcription(job_id, &mut cancel, &request).await;
        self.jobs.finish(job_id)?;
        result
    }

    async fn run_transcription(
        &self,
        job_id: Uuid,
        cancel: &mut tokio::sync::oneshot::Receiver<()>,
        request: &TranscriptionRequest,
    ) -> Result<TranscriptionResult, AppError> {
        if !self.engine.diagnose().await?.is_ready() {
            return Err(AppError::new(
                "SETUP_REQUIRED",
                "먼저 엔진과 모델 준비를 완료해 주세요.",
            ));
        }
        let (input, output) = self
            .transcription
            .prepare_job(
                Path::new(&request.input_path),
                Path::new(&request.output_root),
            )
            .await?;
        let result = self
            .transcription
            .transcribe(job_id, cancel, &input, &output, &request.speaker_hint)
            .await;
        let completed = result?;
        self.jobs.register(job_id, completed.artifacts.clone())?;
        Ok(TranscriptionResult {
            job_id,
            srt: completed.artifacts.srt.to_string_lossy().into_owned(),
            txt: completed.artifacts.txt.to_string_lossy().into_owned(),
            checkpoint: completed
                .artifacts
                .checkpoint
                .to_string_lossy()
                .into_owned(),
            output_directory: completed
                .artifacts
                .output_directory
                .to_string_lossy()
                .into_owned(),
            segments: completed.segments,
            filtered: completed.filtered,
        })
    }

    pub fn cancel(&self, id: Uuid) -> Result<(), AppError> {
        self.jobs.cancel(id)
    }

    pub fn open_artifact(&self, id: Uuid, kind: ArtifactKind) -> Result<(), AppError> {
        let artifacts = self.jobs.artifacts(id)?;
        let path = artifacts.path_for(kind).ok_or_else(|| {
            AppError::new("ARTIFACT_NOT_FOUND", "아직 만들어진 회의록이 없습니다.")
        })?;
        self.artifacts.open_file(path, &artifacts.output_directory)
    }

    pub fn reveal_output(&self, id: Uuid) -> Result<(), AppError> {
        let artifacts = self.jobs.artifacts(id)?;
        self.artifacts.open_directory(&artifacts.output_directory)
    }

    pub async fn start_recording(&self, output_root: String) -> Result<RecordingStatus, AppError> {
        let mut active = self.active_recording.lock().await;
        if active.is_some() {
            return Err(AppError::new(
                "RECORDING_BUSY",
                "이미 마이크 녹음이 진행 중입니다.",
            ));
        }
        let recording_id = Uuid::now_v7();
        let result = self
            .recording
            .start(recording_id, Path::new(&output_root))
            .await;
        if result.is_ok() {
            *active = Some(recording_id);
        }
        result
    }

    pub async fn stop_recording(&self, recording_id: Uuid) -> Result<RecordingResult, AppError> {
        let mut active = self.active_recording.lock().await;
        verify_recording_id(*active, recording_id)?;
        let result = self.recording.stop(recording_id).await;
        *active = None;
        result
    }

    pub async fn cancel_recording(&self, recording_id: Uuid) -> Result<(), AppError> {
        let mut active = self.active_recording.lock().await;
        verify_recording_id(*active, recording_id)?;
        let result = self.recording.cancel(recording_id).await;
        *active = None;
        result
    }
}

fn verify_recording_id(active: Option<Uuid>, requested: Uuid) -> Result<(), AppError> {
    match active {
        Some(id) if id == requested => Ok(()),
        Some(_) => Err(AppError::new(
            "RECORDING_ID_MISMATCH",
            "다른 녹음 세션은 정지하거나 취소할 수 없습니다.",
        )),
        None => Err(AppError::new(
            "RECORDING_NOT_ACTIVE",
            "진행 중인 마이크 녹음이 없습니다.",
        )),
    }
}
