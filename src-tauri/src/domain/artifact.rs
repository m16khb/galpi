use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Artifacts {
    pub srt: PathBuf,
    pub txt: PathBuf,
    pub checkpoint: PathBuf,
    pub minutes: Option<PathBuf>,
    pub output_directory: PathBuf,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    Srt,
    SpeakerText,
    Checkpoint,
    Minutes,
}

impl Artifacts {
    pub fn path_for(&self, kind: ArtifactKind) -> Option<&PathBuf> {
        match kind {
            ArtifactKind::Srt => Some(&self.srt),
            ArtifactKind::SpeakerText => Some(&self.txt),
            ArtifactKind::Checkpoint => Some(&self.checkpoint),
            ArtifactKind::Minutes => self.minutes.as_ref(),
        }
    }
}

/// Sibling meeting-minutes path for a speaker-labeled transcript.
pub fn minutes_path(transcript: &Path) -> PathBuf {
    let stem = transcript.file_stem().map_or_else(
        || "meeting".to_owned(),
        |stem| stem.to_string_lossy().into_owned(),
    );
    let base = stem.strip_suffix("_화자별").unwrap_or(stem.as_str());
    transcript.with_file_name(format!("{base}_회의록.md"))
}

#[cfg(test)]
mod tests {
    use super::minutes_path;
    use std::path::{Path, PathBuf};

    #[test]
    fn derives_minutes_name_from_the_speaker_transcript() {
        // Given
        let transcript = Path::new("/tmp/job/meeting_화자별.txt");

        // When
        let minutes = minutes_path(transcript);

        // Then
        assert_eq!(minutes, PathBuf::from("/tmp/job/meeting_회의록.md"));
    }

    #[test]
    fn keeps_the_stem_when_the_speaker_suffix_is_absent() {
        // Given
        let transcript = Path::new("/tmp/job/notes.txt");

        // When
        let minutes = minutes_path(transcript);

        // Then
        assert_eq!(minutes, PathBuf::from("/tmp/job/notes_회의록.md"));
    }
}
