use crate::error::{AppError, AppResult};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum InstagramUrlType {
    Reel(String),
    Post(String),
    Tv(String),
    Stories {
        username: String,
        story_id: Option<String>,
    },
    Highlight(String),
    Profile(String),
}

impl InstagramUrlType {
    pub fn label(&self) -> &'static str {
        match self {
            InstagramUrlType::Reel(_) => "REEL",
            InstagramUrlType::Post(_) => "POST",
            InstagramUrlType::Tv(_) => "TV",
            InstagramUrlType::Stories { .. } => "STORY",
            InstagramUrlType::Highlight(_) => "HIGHLIGHT",
            InstagramUrlType::Profile(_) => "PROFILE",
        }
    }

    pub fn kind_slug(&self) -> &'static str {
        match self {
            InstagramUrlType::Reel(_) => "reel",
            InstagramUrlType::Post(_) => "post",
            InstagramUrlType::Tv(_) => "tv",
            InstagramUrlType::Stories { .. } => "stories",
            InstagramUrlType::Highlight(_) => "highlight",
            InstagramUrlType::Profile(_) => "profile",
        }
    }

    pub fn shortcode_or_id(&self) -> String {
        match self {
            InstagramUrlType::Reel(sc)
            | InstagramUrlType::Post(sc)
            | InstagramUrlType::Tv(sc)
            | InstagramUrlType::Highlight(sc)
            | InstagramUrlType::Profile(sc) => sc.clone(),
            InstagramUrlType::Stories {
                username,
                story_id,
            } => story_id.clone().unwrap_or_else(|| username.clone()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NormalizedInstagramUrl {
    pub canonical_url: String,
    pub original_url: String,
    pub url_type: InstagramUrlType,
    pub type_badge: String,
    pub shortcode: Option<String>,
    pub username_hint: Option<String>,
    pub img_index_hint: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchUrlParseResult {
    pub valid_urls: Vec<NormalizedInstagramUrl>,
    pub invalid_lines: Vec<String>,
    pub duplicate_count: usize,
}

const RESERVED_SEGMENTS: &[&str] = &[
    "explore",
    "accounts",
    "direct",
    "about",
    "legal",
    "developer",
    "press",
    "api",
    "static",
    "challenge",
    "oauth",
];

/// Normalizes and classifies an Instagram URL, stripping tracking query parameters (`?igsh=...`, `?utm_source=...`)
/// while preserving `img_index` as an optional 1-based carousel slide hint.
pub fn normalize_instagram_url(raw_input: &str) -> AppResult<NormalizedInstagramUrl> {
    let trimmed = raw_input.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidUrl("Empty URL".to_string()));
    }

    let with_scheme = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else if trimmed.starts_with("instagram.com/") || trimmed.starts_with("www.instagram.com/") {
        format!("https://{}", trimmed)
    } else {
        return Err(AppError::InvalidUrl(format!(
            "Unsupported URL scheme: {}",
            trimmed
        )));
    };

    let parsed = Url::parse(&with_scheme)
        .map_err(|e| AppError::InvalidUrl(format!("Malformed URL '{}': {}", trimmed, e)))?;

    let host = parsed
        .host_str()
        .ok_or_else(|| AppError::InvalidUrl("Missing URL host".to_string()))?
        .to_lowercase();

    if host != "instagram.com"
        && host != "www.instagram.com"
        && host != "m.instagram.com"
        && host != "ddinstagram.com"
    {
        return Err(AppError::InvalidUrl(format!(
            "Not an Instagram domain: {}",
            host
        )));
    }

    // Extract optional img_index hint before stripping query parameters
    let mut img_index_hint: Option<usize> = None;
    for (k, v) in parsed.query_pairs() {
        if k.eq_ignore_ascii_case("img_index") {
            if let Ok(idx) = v.parse::<usize>() {
                if idx >= 1 {
                    img_index_hint = Some(idx);
                }
            }
        }
    }

    let segments: Vec<&str> = parsed
        .path_segments()
        .map(|s| s.filter(|seg| !seg.is_empty()).collect())
        .unwrap_or_default();

    if segments.is_empty() {
        return Err(AppError::InvalidUrl(
            "Instagram homepage has no downloadable media".to_string(),
        ));
    }

    let (url_type, canonical_path, shortcode, username_hint) = match segments.as_slice() {
        // /reel/{shortcode} or /reels/{shortcode}
        ["reel" | "reels", code, ..] if is_valid_shortcode(code) => (
            InstagramUrlType::Reel((*code).to_string()),
            format!("/reel/{}/", code),
            Some((*code).to_string()),
            None,
        ),
        // /p/{shortcode}
        ["p", code, ..] if is_valid_shortcode(code) => (
            InstagramUrlType::Post((*code).to_string()),
            format!("/p/{}/", code),
            Some((*code).to_string()),
            None,
        ),
        // /tv/{shortcode}
        ["tv", code, ..] if is_valid_shortcode(code) => (
            InstagramUrlType::Tv((*code).to_string()),
            format!("/tv/{}/", code),
            Some((*code).to_string()),
            None,
        ),
        // /stories/highlights/{id}
        ["stories", "highlights", highlight_id, ..] if !highlight_id.is_empty() => (
            InstagramUrlType::Highlight((*highlight_id).to_string()),
            format!("/stories/highlights/{}/", highlight_id),
            Some((*highlight_id).to_string()),
            None,
        ),
        // /s/{base64_highlight}
        ["s", highlight_code, ..] if !highlight_code.is_empty() => (
            InstagramUrlType::Highlight((*highlight_code).to_string()),
            format!("/s/{}/", highlight_code),
            Some((*highlight_code).to_string()),
            None,
        ),
        // /stories/{username}/{story_id?}
        ["stories", username, rest @ ..] if is_valid_username(username) => {
            let story_id = rest.first().map(|s| (*s).to_string());
            let canonical = match &story_id {
                Some(sid) => format!("/stories/{}/{}/", username, sid),
                None => format!("/stories/{}/", username),
            };
            (
                InstagramUrlType::Stories {
                    username: (*username).to_string(),
                    story_id: story_id.clone(),
                },
                canonical,
                story_id,
                Some((*username).to_string()),
            )
        }
        // /{username}/p/{shortcode}
        [username, "p", code, ..] if is_valid_username(username) && is_valid_shortcode(code) => (
            InstagramUrlType::Post((*code).to_string()),
            format!("/p/{}/", code),
            Some((*code).to_string()),
            Some((*username).to_string()),
        ),
        // /{username}/reel/{shortcode}
        [username, "reel" | "reels", code, ..]
            if is_valid_username(username) && is_valid_shortcode(code) =>
        {
            (
                InstagramUrlType::Reel((*code).to_string()),
                format!("/reel/{}/", code),
                Some((*code).to_string()),
                Some((*username).to_string()),
            )
        }
        // /{username}
        [username]
            if is_valid_username(username)
                && !RESERVED_SEGMENTS.contains(&username.to_lowercase().as_str()) =>
        {
            (
                InstagramUrlType::Profile((*username).to_string()),
                format!("/{}/", username),
                None,
                Some((*username).to_string()),
            )
        }
        _ => {
            return Err(AppError::InvalidUrl(format!(
                "Unrecognized Instagram path: {}",
                parsed.path()
            )))
        }
    };

    let canonical_url = format!("https://www.instagram.com{}", canonical_path);
    let type_badge = url_type.label().to_string();

    Ok(NormalizedInstagramUrl {
        canonical_url,
        original_url: trimmed.to_string(),
        url_type,
        type_badge,
        shortcode,
        username_hint,
        img_index_hint,
    })
}

/// Parses a multi-line or freeform text block into deduplicated valid Instagram URLs and invalid lines.
pub fn parse_batch_urls(raw_text: &str) -> BatchUrlParseResult {
    static URL_FINDER_RE: OnceLock<Regex> = OnceLock::new();
    let finder = URL_FINDER_RE.get_or_init(|| {
        Regex::new(r"(?i)(?:https?://)?(?:www\.|m\.)?instagram\.com/[^\s,;,\x22'<>]+").unwrap()
    });

    let mut seen_canonical = HashSet::new();
    let mut valid_urls = Vec::new();
    let mut invalid_lines = Vec::new();
    let mut duplicate_count = 0usize;

    for raw_line in raw_text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut matched_in_line = false;
        for mat in finder.find_iter(line) {
            matched_in_line = true;
            match normalize_instagram_url(mat.as_str()) {
                Ok(norm) => {
                    let dedup_key = match norm.img_index_hint {
                        Some(idx) => format!("{}#{}", norm.canonical_url, idx),
                        None => norm.canonical_url.clone(),
                    };
                    if seen_canonical.insert(dedup_key) {
                        valid_urls.push(norm);
                    } else {
                        duplicate_count += 1;
                    }
                }
                Err(_) => {
                    invalid_lines.push(mat.as_str().to_string());
                }
            }
        }

        if !matched_in_line {
            match normalize_instagram_url(line) {
                Ok(norm) => {
                    if seen_canonical.insert(norm.canonical_url.clone()) {
                        valid_urls.push(norm);
                    } else {
                        duplicate_count += 1;
                    }
                }
                Err(_) => {
                    invalid_lines.push(line.to_string());
                }
            }
        }
    }

    BatchUrlParseResult {
        valid_urls,
        invalid_lines,
        duplicate_count,
    }
}

fn is_valid_shortcode(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn is_valid_username(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 35
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
}

/// Sanitizes a single filename or directory component for Windows NTFS safety.
pub fn sanitize_windows_component(raw: &str) -> String {
    let mut cleaned: String = raw
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' | '\0'..='\x1F' => '_',
            _ => c,
        })
        .collect();

    // Collapse multiple spaces or underscores
    cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = cleaned.trim().trim_end_matches(['.', ' ']).to_string();

    if trimmed.is_empty() {
        return "untitled".to_string();
    }

    // Check Windows reserved device names
    let upper = trimmed
        .split('.')
        .next()
        .unwrap_or("")
        .to_ascii_uppercase();
    let is_reserved = matches!(
        upper.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    );

    let safe = if is_reserved {
        format!("_{}", trimmed)
    } else {
        trimmed
    };

    safe.chars().take(80).collect()
}

