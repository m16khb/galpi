use super::paths::AppPaths;
use crate::application::error::AppError;
use crate::application::model::EnvironmentStatus;
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

pub const ENGINE_VERSION: &str = "3.8.6";

pub fn diagnose(app: &AppHandle) -> Result<EnvironmentStatus, AppError> {
    let paths = AppPaths::resolve(app)?;
    Ok(status(&paths))
}

pub fn status(paths: &AppPaths) -> EnvironmentStatus {
    EnvironmentStatus {
        engine_ready: paths.python.is_file()
            && std::fs::read_to_string(&paths.engine_manifest)
                .is_ok_and(|version| version == ENGINE_VERSION),
        models_ready: model_manifest_ready(paths),
        ffmpeg_ready: paths.engine_bin.join("ffmpeg").is_file(),
        data_directory: paths.root.to_string_lossy().into_owned(),
        default_output_directory: home_directory()
            .join("Downloads/whisperx-out")
            .to_string_lossy()
            .into_owned(),
        engine_version: ENGINE_VERSION.to_owned(),
    }
}

pub fn process_environment(
    paths: &AppPaths,
    worker_root: &Path,
    token: Option<&str>,
) -> HashMap<OsString, OsString> {
    let mut env = HashMap::from([
        ("HOME".into(), home_directory().into_os_string()),
        ("LANG".into(), "ko_KR.UTF-8".into()),
        ("LC_ALL".into(), "ko_KR.UTF-8".into()),
        ("PYTHONUTF8".into(), "1".into()),
        ("PYTHONSAFEPATH".into(), "1".into()),
        ("PYTHONDONTWRITEBYTECODE".into(), "1".into()),
        ("PYTHONPATH".into(), worker_root.as_os_str().to_owned()),
        (
            "HF_HOME".into(),
            paths.cache.join("huggingface").into_os_string(),
        ),
        (
            "TORCH_HOME".into(),
            paths.cache.join("torch").into_os_string(),
        ),
        ("HF_HUB_DISABLE_IMPLICIT_TOKEN".into(), "1".into()),
        ("HF_HUB_DISABLE_TELEMETRY".into(), "1".into()),
        ("PYANNOTE_METRICS_ENABLED".into(), "false".into()),
        ("DO_NOT_TRACK".into(), "1".into()),
        (
            "UV_PYTHON_INSTALL_DIR".into(),
            paths.python_installations.clone().into_os_string(),
        ),
        (
            "PATH".into(),
            format!(
                "{}:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin",
                paths.engine_bin.to_string_lossy()
            )
            .into(),
        ),
        (
            "TMPDIR".into(),
            std::env::var_os("TMPDIR").unwrap_or_else(|| "/tmp".into()),
        ),
    ]);
    if let Some(token) = token.map(str::trim).filter(|token| !token.is_empty()) {
        env.insert("HF_TOKEN".into(), token.into());
    }
    env
}

/// Worker environment for assistant calls, carrying the assistant credential
/// and, when configured, an OpenAI-compatible endpoint override.
pub fn assistant_environment(
    paths: &AppPaths,
    worker_root: &Path,
    api_key: &str,
    base_url: Option<&str>,
    reasoning_effort: Option<&str>,
) -> HashMap<OsString, OsString> {
    let mut env = process_environment(paths, worker_root, None);
    env.insert("GALPI_ASSISTANT_API_KEY".into(), api_key.into());
    if let Some(base_url) = base_url {
        env.insert("GALPI_ASSISTANT_BASE_URL".into(), base_url.into());
    }
    if let Some(reasoning_effort) = reasoning_effort {
        env.insert(
            "GALPI_ASSISTANT_REASONING_EFFORT".into(),
            reasoning_effort.into(),
        );
    }
    env
}

fn model_manifest_ready(paths: &AppPaths) -> bool {
    let manifest = std::fs::read_to_string(&paths.models_manifest)
        .ok()
        .and_then(|contents| serde_json::from_str::<serde_json::Value>(&contents).ok());
    let manifest_valid = manifest.is_some_and(|value| {
        value.get("protocol").and_then(serde_json::Value::as_u64) == Some(1)
            && value.get("whisperx").and_then(serde_json::Value::as_str) == Some(ENGINE_VERSION)
    });
    let hub = paths.cache.join("huggingface/hub");
    manifest_valid
        && [
            "models--mobiuslabsgmbh--faster-whisper-large-v3-turbo",
            "models--kresnik--wav2vec2-large-xlsr-korean",
            "models--pyannote--speaker-diarization-community-1",
        ]
        .iter()
        .all(|model| hub.join(model).is_dir())
}

fn home_directory() -> PathBuf {
    std::env::var_os("HOME").map_or_else(|| PathBuf::from("/tmp"), PathBuf::from)
}
