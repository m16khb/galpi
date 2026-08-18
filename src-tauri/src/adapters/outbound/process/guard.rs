use crate::application::error::AppError;
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;

pub struct ProcessGroupGuard {
    process_id: u32,
    armed: bool,
}

impl ProcessGroupGuard {
    pub fn new(process_id: u32) -> Self {
        Self {
            process_id,
            armed: true,
        }
    }

    pub fn terminate(&self, signal: Signal) -> Result<(), AppError> {
        let raw_id = i32::try_from(self.process_id)
            .map_err(|_| AppError::new("PROCESS_ERROR", "프로세스 ID 범위를 벗어났습니다."))?;
        kill(Pid::from_raw(-raw_id), signal)
            .map_err(|error| AppError::new("PROCESS_ERROR", error.to_string()))
    }

    pub fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for ProcessGroupGuard {
    fn drop(&mut self) {
        if self.armed
            && let Err(error) = self.terminate(Signal::SIGKILL)
        {
            eprintln!("process group cleanup failed: {error}");
        }
    }
}
