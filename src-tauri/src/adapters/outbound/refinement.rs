use super::environment::assistant_environment;
use super::paths::{AppPaths, worker_root};
use super::process::{ProcessSpec, run_process};
use crate::application::error::AppError;
use crate::application::model::Participant;
use crate::application::ports::{JobEvents, RefinementJob};
use crate::domain::worker::WorkerEvent;
use std::path::{Path, PathBuf};
use tauri::AppHandle;
use tokio::sync::oneshot;
use uuid::Uuid;

pub struct Runtime<'a> {
    pub app: &'a AppHandle,
    pub events: &'a dyn JobEvents,
    pub paths: &'a AppPaths,
}

pub async fn run(
    runtime: Runtime<'_>,
    job_id: Uuid,
    cancel: &mut oneshot::Receiver<()>,
    job: RefinementJob<'_>,
) -> Result<PathBuf, AppError> {
    let root = worker_root(runtime.app)?;
    let background = match job.background {
        Some(background) => Some(write_private_file(job_id, "background", background).await?),
        None => None,
    };
    let attendees = match job.participants {
        [] => None,
        participants => Some(
            write_private_file(job_id, "participants", &participants_json(participants)?).await?,
        ),
    };
    let result = run_worker(
        &runtime,
        job_id,
        cancel,
        &job,
        &root,
        background.as_deref(),
        attendees.as_deref(),
    )
    .await;
    for temporary in [background, attendees].into_iter().flatten() {
        let _removed = tokio::fs::remove_file(&temporary).await;
    }
    let minutes = result?;
    let directory = job.output.parent().ok_or_else(|| {
        AppError::new(
            "ARTIFACT_PATH_ERROR",
            "회의록 저장 폴더를 확인하지 못했습니다.",
        )
    })?;
    canonical_minutes(directory, &minutes).await
}

async fn run_worker(
    runtime: &Runtime<'_>,
    job_id: Uuid,
    cancel: &mut oneshot::Receiver<()>,
    job: &RefinementJob<'_>,
    root: &Path,
    background: Option<&Path>,
    attendees: Option<&Path>,
) -> Result<PathBuf, AppError> {
    let mut args = vec![
        "-m".into(),
        "galpi_worker".into(),
        "refine".into(),
        "--transcript".into(),
        job.transcript.as_os_str().to_owned(),
        "--output".into(),
        job.output.as_os_str().to_owned(),
    ];
    if let Some(background) = background {
        args.push("--background".into());
        args.push(background.as_os_str().to_owned());
    }
    if let Some(attendees) = attendees {
        args.push("--participants".into());
        args.push(attendees.as_os_str().to_owned());
    }
    if let Some(model) = job.model {
        args.push("--model".into());
        args.push(model.into());
    }

    let process = run_process(
        runtime.events,
        job_id,
        ProcessSpec {
            program: runtime.paths.python.clone(),
            current_dir: root.to_path_buf(),
            args,
            env: assistant_environment(runtime.paths, root, job.api_key),
            worker_protocol: true,
        },
        cancel,
    )
    .await?;
    match process.completed {
        Some(WorkerEvent::Refined { minutes }) => Ok(PathBuf::from(minutes)),
        _ => Err(AppError::new(
            "WORKER_PROTOCOL_ERROR",
            "워커가 회의록 결과를 전달하지 않았습니다.",
        )),
    }
}

fn participants_json(participants: &[Participant]) -> Result<String, AppError> {
    serde_json::to_string(participants).map_err(|error| {
        AppError::new(
            "SETTINGS_INVALID",
            format!("참석자 명단을 준비하지 못했습니다: {error}"),
        )
    })
}

/// Hand context to the worker through a 0600 temp file instead of the argument vector.
async fn write_private_file(job_id: Uuid, kind: &str, contents: &str) -> Result<PathBuf, AppError> {
    let path = std::env::temp_dir().join(format!("galpi-{kind}-{job_id}"));
    tokio::fs::write(&path, contents)
        .await
        .map_err(|error| AppError::io("사전 정보를 임시로 저장하지 못했습니다", &error))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .await
            .map_err(|error| AppError::io("사전 정보 파일 권한을 지정하지 못했습니다", &error))?;
    }
    Ok(path)
}

async fn canonical_minutes(directory: &Path, minutes: &Path) -> Result<PathBuf, AppError> {
    let root = tokio::fs::canonicalize(directory)
        .await
        .map_err(|error| AppError::io("작업 디렉터리를 확인하지 못했습니다", &error))?;
    let minutes = tokio::fs::canonicalize(minutes)
        .await
        .map_err(|error| AppError::io("회의록 파일을 확인하지 못했습니다", &error))?;
    if !minutes.starts_with(&root) {
        return Err(AppError::new(
            "WORKER_PROTOCOL_ERROR",
            "워커가 작업 디렉터리 밖의 회의록을 반환했습니다.",
        ));
    }
    Ok(minutes)
}
