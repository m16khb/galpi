use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Artifacts {
    pub srt: PathBuf,
    pub txt: PathBuf,
    pub checkpoint: PathBuf,
    pub output_directory: PathBuf,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    Srt,
    SpeakerText,
    Checkpoint,
}

impl Artifacts {
    pub fn path_for(&self, kind: ArtifactKind) -> &PathBuf {
        match kind {
            ArtifactKind::Srt => &self.srt,
            ArtifactKind::SpeakerText => &self.txt,
            ArtifactKind::Checkpoint => &self.checkpoint,
        }
    }
}