/// Builds a sanitized Windows file path from a user template (`{username}/{date}_{shortcode}_{index}.{ext}`),
/// ensuring total path length never exceeds 238 characters (`MAX_PATH` protection).
pub fn resolve_output_path(
    base_dir: &Path,
    template: &str,
    username: Option<&str>,
    shortcode: Option<&str>,
    date_str: &str,
    slide_index: usize,
    ext: &str,
) -> PathBuf {
    let safe_user = sanitize_windows_component(username.unwrap_or("instagram_user"));
    let safe_code = sanitize_windows_component(shortcode.unwrap_or("media"));
    let safe_date = sanitize_windows_component(date_str);
    let safe_ext = ext.trim_start_matches('.').to_lowercase();
    let idx_str = format!("{:02}", slide_index.max(1));

    let expanded = template
        .replace("{username}", &safe_user)
        .replace("{shortcode}", &safe_code)
        .replace("{date}", &safe_date)
        .replace("{index}", &idx_str)
        .replace("{ext}", &safe_ext);

    let parts: Vec<String> = expanded
        .split(['/', '\\'])
        .filter(|p| !p.trim().is_empty() && *p != ".." && *p != ".")
        .map(sanitize_windows_component)
        .collect();

    let mut full = base_dir.to_path_buf();
    for part in parts {
        full.push(part);
    }

    // Ensure extension is present
    if full.extension().is_none() && !safe_ext.is_empty() {
        full.set_extension(&safe_ext);
    }

    // Cap total path length below 238 chars for Windows MAX_PATH safety
    let full_str = full.to_string_lossy().to_string();
    if full_str.len() > 238 {
        let fallback_name = format!("{}_{}.{}", safe_code, idx_str, safe_ext);
        base_dir.join(fallback_name)
    } else {
        full
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_all_instagram_url_permutations() {
        // 1. Reel with tracking params
        let r1 = normalize_instagram_url(
            "https://www.instagram.com/reel/C8abc_123/?igsh=MzRlODBiNWFlZA==&utm_source=ig_web_copy_link",
        )
        .unwrap();
        assert_eq!(r1.canonical_url, "https://www.instagram.com/reel/C8abc_123/");
        assert_eq!(r1.url_type, InstagramUrlType::Reel("C8abc_123".to_string()));
        assert_eq!(r1.type_badge, "REEL");

        // 2. Post with username prefix and img_index hint
        let p1 = normalize_instagram_url(
            "https://instagram.com/natgeo/p/C9xyz987/?igsh=12345&img_index=4",
        )
        .unwrap();
        assert_eq!(p1.canonical_url, "https://www.instagram.com/p/C9xyz987/");
        assert_eq!(p1.url_type, InstagramUrlType::Post("C9xyz987".to_string()));
        assert_eq!(p1.username_hint.as_deref(), Some("natgeo"));
        assert_eq!(p1.img_index_hint, Some(4));

        // 3. Stories URL
        let s1 = normalize_instagram_url(
            "https://www.instagram.com/stories/design.daily/33918273618273/",
        )
        .unwrap();
        assert_eq!(
            s1.url_type,
            InstagramUrlType::Stories {
                username: "design.daily".to_string(),
                story_id: Some("33918273618273".to_string())
            }
        );

        // 4. Highlight URL
        let h1 = normalize_instagram_url(
            "https://www.instagram.com/stories/highlights/17945829102938/",
        )
        .unwrap();
        assert_eq!(
            h1.url_type,
            InstagramUrlType::Highlight("17945829102938".to_string())
        );

        // 5. Profile URL
        let prof = normalize_instagram_url("https://www.instagram.com/nasa/?hl=en").unwrap();
        assert_eq!(prof.url_type, InstagramUrlType::Profile("nasa".to_string()));

        // 6. Reject non-Instagram URL
        assert!(normalize_instagram_url("https://www.youtube.com/watch?v=123").is_err());
    }

    #[test]
    fn test_windows_filename_sanitization_and_max_path() {
        assert_eq!(sanitize_windows_component("CON"), "_CON");
        assert_eq!(sanitize_windows_component("NUL.txt"), "_NUL.txt");
        assert_eq!(
            sanitize_windows_component("Reel: <Best>*? \"Clip\" | 2026"),
            "Reel_ _Best___ _Clip_ _ 2026"
        );

        let out = resolve_output_path(
            Path::new("C:\\Users\\Test\\Downloads"),
            "{username}/{date}_{shortcode}_{index}.{ext}",
            Some("natgeo"),
            Some("C8abc_123"),
            "2026-11-02",
            3,
            "mp4",
        );
        assert!(out
            .to_string_lossy()
            .ends_with("natgeo\\2026-11-02_C8abc_123_03.mp4"));
    }

    #[test]
    fn test_batch_url_deduplication() {
        let batch_input = r#"
            https://www.instagram.com/reel/ABC1234/?igsh=aaa
            https://www.instagram.com/reel/ABC1234/?igsh=bbb
            https://www.instagram.com/p/XYZ9876/
            not_a_valid_link
        "#;
        let res = parse_batch_urls(batch_input);
        assert_eq!(res.valid_urls.len(), 2);
        assert_eq!(res.duplicate_count, 1);
        assert_eq!(res.invalid_lines.len(), 1);
    }
}
