use regex::Regex;
use serde::{Serialize, Serializer};
use std::sync::OnceLock;
use thiserror::Error;

/// Unified application error taxonomy for InstaFlow.
/// Serializes into a structured payload `{ code, user_message, technical_details, remediation_action }`
/// so the React frontend can display actionable recovery CTAs.
#[derive(Debug, Error, Clone)]
pub enum AppError {
    #[error("Instagram rate limit reached")]
    RateLimited { retry_after_secs: Option<u64> },

    #[error("Instagram authentication is required to access this media")]
    AuthenticationRequired,

    #[error("This Instagram account is private and not accessible with the current session")]
    PrivateAccount,

    #[error("Media has been deleted, expired, or is no longer available on Instagram")]
    MediaDeletedOrUnavailable,

    #[error("Required media engine binary is missing: {0}")]
    BinaryMissing(String),

    #[error("SHA-256 checksum verification failed for downloaded binary")]
    ChecksumMismatch,

    #[error("Insufficient disk space in destination directory")]
    DiskFull,

    #[error("Network connection is offline or unreachable")]
    NetworkOffline,

    #[error("Engine output parse error: {0}")]
    EngineParseError(String),

    #[error("SQLite database error: {0}")]
    DatabaseError(String),

    #[error("Invalid or unsupported Instagram URL: {0}")]
    InvalidUrl(String),

    #[error("Authentication vault error: {0}")]
    VaultError(String),

    #[error("Process execution error: {0}")]
    ProcessError(String),
}

#[derive(Debug, Serialize, Clone)]
pub struct SerializedAppError {
    pub code: &'static str,
    pub user_message: String,
    pub technical_details: String,
    pub remediation_action: &'static str,
    pub retry_after_secs: Option<u64>,
}

impl AppError {
    pub fn to_serialized(&self) -> SerializedAppError {
        match self {
            AppError::RateLimited { retry_after_secs } => SerializedAppError {
                code: "RATE_LIMITED",
                user_message: format!(
                    "Instagram temporarily rate-limited requests (HTTP 429 / Challenge). Queue paused for {}s to protect your account.",
                    retry_after_secs.unwrap_or(120)
                ),
                technical_details: redact_sensitive(&self.to_string()),
                remediation_action: "WAIT_COOLDOWN",
                retry_after_secs: *retry_after_secs,
            },
            AppError::AuthenticationRequired => SerializedAppError {
                code: "AUTHENTICATION_REQUIRED",
                user_message: "Instagram requires an active session to inspect or download this content (Stories, Highlights, or age-restricted Reels).".to_string(),
                technical_details: "login_required / session cookie missing or expired".to_string(),
                remediation_action: "CONNECT_ACCOUNT",
                retry_after_secs: None,
            },
            AppError::PrivateAccount => SerializedAppError {
                code: "PRIVATE_ACCOUNT",
                user_message: "This post belongs to a private account. InstaFlow only downloads media that your connected account is permitted to view.".to_string(),
                technical_details: "Private profile restriction".to_string(),
                remediation_action: "CHECK_PERMISSIONS",
                retry_after_secs: None,
            },
            AppError::MediaDeletedOrUnavailable => SerializedAppError {
                code: "MEDIA_UNAVAILABLE",
                user_message: "This post or story could not be found. It may have been deleted or expired.".to_string(),
                technical_details: "HTTP 404 / Media unavailable".to_string(),
                remediation_action: "VERIFY_URL",
                retry_after_secs: None,
            },
            AppError::BinaryMissing(bin) => SerializedAppError {
                code: "BINARY_MISSING",
                user_message: format!(
                    "Required media engine ({}) is not installed in %LOCALAPPDATA%\\InstaFlow\\bin.",
                    bin
                ),
                technical_details: format!("Missing binary: {}", bin),
                remediation_action: "BOOTSTRAP_BINARIES",
                retry_after_secs: None,
            },
            AppError::ChecksumMismatch => SerializedAppError {
                code: "CHECKSUM_MISMATCH",
                user_message: "Security check failed: Downloaded binary SHA-256 hash did not match the official release checksum.".to_string(),
                technical_details: "SHA-256 verification mismatch".to_string(),
                retry_after_secs: None,
                remediation_action: "RETRY_BOOTSTRAP",
            },
            AppError::DiskFull => SerializedAppError {
                code: "DISK_FULL",
                user_message: "Your target storage drive is out of space. Free up disk space or choose another output folder.".to_string(),
                technical_details: "No space left on device (OS error 112 / ENOSPC)".to_string(),
                remediation_action: "CHANGE_OUTPUT_DIR",
                retry_after_secs: None,
            },
            AppError::NetworkOffline => SerializedAppError {
                code: "NETWORK_OFFLINE",
                user_message: "Unable to reach Instagram or GitHub servers. Check your internet connection or proxy settings.".to_string(),
                technical_details: "Network request timed out or DNS resolution failed".to_string(),
                remediation_action: "CHECK_NETWORK",
                retry_after_secs: None,
            },
            AppError::EngineParseError(details) => SerializedAppError {
                code: "ENGINE_PARSE_ERROR",
                user_message: "Could not parse media metadata from the extraction engine. Updating engines may resolve this.".to_string(),
                technical_details: redact_sensitive(details),
                remediation_action: "UPDATE_ENGINES",
                retry_after_secs: None,
            },
            AppError::DatabaseError(details) => SerializedAppError {
                code: "DATABASE_ERROR",
                user_message: "An internal database error occurred while saving queue or history state.".to_string(),
                technical_details: redact_sensitive(details),
                remediation_action: "RETRY_OPERATION",
                retry_after_secs: None,
            },
            AppError::InvalidUrl(details) => SerializedAppError {
                code: "INVALID_URL",
                user_message: "Please enter a valid Instagram link (Reel, Post, Carousel, Story, Highlight, or Profile).".to_string(),
                technical_details: redact_sensitive(details),
                remediation_action: "FIX_URL",
                retry_after_secs: None,
            },
            AppError::VaultError(details) => SerializedAppError {
                code: "VAULT_ERROR",
                user_message: "Could not access the encrypted session vault or Windows Credential Manager.".to_string(),
                technical_details: redact_sensitive(details),
                remediation_action: "RECONNECT_ACCOUNT",
                retry_after_secs: None,
            },
            AppError::ProcessError(details) => SerializedAppError {
                code: "PROCESS_ERROR",
                user_message: "The background media engine encountered an unexpected error.".to_string(),
                technical_details: redact_sensitive(details),
                remediation_action: "VIEW_LOGS",
                retry_after_secs: None,
            },
        }
    }

