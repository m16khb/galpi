use super::writer::WriterHandle;
use crate::application::error::AppError;
use cpal::ErrorKind;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

impl Drop for super::NativeRecorder {
    fn drop(&mut self) {
        if Arc::strong_count(&self.active) != 1 {
            return;
        }
        let recording = self.active.lock().ok().and_then(|mut active| active.take());
        if let Some(recording) = recording {
            drop(recording.stream);
            let writer_result = recording.writer.cancel();
            let remove_result = remove_partial(&recording.partial_path, &recording.folder);
            if let Err(error) = writer_result.and(remove_result) {
                eprintln!("recording shutdown cleanup failed: {error}");
            }
        }
    }
}

pub fn state_lock<T>(state: &Mutex<T>) -> Result<MutexGuard<'_, T>, AppError> {
    state
        .lock()
        .map_err(|_| AppError::new("STATE_ERROR", "녹음 상태 잠금이 손상되었습니다."))
}

pub fn microphone_error(error: &cpal::Error) -> AppError {
    let code = match error.kind() {
        ErrorKind::PermissionDenied => "MICROPHONE_PERMISSION_DENIED",
        ErrorKind::DeviceNotAvailable => "MICROPHONE_UNAVAILABLE",
        ErrorKind::DeviceBusy => "MICROPHONE_BUSY",
        ErrorKind::UnsupportedConfig => "UNSUPPORTED_AUDIO_CONFIG",
        _ => "MICROPHONE_ERROR",
    };
    AppError::new(code, error.to_string())
}

pub fn remove_if_exists(path: &Path) -> Result<(), AppError> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(AppError::io("녹음 파일을 지우지 못했습니다", &error)),
    }
}

pub fn cancel_and_remove(writer: WriterHandle, path: &Path, folder: &Path) -> Result<(), AppError> {
    let writer_result = writer.cancel();
    let remove_result = remove_partial(path, folder);
    writer_result.and(remove_result)
}

/// Remove the partial recording and, when it is left empty, its meeting folder.
pub fn remove_partial(path: &Path, folder: &Path) -> Result<(), AppError> {
    let file = remove_if_exists(path);
    let _ = std::fs::remove_dir(folder);
    file
}

pub fn with_cleanup(error: AppError, cleanup: Result<(), AppError>) -> AppError {
    match cleanup {
        Ok(()) => error,
        Err(cleanup_error) => AppError::new(
            "RECORDING_CLEANUP_FAILED",
            format!("{error}; cleanup: {cleanup_error}"),
        ),
    }
}
