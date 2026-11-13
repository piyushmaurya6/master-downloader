use crate::error::{AppError, AppResult};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const KEYRING_SERVICE: &str = "MasterDownloader";
const KEYRING_USER: &str = "CookieVaultMasterKey";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSessionStatus {
    pub authenticated: bool,
    pub mode: String, // "none" | "cookies_txt" | "webview" | "browser"
    pub username: Option<String>,
    pub browser_source: Option<String>,
    pub instagram_authenticated: bool,
    pub instagram_username: Option<String>,
    pub facebook_authenticated: bool,
    pub facebook_username: Option<String>,
    pub youtube_authenticated: bool,
    pub cookie_count: usize,
    pub updated_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredVaultPayload {
    pub mode: String,
    pub username: Option<String>,
    pub browser_source: Option<String>,
    pub instagram_username: Option<String>,
    pub facebook_username: Option<String>,
    pub netscape_cookies: String,
    pub updated_at: i64,
}

/// RAII guard for ephemeral Netscape cookie files passed to `yt-dlp` / `gallery-dl`.
/// Overwrites the temporary file with zeroes and deletes it on Drop.
pub struct EphemeralCookieGuard {
    pub path: PathBuf,
}

impl Drop for EphemeralCookieGuard {
    fn drop(&mut self) {
        if self.path.exists() {
            if let Ok(meta) = fs::metadata(&self.path) {
                let len = meta.len() as usize;
                if let Ok(mut f) = File::create(&self.path) {
                    let zeroes = vec![0u8; len.min(65536)];
                    let _ = f.write_all(&zeroes);
                    let _ = f.flush();
                }
            }
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[derive(Clone)]
pub struct AuthVault {
    vault_file: PathBuf,
    fallback_key_file: PathBuf,
    temp_dir: PathBuf,
}

impl AuthVault {
    pub fn new(app_data_dir: &Path) -> AppResult<Self> {
        let temp_dir = app_data_dir.join("tmp");
        fs::create_dir_all(&temp_dir)?;
        Ok(Self {
            vault_file: app_data_dir.join("vault.bin"),
            fallback_key_file: app_data_dir.join(".vault_key"),
            temp_dir,
        })
    }

    fn get_or_create_master_key(&self) -> AppResult<[u8; 32]> {
        if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
            if let Ok(b64) = entry.get_password() {
                if let Ok(bytes) = BASE64.decode(b64.trim()) {
                    if bytes.len() == 32 {
                        let mut key = [0u8; 32];
                        key.copy_from_slice(&bytes);
                        return Ok(key);
                    }
                }
            }

            let mut key = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut key);
            let encoded = BASE64.encode(key);
            if entry.set_password(&encoded).is_ok() {
                return Ok(key);
            }
        }

        if self.fallback_key_file.exists() {
            let b64 = fs::read_to_string(&self.fallback_key_file)?;
            if let Ok(bytes) = BASE64.decode(b64.trim()) {
                if bytes.len() == 32 {
                    let mut key = [0u8; 32];
                    key.copy_from_slice(&bytes);
                    return Ok(key);
                }
            }
        }

        let mut key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        fs::write(&self.fallback_key_file, BASE64.encode(key))?;
        Ok(key)
    }

    pub fn store_cookies_txt(
        &self,
        mode: &str,
        username: Option<String>,
        netscape_cookies: &str,
    ) -> AppResult<AuthSessionStatus> {
        let existing = self.load_payload().ok().flatten();
        let combined_cookies = if let Some(ref prev) = existing {
            merge_netscape_cookies(&prev.netscape_cookies, netscape_cookies)
        } else {
            netscape_cookies.to_string()
        };

        let (ig_auth, fb_auth, yt_auth, count) = inspect_cookie_platforms(&combined_cookies);
        if count == 0 {
            return Err(AppError::AuthenticationRequired(
                "No valid YouTube, Instagram, or Facebook cookies were found in the provided input."
                    .into(),
            ));
        }

        let ig_user = if mode == "instagram_webview" {
            username.clone()
        } else {
            existing.as_ref().and_then(|e| e.instagram_username.clone())
        };
        let fb_user = if mode == "facebook_webview" {
            username.clone()
        } else {
            existing.as_ref().and_then(|e| e.facebook_username.clone())
        };

        let display_user = username
            .or_else(|| fb_user.clone())
            .or_else(|| ig_user.clone());

        let payload = StoredVaultPayload {
            mode: mode.to_string(),
            username: display_user.clone(),
            browser_source: None,
            instagram_username: ig_user.clone(),
            facebook_username: fb_user.clone(),
            netscape_cookies: combined_cookies,
            updated_at: chrono::Utc::now().timestamp(),
        };

        self.save_encrypted_payload(&payload)?;

        Ok(AuthSessionStatus {
            authenticated: true,
            mode: payload.mode,
            username: display_user,
            browser_source: None,
            instagram_authenticated: ig_auth,
            instagram_username: ig_user,
            facebook_authenticated: fb_auth,
            facebook_username: fb_user,
            youtube_authenticated: yt_auth,
            cookie_count: count,
            updated_at: Some(payload.updated_at),
        })
    }

    pub fn store_browser_delegation(&self, browser: Option<String>) -> AppResult<AuthSessionStatus> {
        let Some(browser_name) = browser.filter(|b| !b.trim().is_empty() && b != "none") else {
            return self.clear_vault();
        };

        let payload = StoredVaultPayload {
            mode: "browser".into(),
            username: Some(format!("{} profile", browser_name)),
            browser_source: Some(browser_name.clone()),
            instagram_username: Some(format!("{} browser", browser_name)),
            facebook_username: Some(format!("{} browser", browser_name)),
            netscape_cookies: String::new(),
            updated_at: chrono::Utc::now().timestamp(),
        };

        self.save_encrypted_payload(&payload)?;

        Ok(AuthSessionStatus {
            authenticated: true,
            mode: "browser".into(),
            username: payload.username,
            browser_source: Some(browser_name),
            instagram_authenticated: true,
            instagram_username: payload.instagram_username,
            facebook_authenticated: true,
            facebook_username: payload.facebook_username,
            youtube_authenticated: true,
            cookie_count: 0,
            updated_at: Some(payload.updated_at),
        })
    }

    pub fn get_status(&self) -> AuthSessionStatus {
        match self.load_payload() {
            Ok(Some(payload)) => {
                let is_browser = payload.mode == "browser" && payload.browser_source.is_some();
                let (ig_auth, fb_auth, yt_auth, count) =
                    inspect_cookie_platforms(&payload.netscape_cookies);
                AuthSessionStatus {
                    authenticated: is_browser || count > 0,
                    mode: payload.mode,
                    username: payload.username,
                    browser_source: payload.browser_source,
                    instagram_authenticated: is_browser || ig_auth,
                    instagram_username: payload.instagram_username,
                    facebook_authenticated: is_browser || fb_auth,
                    facebook_username: payload.facebook_username,
                    youtube_authenticated: is_browser || yt_auth,
                    cookie_count: count,
                    updated_at: Some(payload.updated_at),
                }
            }
            _ => AuthSessionStatus {
                authenticated: false,
                mode: "none".into(),
                username: None,
                browser_source: None,
                instagram_authenticated: false,
                instagram_username: None,
                facebook_authenticated: false,
                facebook_username: None,
                youtube_authenticated: false,
                cookie_count: 0,
                updated_at: None,
            },
        }
    }

    pub fn clear_vault(&self) -> AppResult<AuthSessionStatus> {
        if self.vault_file.exists() {
            let _ = fs::remove_file(&self.vault_file);
        }
        Ok(AuthSessionStatus {
            authenticated: false,
            mode: "none".into(),
            username: None,
            browser_source: None,
            instagram_authenticated: false,
            instagram_username: None,
            facebook_authenticated: false,
            facebook_username: None,
            youtube_authenticated: false,
            cookie_count: 0,
            updated_at: None,
        })
    }

    pub fn create_ephemeral_cookie_guard(&self) -> AppResult<Option<EphemeralCookieGuard>> {
        let Some(payload) = self.load_payload()? else {
            return Ok(None);
        };
        if payload.netscape_cookies.trim().is_empty() {
            return Ok(None);
        }

        let file_name = format!("cookies_{}.txt", Uuid::new_v4());
        let path = self.temp_dir.join(file_name);
        let mut file = File::create(&path)?;
        file.write_all(payload.netscape_cookies.as_bytes())?;
        file.flush()?;
        drop(file);

        Ok(Some(EphemeralCookieGuard { path }))
    }

    pub fn get_browser_delegation(&self) -> Option<String> {
        self.load_payload()
            .ok()
            .flatten()
            .and_then(|p| p.browser_source)
    }

    fn save_encrypted_payload(&self, payload: &StoredVaultPayload) -> AppResult<()> {
        let key_bytes = self.get_or_create_master_key()?;
        let cipher = Aes256Gcm::new_from_slice(&key_bytes)
            .map_err(|e| AppError::Internal(format!("Cipher init error: {}", e)))?;

        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let plaintext = serde_json::to_vec(payload)
            .map_err(|e| AppError::Internal(format!("Vault serialize error: {}", e)))?;

        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_ref())
            .map_err(|e| AppError::Internal(format!("Vault encryption error: {}", e)))?;

        let mut combined = Vec::with_capacity(12 + ciphertext.len());
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);

        fs::write(&self.vault_file, combined)?;
        Ok(())
    }

    fn load_payload(&self) -> AppResult<Option<StoredVaultPayload>> {
        if !self.vault_file.exists() {
            return Ok(None);
        }
        let raw = fs::read(&self.vault_file)?;
        if raw.len() <= 12 {
            return Ok(None);
        }

        let key_bytes = self.get_or_create_master_key()?;
        let cipher = Aes256Gcm::new_from_slice(&key_bytes)
            .map_err(|e| AppError::Internal(format!("Cipher init error: {}", e)))?;

        let nonce = Nonce::from_slice(&raw[..12]);
        let plaintext = cipher
            .decrypt(nonce, &raw[12..])
            .map_err(|_| AppError::Internal("Failed to decrypt cookie vault.".into()))?;

        let payload: StoredVaultPayload = serde_json::from_slice(&plaintext)
            .map_err(|e| AppError::Internal(format!("Invalid vault JSON: {}", e)))?;

        Ok(Some(payload))
    }
}

