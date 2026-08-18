use crate::application::error::AppError;
use crate::application::ports::JobEvents;
use crate::domain::worker::{WorkerEvent, parse_worker_event};
use nix::sys::signal::Signal;
use std::collections::HashMap;
use std::ffi::OsString;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::oneshot;
use uuid::Uuid;

mod guard;

use guard::ProcessGroupGuard;

const MAX_LINE_BYTES: usize = 64 * 1024;

#[derive(Debug)]
pub struct ProcessSpec {
    pub program: PathBuf,
    pub current_dir: PathBuf,
    pub args: Vec<OsString>,
    pub env: HashMap<OsString, OsString>,
    pub worker_protocol: bool,
}

#[derive(Debug, Default)]
pub struct ProcessResult {
    pub completed: Option<WorkerEvent>,
}

pub async fn run_process(
    events: &dyn JobEvents,
    job_id: Uuid,
    spec: ProcessSpec,
    cancel: &mut oneshot::Receiver<()>,
) -> Result<ProcessResult, AppError> {
    let mut command = Command::new(&spec.program);
    command
        .args(&spec.args)
        .current_dir(&spec.current_dir)
        .env_clear()
        .envs(&spec.env)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)]
    {
        command.as_std_mut().process_group(0);
    }

    let mut child = command
        .spawn()
        .map_err(|error| AppError::io("프로세스를 시작하지 못했습니다", &error))?;
    let process_id = child
        .id()
        .ok_or_else(|| AppError::new("PROCESS_ERROR", "프로세스 ID를 확인하지 못했습니다."))?;
    let mut process_guard = ProcessGroupGuard::new(process_id);
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AppError::new("PROCESS_ERROR", "stdout을 연결하지 못했습니다."))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| AppError::new("PROCESS_ERROR", "stderr를 연결하지 못했습니다."))?;
    let mut stdout = BufReader::new(stdout);
    let mut stderr = BufReader::new(stderr);
    let mut stdout_buffer = Vec::new();
    let mut stderr_buffer = Vec::new();
    let mut stdout_open = true;
    let mut stderr_open = true;
    let mut result = ProcessResult::default();
    let mut error_tail = Vec::new();

    while stdout_open || stderr_open {
        tokio::select! {
            line = read_bounded_line(&mut stdout, &mut stdout_buffer), if stdout_open => {
                match line? {
                    Some(line) => handle_stdout(events, job_id, &line, spec.worker_protocol, &mut result)?,
                    None => stdout_open = false,
                }
            }
            line = read_bounded_line(&mut stderr, &mut stderr_buffer), if stderr_open => {
                match line? {
                    Some(line) => {
                        emit(events, job_id, WorkerEvent::Log {
                            stream: "stderr".to_owned(),
                            message: line.clone(),
                        })?;
                        error_tail.push(line);
                        if error_tail.len() > 20 {
                            error_tail.remove(0);
                        }
                    }
                    None => stderr_open = false,
                }
            }
            _ = &mut *cancel => {
                process_guard.terminate(Signal::SIGTERM)?;
                let wait = tokio::time::timeout(Duration::from_secs(3), child.wait()).await;
                if wait.is_err() {
                    process_guard.terminate(Signal::SIGKILL)?;
                    let _status = child.wait().await;
                }
                process_guard.disarm();
                return Err(AppError::new("CANCELLED", "사용자가 작업을 취소했습니다."));
            }
        }
    }

    let status = tokio::select! {
        status = child.wait() => {
            status.map_err(|error| AppError::io("프로세스 종료 상태를 확인하지 못했습니다", &error))?
        }
        _ = &mut *cancel => {
            process_guard.terminate(Signal::SIGTERM)?;
            let wait = tokio::time::timeout(Duration::from_secs(3), child.wait()).await;
            if wait.is_err() {
                process_guard.terminate(Signal::SIGKILL)?;
                let _status = child.wait().await;
            }
            process_guard.disarm();
            return Err(AppError::new("CANCELLED", "사용자가 작업을 취소했습니다."));
        }
    };
    process_guard.disarm();
    if !status.success() {
        let detail = error_tail
            .last()
            .cloned()
            .unwrap_or_else(|| format!("exit status {status}"));
        return Err(AppError::new("PROCESS_FAILED", detail));
    }
    Ok(result)
}

pub fn emit(events: &dyn JobEvents, job_id: Uuid, event: WorkerEvent) -> Result<(), AppError> {
    events.emit(job_id, event)
}

fn handle_stdout(
    events: &dyn JobEvents,
    job_id: Uuid,
    line: &str,
    worker_protocol: bool,
    result: &mut ProcessResult,
) -> Result<(), AppError> {
    if worker_protocol {
        let envelope = parse_worker_event(line)
            .map_err(|error| AppError::new("WORKER_PROTOCOL_ERROR", error.to_string()))?;
        if matches!(envelope.event, WorkerEvent::Completed { .. }) {
            if result.completed.is_some() {
                return Err(AppError::new(
                    "WORKER_PROTOCOL_ERROR",
                    "완료 이벤트가 두 번 전달되었습니다.",
                ));
            }
            result.completed = Some(envelope.event.clone());
        }
        emit(events, job_id, envelope.event)
    } else {
        emit(
            events,
            job_id,
            WorkerEvent::Log {
                stream: "stdout".to_owned(),
                message: line.to_owned(),
            },
        )
    }
}

async fn read_bounded_line<R>(
    reader: &mut R,
    buffer: &mut Vec<u8>,
) -> Result<Option<String>, AppError>
where
    R: AsyncBufRead + Unpin,
{
    buffer.clear();
    loop {
        let available = reader
            .fill_buf()
            .await
            .map_err(|error| AppError::io("프로세스 출력을 읽지 못했습니다", &error))?;
        if available.is_empty() {
            if buffer.is_empty() {
                return Ok(None);
            }
            break;
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let consumed = newline.map_or(available.len(), |index| index + 1);
        if buffer.len().saturating_add(consumed) > MAX_LINE_BYTES {
            return Err(AppError::new(
                "WORKER_PROTOCOL_ERROR",
                "워커 출력 한 줄이 허용 크기를 초과했습니다.",
            ));
        }
        buffer.extend_from_slice(&available[..consumed]);
        reader.consume(consumed);
        if newline.is_some() {
            break;
        }
    }
    if buffer.last() == Some(&b'\n') {
        buffer.pop();
    }
    if buffer.last() == Some(&b'\r') {
        buffer.pop();
    }
    String::from_utf8(buffer.clone())
        .map(Some)
        .map_err(|error| AppError::new("WORKER_PROTOCOL_ERROR", error.to_string()))
}

#[cfg(test)]
mod tests;
