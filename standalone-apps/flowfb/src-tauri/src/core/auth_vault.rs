use crate::error::{AppError, AppResult};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use keyring::Entry;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const KEYRING_SERVICE: &str = "FlowFB";
const KEYRING_USER: &str = "fb_session_master_key";
const NONCE_LEN: usize = 12;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSessionStatus {
    pub mode: String, // "anonymous" | "webview_vault" | "browser_cookies" | "cookie_file"
    pub authenticated: bool,
    pub username: Option<String>,
    pub fb_user_id: Option<String>, // Facebook `c_user` cookie value
    pub selected_browser: Option<String>,
    pub vault_present: bool,
}

/// RAII guard for an ephemeral Netscape `cookies.txt` file passed to `yt-dlp` or `gallery-dl`.
/// Automatically overwrites the file contents with zeros and deletes it from disk when dropped.
pub struct EphemeralCookieGuard {
    path: PathBuf,
    byte_len: usize,
}

impl EphemeralCookieGuard {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for EphemeralCookieGuard {
    fn drop(&mut self) {
        if self.path.exists() {
            if let Ok(mut f) = std::fs::OpenOptions::new().write(true).open(&self.path) {
                let zeros = vec![0u8; self.byte_len.max(512)];
                let _ = f.write_all(&zeros);
                let _ = f.sync_all();
            }
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[derive(Clone)]
pub struct AuthVault {
    data_dir: PathBuf,
}

impl AuthVault {
    pub fn new(data_dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&data_dir);
        Self { data_dir }
    }

    pub fn vault_file_path(&self) -> PathBuf {
        self.data_dir.join("fb_vault.bin")
    }

    fn key_fallback_path(&self) -> PathBuf {
        self.data_dir.join(".fb_vault_key.bin")
    }

    pub fn has_encrypted_vault(&self) -> bool {
        self.vault_file_path().exists()
    }

    /// Retrieves or generates the 256-bit master key stored in Windows Credential Manager via `keyring`.
    fn get_or_create_master_key(&self) -> AppResult<[u8; 32]> {
        if let Ok(entry) = Entry::new(KEYRING_SERVICE, KEYRING_USER) {
            if let Ok(existing_b64) = entry.get_password() {
                if let Ok(decoded) = B64.decode(existing_b64.trim()) {
                    if decoded.len() == 32 {
                        let mut key = [0u8; 32];
                        key.copy_from_slice(&decoded);
                        return Ok(key);
                    }
                }
            }

            let mut fresh_key = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut fresh_key);
            let encoded = B64.encode(fresh_key);
            if entry.set_password(&encoded).is_ok() {
                return Ok(fresh_key);
            }
        }

        // Local fallback for headless CI environments where Windows Credential Manager service is unavailable
        let fb_path = self.key_fallback_path();
        if fb_path.exists() {
            let bytes = std::fs::read(&fb_path)?;
            if bytes.len() == 32 {
                let mut key = [0u8; 32];
                key.copy_from_slice(&bytes);
                return Ok(key);
            }
        }

        let mut fresh_key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut fresh_key);
        std::fs::write(&fb_path, fresh_key)?;
        Ok(fresh_key)
    }

    /// Encrypts a Netscape HTTP Cookie string using AES-256-GCM and writes the ciphertext to `fb_vault.bin`.
    /// Validates that the cookie payload contains `.facebook.com` cookies and extracts `c_user`.
    pub fn seal_netscape_cookies(&self, netscape_cookie_txt: &str) -> AppResult<Option<String>> {
        if !netscape_cookie_txt.contains("facebook.com") {
            return Err(AppError::VaultError(
                "Cookie data does not contain any .facebook.com cookies".to_string(),
            ));
        }

        let key_bytes = self.get_or_create_master_key()?;
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));

