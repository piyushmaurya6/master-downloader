use regex::Regex;
use serde::{Serialize, Serializer};
use std::sync::OnceLock;
use thiserror::Error;

/// Unified application error taxonomy for FlowFB.
/// Serializes into a structured payload `{ code, user_message, technical_details, remediation_action }`
/// so the React frontend can display actionable recovery CTAs.
#[derive(Debug, Error, Clone)]
pub enum AppError {
    #[error("Facebook rate limit reached")]
    RateLimited { retry_after_secs: Option<u64> },

    #[error("Facebook authentication is required to access this media")]
    AuthenticationRequired,

    #[error("This Facebook video or post is private or restricted to an audience you cannot access")]
    PrivateAccount,

    #[error("Media has been deleted, expired, or is no longer available on Facebook")]
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

    #[error("Invalid or unsupported Facebook URL: {0}")]
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
                    "Facebook temporarily rate-limited requests (HTTP 429 / Security Checkpoint). Queue paused for {}s to protect your session.",
                    retry_after_secs.unwrap_or(120)
                ),
                technical_details: redact_sensitive(&self.to_string()),
                remediation_action: "WAIT_COOLDOWN",
                retry_after_secs: *retry_after_secs,
            },
            AppError::AuthenticationRequired => SerializedAppError {
                code: "AUTHENTICATION_REQUIRED",
                user_message: "Facebook requires an authenticated session to inspect or download this Reel, Watch video, or age-restricted post.".to_string(),
                technical_details: "login_required / c_user & xs session cookies missing or expired".to_string(),
                remediation_action: "CONNECT_ACCOUNT",
                retry_after_secs: None,
            },
            AppError::PrivateAccount => SerializedAppError {
                code: "PRIVATE_POST",
                user_message: "This Facebook post or video is private or restricted to friends/groups. FlowFB only downloads media that your connected account is authorized to view.".to_string(),
                technical_details: "Private or restricted Facebook audience".to_string(),
                remediation_action: "CHECK_PERMISSIONS",
                retry_after_secs: None,
            },
            AppError::MediaDeletedOrUnavailable => SerializedAppError {
                code: "MEDIA_UNAVAILABLE",
                user_message: "This Facebook video, Reel, or post could not be found. It may have been removed or made private.".to_string(),
                technical_details: "HTTP 404 / Media unavailable".to_string(),
                remediation_action: "VERIFY_URL",
                retry_after_secs: None,
            },
            AppError::BinaryMissing(bin) => SerializedAppError {
                code: "BINARY_MISSING",
                user_message: format!(
                    "Required media engine ({}) is not installed in %LOCALAPPDATA%\\FlowFB\\bin.",
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
                remediation_action: "RETRY_BOOTSTRAP",
                retry_after_secs: None,
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
                user_message: "Unable to reach Facebook or GitHub servers. Check your internet connection or proxy settings.".to_string(),
                technical_details: "Network request timed out or DNS resolution failed".to_string(),
                remediation_action: "CHECK_NETWORK",
                retry_after_secs: None,
            },
            AppError::EngineParseError(details) => SerializedAppError {
                code: "ENGINE_PARSE_ERROR",
                user_message: "Could not parse Facebook DASH manifest or metadata from the extraction engine. Updating engines or connecting your session may resolve this.".to_string(),
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
                user_message: "Please enter a valid Facebook link (Watch Video, Reel, fb.watch, Share Link, Story, or Post).".to_string(),
                technical_details: redact_sensitive(details),
                remediation_action: "FIX_URL",
                retry_after_secs: None,
            },
            AppError::VaultError(details) => SerializedAppError {
                code: "VAULT_ERROR",
                user_message: "Could not access the encrypted Facebook session vault or Windows Credential Manager.".to_string(),
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
            || lower.contains("checkpoint")
            || lower.contains("temporarily blocked")
            || lower.contains("please wait a few minutes")
        {
            return AppError::RateLimited {
                retry_after_secs: Some(180),
            };
        }
        if lower.contains("registered users")
            || lower.contains("login_required")
            || lower.contains("login required")
            || lower.contains("authentication required")
            || lower.contains("cookies-from-browser")
            || lower.contains("you must log in")
            || lower.contains("log in to continue")
            || lower.contains("sign in to confirm")
            || lower.contains("not logged in")
            || lower.contains("401 unauthorized")
        {
            return AppError::AuthenticationRequired;
        }
        if lower.contains("private")
            || lower.contains("content isn't available right now")
            || lower.contains("only shared with a small group")
        {
            return AppError::PrivateAccount;
        }
        if lower.contains("404")
            || lower.contains("not found")
            || lower.contains("media unavailable")
            || lower.contains("video unavailable")
            || lower.contains("this video has been removed")
            || lower.contains("cannot parse data")
        {
            return AppError::MediaDeletedOrUnavailable;
        }
        if lower.contains("no space left")
            || lower.contains("os error 112")
            || lower.contains("disk full")
        {
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

/// Scrubs sensitive Facebook cookies (`c_user`, `xs`, `datr`, `fr`, `sb`, `sessionid`), cookie file paths, and Windows user home paths.
pub fn redact_sensitive(input: &str) -> String {
    static COOKIE_RE: OnceLock<Regex> = OnceLock::new();
    static USER_PATH_RE: OnceLock<Regex> = OnceLock::new();
    static COOKIE_ARG_RE: OnceLock<Regex> = OnceLock::new();

    let cookie_re = COOKIE_RE.get_or_init(|| {
        Regex::new(r"(?i)(c_user|xs|datr|fr|sb|wd|presence|sessionid|csrftoken|Bearer)\s*[:=]\s*[^\s;&\x22']+").unwrap()
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
    fn test_redact_sensitive_facebook_tokens_and_paths() {
        let raw = r#"Error running --cookies C:\Users\Alice\AppData\Local\Temp\fb_cookie_123.txt with c_user=100099887766; xs=44%3AsecretToken%3A2; datr=xyz987"#;
        let scrubbed = redact_sensitive(raw);
        assert!(!scrubbed.contains("100099887766"));
        assert!(!scrubbed.contains("secretToken"));
        assert!(!scrubbed.contains("xyz987"));
        assert!(!scrubbed.contains("Alice"));
        assert!(scrubbed.contains("c_user=[REDACTED]"));
        assert!(scrubbed.contains("xs=[REDACTED]"));
    }

    #[test]
    fn test_facebook_error_classification_from_stderr() {
        let err1 = AppError::from_engine_stderr("ERROR: [facebook] HTTP Error 429: Too Many Requests");
        assert!(matches!(err1, AppError::RateLimited { .. }));

        let err2 = AppError::from_engine_stderr(
            "ERROR: [facebook] 123456: This video is only available for registered users. Use --cookies",
        );
        assert!(matches!(err2, AppError::AuthenticationRequired));

        let err3 = AppError::from_engine_stderr(
            "ERROR: [facebook] This content isn't available right now",
        );
        assert!(matches!(err3, AppError::PrivateAccount));
    }
}
