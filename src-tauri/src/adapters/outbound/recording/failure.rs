use crate::application::error::AppError;
use crate::application::model::RecordingFailure;
use crate::application::ports::RecordingEvents;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub struct FailureState {
    recording_id: Uuid,
    value: Mutex<Option<(String, String)>>,
    events: Arc<dyn RecordingEvents>,
}

pub type SharedFailure = Arc<FailureState>;

pub fn new(recording_id: Uuid, events: Arc<dyn RecordingEvents>) -> SharedFailure {
    Arc::new(FailureState {
        recording_id,
        value: Mutex::new(None),
        events,
    })
}

pub fn set_failure(failure: &SharedFailure, code: &str, message: String) {
    let mut slot = failure
        .value
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if slot.is_none() {
        *slot = Some((code.to_owned(), message.clone()));
        drop(slot);
        if let Err(error) = failure.events.emit_failure(RecordingFailure {
            recording_id: failure.recording_id,
            code: code.to_owned(),
            message,
        }) {
            eprintln!("recording event delivery failed: {error}");
        }
    }
}

pub fn has_failure(failure: &SharedFailure) -> bool {
    failure
        .value
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .is_some()
}

pub fn take_failure(failure: &SharedFailure) -> Result<Option<(String, String)>, AppError> {
    Ok(failure
        .value
        .lock()
        .map_err(|_| AppError::new("RECORDING_ERROR", "녹음 오류 잠금이 손상되었습니다."))?
        .take())
}
