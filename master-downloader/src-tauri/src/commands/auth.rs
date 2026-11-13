use crate::core::auth_vault::AuthSessionStatus;
use crate::error::{AppError, AppResult};
use crate::AppState;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

#[tauri::command]
pub async fn get_auth_status(state: State<'_, AppState>) -> AppResult<AuthSessionStatus> {
    Ok(state.vault.get_status())
}

/// Opens an isolated WebView2 window for Instagram login and captures `sessionid` + `ds_user_id`.
#[tauri::command]
pub async fn open_instagram_login_webview(
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<()> {
    let label = "ig-auth";
    if let Some(existing) = app.get_webview_window(label) {
        let _ = existing.set_focus();
        return Ok(());
    }

    let login_url: url::Url = "https://www.instagram.com/accounts/login/"
        .parse()
        .map_err(|e| AppError::Internal(format!("Invalid login URL: {}", e)))?;

    let auth_win = WebviewWindowBuilder::new(&app, label, WebviewUrl::External(login_url))
        .title("Instagram — Secure Login (Master Downloader Vault)")
        .inner_size(480.0, 680.0)
        .resizable(true)
        .center()
        .build()
        .map_err(|e| AppError::Internal(format!("Failed to create Instagram login window: {}", e)))?;

    let vault = state.vault.clone();
    let app_clone = app.clone();

    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(1200));
        loop {
            interval.tick().await;
            if app_clone.get_webview_window(label).is_none() {
                break;
            }

            let Ok(cookies) = auth_win.cookies() else {
                continue;
            };

            let has_session = cookies
                .iter()
                .any(|c| c.name() == "sessionid" && !c.value().is_empty());

            if has_session {
                let mut netscape_lines = Vec::new();
                let mut ds_user_id: Option<String> = None;

                for c in &cookies {
                    let domain = c.domain().unwrap_or(".instagram.com");
                    if !domain.contains("instagram.com") {
                        continue;
                    }
                    if c.name() == "ds_user_id" {
                        ds_user_id = Some(format!("IG #{}", c.value()));
                    }
                    let include_subdomains = if domain.starts_with('.') {
                        "TRUE"
                    } else {
                        "FALSE"
                    };
                    let path = c.path().unwrap_or("/");
                    let secure = if c.secure().unwrap_or(true) {
                        "TRUE"
                    } else {
                        "FALSE"
                    };
                    let expiry = 1999999999i64;
                    netscape_lines.push(format!(
                        "{}\t{}\t{}\t{}\t{}\t{}\t{}",
                        domain,
                        include_subdomains,
                        path,
                        secure,
                        expiry,
                        c.name(),
                        c.value()
                    ));
                }

                let netscape_blob = netscape_lines.join("\n") + "\n";
                if let Ok(status) =
                    vault.store_cookies_txt("instagram_webview", ds_user_id, &netscape_blob)
                {
                    let _ = app_clone.emit("auth-status-changed", status);
                    let _ = auth_win.close();
                    break;
                }
            }
        }
    });

    Ok(())
}

/// Opens an isolated WebView2 window for Facebook login and captures `c_user` + `xs`.
#[tauri::command]
pub async fn open_facebook_login_webview(
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<()> {
    let label = "fb-auth";
    if let Some(existing) = app.get_webview_window(label) {
        let _ = existing.set_focus();
        return Ok(());
    }

    let login_url: url::Url = "https://www.facebook.com/login"
        .parse()
        .map_err(|e| AppError::Internal(format!("Invalid login URL: {}", e)))?;

    let auth_win = WebviewWindowBuilder::new(&app, label, WebviewUrl::External(login_url))
        .title("Facebook — Secure Login (Master Downloader Vault)")
        .inner_size(500.0, 700.0)
        .resizable(true)
        .center()
        .build()
        .map_err(|e| AppError::Internal(format!("Failed to create Facebook login window: {}", e)))?;

    let vault = state.vault.clone();
    let app_clone = app.clone();

    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(1200));
        loop {
            interval.tick().await;
            if app_clone.get_webview_window(label).is_none() {
                break;
            }

            let Ok(cookies) = auth_win.cookies() else {
                continue;
            };

            let has_c_user = cookies
                .iter()
                .any(|c| c.name() == "c_user" && !c.value().is_empty());
            let has_xs = cookies
                .iter()
                .any(|c| c.name() == "xs" && !c.value().is_empty());

            if has_c_user && has_xs {
                let mut netscape_lines = Vec::new();
                let mut fb_user_id: Option<String> = None;

                for c in &cookies {
                    let domain = c.domain().unwrap_or(".facebook.com");
                    if !domain.contains("facebook.com") && !domain.contains("fb.com") {
                        continue;
                    }
                    if c.name() == "c_user" {
                        fb_user_id = Some(format!("FB #{}", c.value()));
                    }
                    let include_subdomains = if domain.starts_with('.') {
                        "TRUE"
                    } else {
                        "FALSE"
                    };
                    let path = c.path().unwrap_or("/");
                    let secure = if c.secure().unwrap_or(true) {
                        "TRUE"
                    } else {
                        "FALSE"
                    };
                    let expiry = 1999999999i64;
                    netscape_lines.push(format!(
                        "{}\t{}\t{}\t{}\t{}\t{}\t{}",
                        domain,
                        include_subdomains,
                        path,
                        secure,
                        expiry,
                        c.name(),
                        c.value()
                    ));
                }

                let netscape_blob = netscape_lines.join("\n") + "\n";
                if let Ok(status) =
                    vault.store_cookies_txt("facebook_webview", fb_user_id, &netscape_blob)
                {
                    let _ = app_clone.emit("auth-status-changed", status);
                    let _ = auth_win.close();
                    break;
                }
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn import_cookies_txt(
    state: State<'_, AppState>,
    app: AppHandle,
    cookie_content: String,
    username_hint: Option<String>,
) -> AppResult<AuthSessionStatus> {
    let status = state
        .vault
        .store_cookies_txt("cookies_txt", username_hint, &cookie_content)?;
    let _ = app.emit("auth-status-changed", &status);
    Ok(status)
}

#[tauri::command]
pub async fn set_browser_cookie_mode(
    state: State<'_, AppState>,
    app: AppHandle,
    browser: Option<String>,
) -> AppResult<AuthSessionStatus> {
    let status = state.vault.store_browser_delegation(browser.clone())?;
    if let Ok(mut settings) = state.db.get_settings() {
        settings.browser_cookie_source = browser;
        let _ = state.db.save_settings(&settings);
    }
    let _ = app.emit("auth-status-changed", &status);
    Ok(status)
}

#[tauri::command]
pub async fn logout_and_wipe_vault(
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<AuthSessionStatus> {
    let status = state.vault.clear_vault()?;
    if let Ok(mut settings) = state.db.get_settings() {
        settings.browser_cookie_source = None;
        let _ = state.db.save_settings(&settings);
    }
    let _ = app.emit("auth-status-changed", &status);
    Ok(status)
}
