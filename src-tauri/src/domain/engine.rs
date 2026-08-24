use serde::{Deserialize, Serialize};

/// Selectable transcription engine preset. `Qwen3` is the default; the legacy
/// `WhisperX` stack stays selectable as the previous engine set.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EnginePreset {
    #[default]
    Qwen3,
    WhisperX,
}

impl EnginePreset {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Qwen3 => "qwen3",
            Self::WhisperX => "whisperx",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::EnginePreset;

    #[test]
    fn defaults_to_qwen3_and_round_trips_wire_names() {
        // Given / When / Then: fresh installs start on the candidate set
        assert!(matches!(
            serde_json::to_value(EnginePreset::default()),
            Ok(value) if value == serde_json::json!("qwen3")
        ));
        let parsed: Result<EnginePreset, _> = serde_json::from_value(serde_json::json!("whisperx"));
        assert!(matches!(parsed, Ok(EnginePreset::WhisperX)));
    }

    #[test]
    fn rejects_unknown_preset_names() {
        let parsed: Result<EnginePreset, _> = serde_json::from_value(serde_json::json!("turbo"));
        assert!(parsed.is_err());
    }
}
