use super::paths::AppPaths;
use super::secrets::{Keychain, Secret, SecretStore};
use crate::application::error::AppError;
use crate::application::ports::SettingsPort;
use crate::domain::engine::EnginePreset;
use crate::domain::roster::{AssistantSettings, GlossaryEntry, Participant};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

#[derive(Debug)]
pub struct LocalSettingsStore {
    path: PathBuf,
    secrets: Box<dyn SecretStore>,
    /// Serializes read-modify-write cycles and caches the parsed file.
    ///
    /// Every save rewrites the whole document, so two concurrent saves would
    /// each read the same starting state and the second would drop the first's
    /// field. Holding the lock across the cycle makes that impossible, and the
    /// cached value spares a read and a parse on the transcription path.
    state: tokio::sync::Mutex<Option<LocalSettings>>,
}

impl LocalSettingsStore {
    pub fn new(app: &AppHandle) -> Result<Self, AppError> {
        Ok(Self {
            path: AppPaths::resolve(app)?.root.join("settings.json"),
            secrets: Box::new(Keychain),
            state: tokio::sync::Mutex::new(None),
        })
    }

    #[cfg(test)]
    fn for_path(path: PathBuf) -> Self {
        Self {
            path,
            secrets: Box::new(super::secrets::InMemorySecrets::default()),
            state: tokio::sync::Mutex::new(None),
        }
    }

    /// Read a secret, moving it out of the settings file the first time.
    ///
    /// Installs that predate keychain storage still have their tokens in
    /// `settings.json`. Reading one migrates it and clears the plaintext copy,
    /// so the file stops carrying the secret without the user re-entering it.
    async fn secret(&self, secret: Secret) -> Result<Option<String>, AppError> {
        if let Some(value) = self.secrets.read(secret)? {
            return Ok(Some(value));
        }
        let legacy = {
            let state = self.load().await?;
            state.as_ref().and_then(|settings| match secret {
                Secret::HuggingFaceToken => settings.hugging_face_token.clone(),
                Secret::AssistantApiKey => settings.assistant_api_key.clone(),
            })
        };
        let Some(value) = legacy else {
            return Ok(None);
        };
        self.store_secret(secret, Some(&value)).await?;
        Ok(Some(value))
    }

    /// Publish a secret to the keychain and keep the file free of plaintext.
    async fn store_secret(&self, secret: Secret, value: Option<&str>) -> Result<(), AppError> {
        self.secrets.write(secret, value)?;
        self.update(|settings| match secret {
            Secret::HuggingFaceToken => settings.hugging_face_token = None,
            Secret::AssistantApiKey => settings.assistant_api_key = None,
        })
        .await
    }

