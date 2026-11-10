use crate::state::AppState;
use archiveplayer_core::models::{AppInfo, SettingsInfo};
use archiveplayer_core::{CoreError, CoreResult};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;
use url::Url;

#[tauri::command]
pub fn get_settings(app: AppHandle, state: State<'_, AppState>) -> CoreResult<SettingsInfo> {
    state.current_user()?;
    Ok(state
        .service
        .settings_info(&app.package_info().version.to_string()))
}

#[tauri::command]
pub fn save_tmdb_key(state: State<'_, AppState>, api_key: Option<String>) -> CoreResult<()> {
    state.current_user()?;
    let key = api_key
        .map(|key| key.trim().to_string())
        .filter(|key| !key.is_empty());

    state.service.settings.set_tmdb_api_key(key)?;
    state.service.tmdb.clear_cache();
    Ok(())
}

#[tauri::command]
pub fn clear_tmdb_key(state: State<'_, AppState>) -> CoreResult<()> {
    state.current_user()?;
    state.service.settings.clear_tmdb_api_key()?;
    state.service.tmdb.clear_cache();
    Ok(())
}

#[tauri::command]
pub async fn test_tmdb_key(state: State<'_, AppState>) -> CoreResult<()> {
    state.current_user()?;
    state.service.tmdb.validate_key().await
}

#[tauri::command]
pub fn get_app_info(app: AppHandle, state: State<'_, AppState>) -> CoreResult<AppInfo> {
    state.current_user()?;
    Ok(AppInfo {
        version: app.package_info().version.to_string(),
        data_dir: state.service.data_dir.display().to_string(),
        settings_path: state.service.settings.path().display().to_string(),
        database_path: state.service.database_path.display().to_string(),
    })
}

#[tauri::command]
pub fn open_external(app: AppHandle, url: String) -> CoreResult<()> {
    let parsed = Url::parse(url.trim())
        .map_err(|_| CoreError::Invalid("That link is not a valid URL.".into()))?;

    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(CoreError::Invalid("Only web links can be opened.".into()));
    }

    let host = parsed.host_str().unwrap_or_default().to_ascii_lowercase();
    let allowed = host == "archive.org"
        || host.ends_with(".archive.org")
        || host == "themoviedb.org"
        || host.ends_with(".themoviedb.org")
        || host == "github.com"
        || host.ends_with(".github.com");

    if !allowed {
        return Err(CoreError::Invalid(
            "ArchivePlayer only opens Archive.org, TMDB, and GitHub links.".into(),
        ));
    }

    app.opener()
        .open_url(parsed.as_str(), None::<&str>)
        .map_err(|error| CoreError::Message(format!("Could not open the link: {error}")))
}