fn inspect_cookie_platforms(content: &str) -> (bool, bool, bool, usize) {
    let mut ig = false;
    let mut fb = false;
    let mut yt = false;
    let mut count = 0usize;

    for line in content.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = t.split('\t').collect();
        if parts.len() >= 7 {
            let domain = parts[0].to_lowercase();
            if domain.contains("instagram.com") {
                ig = true;
                count += 1;
            } else if domain.contains("facebook.com") || domain.contains("fb.com") {
                fb = true;
                count += 1;
            } else if domain.contains("youtube.com") || domain.contains("google.com") {
                yt = true;
                count += 1;
            }
        } else if t.contains("sessionid=") {
            ig = true;
            count += 1;
        } else if t.contains("c_user=") || t.contains("xs=") {
            fb = true;
            count += 1;
        } else if t.contains("SAPISID=") || t.contains("LOGIN_INFO=") {
            yt = true;
            count += 1;
        }
    }

    (ig, fb, yt, count)
}

fn merge_netscape_cookies(existing: &str, incoming: &str) -> String {
    let mut lines: Vec<String> = vec![
        "# Netscape HTTP Cookie File".to_string(),
        "# Generated by Master Downloader Encrypted AuthVault".to_string(),
    ];

    let mut incoming_domains = Vec::new();
    for line in incoming.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = t.split('\t').collect();
        if parts.len() >= 7 {
            let dom = parts[0].to_lowercase();
            if dom.contains("instagram.com") {
                incoming_domains.push("instagram.com");
            } else if dom.contains("facebook.com") {
                incoming_domains.push("facebook.com");
            } else if dom.contains("youtube.com") || dom.contains("google.com") {
                incoming_domains.push("youtube.com");
            }
        }
    }

    for line in existing.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = t.split('\t').collect();
        if parts.len() >= 7 {
            let dom = parts[0].to_lowercase();
            let overlapped = incoming_domains.iter().any(|d| dom.contains(d));
            if !overlapped {
                lines.push(t.to_string());
            }
        }
    }

    for line in incoming.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        lines.push(t.to_string());
    }

    lines.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_platform_vault_roundtrip() {
        let tmp = std::env::temp_dir().join(format!("masterdl_vault_test_{}", Uuid::new_v4()));
        fs::create_dir_all(&tmp).unwrap();
        let vault = AuthVault::new(&tmp).unwrap();

        let ig_cookies = ".instagram.com\tTRUE\t/\tTRUE\t1999999999\tsessionid\tsecret_ig\n";
        let s1 = vault
            .store_cookies_txt("instagram_webview", Some("ig_user".into()), ig_cookies)
            .unwrap();
        assert!(s1.instagram_authenticated);
        assert!(!s1.facebook_authenticated);

        let fb_cookies = ".facebook.com\tTRUE\t/\tTRUE\t1999999999\tc_user\t10001\n.facebook.com\tTRUE\t/\tTRUE\t1999999999\txs\tsecret_fb\n";
        let s2 = vault
            .store_cookies_txt("facebook_webview", Some("fb_user".into()), fb_cookies)
            .unwrap();
        assert!(s2.instagram_authenticated);
        assert!(s2.facebook_authenticated);
        assert_eq!(s2.cookie_count, 3);

        let _ = fs::remove_dir_all(&tmp);
    }
}