        let mut nonce_bytes = [0u8; NONCE_LEN];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, netscape_cookie_txt.as_bytes())
            .map_err(|e| AppError::VaultError(format!("AES-256-GCM encryption failed: {}", e)))?;

        let mut payload = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        payload.extend_from_slice(&nonce_bytes);
        payload.extend_from_slice(&ciphertext);

        std::fs::write(self.vault_file_path(), &payload)?;

        Ok(extract_cookie_value(netscape_cookie_txt, "c_user"))
    }

    /// Decrypts `fb_vault.bin` using the 256-bit key from Windows Credential Manager.
    pub fn unseal_netscape_cookies(&self) -> AppResult<String> {
        let vault_path = self.vault_file_path();
        if !vault_path.exists() {
            return Err(AppError::AuthenticationRequired);
        }

        let payload = std::fs::read(&vault_path)?;
        if payload.len() <= NONCE_LEN {
            return Err(AppError::VaultError("Corrupted fb_vault.bin file".to_string()));
        }

        let key_bytes = self.get_or_create_master_key()?;
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));
        let nonce = Nonce::from_slice(&payload[..NONCE_LEN]);
        let ciphertext = &payload[NONCE_LEN..];

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| AppError::VaultError(format!("AES-256-GCM decryption failed: {}", e)))?;

        String::from_utf8(plaintext)
            .map_err(|e| AppError::VaultError(format!("Invalid UTF-8 in cookie vault: {}", e)))
    }

    /// Materializes the decrypted Netscape cookie file into an ephemeral, zeroized-on-drop temp file
    /// immediately before spawning `yt-dlp.exe` or `gallery-dl.exe`.
    pub fn create_ephemeral_cookie_guard(&self) -> AppResult<EphemeralCookieGuard> {
        let plaintext = self.unseal_netscape_cookies()?;
        let temp_dir = self.data_dir.join(".ephemeral");
        std::fs::create_dir_all(&temp_dir)?;

        let file_name = format!(".fb_session_{}.tmp", Uuid::new_v4());
        let file_path = temp_dir.join(file_name);

        let mut file = std::fs::File::create(&file_path)?;
        file.write_all(plaintext.as_bytes())?;
        file.sync_all()?;

        Ok(EphemeralCookieGuard {
            path: file_path,
            byte_len: plaintext.len(),
        })
    }

    /// Wipes `fb_vault.bin` with zeros, deletes it from disk, and removes the master key from Windows Credential Manager.
    pub fn clear_vault(&self) -> AppResult<()> {
        let vault_path = self.vault_file_path();
        if vault_path.exists() {
            if let Ok(meta) = std::fs::metadata(&vault_path) {
                if let Ok(mut f) = std::fs::OpenOptions::new().write(true).open(&vault_path) {
                    let zeros = vec![0u8; (meta.len() as usize).max(256)];
                    let _ = f.write_all(&zeros);
                    let _ = f.sync_all();
                }
            }
            let _ = std::fs::remove_file(&vault_path);
        }

        if let Ok(entry) = Entry::new(KEYRING_SERVICE, KEYRING_USER) {
            let _ = entry.delete_credential();
        }

        let fb = self.key_fallback_path();
        if fb.exists() {
            let _ = std::fs::remove_file(fb);
        }

        Ok(())
    }
}

/// Formats raw `(name, value, domain, path, secure, expires_unix)` tuples into standard Netscape HTTP Cookie format.
pub fn format_cookies_to_netscape(
    cookies: &[(String, String, String, String, bool, i64)],
) -> String {
    let mut out = String::from("# Netscape HTTP Cookie File\n# Generated by FlowFB Secure Vault\n\n");
    for (name, value, domain, path, secure, expires) in cookies {
        let dot_domain = if domain.starts_with('.') {
            domain.clone()
        } else {
            format!(".{}", domain)
        };
        let include_subdomains = "TRUE";
        let secure_flag = if *secure { "TRUE" } else { "FALSE" };
        let exp = (*expires).max(1893456000);
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            dot_domain, include_subdomains, path, secure_flag, exp, name, value
        ));
    }
    out
}

pub fn extract_cookie_value(netscape_txt: &str, target_name: &str) -> Option<String> {
    for line in netscape_txt.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let cols: Vec<&str> = trimmed.split('\t').collect();
        if cols.len() >= 7 && cols[5] == target_name {
            return Some(cols[6].to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aes_256_gcm_facebook_vault_seal_unseal_and_ephemeral_zeroize() {
        let temp_dir = std::env::temp_dir().join(format!("flowfb_vault_test_{}", Uuid::new_v4()));
        let vault = AuthVault::new(temp_dir.clone());

        let cookies = vec![
            (
                "c_user".to_string(),
                "100089876543210".to_string(),
                ".facebook.com".to_string(),
                "/".to_string(),
                true,
                1900000000,
            ),
            (
                "xs".to_string(),
                "44%3AsecretFbXsToken%3A2%3A1700000000".to_string(),
                ".facebook.com".to_string(),
                "/".to_string(),
                true,
                1900000000,
            ),
        ];

        let netscape = format_cookies_to_netscape(&cookies);
        let fb_user_id = vault.seal_netscape_cookies(&netscape).expect("seal");
        assert_eq!(fb_user_id.as_deref(), Some("100089876543210"));
        assert!(vault.has_encrypted_vault());

        // Verify ciphertext on disk does not contain plaintext xs token
        let raw_disk = std::fs::read(vault.vault_file_path()).unwrap();
        let raw_str = String::from_utf8_lossy(&raw_disk);
        assert!(!raw_str.contains("secretFbXsToken"));

        // Verify ephemeral guard creates readable cookie file and deletes it on drop
        let eph_path = {
            let guard = vault.create_ephemeral_cookie_guard().expect("create guard");
            let p = guard.path().to_path_buf();
            assert!(p.exists());
            let content = std::fs::read_to_string(&p).unwrap();
            assert!(content.contains("secretFbXsToken"));
            p
        };
        assert!(!eph_path.exists());

        vault.clear_vault().expect("clear vault");
        assert!(!vault.has_encrypted_vault());
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
