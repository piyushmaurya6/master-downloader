use crate::core::parser::MediaInspectionResult;
use crate::core::url_normalizer::{
    normalize_media_url, parse_batch_urls, BatchUrlParseResult, NormalizedMediaUrl,
};
use crate::error::AppResult;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn validate_and_normalize_url(url: String) -> AppResult<NormalizedMediaUrl> {
    normalize_media_url(&url)
}

#[tauri::command]
pub async fn parse_batch_links(raw_text: String) -> AppResult<BatchUrlParseResult> {
    Ok(parse_batch_urls(&raw_text))
}

#[tauri::command]
pub async fn analyze_media_url(
    state: State<'_, AppState>,
    url: String,
) -> AppResult<MediaInspectionResult> {
    state.router.inspect_url(&url).await
}

#[tauri::command]
pub async fn proxy_thumbnail_url(state: State<'_, AppState>, url: String) -> AppResult<String> {
    state.router.proxy_thumbnail(&url).await
}
