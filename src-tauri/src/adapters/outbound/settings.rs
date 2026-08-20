use super::paths::AppPaths;
use crate::application::error::AppError;
use crate::application::model::{AssistantSettings, Participant};
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
#[serde(default, rename_all = "camelCase")]
struct LocalSettings {
    hugging_face_token: Option<String>,
    assistant_api_key: Option<String>,
    assistant_model: Option<String>,
    assistant_background: Option<String>,
    participants: Vec<Participant>,
}

impl LocalSettings {
    fn is_empty(&self) -> bool {
        self.hugging_face_token.is_none()
            && self.assistant_api_key.is_none()
            && self.assistant_model.is_none()
            && self.assistant_background.is_none()
            && self.participants.is_empty()
    }
}

#[async_trait]
impl SettingsPort for LocalSettingsStore {
    async fn load_hugging_face_token(&self) -> Result<Option<String>, AppError> {
        Ok(read_settings(&self.path).await?.hugging_face_token)
    }

    async fn save_hugging_face_token(&self, token: Option<String>) -> Result<(), AppError> {
        let mut settings = read_settings(&self.path).await?;
        settings.hugging_face_token = token;
        store_settings(&self.path, &settings).await
    }

    async fn load_assistant(&self) -> Result<AssistantSettings, AppError> {
        let settings = read_settings(&self.path).await?;
        Ok(AssistantSettings {
            api_key: settings.assistant_api_key,
            model: settings.assistant_model,
            background: settings.assistant_background,
            participants: settings.participants,
        })
    }

    async fn save_assistant(&self, assistant: AssistantSettings) -> Result<(), AppError> {
        let mut settings = read_settings(&self.path).await?;
        settings.assistant_api_key = assistant.api_key;
        settings.assistant_model = assistant.model;
        settings.assistant_background = assistant.background;
        settings.participants = assistant.participants;
        store_settings(&self.path, &settings).await
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

async fn store_settings(path: &Path, settings: &LocalSettings) -> Result<(), AppError> {
    if settings.is_empty() {
        return remove_settings(path).await;
    }
    write_settings(path, settings).await
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
    use crate::application::error::AppError;
    use crate::application::model::{AssistantSettings, Participant};
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

    #[tokio::test]
    async fn keeps_assistant_settings_when_the_hugging_face_token_is_cleared()
    -> Result<(), Box<dyn std::error::Error>> {
        // Given
        let directory = std::env::temp_dir().join(format!("galpi-settings-{}", Uuid::now_v7()));
        let store = LocalSettingsStore {
            path: directory.join("settings.json"),
        };
        store
            .save_hugging_face_token(Some("hf_saved".to_owned()))
            .await?;
        store
            .save_assistant(AssistantSettings {
                api_key: Some("zai_key".to_owned()),
                model: Some("glm-5.2".to_owned()),
                background: Some("제품: 갈피".to_owned()),
                participants: vec![Participant {
                    id: "hb".to_owned(),
                    name: "하빈".to_owned(),
                    role: Some("팀리더".to_owned()),
                    aliases: vec!["프로님".to_owned()],
                }],
            })
            .await?;

        // When
        store.save_hugging_face_token(None).await?;

        // Then
        let assistant = store.load_assistant().await?;
        assert_eq!(assistant.api_key.as_deref(), Some("zai_key"));
        assert_eq!(assistant.model.as_deref(), Some("glm-5.2"));
        assert_eq!(assistant.background.as_deref(), Some("제품: 갈피"));
        let saved = assistant
            .participants
            .first()
            .ok_or_else(|| AppError::new("TEST_ERROR", "participant was not persisted"))?;
        assert_eq!(saved.name, "하빈");
        assert_eq!(saved.aliases, ["프로님"]);
        tokio::fs::remove_dir_all(directory).await?;
        Ok(())
    }
}
