use crate::utils::sanitize_text_for_log;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClassifiedError {
    /// Stable canonical error code (e.g. NETWORK_ERROR, INVALID_URL, ENGINE_INCOMPATIBLE)
    pub code: String,
    /// Legacy/UI category alias for backward compatibility
    pub category: String,
    /// User-friendly explanation
    pub user_message: String,
    /// Sanitized technical details
    pub technical_details: String,
    /// Recommended recovery action for the user
    pub recovery_action: String,
    /// Whether automatic or manual retry is appropriate without changing settings
    pub retryable: bool,
    /// Whether this error indicates a provider-side extractor breakage that may be fixed by updating yt-dlp
    pub suggest_engine_update: bool,
}

pub fn classify_error(raw_error: &str) -> ClassifiedError {
    let sanitized_tech = sanitize_text_for_log(raw_error.trim());
    let lower = raw_error.to_lowercase();

    if lower.contains("cancelled by user") || lower.contains("download_cancelled") {
        return ClassifiedError {
            code: "DOWNLOAD_CANCELLED".to_string(),
            category: "cancelled".to_string(),
            user_message: "Download was cancelled.".to_string(),
            technical_details: sanitized_tech,
            recovery_action: "Click Retry if you want to restart the download.".to_string(),
            retryable: true,
            suggest_engine_update: false,
        };
    }

    if lower.contains("unsupported url") || lower.contains("no suitable infoextractor") {
        return ClassifiedError {
            code: "UNSUPPORTED_PROVIDER".to_string(),
            category: "invalid_url".to_string(),
            user_message: "That doesn't look like a supported video link.".to_string(),
            technical_details: sanitized_tech,
            recovery_action: "Paste a direct link to a supported video or audio page.".to_string(),
            retryable: false,
            suggest_engine_update: false,
        };
    }

    if lower.contains("is not a valid url") || lower.contains("invalid url") {
        return ClassifiedError {
            code: "INVALID_URL".to_string(),
            category: "invalid_url".to_string(),
            user_message: "That doesn't look like a supported video link.".to_string(),
            technical_details: sanitized_tech,
            recovery_action: "Verify that the URL starts with https:// and points to a valid page."
                .to_string(),
            retryable: false,
            suggest_engine_update: false,
        };
    }

    if lower.contains("sign in to confirm")
        || lower.contains("private video")
        || lower.contains("members-only")
        || lower.contains("login required")
        || lower.contains("cookies")
        || lower.contains("authentication")
        || lower.contains("drm protected")
    {
        return ClassifiedError {
            code: "AUTH_REQUIRED".to_string(),
            category: "auth_required".to_string(),
            user_message:
                "This video requires authentication. FlowDL cannot access it with the current session."
                    .to_string(),
            technical_details: sanitized_tech,
            recovery_action:
                "Use a publicly accessible video link. Private, members-only, and DRM-protected streams cannot be downloaded without authorization."
                    .to_string(),
            retryable: false,
            suggest_engine_update: false,
        };
    }

    if lower.contains("video unavailable")
        || lower.contains("has been removed")
        || lower.contains("account associated with this video has been terminated")
        || lower.contains("http error 404")
        || lower.contains("not available in your country")
    {
        return ClassifiedError {
            code: "VIDEO_UNAVAILABLE".to_string(),
            category: "unavailable".to_string(),
            user_message: "This video is unavailable or may have been removed.".to_string(),
            technical_details: sanitized_tech,
            recovery_action:
                "Check if the video still plays in your browser and is available in your region."
                    .to_string(),
            retryable: false,
            suggest_engine_update: false,
        };
    }

    if lower.contains("ffmpeg not found")
        || lower.contains("ffprobe and ffmpeg not found")
        || lower.contains("merging of multiple formats requires ffmpeg")
        || lower.contains("ffmpeg is not installed")
        || lower.contains("missing_ffmpeg")
        || lower.contains("missing_ytdlp")
    {
        return ClassifiedError {
            code: "DEPENDENCY_MISSING".to_string(),
            category: "missing_ffmpeg".to_string(),
            user_message: "FFmpeg is needed to merge this download. Install it now?".to_string(),
            technical_details: sanitized_tech,
            recovery_action:
                "Open Settings -> Dependency Manager and click Repair / Re-verify Bundled Engines."
                    .to_string(),
            retryable: false,
            suggest_engine_update: false,
        };
    }

    if lower.contains("requested format is not available")
        || lower.contains("no video formats found")
    {
        return ClassifiedError {
            code: "FORMAT_UNAVAILABLE".to_string(),
            category: "unsupported_resolution".to_string(),
            user_message:
                "The selected resolution or format isn't available for this video. Choose another quality option."
                    .to_string(),
            technical_details: sanitized_tech,
            recovery_action:
                "Re-analyze the URL or select 'Best available' quality and 'Auto' container."
                    .to_string(),
            retryable: false,
            suggest_engine_update: false,
        };
    }

    if lower.contains("unable to extract")
        || lower.contains("nsig extraction failed")
        || lower.contains("signature extraction failed")
        || lower.contains("player_response")
        || lower.contains("please report this issue on https://github.com/yt-dlp/yt-dlp")
        || lower.contains("http error 403: forbidden")
    {
        return ClassifiedError {
            code: "ENGINE_INCOMPATIBLE".to_string(),
            category: "engine_incompatible".to_string(),
            user_message:
                "The video provider changed its playback format. Updating the download engine may fix this."
                    .to_string(),
            technical_details: sanitized_tech,
            recovery_action:
                "Click 'Update Engine & Retry' or open Settings -> Check for Engine Updates to fetch the latest yt-dlp extractor."
                    .to_string(),
            retryable: true,
            suggest_engine_update: true,
        };
    }

    if lower.contains("no space left on device")
        || lower.contains("there is not enough space on the disk")
        || lower.contains("os error 112")
    {
        return ClassifiedError {
            code: "DISK_FULL".to_string(),
            category: "disk_space".to_string(),
            user_message: "Not enough storage space in your selected download folder.".to_string(),
            technical_details: sanitized_tech,
            recovery_action:
                "Free up disk space or choose a download folder on another drive in Settings."
                    .to_string(),
            retryable: false,
            suggest_engine_update: false,
        };
    }

    if lower.contains("permission denied")
        || lower.contains("access is denied")
        || lower.contains("os error 5")
    {
        return ClassifiedError {
            code: "PERMISSION_DENIED".to_string(),
            category: "permission_denied".to_string(),
            user_message:
                "FlowDL does not have permission to write to the selected download folder."
                    .to_string(),
            technical_details: sanitized_tech,
            recovery_action:
                "Select a user-writable folder (such as your Downloads folder) in Settings."
                    .to_string(),
            retryable: false,
            suggest_engine_update: false,
        };
    }

    if lower.contains("postprocessing:")
        || lower.contains("error opening output file")
        || lower.contains("conversion failed")
        || lower.contains("exited with code")
    {
        return ClassifiedError {
            code: "POSTPROCESSING_FAILED".to_string(),
            category: "postprocessing_failed".to_string(),
            user_message:
                "Media merging or audio conversion failed during FFmpeg post-processing."
                    .to_string(),
            technical_details: sanitized_tech,
            recovery_action:
                "Verify FFmpeg health in Settings or try selecting 'Auto' container format."
                    .to_string(),
            retryable: true,
            suggest_engine_update: false,
        };
    }

    if lower.contains("getaddrinfo failed")
        || lower.contains("timed out")
        || lower.contains("connection reset")
        || lower.contains("network is unreachable")
        || lower.contains("unable to download webpage")
        || lower.contains("temporary failure in name resolution")
    {
        return ClassifiedError {
            code: "NETWORK_ERROR".to_string(),
            category: "network".to_string(),
            user_message: "Couldn't connect. Check your internet connection and try again."
                .to_string(),
            technical_details: sanitized_tech,
            recovery_action:
                "Check your network connection, firewall, or VPN and click Retry.".to_string(),
            retryable: true,
            suggest_engine_update: false,
        };
    }

    ClassifiedError {
        code: "UNKNOWN_ERROR".to_string(),
        category: "unknown".to_string(),
        user_message: "Download couldn't be completed. Expand diagnostic details for more info."
            .to_string(),
        technical_details: sanitized_tech,
        recovery_action:
            "Review the diagnostic details or check for engine updates in Settings.".to_string(),
        retryable: true,
        suggest_engine_update: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_invalid_url() {
        let err = classify_error("ERROR: Unsupported URL: https://example.com/not-a-video");
        assert_eq!(err.code, "UNSUPPORTED_PROVIDER");
        assert_eq!(err.category, "invalid_url");
        assert_eq!(
            err.user_message,
            "That doesn't look like a supported video link."
        );
        assert!(!err.retryable);
        assert!(!err.suggest_engine_update);
    }

    #[test]
    fn test_classify_network_error() {
        let err = classify_error("ERROR: Unable to download webpage: <urlopen error [Errno 11001] getaddrinfo failed>");
        assert_eq!(err.code, "NETWORK_ERROR");
        assert_eq!(err.category, "network");
        assert!(err.retryable);
        assert!(!err.suggest_engine_update);
    }

    #[test]
    fn test_classify_missing_ffmpeg() {
        let err = classify_error("ERROR: You have requested merging of multiple formats but ffmpeg is not installed. Aborting due to --abort-on-error");
        assert_eq!(err.code, "DEPENDENCY_MISSING");
        assert_eq!(err.category, "missing_ffmpeg");
    }

    #[test]
    fn test_classify_engine_incompatible_provider_change() {
        let err = classify_error(
            "ERROR: [youtube] dQw4w9WgXcQ: Signature extraction failed: Traceback (most recent call last); please report this issue on https://github.com/yt-dlp/yt-dlp",
        );
        assert_eq!(err.code, "ENGINE_INCOMPATIBLE");
        assert_eq!(err.category, "engine_incompatible");
        assert!(err.suggest_engine_update);
        assert!(err.retryable);
    }

    #[test]
    fn test_classify_permission_and_disk_full() {
        let disk = classify_error("ERROR: [Errno 28] No space left on device");
        assert_eq!(disk.code, "DISK_FULL");
        assert!(!disk.retryable);

        let perm = classify_error("ERROR: unable to open for writing: [Errno 13] Permission denied");
        assert_eq!(perm.code, "PERMISSION_DENIED");
        assert!(!perm.retryable);
    }
}
