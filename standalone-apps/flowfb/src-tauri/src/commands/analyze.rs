use crate::core::parser::MediaInspectionResult;
use crate::core::url_normalizer::{
    normalize_facebook_url, parse_batch_urls, BatchUrlParseResult, NormalizedFacebookUrl,
};
use crate::error::AppResult;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn validate_and_normalize_url(url: String) -> AppResult<NormalizedFacebookUrl> {
    normalize_facebook_url(&url)
}

#[tauri::command]
pub async fn parse_batch_links(raw_text: String) -> AppResult<BatchUrlParseResult> {
    Ok(parse_batch_urls(&raw_text))
}

#[tauri::command]
pub async fn analyze_facebook_url(
    url: String,
    state: State<'_, AppState>,
) -> AppResult<MediaInspectionResult> {
    state.log_buffer.push(
        "INFO",
        "Inspector",
        &format!("Analyzing Facebook URL: {}", url),
    );
    let settings = state.repo.get_settings()?;
    let res = state.router.analyze_url(&url, &settings).await?;
    state.log_buffer.push(
        "INFO",
        "Inspector",
        &format!(
            "Inspected {} ({} items, type={}, formats={})",
            res.owner_username,
            res.total_items,
            res.type_badge,
            res.format_options.len()
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
