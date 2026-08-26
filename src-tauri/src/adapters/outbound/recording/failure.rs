use crate::application::error::AppError;
use crate::application::model::RecordingFailure;
use crate::application::ports::RecordingEvents;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub struct FailureState {
    recording_id: Uuid,
    /// Mirrors `value.is_some()` so the audio callback can check for a failure
    /// with one relaxed load instead of taking a lock on the realtime thread.
    failed: AtomicBool,
    value: Mutex<Option<(String, String)>>,
    events: Arc<dyn RecordingEvents>,
}

pub type SharedFailure = Arc<FailureState>;

pub fn new(recording_id: Uuid, events: Arc<dyn RecordingEvents>) -> SharedFailure {
    Arc::new(FailureState {
        recording_id,
        failed: AtomicBool::new(false),
        value: Mutex::new(None),
        events,
    })
}

/// Record a failure and tell the window about it.
///
/// Only callers off the audio thread may use this: emitting serializes JSON and
/// crosses the IPC boundary. The audio callback uses [`record_failure`].
pub fn set_failure(failure: &SharedFailure, code: &str, message: String) {
    if !store(failure, code, &message) {
        return;
    }
    if let Err(error) = failure.events.emit_failure(RecordingFailure {
        recording_id: failure.recording_id,
        code: code.to_owned(),
        message,
    }) {
        eprintln!("recording event delivery failed: {error}");
    }
}

/// Record a failure raised on the audio thread, without emitting.
///
/// The writer thread raises and emits its own failures, so the callback only
/// needs the state to be visible; `stop` surfaces it through `take_failure`.
pub fn record_failure(failure: &SharedFailure, code: &str, message: &str) {
    let _stored = store(failure, code, message);
}

fn store(failure: &SharedFailure, code: &str, message: &str) -> bool {
    let mut slot = failure
        .value
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if slot.is_some() {
        return false;
    }
    *slot = Some((code.to_owned(), message.to_owned()));
    failure.failed.store(true, Ordering::Release);
    true
}

pub fn has_failure(failure: &SharedFailure) -> bool {
    failure.failed.load(Ordering::Acquire)
}

pub fn take_failure(failure: &SharedFailure) -> Result<Option<(String, String)>, AppError> {
    let taken = failure
        .value
        .lock()
        .map_err(|_| AppError::new("RECORDING_ERROR", "녹음 오류 잠금이 손상되었습니다."))?
        .take();
    failure.failed.store(false, Ordering::Release);
    Ok(taken)
}
