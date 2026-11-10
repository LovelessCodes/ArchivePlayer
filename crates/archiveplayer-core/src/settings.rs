use crate::error::CoreResult;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub tmdb_api_key: Option<String>,
    pub last_email: Option<String>,
}

impl AppSettings {
    pub fn usable_tmdb_key(&self) -> Option<String> {
        let key = self.tmdb_api_key.as_deref()?.trim();
        if key.is_empty()
            || key.eq_ignore_ascii_case("YOUR_API_KEY_HERE")
            || key.eq_ignore_ascii_case("your api key here")
        {
            return None;
        }
        Some(key.to_string())
    }
}

/// A tiny JSON settings store in the app data directory.
pub struct SettingsStore {
    path: PathBuf,
    inner: Mutex<AppSettings>,
}

impl SettingsStore {
    pub fn load(path: PathBuf) -> CoreResult<Self> {
        let settings = read_settings(&path).unwrap_or_default();
        Ok(Self {
            path,
            inner: Mutex::new(settings),
        })
    }

    pub fn get(&self) -> AppSettings {
        self.inner
            .lock()
            .map(|settings| settings.clone())
            .unwrap_or_default()
    }

    pub fn tmdb_api_key(&self) -> Option<String> {
        self.get().usable_tmdb_key()
    }

    pub fn set_tmdb_api_key(&self, key: Option<String>) -> CoreResult<()> {
        let mut settings = self
            .inner
            .lock()
            .map_err(|_| crate::error::CoreError::Message("Settings lock is poisoned.".into()))?;
        settings.tmdb_api_key = key
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        save_settings(&self.path, &settings)
    }

    pub fn clear_tmdb_api_key(&self) -> CoreResult<()> {
        let mut settings = self
            .inner
            .lock()
            .map_err(|_| crate::error::CoreError::Message("Settings lock is poisoned.".into()))?;
        settings.tmdb_api_key = None;
        save_settings(&self.path, &settings)
    }

    pub fn set_last_email(&self, email: Option<String>) -> CoreResult<()> {
        let mut settings = self
            .inner
            .lock()
            .map_err(|_| crate::error::CoreError::Message("Settings lock is poisoned.".into()))?;
        settings.last_email = email;
        save_settings(&self.path, &settings)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn read_settings(path: &Path) -> Option<AppSettings> {
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn save_settings(path: &Path, settings: &AppSettings) -> CoreResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_vec_pretty(settings)?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, json)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600));
    }

    // Rename is atomic on Unix. Windows does not replace existing files, so
    // remove the old file first (it is a tiny, non-critical settings file).
    if path.exists() {
        let _ = fs::remove_file(path);
    }
    fs::rename(temporary, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_placeholder_keys() {
        let settings = AppSettings {
            tmdb_api_key: Some("YOUR_API_KEY_HERE".into()),
            last_email: None,
        };
        assert_eq!(settings.usable_tmdb_key(), None);
    }

    #[test]
    fn round_trips_settings() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let store = SettingsStore::load(path.clone()).unwrap();
        store.set_tmdb_api_key(Some("  abc123  ".into())).unwrap();
        store.set_last_email(Some("a@b.co".into())).unwrap();

        let reloaded = SettingsStore::load(path).unwrap();
        assert_eq!(reloaded.tmdb_api_key().as_deref(), Some("abc123"));
        assert_eq!(reloaded.get().last_email.as_deref(), Some("a@b.co"));
    }
}
