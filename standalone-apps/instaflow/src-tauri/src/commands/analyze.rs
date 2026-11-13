use crate::core::parser::MediaInspectionResult;
use crate::core::url_normalizer::{
    normalize_instagram_url, parse_batch_urls, BatchUrlParseResult, NormalizedInstagramUrl,
};
use crate::error::AppResult;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn validate_and_normalize_url(url: String) -> AppResult<NormalizedInstagramUrl> {
    normalize_instagram_url(&url)
}

#[tauri::command]
pub async fn parse_batch_links(raw_text: String) -> AppResult<BatchUrlParseResult> {
    Ok(parse_batch_urls(&raw_text))
}

#[tauri::command]
pub async fn analyze_instagram_url(
    url: String,
    state: State<'_, AppState>,
) -> AppResult<MediaInspectionResult> {
    state.log_buffer.push(
        "INFO",
        "Inspector",
        &format!("Analyzing Instagram URL: {}", url),
    );
    let settings = state.repo.get_settings()?;
    let res = state.router.analyze_url(&url, &settings).await?;
    state.log_buffer.push(
        "INFO",
        "Inspector",
        &format!(
            "Inspected @{} ({} items, type={})",
            res.owner_username, res.total_items, res.type_badge
        ),
    );
    Ok(res)
}

#[tauri::command]
pub async fn proxy_thumbnail_url(
    url: String,
    state: State<'_, AppState>,
) -> AppResult<String> {
    state.router.proxy_thumbnail(&url).await
}
