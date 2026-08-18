use super::paths::AppPaths;
use crate::application::error::AppError;
use crate::application::ports::SettingsPort;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

#[derive(Debug, Clone)]
pub struct LocalSettingsStore {
    path: PathBuf,
}

impl LocalSettingsStore {
    pub fn new(app: &AppHandle) -> Result<Self, AppError> {
        Ok(Self {
            path: AppPaths::resolve(app)?.root.join("settings.json"),
        })
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct LocalSettings {
    hugging_face_token: Option<String>,
}

#[async_trait]
impl SettingsPort for LocalSettingsStore {
    async fn load_hugging_face_token(&self) -> Result<Option<String>, AppError> {
        Ok(read_settings(&self.path).await?.hugging_face_token)
    }

    async fn save_hugging_face_token(&self, token: Option<String>) -> Result<(), AppError> {
        if token.is_none() {
            return remove_settings(&self.path).await;
        }
        write_settings(
            &self.path,
            &LocalSettings {
                hugging_face_token: token,
            },
        )
        .await
    }
}

async fn read_settings(path: &Path) -> Result<LocalSettings, AppError> {
    match tokio::fs::read_to_string(path).await {
        Ok(contents) => serde_json::from_str(&contents).map_err(|error| {
            AppError::new(
                "SETTINGS_INVALID",
                format!("앱 설정 파일을 읽지 못했습니다: {error}"),
            )
        }),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(LocalSettings::default()),
        Err(error) => Err(AppError::io("앱 설정 파일을 읽지 못했습니다", &error)),
    }
}

async fn write_settings(path: &Path, settings: &LocalSettings) -> Result<(), AppError> {
    let parent = path
        .parent()
        .ok_or_else(|| AppError::new("SETTINGS_PATH_ERROR", "앱 설정 경로가 올바르지 않습니다."))?;
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|error| AppError::io("앱 설정 폴더를 만들지 못했습니다", &error))?;
    let contents = serde_json::to_vec(settings).map_err(|error| {
        AppError::new(
            "SETTINGS_SERIALIZE_ERROR",
            format!("앱 설정을 저장 형식으로 바꾸지 못했습니다: {error}"),
        )
    })?;
    let temporary = path.with_extension("json.part");
    tokio::fs::write(&temporary, contents)
        .await
        .map_err(|error| AppError::io("임시 앱 설정 파일을 쓰지 못했습니다", &error))?;
    tokio::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(0o600))
        .await
        .map_err(|error| AppError::io("앱 설정 파일 권한을 지정하지 못했습니다", &error))?;
    if let Err(error) = tokio::fs::rename(&temporary, path).await {
        let _cleanup_result = tokio::fs::remove_file(&temporary).await;
        return Err(AppError::io("앱 설정 파일을 교체하지 못했습니다", &error));
    }
    Ok(())
}

async fn remove_settings(path: &Path) -> Result<(), AppError> {
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(AppError::io("저장된 앱 설정을 지우지 못했습니다", &error)),
    }
}

#[cfg(test)]
mod tests {
    use super::LocalSettingsStore;
    use crate::application::ports::SettingsPort;
    use std::os::unix::fs::PermissionsExt;
    use uuid::Uuid;

    #[tokio::test]
    async fn stores_token_with_private_permissions_and_removes_it()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = std::env::temp_dir().join(format!("galpi-settings-{}", Uuid::now_v7()));
        let path = directory.join("settings.json");
        let store = LocalSettingsStore { path: path.clone() };

        store
            .save_hugging_face_token(Some("hf_saved".to_owned()))
            .await?;

        assert_eq!(
            store.load_hugging_face_token().await?.as_deref(),
            Some("hf_saved")
        );
        assert_eq!(
            tokio::fs::metadata(&path).await?.permissions().mode() & 0o777,
            0o600
        );

        store.save_hugging_face_token(None).await?;
        assert_eq!(store.load_hugging_face_token().await?, None);
        tokio::fs::remove_dir_all(directory).await?;
        Ok(())
    }
}
