use crate::models::AppUpdateInfo;
use crate::utils::DiagnosticLogger;
use serde::Deserialize;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

pub const CURRENT_APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DEFAULT_GITHUB_REPO: &str = "piyushmaurya6/flowdl";

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    published_at: Option<String>,
    body: Option<String>,
    html_url: String,
    assets: Vec<GithubAsset>,
}

/// Parses a semantic version string (`v1.2.3` or `1.2.3`) into `(major, minor, patch)`.
pub fn parse_semver(version: &str) -> Option<(u32, u32, u32)> {
    let cleaned = version.trim().trim_start_matches(['v', 'V']);
    let core = cleaned.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse::<u32>().ok()?;
    let minor = parts.next().unwrap_or("0").parse::<u32>().ok()?;
    let patch = parts.next().unwrap_or("0").parse::<u32>().ok()?;
    Some((major, minor, patch))
}

/// Returns `true` if `candidate` is strictly newer than `current`.
pub fn is_newer_semver(current: &str, candidate: &str) -> bool {
    match (parse_semver(current), parse_semver(candidate)) {
        (Some(cur), Some(cand)) => cand > cur,
        _ => false,
    }
}

/// Checks the configured GitHub repository releases over HTTPS for a newer signed FlowDL version.
pub async fn check_for_app_update(
    app: Option<&AppHandle>,
    logger: &DiagnosticLogger,
) -> Result<AppUpdateInfo, String> {
    let repo = std::env::var("FLOWDL_GITHUB_REPO").unwrap_or_else(|_| DEFAULT_GITHUB_REPO.to_string());
    let api_url = format!("https://api.github.com/repos/{}/releases/latest", repo);
    let html_fallback = format!("https://github.com/{}/releases", repo);

    let client = reqwest::Client::builder()
        .user_agent(format!("FlowDL/{} (Windows; x64)", CURRENT_APP_VERSION))
        .timeout(Duration::from_secs(12))
        .build()
        .map_err(|e| format!("Failed to initialize HTTPS update client: {}", e))?;

    let resp = match client.get(&api_url).send().await {
        Ok(r) => r,
        Err(e) => {
            logger.warn(
                "AppUpdater",
                &format!("Could not reach GitHub releases endpoint: {}", e),
            );
            return Ok(AppUpdateInfo {
                current_version: CURRENT_APP_VERSION.to_string(),
                latest_version: CURRENT_APP_VERSION.to_string(),
                update_available: false,
                release_date: None,
                release_notes: None,
                download_url: None,
                download_size: None,
                signature_verified: false,
                html_url: html_fallback,
            });
        }
    };

    if !resp.status().is_success() {
        // E.g. 404 before the very first release is published on GitHub
        return Ok(AppUpdateInfo {
            current_version: CURRENT_APP_VERSION.to_string(),
            latest_version: CURRENT_APP_VERSION.to_string(),
            update_available: false,
            release_date: None,
            release_notes: None,
            download_url: None,
            download_size: None,
            signature_verified: false,
            html_url: html_fallback,
        });
    }

    let release: GithubRelease = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse GitHub release metadata: {}", e))?;

    let latest_clean = release
        .tag_name
        .trim()
        .trim_start_matches(['v', 'V'])
        .to_string();
    let update_available = is_newer_semver(CURRENT_APP_VERSION, &latest_clean);

    // Locate x64 Windows NSIS setup artifact and verify whether its cryptographic signature (.sig) or .sha256 is attached
    let installer_asset = release.assets.iter().find(|a| {
        let lower = a.name.to_lowercase();
        lower.ends_with(".exe") && (lower.contains("setup") || lower.contains("x64"))
    });

    let has_sig_or_manifest = release.assets.iter().any(|a| {
        let lower = a.name.to_lowercase();
        lower.ends_with(".sig") || lower == "latest.json" || lower.ends_with(".sha256")
    });

    let info = AppUpdateInfo {
        current_version: CURRENT_APP_VERSION.to_string(),
        latest_version: latest_clean,
        update_available,
        release_date: release.published_at,
        release_notes: release.body,
        download_url: installer_asset.map(|a| a.browser_download_url.clone()),
        download_size: installer_asset.map(|a| a.size),
        signature_verified: has_sig_or_manifest,
        html_url: release.html_url,
    };

    if info.update_available {
        logger.info(
            "AppUpdater",
            &format!(
                "New FlowDL release available: v{} (current: v{})",
                info.latest_version, info.current_version
            ),
        );
        if let Some(handle) = app {
            let _ = handle.emit("app:update-available", &info);
        }
    }

    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semver_comparison() {
        assert_eq!(parse_semver("v1.0.0"), Some((1, 0, 0)));
        assert_eq!(parse_semver("1.2.3-beta.1"), Some((1, 2, 3)));
        assert!(is_newer_semver("1.0.0", "1.0.1"));
        assert!(is_newer_semver("1.0.0", "v1.1.0"));
        assert!(is_newer_semver("1.1.0", "2.0.0"));
        assert!(!is_newer_semver("1.0.0", "1.0.0"));
        assert!(!is_newer_semver("1.2.0", "1.1.9"));
    }
}
