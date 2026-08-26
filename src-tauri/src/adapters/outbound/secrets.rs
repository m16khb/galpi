//! Secret storage for the two credentials Galpi holds.
//!
//! Tokens used to live in `settings.json` alongside the roster and the model
//! choice. That file is readable by anything running as the user and travels
//! into backups and sync folders, which is not where an API key belongs. The
//! Keychain is the platform's answer, and it is what this module wraps.

use crate::application::error::AppError;
use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};

/// The Keychain service every Galpi secret is filed under.
const SERVICE: &str = "com.m16khb.galpi";

/// One stored credential.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Secret {
    HuggingFaceToken,
    AssistantApiKey,
}

impl Secret {
    const fn account(self) -> &'static str {
        match self {
            Self::HuggingFaceToken => "hugging-face-token",
            Self::AssistantApiKey => "assistant-api-key",
        }
    }
}

/// Where secrets are kept.
///
/// A trait rather than free functions so the storage logic around it — reading
/// through to the legacy file, migrating, clearing — can be tested without
/// touching the login keychain of whoever runs the suite.
pub trait SecretStore: std::fmt::Debug + Send + Sync {
    fn read(&self, secret: Secret) -> Result<Option<String>, AppError>;
    fn write(&self, secret: Secret, value: Option<&str>) -> Result<(), AppError>;
}

/// The macOS login keychain.
#[derive(Debug, Default)]
pub struct Keychain;

impl SecretStore for Keychain {
    fn read(&self, secret: Secret) -> Result<Option<String>, AppError> {
        match get_generic_password(SERVICE, secret.account()) {
            Ok(bytes) => String::from_utf8(bytes)
                .map(Some)
                .map_err(|error| AppError::new("KEYCHAIN_INVALID", error.to_string())),
            // Any read failure is treated as "nothing stored": the item may be
            // absent, or the user may have declined access. Either way there is
            // no token to work with, and the caller's own message about a
            // missing token is more useful than a Keychain error code.
            Err(_) => Ok(None),
        }
    }

    fn write(&self, secret: Secret, value: Option<&str>) -> Result<(), AppError> {
        let Some(value) = value else {
            // Deleting something that was never there is the desired end
            // state, not a failure.
            let _removed = delete_generic_password(SERVICE, secret.account());
            return Ok(());
        };
        set_generic_password(SERVICE, secret.account(), value.as_bytes()).map_err(|error| {
            AppError::new(
                "KEYCHAIN_WRITE_FAILED",
                format!("키체인에 토큰을 저장하지 못했습니다: {error}"),
            )
        })
    }
}

/// An in-process store used by tests, so the suite never reads or writes the
/// login keychain of whoever runs it.
#[cfg(test)]
#[derive(Debug, Default)]
pub struct InMemorySecrets {
    values: std::sync::Mutex<std::collections::HashMap<&'static str, String>>,
}

#[cfg(test)]
impl SecretStore for InMemorySecrets {
    fn read(&self, secret: Secret) -> Result<Option<String>, AppError> {
        Ok(self
            .values
            .lock()
            .map_err(|_| AppError::new("KEYCHAIN_LOCKED", "테스트 저장소 잠금이 손상되었습니다."))?
            .get(secret.account())
            .cloned())
    }

    fn write(&self, secret: Secret, value: Option<&str>) -> Result<(), AppError> {
        let mut values = self.values.lock().map_err(|_| {
            AppError::new("KEYCHAIN_LOCKED", "테스트 저장소 잠금이 손상되었습니다.")
        })?;
        match value {
            Some(value) => {
                let _replaced = values.insert(secret.account(), value.to_owned());
            }
            None => {
                let _removed = values.remove(secret.account());
            }
        }
        Ok(())
    }
}
