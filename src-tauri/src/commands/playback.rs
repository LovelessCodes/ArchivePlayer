use crate::state::AppState;
use archiveplayer_core::CoreResult;
use archiveplayer_core::models::MediaDetails;
use tauri::State;

#[tauri::command]
pub async fn get_playback_details(
    state: State<'_, AppState>,
    identifier: String,
) -> CoreResult<MediaDetails> {
    let user = state.current_user()?;
    state
        .service
        .playback_details(user.id, &identifier, None, None)
        .await
}

#[tauri::command]
pub fn save_progress(
    state: State<'_, AppState>,
    identifier: String,
    file_name: String,
    position_seconds: i64,
    duration_seconds: i64,
) -> CoreResult<()> {
    let user = state.current_user()?;
    state.service.db.save_progress(
        user.id,
        &identifier,
        &file_name,
        position_seconds,
        duration_seconds,
        false,
    )
}

#[tauri::command]
pub fn mark_completed(
    state: State<'_, AppState>,
    identifier: String,
    file_name: String,
    item_completed: bool,
) -> CoreResult<()> {
    let user = state.current_user()?;
    state
        .service
        .db
        .mark_completed(user.id, &identifier, &file_name, item_completed)
}
