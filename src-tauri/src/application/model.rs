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

/// One saved meeting participant reused across meetings.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Participant {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub team: Option<String>,
    pub role: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
}

impl Participant {
    /// Drop a nameless entry; a participant without a name cannot label a speaker.
    fn trimmed(self) -> Option<Self> {
        let name = keep_filled(Some(self.name))?;
        Some(Self {
            id: self.id,
            name,
            team: keep_filled(self.team),
            role: keep_filled(self.role),
            description: keep_filled(self.description),
            aliases: self
                .aliases
                .into_iter()
                .filter_map(|alias| keep_filled(Some(alias)))
                .collect(),
        })
    }
}

/// One glossary term applied to every refinement to correct misheard words.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlossaryEntry {
    pub id: String,
    pub term: String,
    #[serde(default)]
    pub description: Option<String>,
}

impl GlossaryEntry {
    /// Drop a termless row; a glossary corrects terms, not prose.
    fn trimmed(self) -> Option<Self> {
        let term = keep_filled(Some(self.term))?;
        Some(Self {
            id: self.id,
            term,
            description: keep_filled(self.description),
        })
    }
}

/// Assistant credentials, background context, and the saved participant roster.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantSettings {
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub background: Option<String>,
    #[serde(default)]
    pub participants: Vec<Participant>,
    #[serde(default)]
    pub glossary: Vec<GlossaryEntry>,
}

impl AssistantSettings {
    pub fn trimmed(self) -> Self {
        Self {
            api_key: keep_filled(self.api_key),
            model: keep_filled(self.model),
            background: keep_filled(self.background),
            participants: self
                .participants
                .into_iter()
                .filter_map(Participant::trimmed)
                .collect(),
            glossary: self
                .glossary
                .into_iter()
                .filter_map(GlossaryEntry::trimmed)
                .collect(),
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