    /// Read the settings, parsing the file only the first time.
    async fn load(&self) -> Result<tokio::sync::MutexGuard<'_, Option<LocalSettings>>, AppError> {
        let mut state = self.state.lock().await;
        if state.is_none() {
            *state = Some(read_settings(&self.path).await?);
        }
        Ok(state)
    }

    /// Apply a change and publish it, keeping the cache and file in step.
    async fn update(&self, change: impl FnOnce(&mut LocalSettings)) -> Result<(), AppError> {
        let mut state = self.load().await?;
        let settings = state
            .as_mut()
            .ok_or_else(|| AppError::new("SETTINGS_INVALID", "앱 설정을 읽지 못했습니다."))?;
        change(settings);
        let result = store_settings(&self.path, settings).await;
        if result.is_err() {
            // The file no longer matches the cache, so drop it and re-read.
            *state = None;
        }
        result
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
struct LocalSettings {
    engine_preset: EnginePreset,
    hugging_face_token: Option<String>,
    assistant_api_key: Option<String>,
    assistant_model: Option<String>,
    assistant_base_url: Option<String>,
    assistant_reasoning_effort: Option<String>,
    assistant_background: Option<String>,
    participants: Vec<Participant>,
    glossary: Vec<GlossaryEntry>,
}

impl LocalSettings {
    fn is_empty(&self) -> bool {
        // The default preset is what an absent file already means, so storing
        // only the default does not justify keeping the file around.
        self.engine_preset == EnginePreset::default()
            && self.hugging_face_token.is_none()
            && self.assistant_api_key.is_none()
            && self.assistant_model.is_none()
            && self.assistant_base_url.is_none()
            && self.assistant_reasoning_effort.is_none()
            && self.assistant_background.is_none()
            && self.participants.is_empty()
            && self.glossary.is_empty()
    }
}

#[async_trait]
impl SettingsPort for LocalSettingsStore {
    async fn load_hugging_face_token(&self) -> Result<Option<String>, AppError> {
        self.secret(Secret::HuggingFaceToken).await
    }

    async fn save_hugging_face_token(&self, token: Option<String>) -> Result<(), AppError> {
        self.store_secret(Secret::HuggingFaceToken, token.as_deref())
            .await
    }

    async fn load_engine_preset(&self) -> Result<EnginePreset, AppError> {
        Ok(self
            .load()
            .await?
            .as_ref()
            .map(|settings| settings.engine_preset)
            .unwrap_or_default())
    }

    async fn save_engine_preset(&self, preset: EnginePreset) -> Result<(), AppError> {
        self.update(|settings| settings.engine_preset = preset)
            .await
    }

    async fn load_assistant(&self) -> Result<AssistantSettings, AppError> {
        let api_key = self.secret(Secret::AssistantApiKey).await?;
        let state = self.load().await?;
        let settings = state
            .as_ref()
            .ok_or_else(|| AppError::new("SETTINGS_INVALID", "앱 설정을 읽지 못했습니다."))?;
        Ok(AssistantSettings {
            api_key,
            model: settings.assistant_model.clone(),
            base_url: settings.assistant_base_url.clone(),
            reasoning_effort: settings.assistant_reasoning_effort.clone(),
            background: settings.assistant_background.clone(),
            participants: settings.participants.clone(),
            glossary: settings.glossary.clone(),
        })
    }

    async fn save_assistant(&self, assistant: AssistantSettings) -> Result<(), AppError> {
        self.secrets
            .write(Secret::AssistantApiKey, assistant.api_key.as_deref())?;
        self.update(|settings| {
            settings.assistant_api_key = None;
            settings.assistant_model = assistant.model;
            settings.assistant_base_url = assistant.base_url;
            settings.assistant_reasoning_effort = assistant.reasoning_effort;
            settings.assistant_background = assistant.background;
            settings.participants = assistant.participants;
            settings.glossary = assistant.glossary;
        })
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
    use crate::application::ports::SettingsPort;
    use crate::domain::engine::EnginePreset;
    use crate::domain::roster::{AssistantSettings, GlossaryEntry, Participant};
    use std::os::unix::fs::PermissionsExt;
    use uuid::Uuid;

    #[tokio::test]
    async fn a_corrupted_settings_file_is_reported_rather_than_reset()
    -> Result<(), Box<dyn std::error::Error>> {
        // Given: a settings file that is not valid JSON
        let directory = std::env::temp_dir().join(format!("galpi-settings-{}", Uuid::now_v7()));
        tokio::fs::create_dir_all(&directory).await?;
        let path = directory.join("settings.json");
        tokio::fs::write(&path, "{ not json").await?;
        let store = LocalSettingsStore::for_path(path.clone());

        // When
        let result = store.load_assistant().await;

        // Then: the caller learns the file is bad, and it is still on disk
        assert_eq!(
            result.err().map(|error| error.code),
            Some("SETTINGS_INVALID".to_owned())
        );
        assert!(tokio::fs::try_exists(&path).await?);
        tokio::fs::remove_dir_all(directory).await?;
        Ok(())
    }

    #[tokio::test]
    async fn keeps_the_token_out_of_the_settings_file() -> Result<(), Box<dyn std::error::Error>> {
        // Given
        let directory = std::env::temp_dir().join(format!("galpi-settings-{}", Uuid::now_v7()));
        let path = directory.join("settings.json");
        let store = LocalSettingsStore::for_path(path.clone());

        // When: a token is saved alongside an ordinary preference
        store
            .save_hugging_face_token(Some("hf_saved".to_owned()))
            .await?;
        store.save_engine_preset(EnginePreset::WhisperX).await?;

        // Then: it reads back, but the file on disk never carries it
        assert_eq!(
            store.load_hugging_face_token().await?.as_deref(),
            Some("hf_saved")
        );
        let document = tokio::fs::read_to_string(&path).await?;
        assert!(
            !document.contains("hf_saved"),
            "the token must not be written in plaintext:\n{document}"
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
    async fn migrates_a_token_left_in_an_older_settings_file()
    -> Result<(), Box<dyn std::error::Error>> {
        // Given: a settings file written before tokens moved to the keychain
        let directory = std::env::temp_dir().join(format!("galpi-settings-{}", Uuid::now_v7()));
        tokio::fs::create_dir_all(&directory).await?;
        let path = directory.join("settings.json");
        tokio::fs::write(&path, r#"{"huggingFaceToken":"hf_legacy"}"#).await?;
        let store = LocalSettingsStore::for_path(path.clone());

        // When: the token is read for the first time
        let token = store.load_hugging_face_token().await?;

        // Then: the user keeps their token and the plaintext copy is gone
        assert_eq!(token.as_deref(), Some("hf_legacy"));
        assert_eq!(
            store.load_hugging_face_token().await?.as_deref(),
            Some("hf_legacy")
        );
        let document = tokio::fs::read_to_string(&path).await.unwrap_or_default();
        assert!(
            !document.contains("hf_legacy"),
            "plaintext survived:\n{document}"
        );
        tokio::fs::remove_dir_all(directory).await?;
        Ok(())
    }

    #[tokio::test]
    async fn keeps_assistant_settings_when_the_hugging_face_token_is_cleared()
    -> Result<(), Box<dyn std::error::Error>> {
        // Given
        let directory = std::env::temp_dir().join(format!("galpi-settings-{}", Uuid::now_v7()));
        let store = LocalSettingsStore::for_path(directory.join("settings.json"));
        store
            .save_hugging_face_token(Some("hf_saved".to_owned()))
            .await?;
        store
            .save_assistant(AssistantSettings {
                api_key: Some("zai_key".to_owned()),
                model: Some("glm-5.2".to_owned()),
                base_url: Some("https://openrouter.ai/api/v1".to_owned()),
                reasoning_effort: Some("max".to_owned()),
                background: Some("제품: 갈피".to_owned()),
                participants: vec![Participant {
                    id: "hb".to_owned(),
                    name: "하빈".to_owned(),
                    team: Some("갈피팀".to_owned()),
                    role: Some("팀리더".to_owned()),
                    description: Some("녹음 파이프라인 담당".to_owned()),
                    aliases: vec!["프로님".to_owned()],
                }],
                glossary: vec![GlossaryEntry {
                    id: "term-galpi".to_owned(),
                    term: "갈피".to_owned(),
                    description: Some("회의 녹음·전사 데스크톱 앱".to_owned()),
                }],
            })
            .await?;

        // When
        store.save_hugging_face_token(None).await?;

        // Then
        let assistant = store.load_assistant().await?;
        assert_eq!(assistant.api_key.as_deref(), Some("zai_key"));
        assert_eq!(assistant.model.as_deref(), Some("glm-5.2"));
        assert_eq!(
            assistant.base_url.as_deref(),
            Some("https://openrouter.ai/api/v1")
        );
        assert_eq!(assistant.reasoning_effort.as_deref(), Some("max"));
        assert_eq!(assistant.background.as_deref(), Some("제품: 갈피"));
        let saved = assistant
            .participants
            .first()
            .ok_or_else(|| AppError::new("TEST_ERROR", "participant was not persisted"))?;
        assert_eq!(saved.name, "하빈");
        assert_eq!(saved.aliases, ["프로님"]);
        let term = assistant
            .glossary
            .first()
            .ok_or_else(|| AppError::new("TEST_ERROR", "glossary entry was not persisted"))?;
        assert_eq!(term.term, "갈피");
        assert_eq!(
            term.description.as_deref(),
            Some("회의 녹음·전사 데스크톱 앱")
        );
        tokio::fs::remove_dir_all(directory).await?;
        Ok(())
    }
}
