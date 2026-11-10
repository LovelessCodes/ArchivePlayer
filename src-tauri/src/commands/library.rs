use crate::state::AppState;
use archiveplayer_core::CoreResult;
use archiveplayer_core::models::{LibraryItem, LibraryResponse};
use tauri::State;

#[tauri::command]
pub fn get_library(state: State<'_, AppState>) -> CoreResult<LibraryResponse> {
    let user = state.current_user()?;
    state.service.db.library_for_user(user.id)
}

#[tauri::command]
pub fn get_continue_watching(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> CoreResult<Vec<LibraryItem>> {
    let user = state.current_user()?;
    let limit = limit.unwrap_or(10).clamp(1, 50);
    state.service.db.continue_watching(user.id, limit)
}

#[tauri::command]
pub async fn add_to_library(
    state: State<'_, AppState>,
    identifier: String,
    category_name: Option<String>,
) -> CoreResult<LibraryItem> {
    let user = state.current_user()?;
    state
        .service
        .add_to_library(
            user.id,
            &identifier,
            category_name.as_deref().unwrap_or("Uncategorized"),
        )
        .await
}

#[tauri::command]
pub fn remove_from_library(state: State<'_, AppState>, id: i64) -> CoreResult<()> {
    let user = state.current_user()?;
    state.service.db.remove_user_media(user.id, id)
}

#[tauri::command]
pub fn clear_watch_history(state: State<'_, AppState>, id: i64) -> CoreResult<()> {
    let user = state.current_user()?;
    state.service.db.clear_watch_history(user.id, id)
}

#[tauri::command]
pub fn set_autoplay(state: State<'_, AppState>, id: i64, enabled: bool) -> CoreResult<()> {
    let user = state.current_user()?;
    state.service.db.set_autoplay(user.id, id, enabled)
}
