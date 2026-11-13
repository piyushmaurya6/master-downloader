use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    InvalidUrl(String),

    #[error("{0}")]
    EngineIncompatible(String),

    #[error("{0}")]
    RateLimited(String),

    #[error("{0}")]
    AuthenticationRequired(String),

    #[error("{0}")]
    PrivateAccount(String),

    #[error("{0}")]
    MediaUnavailable(String),

    #[error("{0}")]
    BinaryMissing(String),

    #[error("{0}")]
    ChecksumMismatch(String),

    #[error("{0}")]
    DiskFull(String),

    #[error("{0}")]
    Database(String),

    #[error("{0}")]
    Network(String),

    #[error("{0}")]
    Io(String),

    #[error("{0}")]
    Cancelled(String),

    #[error("{0}")]
    Internal(String),
}

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedAppError {
    pub code: String,
    pub user_message: String,
    pub technical_details: String,
    pub remediation_action: Option<String>,
}

impl AppError {
    pub fn to_serialized(&self) -> SerializedAppError {
        match self {
            AppError::InvalidUrl(msg) => SerializedAppError {
                code: "INVALID_URL".into(),
                user_message: msg.clone(),
                technical_details: redact_sensitive_text(msg),
                remediation_action: None,
            },
            AppError::EngineIncompatible(msg) => SerializedAppError {
                code: "ENGINE_INCOMPATIBLE".into(),
                user_message:
                    "This media requires an engine update or an active platform session in the Auth Vault."
                        .into(),
                technical_details: redact_sensitive_text(msg),
                remediation_action: Some("OPEN_AUTH_MODAL".into()),
            },
            AppError::RateLimited(msg) => SerializedAppError {
                code: "RATE_LIMITED".into(),
                user_message:
                    "Temporary rate-limit (HTTP 429) or security checkpoint detected. Active queue paused to protect your IP/account."
                        .into(),
                technical_details: redact_sensitive_text(msg),
                remediation_action: Some("WAIT_OR_REAUTH".into()),
            },
            AppError::AuthenticationRequired(msg) => SerializedAppError {
                code: "AUTHENTICATION_REQUIRED".into(),
                user_message:
                    "This content requires authentication (Stories, private group/friends-only posts, or age-restricted videos). Connect your account in the Session Vault."
                        .into(),
                technical_details: redact_sensitive_text(msg),
                remediation_action: Some("OPEN_AUTH_MODAL".into()),
            },
            AppError::PrivateAccount(msg) => SerializedAppError {
                code: "PRIVATE_ACCOUNT".into(),
                user_message:
                    "This content belongs to a private account, restricted group, or members-only channel. Connect an authorized account in the Session Vault."
                        .into(),
                technical_details: redact_sensitive_text(msg),
                remediation_action: Some("OPEN_AUTH_MODAL".into()),
            },
            AppError::MediaUnavailable(msg) => SerializedAppError {
                code: "MEDIA_UNAVAILABLE".into(),
                user_message:
                    "The requested media was deleted, expired, geo-restricted, or is no longer available."
                        .into(),
                technical_details: redact_sensitive_text(msg),
                remediation_action: None,
            },
            AppError::BinaryMissing(msg) => SerializedAppError {
                code: "BINARY_MISSING".into(),
                user_message:
                    "Required download engines (yt-dlp, gallery-dl, Deno, or FFmpeg) are missing or not initialized."
                        .into(),
                technical_details: redact_sensitive_text(msg),
                remediation_action: Some("BOOTSTRAP_BINARIES".into()),
            },
            AppError::ChecksumMismatch(msg) => SerializedAppError {
                code: "CHECKSUM_MISMATCH".into(),
                user_message:
                    "Binary integrity verification (SHA-256) failed. The file was quarantined for your safety."
                        .into(),
                technical_details: redact_sensitive_text(msg),
                remediation_action: Some("BOOTSTRAP_BINARIES".into()),
            },
            AppError::DiskFull(msg) => SerializedAppError {
                code: "DISK_FULL".into(),
                user_message:
                    "Not enough free disk space in the target download directory."
                        .into(),
                technical_details: redact_sensitive_text(msg),
                remediation_action: Some("CHANGE_OUTPUT_DIR".into()),
            },
            AppError::Database(msg) => SerializedAppError {
                code: "DATABASE_ERROR".into(),
                user_message: "A local database error occurred.".into(),
                technical_details: redact_sensitive_text(msg),
                remediation_action: None,
            },
            AppError::Network(msg) => SerializedAppError {
                code: "NETWORK_ERROR".into(),
                user_message:
                    "Network connection failed while communicating with the media servers."
                        .into(),
                technical_details: redact_sensitive_text(msg),
                remediation_action: Some("RETRY".into()),
            },
            AppError::Io(msg) => SerializedAppError {
                code: "IO_ERROR".into(),
                user_message: "File system read/write operation failed.".into(),
                technical_details: redact_sensitive_text(msg),
                remediation_action: None,
            },
            AppError::Cancelled(msg) => SerializedAppError {
                code: "CANCELLED".into(),
                user_message: "Download cancelled by user.".into(),
                technical_details: redact_sensitive_text(msg),
                remediation_action: None,
            },
            AppError::Internal(msg) => SerializedAppError {
                code: "INTERNAL_ERROR".into(),
                user_message: msg.clone(),
                technical_details: redact_sensitive_text(msg),
                remediation_action: Some("VIEW_LOGS".into()),
            },
        }
    }

