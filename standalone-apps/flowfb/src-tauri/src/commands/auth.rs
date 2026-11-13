use crate::core::auth_vault::{
    extract_cookie_value, format_cookies_to_netscape, AuthSessionStatus,
};
use crate::error::{AppError, AppResult};
use crate::AppState;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};
use url::Url;

#[tauri::command]
pub async fn get_auth_status(state: State<'_, AppState>) -> AppResult<AuthSessionStatus> {
    let settings = state.repo.get_settings()?;
    let vault_present = state.auth_vault.has_encrypted_vault();
    let mut fb_user_id = None;

    if vault_present {
        if let Ok(plain) = state.auth_vault.unseal_netscape_cookies() {
            fb_user_id = extract_cookie_value(&plain, "c_user");
        }
    }

    let authenticated = match settings.auth_mode.as_str() {
        "webview_vault" | "cookie_file" => vault_present,
        "browser_cookies" => settings.selected_browser.is_some(),
        _ => false,
    };

    Ok(AuthSessionStatus {
        mode: settings.auth_mode,
        authenticated,
        username: settings.connected_username.or_else(|| {
            fb_user_id
                .as_ref()
                .map(|id| format!("fb_user_{}", id))
        }),
        fb_user_id,
        selected_browser: settings.selected_browser,
        vault_present,
    })
}

/// Spawns a secondary, sandboxed Tauri `WebviewWindow` pointed to `https://www.facebook.com/login`.
/// Polls the WebView2 cookie manager for `.facebook.com` cookies (`c_user` + `xs`), encrypts them
/// into the AES-256-GCM vault backed by Windows Credential Manager, and closes the login window automatically.
#[tauri::command]
pub async fn open_facebook_login_webview(
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<()> {
    if let Some(existing) = app.get_webview_window("fb-auth") {
        let _ = existing.set_focus();
        return Ok(());
    }

    let login_url = Url::parse("https://www.facebook.com/login")
        .map_err(|e| AppError::InvalidUrl(e.to_string()))?;

    let auth_win = WebviewWindowBuilder::new(&app, "fb-auth", WebviewUrl::External(login_url))
        .title("Connect Facebook Account — FlowFB Secure Webview")
        .inner_size(500.0, 700.0)
        .resizable(true)
        .center()
        .build()
        .map_err(|e| AppError::VaultError(format!("Failed to open Webview window: {}", e)))?;

    let vault = state.auth_vault.clone();
    let repo = state.repo.clone();
    let log_buffer = state.log_buffer.clone();
    let queue_worker = state.queue_worker.clone();
    let app_clone = app.clone();

    tauri::async_runtime::spawn(async move {
        let fb_origin = Url::parse("https://www.facebook.com").expect("valid url");

        for _ in 0..300 {
            tokio::time::sleep(Duration::from_millis(1200)).await;

            if app_clone.get_webview_window("fb-auth").is_none() {
                break;
            }

            if let Ok(cookies) = auth_win.cookies_for_url(fb_origin.clone()) {
                let has_xs = cookies
                    .iter()
                    .any(|c| c.name() == "xs" && !c.value().is_empty());
                let c_user = cookies
                    .iter()
                    .find(|c| c.name() == "c_user" && !c.value().is_empty())
                    .map(|c| c.value().to_string());

                if has_xs && c_user.is_some() {
                    let tuples: Vec<(String, String, String, String, bool, i64)> = cookies
                        .iter()
                        .map(|c| {
                            (
                                c.name().to_string(),
                                c.value().to_string(),
                                c.domain().unwrap_or(".facebook.com").to_string(),
                                c.path().unwrap_or("/").to_string(),
                                c.secure().unwrap_or(true),
                                1924905600,
                            )
                        })
                        .collect();

                    let netscape = format_cookies_to_netscape(&tuples);
                    if vault.seal_netscape_cookies(&netscape).is_ok() {
                        if let Ok(mut settings) = repo.get_settings() {
                            settings.auth_mode = "webview_vault".to_string();
                            settings.connected_username = c_user
                                .as_ref()
                                .map(|id| format!("fb_uid_{}", id))
                                .or(Some("facebook_connected".to_string()));
                            let _ = repo.save_settings(settings);
                        }

                        log_buffer.push(
                            "INFO",
                            "AuthVault",
                            "Captured Facebook session (c_user + xs) via sandboxed Webview and sealed into AES-256-GCM vault.",
                        );
                        queue_worker.reset_circuit_breaker();
                        let _ = auth_win.close();
                        let _ = app_clone.emit("auth-status-changed", "connected");
                        break;
                    }
                }
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn import_cookies_txt(
    cookie_content: String,
    username_hint: Option<String>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<AuthSessionStatus> {
    let fb_user_id = state.auth_vault.seal_netscape_cookies(&cookie_content)?;
    let mut settings = state.repo.get_settings()?;
    settings.auth_mode = "cookie_file".to_string();
    settings.connected_username = username_hint.or_else(|| {
        fb_user_id
            .as_ref()
            .map(|id| format!("fb_uid_{}", id))
            .or_else(|| Some("facebook_cookie_vault".to_string()))
    });
    state.repo.save_settings(settings)?;
    state.queue_worker.reset_circuit_breaker();
    state.log_buffer.push(
        "INFO",
        "AuthVault",
        "Imported Netscape cookies.txt into encrypted AES-256-GCM vault.",
    );
    let _ = app.emit("auth-status-changed", "cookie_file");
    get_auth_status(state).await
}

#[tauri::command]
pub async fn set_browser_cookie_mode(
    browser: Option<String>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<AuthSessionStatus> {
    let mut settings = state.repo.get_settings()?;
    if let Some(b) = browser {
        let cleaned = b.trim().to_lowercase();
        if cleaned.is_empty() || cleaned == "none" {
            settings.auth_mode = "anonymous".to_string();
            settings.selected_browser = None;
        } else {
            settings.auth_mode = "browser_cookies".to_string();
            settings.selected_browser = Some(cleaned.clone());
            settings.connected_username = Some(format!("{}_browser_profile", cleaned));
        }
    } else {
        settings.auth_mode = "anonymous".to_string();
        settings.selected_browser = None;
    }

    state.repo.save_settings(settings)?;
    state.queue_worker.reset_circuit_breaker();
    let _ = app.emit("auth-status-changed", "browser");
    get_auth_status(state).await
}

#[tauri::command]
pub async fn logout_and_wipe_vault(
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<AuthSessionStatus> {
    state.auth_vault.clear_vault()?;
    let mut settings = state.repo.get_settings()?;
    settings.auth_mode = "anonymous".to_string();
    settings.selected_browser = None;
    settings.connected_username = None;
    state.repo.save_settings(settings)?;
    state.log_buffer.push(
        "INFO",
        "AuthVault",
        "Cleared Facebook session cookies and zeroized encrypted vault.",
    );
    let _ = app.emit("auth-status-changed", "anonymous");
    get_auth_status(state).await
}
