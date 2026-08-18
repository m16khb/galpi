pub use super::environment::diagnose;
use super::environment::{ENGINE_VERSION, process_environment, status};
use super::model_cache::{can_use_offline_cache, import_standard_cache};
use crate::adapters::outbound::paths::{AppPaths, uv_binary, worker_root};
use crate::adapters::outbound::process::{ProcessSpec, emit, run_process};
use crate::application::error::AppError;
use crate::application::model::EnvironmentStatus;
use crate::application::ports::JobEvents;
use crate::domain::job::SetupRequest;
use crate::domain::worker::WorkerEvent;
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use tauri::AppHandle;
use tokio::sync::oneshot;
use uuid::Uuid;

pub async fn prepare(
    app: &AppHandle,
    events: &dyn JobEvents,
    job_id: Uuid,
    cancel: &mut oneshot::Receiver<()>,
    request: &SetupRequest,
) -> Result<EnvironmentStatus, AppError> {
    let paths = AppPaths::resolve(app)?;
    paths.create_directories().await?;
    let root = worker_root(app)?;
    let install_env = process_environment(&paths, &root, None);
    let mut model_env = process_environment(&paths, &root, request.hugging_face_token.as_deref());
    let current = status(&paths);
    if current.is_ready() {
        emit_phase(
            events,
            job_id,
            "ready",
            100.0,
            "엔진과 모델이 이미 준비되어 있습니다.",
        )?;
        return Ok(current);
    }

    if !current.engine_ready {
        install_engine(events, job_id, cancel, &paths, &root, &install_env).await?;
    }
    let imported = match import_standard_cache(&paths).await {
        Ok(imported) if imported > 0 => emit_phase(
            events,
            job_id,
            "models",
            45.0,
            "기존 WhisperX 모델 캐시를 앱 전용 공간에서 재사용합니다.",
        )
        .map(|()| imported)?,
        Ok(_) => 0,
        Err(error) => {
            emit(
                events,
                job_id,
                WorkerEvent::Log {
                    stream: "stderr".to_owned(),
                    message: error.to_string(),
                },
            )?;
            0
        }
    };
    if can_use_offline_cache(imported, request.hugging_face_token.as_deref()) {
        model_env.insert("HF_HUB_OFFLINE".into(), "1".into());
        model_env.insert("TRANSFORMERS_OFFLINE".into(), "1".into());
    }
    let installed = status(&paths);
    if !installed.models_ready || !installed.ffmpeg_ready {
        prepare_models(events, job_id, cancel, &paths, &root, &model_env).await?;
    }

    let completed = status(&paths);
    if !completed.is_ready() {
        return Err(AppError::new(
            "SETUP_INCOMPLETE",
            "준비 프로세스가 종료됐지만 필수 파일이 확인되지 않았습니다.",
        ));
    }
    Ok(completed)
}

async fn install_engine(
    events: &dyn JobEvents,
    job_id: Uuid,
    cancel: &mut oneshot::Receiver<()>,
    paths: &AppPaths,
    root: &Path,
    env: &HashMap<OsString, OsString>,
) -> Result<(), AppError> {
    let uv = uv_binary()?;
    emit_phase(
        events,
        job_id,
        "engine",
        5.0,
        "Python 3.12 런타임을 준비합니다.",
    )?;
    run_raw(
        events,
        job_id,
        cancel,
        &uv,
        os_args(["python", "install", "3.12"]),
        env,
    )
    .await?;

    emit_phase(
        events,
        job_id,
        "engine",
        22.0,
        "앱 전용 가상환경을 만듭니다.",
    )?;
    run_raw(
        events,
        job_id,
        cancel,
        &uv,
        vec![
            "venv".into(),
            "--python".into(),
            "3.12".into(),
            paths.engine.join(".venv").into_os_string(),
        ],
        env,
    )
    .await?;

    emit_phase(
        events,
        job_id,
        "engine",
        35.0,
        "WhisperX와 내장 ffmpeg를 설치합니다.",
    )?;
    run_raw(
        events,
        job_id,
        cancel,
        &uv,
        vec![
            "pip".into(),
            "install".into(),
            "--python".into(),
            paths.python.clone().into_os_string(),
            "-r".into(),
            root.join("requirements.txt").into_os_string(),
        ],
        env,
    )
    .await?;
    tokio::fs::write(&paths.engine_manifest, ENGINE_VERSION)
        .await
        .map_err(|error| AppError::io("엔진 준비 마커를 쓰지 못했습니다", &error))
}

async fn prepare_models(
    events: &dyn JobEvents,
    job_id: Uuid,
    cancel: &mut oneshot::Receiver<()>,
    paths: &AppPaths,
    root: &Path,
    env: &HashMap<OsString, OsString>,
) -> Result<(), AppError> {
    emit_phase(
        events,
        job_id,
        "models",
        50.0,
        "전사·정렬·화자분리 모델을 확인합니다.",
    )?;
    run_process(
        events,
        job_id,
        ProcessSpec {
            program: paths.python.clone(),
            current_dir: root.to_owned(),
            args: vec![
                "-m".into(),
                "galpi_worker".into(),
                "prepare".into(),
                "--manifest".into(),
                paths.models_manifest.clone().into_os_string(),
                "--engine-bin".into(),
                paths.engine_bin.clone().into_os_string(),
            ],
            env: env.clone(),
            worker_protocol: true,
        },
        cancel,
    )
    .await?;
    Ok(())
}

async fn run_raw(
    events: &dyn JobEvents,
    job_id: Uuid,
    cancel: &mut oneshot::Receiver<()>,
    program: &Path,
    args: Vec<OsString>,
    env: &HashMap<OsString, OsString>,
) -> Result<(), AppError> {
    run_process(
        events,
        job_id,
        ProcessSpec {
            program: program.to_owned(),
            current_dir: PathBuf::from("/"),
            args,
            env: env.clone(),
            worker_protocol: false,
        },
        cancel,
    )
    .await?;
    Ok(())
}

fn emit_phase(
    events: &dyn JobEvents,
    job_id: Uuid,
    phase: &str,
    percent: f32,
    message: &str,
) -> Result<(), AppError> {
    emit(
        events,
        job_id,
        WorkerEvent::Phase {
            phase: phase.to_owned(),
            percent,
            message: message.to_owned(),
        },
    )
}

fn os_args<const N: usize>(values: [&str; N]) -> Vec<OsString> {
    values.into_iter().map(OsString::from).collect()
}