    /// Maps raw stderr text from yt-dlp or gallery-dl into a strongly typed `AppError`.
    pub fn from_engine_stderr(stderr: &str) -> Self {
        let lower = stderr.to_lowercase();
        if lower.contains("429")
            || lower.contains("too many requests")
            || lower.contains("rate-limit")
            || lower.contains("rate limit")
            || lower.contains("checkpoint_required")
            || lower.contains("challenge_required")
            || lower.contains("please wait a few minutes")
        {
            return AppError::RateLimited {
                retry_after_secs: Some(180),
            };
        }
        if lower.contains("private profile")
            || lower.contains("this account is private")
            || lower.contains("private_account")
        {
            return AppError::PrivateAccount;
        }
        if lower.contains("login_required")
            || lower.contains("login required")
            || lower.contains("authentication required")
            || lower.contains("cookies-from-browser")
            || lower.contains("you must log in")
            || lower.contains("sign in to confirm")
            || lower.contains("not logged in")
        {
            return AppError::AuthenticationRequired;
        }
        if lower.contains("404")
            || lower.contains("not found")
            || lower.contains("media unavailable")
            || lower.contains("content is not available")
            || lower.contains("post not found")
        {
            return AppError::MediaDeletedOrUnavailable;
        }
        if lower.contains("no space left") || lower.contains("os error 112") || lower.contains("disk full") {
            return AppError::DiskFull;
        }
        if lower.contains("getaddrinfo failed")
            || lower.contains("network is unreachable")
            || lower.contains("timed out")
            || lower.contains("connection reset")
        {
            return AppError::NetworkOffline;
        }
        AppError::ProcessError(redact_sensitive(stderr.trim()))
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_serialized().serialize(serializer)
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(err: rusqlite::Error) -> Self {
        AppError::DatabaseError(err.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() || err.is_connect() {
            AppError::NetworkOffline
        } else {
            AppError::ProcessError(err.to_string())
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        if err.raw_os_error() == Some(112) {
            AppError::DiskFull
        } else {
            AppError::ProcessError(err.to_string())
        }
    }
}

/// Scrubs sensitive cookies (`sessionid`, `csrftoken`, `ds_user_id`), cookie paths, and Windows user home paths.
pub fn redact_sensitive(input: &str) -> String {
    static COOKIE_RE: OnceLock<Regex> = OnceLock::new();
    static USER_PATH_RE: OnceLock<Regex> = OnceLock::new();
    static COOKIE_ARG_RE: OnceLock<Regex> = OnceLock::new();

    let cookie_re = COOKIE_RE.get_or_init(|| {
        Regex::new(r"(?i)(sessionid|csrftoken|ds_user_id|mid|ig_did|rur|Bearer)\s*[:=]\s*[^\s;&\x22']+").unwrap()
    });
    let user_path_re = USER_PATH_RE.get_or_init(|| {
        Regex::new(r"(?i)([A-Z]:\\Users\\)([^\\/]+)").unwrap()
    });
    let cookie_arg_re = COOKIE_ARG_RE.get_or_init(|| {
        Regex::new(r"(?i)(--cookies\s+)([^\s]+)").unwrap()
    });

    let step1 = cookie_re.replace_all(input, "$1=[REDACTED]");
    let step2 = user_path_re.replace_all(&step1, "${1}<REDACTED_USER>");
    let step3 = cookie_arg_re.replace_all(&step2, "${1}<EPHEMERAL_COOKIE_PIPE>");
    step3.into_owned()
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_sensitive_tokens_and_paths() {
        let raw = r#"Error running --cookies C:\Users\Alice\AppData\Local\Temp\ig_cookie_123.txt with sessionid=72819381%3Aabcd; csrftoken=xyz987"#;
        let scrubbed = redact_sensitive(raw);
        assert!(!scrubbed.contains("72819381"));
        assert!(!scrubbed.contains("xyz987"));
        assert!(!scrubbed.contains("Alice"));
        assert!(scrubbed.contains("sessionid=[REDACTED]"));
        assert!(scrubbed.contains("csrftoken=[REDACTED]"));
    }

    #[test]
    fn test_error_classification_from_stderr() {
        let err1 = AppError::from_engine_stderr("ERROR: [Instagram] HTTP Error 429: Too Many Requests");
        assert!(matches!(err1, AppError::RateLimited { .. }));

        let err2 = AppError::from_engine_stderr("gallery-dl: [instagram][error] HttpError: '401 Unauthorized' - login_required");
        assert!(matches!(err2, AppError::AuthenticationRequired));

        let err3 = AppError::from_engine_stderr("ERROR: This account is private");
        assert!(matches!(err3, AppError::PrivateAccount));
    }
}
