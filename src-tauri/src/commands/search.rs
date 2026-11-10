use crate::state::AppState;
use archiveplayer_core::CoreResult;
use archiveplayer_core::models::{ArchiveSearchResponse, CategorySuggestion};
use tauri::State;

#[tauri::command]
pub async fn search_archive(
    state: State<'_, AppState>,
    query: String,
    page: Option<i64>,
) -> CoreResult<ArchiveSearchResponse> {
    state.current_user()?;
    state
        .service
        .archive
        .search(&query, page.unwrap_or(1))
        .await
}

#[tauri::command]
pub async fn get_category_suggestions(
    state: State<'_, AppState>,
    identifier: String,
) -> CoreResult<Vec<CategorySuggestion>> {
    state.current_user()?;
    state.service.category_suggestions(&identifier).await
}
