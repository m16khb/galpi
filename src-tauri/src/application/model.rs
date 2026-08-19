use crate::domain::artifact::Artifacts;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentStatus {
    pub engine_ready: bool,
    pub models_ready: bool,
    pub ffmpeg_ready: bool,
    pub data_directory: String,
    pub default_output_directory: String,
    pub engine_version: String,
}

impl EnvironmentStatus {
    pub fn is_ready(&self) -> bool {
        self.engine_ready && self.models_ready && self.ffmpeg_ready
    }
}

#[derive(Debug)]
pub struct CompletedTranscription {
    pub artifacts: Artifacts,
    pub segments: usize,
    pub filtered: usize,
}

/// Assistant credentials and the background context sent with every refinement.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantSettings {
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub background: Option<String>,
}

impl AssistantSettings {
    pub fn trimmed(self) -> Self {
        Self {
            api_key: keep_filled(self.api_key),
            model: keep_filled(self.model),
            background: keep_filled(self.background),
        }
    }
}

fn keep_filled(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefinementResult {
    pub job_id: Uuid,
    pub minutes: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupResult {
    pub job_id: Uuid,
    pub status: EnvironmentStatus,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionResult {
    pub job_id: Uuid,
    pub srt: String,
    pub txt: String,
    pub checkpoint: String,
    pub output_directory: String,
    pub segments: usize,
    pub filtered: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingStatus {
    pub recording_id: Uuid,
    pub path: String,
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingResult {
    pub recording_id: Uuid,
    pub path: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub frames: u64,
    pub duration_seconds: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingFailure {
    pub recording_id: Uuid,
    pub code: String,
    pub message: String,
}
