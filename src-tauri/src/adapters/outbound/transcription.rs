use crate::adapters::outbound::environment::process_environment;
use crate::adapters::outbound::paths::{AppPaths, worker_root};
use crate::adapters::outbound::process::{ProcessSpec, run_process};
use crate::adapters::outbound::refinement::write_private_file;
use crate::application::error::AppError;
use crate::application::model::CompletedTranscription;
use crate::application::ports::JobEvents;
use crate::domain::artifact::Artifacts;
use crate::domain::job::SpeakerHint;
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
    input: &Path,
    output: &Path,
    hint: &SpeakerHint,
    asr_context: Option<&str>,
) -> Result<CompletedTranscription, AppError> {
    let context = match asr_context {
        Some(context) => Some(write_private_file(job_id, "asr-context", context).await?),
        None => None,
    };
    let result = run_worker(
        &runtime,
        job_id,
        cancel,
        input,
        output,
        hint,
        context.as_deref(),
    )
    .await;
    if let Some(temporary) = context {
        let _removed = tokio::fs::remove_file(&temporary).await;
    }
    result
}

async fn run_worker(
    runtime: &Runtime<'_>,
    job_id: Uuid,
    cancel: &mut oneshot::Receiver<()>,
    input: &Path,
    output: &Path,
    hint: &SpeakerHint,
    asr_context: Option<&Path>,
) -> Result<CompletedTranscription, AppError> {
    let root = worker_root(runtime.app)?;
    let mut args = vec![
        "-m".into(),
        "galpi_worker".into(),
        "transcribe".into(),
        "--input".into(),
        input.as_os_str().to_owned(),
        "--output".into(),
        output.as_os_str().to_owned(),
    ];
    if let Some(context) = asr_context {
        args.push("--asr-context".into());
        args.push(context.as_os_str().to_owned());
    }
    match hint {
        SpeakerHint::Auto => {}
        SpeakerHint::Exact { count } => {
            args.push("--num-speakers".into());
            args.push(count.to_string().into());
        }
        SpeakerHint::Range { min, max } => {
            args.push("--speaker-range".into());
            args.push(min.to_string().into());
            args.push(max.to_string().into());
        }
    }

    let process = run_process(
        runtime.events,
        job_id,
        ProcessSpec {
            program: runtime.paths.python.clone(),
            current_dir: root.clone(),
            args,
            env: process_environment(runtime.paths, &root, None),
            worker_protocol: true,
        },
        cancel,
    )
    .await?;
    let (srt, txt, checkpoint, segments, filtered) = match process.completed {
        Some(WorkerEvent::Completed {
            srt,
            txt,
            checkpoint,
            segments,
            filtered,
        }) => (
            PathBuf::from(srt),
            PathBuf::from(txt),
            PathBuf::from(checkpoint),
            segments,
            filtered,
        ),
        _ => {
            return Err(AppError::new(
                "WORKER_PROTOCOL_ERROR",
                "워커가 완료 결과를 전달하지 않았습니다.",
            ));
        }
    };
    let artifacts = validate_artifacts(output, &srt, &txt, &checkpoint).await?;
    Ok(CompletedTranscription {
        artifacts,
        segments,
        filtered,
    })
}

async fn validate_artifacts(
    job_directory: &Path,
    srt: &Path,
    txt: &Path,
    checkpoint: &Path,
) -> Result<Artifacts, AppError> {
    let root = tokio::fs::canonicalize(job_directory)
        .await
        .map_err(|error| AppError::io("작업 디렉터리를 확인하지 못했습니다", &error))?;
    let srt = canonical_artifact(&root, srt).await?;
    let txt = canonical_artifact(&root, txt).await?;
    let checkpoint = canonical_artifact(&root, checkpoint).await?;
    Ok(Artifacts {
        srt,
        txt,
        checkpoint,
        minutes: None,
        output_directory: root,
    })
}

async fn canonical_artifact(root: &Path, path: &Path) -> Result<PathBuf, AppError> {
    let path = tokio::fs::canonicalize(path)
        .await
        .map_err(|error| AppError::io("결과 파일을 확인하지 못했습니다", &error))?;
    if !path.starts_with(root) {
        return Err(AppError::new(
            "WORKER_PROTOCOL_ERROR",
            "워커가 작업 디렉터리 밖의 결과를 반환했습니다.",
        ));
    }
    Ok(path)
}
