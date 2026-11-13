use crate::error::{AppError, AppResult};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum FacebookUrlType {
    Reel(String),
    Watch(String),
    Video(String),
    Share {
        share_type: String, // "v" | "r" | "p" | "general"
        token: String,
    },
    Shortlink(String),
    Post(String),
    Story(String),
    Photo(String),
}

impl FacebookUrlType {
    pub fn label(&self) -> &'static str {
        match self {
            FacebookUrlType::Reel(_) => "REEL",
            FacebookUrlType::Watch(_) => "WATCH",
            FacebookUrlType::Video(_) => "VIDEO",
            FacebookUrlType::Share { share_type, .. } => match share_type.as_str() {
                "r" => "REEL",
                "v" => "VIDEO",
                _ => "SHARE",
            },
            FacebookUrlType::Shortlink(_) => "FB.WATCH",
            FacebookUrlType::Post(_) => "POST",
            FacebookUrlType::Story(_) => "STORY",
            FacebookUrlType::Photo(_) => "PHOTO",
        }
    }

    pub fn kind_slug(&self) -> &'static str {
        match self {
            FacebookUrlType::Reel(_) => "reel",
            FacebookUrlType::Watch(_) => "watch",
            FacebookUrlType::Video(_) => "video",
            FacebookUrlType::Share { share_type, .. } => match share_type.as_str() {
                "r" => "reel",
                "v" => "video",
                _ => "post",
            },
            FacebookUrlType::Shortlink(_) => "watch",
            FacebookUrlType::Post(_) => "post",
            FacebookUrlType::Story(_) => "story",
            FacebookUrlType::Photo(_) => "photo",
        }
    }

    pub fn id_or_token(&self) -> String {
        match self {
            FacebookUrlType::Reel(id)
            | FacebookUrlType::Watch(id)
            | FacebookUrlType::Video(id)
            | FacebookUrlType::Shortlink(id)
            | FacebookUrlType::Post(id)
            | FacebookUrlType::Story(id)
            | FacebookUrlType::Photo(id) => id.clone(),
            FacebookUrlType::Share { token, .. } => token.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NormalizedFacebookUrl {
    pub canonical_url: String,
    pub original_url: String,
    pub url_type: FacebookUrlType,
    pub type_badge: String,
    pub shortcode: Option<String>,
    pub username_hint: Option<String>,
    pub img_index_hint: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchUrlParseResult {
    pub valid_urls: Vec<NormalizedFacebookUrl>,
    pub invalid_lines: Vec<String>,
    pub duplicate_count: usize,
}

const RESERVED_SEGMENTS: &[&str] = &[
    "login",
    "recover",
    "help",
    "policies",
    "privacy",
    "marketplace",
    "gaming",
    "friends",
    "bookmarks",
    "notifications",
    "messages",
    "settings",
    "ads",
    "business",
    "dialog",
    "plugins",
];

fn is_supported_facebook_host(host: &str) -> bool {
    matches!(
        host,
        "facebook.com"
            | "www.facebook.com"
            | "m.facebook.com"
            | "web.facebook.com"
            | "touch.facebook.com"
            | "mbasic.facebook.com"
            | "fb.watch"
            | "www.fb.watch"
            | "fb.com"
            | "www.fb.com"
    )
}

/// Normalizes and classifies any supported Facebook video, Reel, Watch, `fb.watch`, `/share/`, story, or post URL,
/// stripping tracking query parameters (`mibextid`, `rdid`, `share_url`, `__cft__`, `__tn__`) while preserving
/// canonical media identifiers (`v`, `story_fbid`, `id`, `fbid`, `set`).
pub fn normalize_facebook_url(raw_input: &str) -> AppResult<NormalizedFacebookUrl> {
    let trimmed = raw_input.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidUrl("Empty URL".to_string()));
    }

    let with_scheme = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else if trimmed.starts_with("facebook.com/")
        || trimmed.starts_with("www.facebook.com/")
        || trimmed.starts_with("m.facebook.com/")
        || trimmed.starts_with("fb.watch/")
        || trimmed.starts_with("fb.com/")
    {
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

    if !is_supported_facebook_host(&host) {
        return Err(AppError::InvalidUrl(format!(
            "Not a Facebook domain: {}",
            host
        )));
    }

    let segments: Vec<&str> = parsed
        .path_segments()
        .map(|s| s.filter(|seg| !seg.is_empty()).collect())
        .unwrap_or_default();

    // Extract key query parameters before stripping tracking params
    let mut query_v: Option<String> = None;
    let mut query_story_fbid: Option<String> = None;
    let mut query_id: Option<String> = None;
    let mut query_fbid: Option<String> = None;
    let mut query_set: Option<String> = None;

    for (k, v) in parsed.query_pairs() {
        let key_lower = k.to_lowercase();
        let val = v.trim().to_string();
        if val.is_empty() {
            continue;
        }
        match key_lower.as_str() {
            "v" => query_v = Some(val),
            "story_fbid" => query_story_fbid = Some(val),
            "id" => query_id = Some(val),
            "fbid" => query_fbid = Some(val),
            "set" => query_set = Some(val),
            _ => {}
        }
    }

    // 1. Handle fb.watch shortlinks: https://fb.watch/{token}/
    if host == "fb.watch" || host == "www.fb.watch" {
        let token = segments
            .first()
            .copied()
            .filter(|s| is_valid_token(s))
            .ok_or_else(|| AppError::InvalidUrl("Missing fb.watch video token".to_string()))?;

        let canonical_url = format!("https://fb.watch/{}/", token);
        let url_type = FacebookUrlType::Shortlink(token.to_string());
        let type_badge = url_type.label().to_string();
        return Ok(NormalizedFacebookUrl {
            canonical_url,
            original_url: trimmed.to_string(),
            url_type,
            type_badge,
            shortcode: Some(token.to_string()),
            username_hint: None,
            img_index_hint: None,
        });
    }

    // 2. Query-based endpoints: /watch/?v={id}, /video.php?v={id}, /story.php?story_fbid=..., /permalink.php, /photo
    match segments.as_slice() {
        ["watch"] | ["watch", "live"] => {
            if let Some(vid) = query_v {
                let canonical_url = format!("https://www.facebook.com/watch/?v={}", vid);
                let url_type = FacebookUrlType::Watch(vid.clone());
                let type_badge = url_type.label().to_string();
                return Ok(NormalizedFacebookUrl {
                    canonical_url,
                    original_url: trimmed.to_string(),
                    url_type,
                    type_badge,
                    shortcode: Some(vid),
                    username_hint: None,
                    img_index_hint: None,
                });
            }
        }
        ["video.php"] => {
            if let Some(vid) = query_v {
                let canonical_url = format!("https://www.facebook.com/watch/?v={}", vid);
                let url_type = FacebookUrlType::Video(vid.clone());
                let type_badge = url_type.label().to_string();
                return Ok(NormalizedFacebookUrl {
                    canonical_url,
                    original_url: trimmed.to_string(),
                    url_type,
                    type_badge,
                    shortcode: Some(vid),
                    username_hint: None,
                    img_index_hint: None,
                });
            }
        }
        ["story.php"] | ["permalink.php"] => {
            if let Some(s_fbid) = query_story_fbid {
                let page_id = query_id.unwrap_or_else(|| "0".to_string());
                let canonical_url = format!(
                    "https://www.facebook.com/story.php?story_fbid={}&id={}",
                    s_fbid, page_id
                );
                let url_type = FacebookUrlType::Post(s_fbid.clone());
                let type_badge = url_type.label().to_string();
                return Ok(NormalizedFacebookUrl {
                    canonical_url,
                    original_url: trimmed.to_string(),
                    url_type,
                    type_badge,
                    shortcode: Some(s_fbid),
                    username_hint: if page_id != "0" { Some(page_id) } else { None },
                    img_index_hint: None,
                });
            }
        }
        ["photo"] | ["photo.php"] => {
            if let Some(fbid) = query_fbid {
                let canonical_url = if let Some(ref set_id) = query_set {
                    format!(
                        "https://www.facebook.com/photo/?fbid={}&set={}",
                        fbid, set_id
                    )
                } else {
                    format!("https://www.facebook.com/photo/?fbid={}", fbid)
                };
                let url_type = FacebookUrlType::Photo(fbid.clone());
                let type_badge = url_type.label().to_string();
                return Ok(NormalizedFacebookUrl {
                    canonical_url,
                    original_url: trimmed.to_string(),
                    url_type,
                    type_badge,
                    shortcode: Some(fbid),
                    username_hint: None,
                    img_index_hint: None,
                });
            }
        }
        ["media", "set"] => {
            if let Some(set_id) = query_set {
                let canonical_url =
                    format!("https://www.facebook.com/media/set/?set={}", set_id);
                let url_type = FacebookUrlType::Photo(set_id.clone());
                let type_badge = url_type.label().to_string();
                return Ok(NormalizedFacebookUrl {
                    canonical_url,
                    original_url: trimmed.to_string(),
                    url_type,
                    type_badge,
                    shortcode: Some(set_id),
                    username_hint: None,
                    img_index_hint: None,
                });
            }
        }
        _ => {}
    }

    if segments.is_empty() {
        return Err(AppError::InvalidUrl(
            "Facebook homepage has no downloadable media".to_string(),
        ));
    }

    // 3. Path-based routes
    let (url_type, canonical_path, shortcode, username_hint) = match segments.as_slice() {
        // /reel/{id} or /reels/{id}
        ["reel" | "reels", id, ..] if is_valid_token(id) => (
            FacebookUrlType::Reel((*id).to_string()),
            format!("/reel/{}/", id),
            Some((*id).to_string()),
            None,
        ),
        // /share/r/{token}, /share/v/{token}, /share/p/{token}
        ["share", share_kind @ ("r" | "v" | "p"), token, ..] if is_valid_token(token) => (
            FacebookUrlType::Share {
                share_type: (*share_kind).to_string(),
                token: (*token).to_string(),
            },
            format!("/share/{}/{}/", share_kind, token),
            Some((*token).to_string()),
            None,
        ),
        // /share/{token}
        ["share", token, ..] if is_valid_token(token) => (
            FacebookUrlType::Share {
                share_type: "general".to_string(),
                token: (*token).to_string(),
            },
            format!("/share/{}/", token),
            Some((*token).to_string()),
            None,
        ),
        // /watch/{id}
        ["watch", id, ..] if is_valid_token(id) && *id != "live" => (
            FacebookUrlType::Watch((*id).to_string()),
            format!("/watch/?v={}", id),
            Some((*id).to_string()),
            None,
        ),
        // /stories/{page_or_user}/{story_id?}
        ["stories", user, rest @ ..] if is_valid_token(user) => {
            let sid = rest.first().copied().unwrap_or(user);
            let path = if let Some(s) = rest.first() {
                format!("/stories/{}/{}/", user, s)
            } else {
                format!("/stories/{}/", user)
            };
            (
                FacebookUrlType::Story(sid.to_string()),
                path,
                Some(sid.to_string()),
                Some((*user).to_string()),
            )
        }
        // /groups/{group_id}/posts/{post_id} or /groups/{group_id}/permalink/{post_id}
        ["groups", group_id, "posts" | "permalink", post_id, ..]
            if is_valid_token(group_id) && is_valid_token(post_id) =>
        {
            (
                FacebookUrlType::Post((*post_id).to_string()),
                format!("/groups/{}/posts/{}/", group_id, post_id),
                Some((*post_id).to_string()),
                Some(format!("group_{}", group_id)),
            )
        }
        // /{user_or_page}/videos/{video_id}
        [user, "videos", video_id] if is_valid_username(user) && is_valid_token(video_id) => (
            FacebookUrlType::Video((*video_id).to_string()),
            format!("/{}/videos/{}/", user, video_id),
            Some((*video_id).to_string()),
            Some((*user).to_string()),
        ),
        // /{user_or_page}/videos/{slug}/{video_id}
        [user, "videos", _slug, video_id, ..]
            if is_valid_username(user) && is_valid_token(video_id) =>
        {
            (
                FacebookUrlType::Video((*video_id).to_string()),
                format!("/{}/videos/{}/", user, video_id),
                Some((*video_id).to_string()),
                Some((*user).to_string()),
            )
        }
        // /{user_or_page}/reel/{reel_id}
        [user, "reel" | "reels", reel_id, ..]
            if is_valid_username(user) && is_valid_token(reel_id) =>
        {
            (
                FacebookUrlType::Reel((*reel_id).to_string()),
                format!("/reel/{}/", reel_id),
                Some((*reel_id).to_string()),
                Some((*user).to_string()),
            )
        }
        // /{user_or_page}/posts/{post_id}
        [user, "posts", post_id, ..] if is_valid_username(user) && is_valid_token(post_id) => (
            FacebookUrlType::Post((*post_id).to_string()),
            format!("/{}/posts/{}/", user, post_id),
            Some((*post_id).to_string()),
            Some((*user).to_string()),
        ),
        // /{user_or_page}/photos/{a.album}/{photo_id} or /{user_or_page}/photos/{photo_id}
        [user, "photos", rest @ ..] if is_valid_username(user) && !rest.is_empty() => {
            let photo_id = rest.last().copied().unwrap_or("photo");
            let joined = rest.join("/");
            (
                FacebookUrlType::Photo(photo_id.to_string()),
                format!("/{}/photos/{}/", user, joined),
                Some(photo_id.to_string()),
                Some((*user).to_string()),
            )
        }
        _ => {
            return Err(AppError::InvalidUrl(format!(
                "Unrecognized Facebook media path: {}",
                parsed.path()
            )))
        }
    };

    let canonical_url = format!("https://www.facebook.com{}", canonical_path);
    let type_badge = url_type.label().to_string();

    Ok(NormalizedFacebookUrl {
        canonical_url,
        original_url: trimmed.to_string(),
        url_type,
        type_badge,
        shortcode,
        username_hint,
        img_index_hint: None,
    })
}

/// Parses a multi-line or freeform text block into deduplicated valid Facebook URLs and invalid lines.
pub fn parse_batch_urls(raw_text: &str) -> BatchUrlParseResult {
    static URL_FINDER_RE: OnceLock<Regex> = OnceLock::new();
    let finder = URL_FINDER_RE.get_or_init(|| {
        Regex::new(
            r"(?i)(?:https?://)?(?:(?:www|m|web|touch)\.facebook\.com|facebook\.com|fb\.watch|fb\.com)/[^\s,;,\x22'<>]+",
        )
        .unwrap()
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
            match normalize_facebook_url(mat.as_str()) {
                Ok(norm) => {
                    if seen_canonical.insert(norm.canonical_url.clone()) {
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
            match normalize_facebook_url(line) {
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

fn is_valid_token(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
}

fn is_valid_username(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && !RESERVED_SEGMENTS.contains(&s.to_lowercase().as_str())
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-')
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

    cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = cleaned.trim().trim_end_matches(['.', ' ']).to_string();

    if trimmed.is_empty() {
        return "untitled".to_string();
    }

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
    let safe_user = sanitize_windows_component(username.unwrap_or("facebook_creator"));
    let safe_code = sanitize_windows_component(shortcode.unwrap_or("fb_media"));
    let safe_date = sanitize_windows_component(date_str);
    let safe_ext = ext.trim_start_matches('.').to_lowercase();
    let idx_str = format!("{:02}", slide_index.max(1));

    let expanded = template
        .replace("{username}", &safe_user)
        .replace("{uploader}", &safe_user)
        .replace("{shortcode}", &safe_code)
        .replace("{id}", &safe_code)
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

    if full.extension().is_none() && !safe_ext.is_empty() {
        full.set_extension(&safe_ext);
    }

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
    fn test_normalize_all_facebook_url_permutations() {
        // 1. Facebook Reel with tracking params
        let r1 = normalize_facebook_url(
            "https://www.facebook.com/reel/123456789012345/?mibextid=rS40aB7S9Ucbxw6v",
        )
        .unwrap();
        assert_eq!(
            r1.canonical_url,
            "https://www.facebook.com/reel/123456789012345/"
        );
        assert_eq!(
            r1.url_type,
            FacebookUrlType::Reel("123456789012345".to_string())
        );
        assert_eq!(r1.type_badge, "REEL");

        // 2. Facebook Watch URL
        let w1 = normalize_facebook_url(
            "https://www.facebook.com/watch/?v=987654321098765&rdid=abc123xyz",
        )
        .unwrap();
        assert_eq!(
            w1.canonical_url,
            "https://www.facebook.com/watch/?v=987654321098765"
        );
        assert_eq!(
            w1.url_type,
            FacebookUrlType::Watch("987654321098765".to_string())
        );
        assert_eq!(w1.type_badge, "WATCH");

        // 3. fb.watch shortlink
        let fw = normalize_facebook_url("https://fb.watch/tXyZ987AbC/?mibextid=w85l2j").unwrap();
        assert_eq!(fw.canonical_url, "https://fb.watch/tXyZ987AbC/");
        assert_eq!(
            fw.url_type,
            FacebookUrlType::Shortlink("tXyZ987AbC".to_string())
        );
        assert_eq!(fw.type_badge, "FB.WATCH");

        // 4. Obfuscated Share Video & Share Reel Links
        let sv = normalize_facebook_url("https://www.facebook.com/share/v/19aBcDeFgH/?mibextid=WC7FNe").unwrap();
        assert_eq!(sv.canonical_url, "https://www.facebook.com/share/v/19aBcDeFgH/");
        assert_eq!(sv.type_badge, "VIDEO");

        let sr = normalize_facebook_url("https://www.facebook.com/share/r/15xYzReEl9/").unwrap();
        assert_eq!(sr.canonical_url, "https://www.facebook.com/share/r/15xYzReEl9/");
        assert_eq!(sr.type_badge, "REEL");

        // 5. Page Video URL with slug
        let pv = normalize_facebook_url(
            "https://www.facebook.com/NASA/videos/artemis-launch-highlights/55443322110099/",
        )
        .unwrap();
        assert_eq!(
            pv.canonical_url,
            "https://www.facebook.com/NASA/videos/55443322110099/"
        );
        assert_eq!(pv.username_hint.as_deref(), Some("NASA"));
        assert_eq!(pv.type_badge, "VIDEO");

        // 6. Story.php permalink
        let sp = normalize_facebook_url(
            "https://m.facebook.com/story.php?story_fbid=pfbid02abcDEF&id=100044556677&mibextid=Nif5oz",
        )
        .unwrap();
        assert_eq!(
            sp.canonical_url,
            "https://www.facebook.com/story.php?story_fbid=pfbid02abcDEF&id=100044556677"
        );
        assert_eq!(sp.type_badge, "POST");

        // 7. Reject non-Facebook URL
        assert!(normalize_facebook_url("https://www.youtube.com/watch?v=123").is_err());
    }

    #[test]
    fn test_windows_filename_sanitization_and_max_path() {
        assert_eq!(sanitize_windows_component("CON"), "_CON");
        assert_eq!(sanitize_windows_component("NUL.txt"), "_NUL.txt");
        assert_eq!(
            sanitize_windows_component("FB Reel: <Best>*? \"Clip\" | 2026"),
            "FB Reel_ _Best___ _Clip_ _ 2026"
        );

        let out = resolve_output_path(
            Path::new("C:\\Users\\Test\\Downloads\\FlowFB"),
            "{username}/{date}_{shortcode}_{index}.{ext}",
            Some("NASA"),
            Some("5544332211"),
            "2026-11-07",
            1,
            "mp4",
        );
        assert!(out
            .to_string_lossy()
            .ends_with("NASA\\2026-11-07_5544332211_01.mp4"));
    }

    #[test]
    fn test_batch_facebook_url_deduplication() {
        let batch_input = r#"
            https://www.facebook.com/reel/1122334455/?mibextid=aaa
            https://www.facebook.com/reel/1122334455/?mibextid=bbb
            https://fb.watch/xYz123AbC/
            not_a_valid_link
        "#;
        let res = parse_batch_urls(batch_input);
        assert_eq!(res.valid_urls.len(), 2);
        assert_eq!(res.duplicate_count, 1);
        assert_eq!(res.invalid_lines.len(), 1);
    }
}
