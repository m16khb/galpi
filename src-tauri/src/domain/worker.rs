use serde::{Deserialize, Serialize};
use thiserror::Error;

const PROTOCOL_VERSION: u8 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum WorkerEvent {
    Phase {
        phase: String,
        percent: f32,
        message: String,
    },
    Completed {
        srt: String,
        txt: String,
        checkpoint: String,
        segments: usize,
        filtered: usize,
    },
    Prepared {
        engine_version: String,
    },
    Refined {
        minutes: String,
    },
    Error {
        code: String,
        message: String,
    },
    Log {
        stream: String,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct WorkerEnvelope {
    pub v: u8,
    pub seq: u64,
    #[serde(flatten)]
    pub event: WorkerEvent,
}

#[derive(Debug, Error, PartialEq)]
pub enum ProtocolError {
    #[error("invalid worker event: {0}")]
    InvalidJson(String),
    #[error("unsupported worker protocol version {0}")]
    UnsupportedVersion(u8),
}

pub fn parse_worker_event(line: &str) -> Result<WorkerEnvelope, ProtocolError> {
    let envelope: WorkerEnvelope = serde_json::from_str(line)
        .map_err(|error| ProtocolError::InvalidJson(error.to_string()))?;
    if envelope.v != PROTOCOL_VERSION {
        return Err(ProtocolError::UnsupportedVersion(envelope.v));
    }
    Ok(envelope)
}

#[cfg(test)]
mod tests {
    use super::{ProtocolError, WorkerEvent, parse_worker_event};

    #[test]
    fn parses_phase_event_when_protocol_version_is_supported() -> Result<(), ProtocolError> {
        // Given
        let line = r#"{"v":1,"seq":7,"type":"phase","phase":"aligning","percent":42.5,"message":"정렬 중"}"#;

        // When
        let event = parse_worker_event(line)?;

        // Then
        assert_eq!(event.seq, 7);
        assert_eq!(
            event.event,
            WorkerEvent::Phase {
                phase: "aligning".to_owned(),
                percent: 42.5,
                message: "정렬 중".to_owned(),
            }
        );
        Ok(())
    }

    #[test]
    fn rejects_event_when_protocol_version_is_newer() {
        // Given
        let line = r#"{"v":2,"seq":1,"type":"phase","phase":"setup","percent":0,"message":"시작"}"#;

        // When / Then
        assert_eq!(
            parse_worker_event(line),
            Err(ProtocolError::UnsupportedVersion(2))
        );
    }

    #[test]
    fn rejects_event_when_json_is_malformed() {
        // Given
        let line = r#"{"v":1,"seq":"broken"}"#;

        // When
        // When / Then
        assert!(matches!(
            parse_worker_event(line),
            Err(ProtocolError::InvalidJson(_))
        ));
    }
}