    pub fn classify_engine_stderr(stderr: &str) -> Self {
        let lower = stderr.to_lowercase();
        let redacted = redact_sensitive_text(stderr);

        if lower.contains("http error 429")
            || lower.contains("status code 429")
            || lower.contains("429: too many requests")
            || lower.contains("429 too many requests")
            || lower.contains("too many requests")
            || lower.contains("rate limit")
            || lower.contains("rate-limit")
            || lower.contains("checkpoint_required")
            || lower.contains("challenge_required")
            || lower.contains("feedback_required")
            || lower.contains("please wait a few minutes")
            || lower.contains("temporarily blocked")
            || lower.contains("you're temporarily blocked")
            || lower.contains("you’re temporarily blocked")
        {
            return AppError::RateLimited(redacted);
        }

        if lower.contains("cannot parse data")
            || lower.contains("unable to extract")
            || lower.contains("object is not subscriptable")
            || lower.contains("keyerror")
        {
            return AppError::EngineIncompatible(redacted);
        }

        if lower.contains("login required")
            || lower.contains("login_required")
            || lower.contains("redirect to login")
            || lower.contains("accounts/login")
            || lower.contains("you must log in")
            || lower.contains("log in to continue")
            || lower.contains("not logged in")
            || lower.contains("authentication")
            || lower.contains("cookies")
            || lower.contains("sign in to confirm your age")
            || lower.contains("sign in to confirm you’re not a bot")
            || lower.contains("sign in to confirm you're not a bot")
            || lower.contains("age-restricted")
            || lower.contains("registered users")
            || lower.contains("401 unauthorized")
        {
            return AppError::AuthenticationRequired(redacted);
        }

        if lower.contains("private")
            || lower.contains("friends-only")
            || lower.contains("members-only")
            || lower.contains("join this channel")
            || lower.contains("403 forbidden")
            || lower.contains("content isn't available right now")
            || lower.contains("this content isn't available")
        {
            return AppError::PrivateAccount(redacted);
        }

        if lower.contains("404 not found")
            || lower.contains("media not found")
            || lower.contains("video unavailable")
            || lower.contains("this video is unavailable")
            || lower.contains("has been removed")
            || lower.contains("post unavailable")
            || lower.contains("no video formats found")
            || lower.contains("unsupported url")
        {
            return AppError::MediaUnavailable(redacted);
        }

        if lower.contains("no space left on device") || lower.contains("disk full") {
            return AppError::DiskFull(redacted);
        }

        AppError::Internal(redacted)
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_serialized().serialize(serializer)
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Io(err.to_string())
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(err: rusqlite::Error) -> Self {
        AppError::Database(err.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        AppError::Network(err.to_string())
    }
}

pub fn redact_sensitive_text(input: &str) -> String {
    let patterns = [
        (r"(?i)(sessionid[=:\s]+)([^\s;&\x22']+)", "${1}[REDACTED]"),
        (r"(?i)(csrftoken[=:\s]+)([^\s;&\x22']+)", "${1}[REDACTED]"),
        (r"(?i)(ds_user_id[=:\s]+)([^\s;&\x22']+)", "${1}[REDACTED]"),
        (r"(?i)(c_user[=:\s]+)([^\s;&\x22']+)", "${1}[REDACTED]"),
        (r"(?i)(xs[=:\s]+)([^\s;&\x22']+)", "${1}[REDACTED]"),
        (r"(?i)(datr[=:\s]+)([^\s;&\x22']+)", "${1}[REDACTED]"),
        (r"(?i)(fr[=:\s]+)([^\s;&\x22']+)", "${1}[REDACTED]"),
        (r"(?i)(sapisid[=:\s]+)([^\s;&\x22']+)", "${1}[REDACTED]"),
        (r"(?i)(login_info[=:\s]+)([^\s;&\x22']+)", "${1}[REDACTED]"),
        (r"(?i)(Bearer\s+)([A-Za-z0-9._\-]+)", "${1}[REDACTED]"),
    ];

    let mut output = input.to_string();
    for (pat, replacement) in patterns {
        if let Ok(re) = Regex::new(pat) {
            output = re.replace_all(&output, replacement).into_owned();
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_sensitive_cookies() {
        let raw = "Cookie: sessionid=12345%3Asecret; c_user=100012345; xs=44%3Asecret_token; SAPISID=yt_secret; Bearer eyJhbGciOi";
        let scrubbed = redact_sensitive_text(raw);
        assert!(!scrubbed.contains("12345%3Asecret"));
        assert!(!scrubbed.contains("100012345"));
        assert!(!scrubbed.contains("44%3Asecret_token"));
        assert!(!scrubbed.contains("yt_secret"));
        assert!(!scrubbed.contains("eyJhbGciOi"));
        assert!(scrubbed.contains("[REDACTED]"));
    }

    #[test]
    fn test_classify_engine_stderr() {
        let err = AppError::classify_engine_stderr("ERROR: [facebook] 123: Cannot parse data");
        assert!(matches!(err, AppError::EngineIncompatible(_)));

        let err2 = AppError::classify_engine_stderr("HTTP Error 429: Too Many Requests");
        assert!(matches!(err2, AppError::RateLimited(_)));

        let err3 = AppError::classify_engine_stderr("Sign in to confirm your age");
        assert!(matches!(err3, AppError::AuthenticationRequired(_)));
    }
}
