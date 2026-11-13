use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    YouTube,
    Instagram,
    Facebook,
    Generic,
}

impl Platform {
    pub fn as_str(&self) -> &'static str {
        match self {
            Platform::YouTube => "youtube",
            Platform::Instagram => "instagram",
            Platform::Facebook => "facebook",
            Platform::Generic => "generic",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaUrlType {
    // Shared / Universal
    Video,
    Reel,
    Post,
    Story,
    Photo,
    // YouTube specific
    Shorts,
    Playlist,
    Music,
    // Instagram specific
    Tv,
    Stories,
    Highlight,
    Profile,
    // Facebook specific
    Watch,
    Share,
    Shortlink,
}

impl MediaUrlType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MediaUrlType::Video => "video",
            MediaUrlType::Reel => "reel",
            MediaUrlType::Post => "post",
            MediaUrlType::Story => "story",
            MediaUrlType::Photo => "photo",
            MediaUrlType::Shorts => "shorts",
            MediaUrlType::Playlist => "playlist",
            MediaUrlType::Music => "music",
            MediaUrlType::Tv => "tv",
            MediaUrlType::Stories => "stories",
            MediaUrlType::Highlight => "highlight",
            MediaUrlType::Profile => "profile",
            MediaUrlType::Watch => "watch",
            MediaUrlType::Share => "share",
            MediaUrlType::Shortlink => "shortlink",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedMediaUrl {
    pub raw_url: String,
    pub canonical_url: String,
    pub platform: Platform,
    pub url_type: MediaUrlType,
    pub shortcode: Option<String>,
    pub username: Option<String>,
    pub playlist_id: Option<String>,
    pub carousel_index_hint: Option<usize>,
    pub requires_auth_hint: bool,
    pub requires_redirect_resolution: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchUrlParseResult {
    pub valid_urls: Vec<NormalizedMediaUrl>,
    pub duplicate_count: usize,
    pub invalid_lines: Vec<String>,
}

const YOUTUBE_HOSTS: &[&str] = &[
    "youtube.com",
    "www.youtube.com",
    "m.youtube.com",
    "music.youtube.com",
    "youtu.be",
    "www.youtu.be",
];

const INSTAGRAM_HOSTS: &[&str] = &[
    "instagram.com",
    "www.instagram.com",
    "instagr.am",
    "www.instagr.am",
    "ddinstagram.com",
    "www.ddinstagram.com",
];

const FACEBOOK_HOSTS: &[&str] = &[
    "facebook.com",
    "www.facebook.com",
    "m.facebook.com",
    "web.facebook.com",
    "mbasic.facebook.com",
    "fb.watch",
    "www.fb.watch",
    "fb.com",
    "www.fb.com",
];

const RESERVED_IG_SEGMENTS: &[&str] = &[
    "explore", "accounts", "direct", "about", "legal", "developer", "press", "api", "static",
    "challenge",
];

const RESERVED_FB_SEGMENTS: &[&str] = &[
    "login", "recover", "help", "settings", "notifications", "messages", "friends", "marketplace",
    "bookmarks", "events", "gaming", "onthisday", "memories", "ads", "business", "policies",
    "privacy",
];

/// Normalizes and classifies any YouTube, Instagram, Facebook, or generic media URL.
pub fn normalize_media_url(raw_input: &str) -> AppResult<NormalizedMediaUrl> {
    let trimmed = raw_input.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidUrl("URL cannot be empty.".into()));
    }

    let with_scheme = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else if YOUTUBE_HOSTS.iter().any(|h| trimmed.to_lowercase().starts_with(h))
        || INSTAGRAM_HOSTS.iter().any(|h| trimmed.to_lowercase().starts_with(h))
        || FACEBOOK_HOSTS.iter().any(|h| trimmed.to_lowercase().starts_with(h))
    {
        format!("https://{}", trimmed)
    } else {
        return Err(AppError::InvalidUrl(
            "Please enter a valid YouTube, Instagram, Facebook, or HTTP(S) media URL.".into(),
        ));
    };

    let parsed = Url::parse(&with_scheme)
        .map_err(|e| AppError::InvalidUrl(format!("Malformed URL: {}", e)))?;

    let host = parsed
        .host_str()
        .ok_or_else(|| AppError::InvalidUrl("URL is missing a hostname.".into()))?
        .to_lowercase();

    if YOUTUBE_HOSTS.contains(&host.as_str()) {
        return normalize_youtube_url(trimmed, &parsed, &host);
    }
    if INSTAGRAM_HOSTS.contains(&host.as_str()) {
        return normalize_instagram_url(trimmed, &parsed);
    }
    if FACEBOOK_HOSTS.contains(&host.as_str()) {
        return normalize_facebook_url(trimmed, &parsed, &host);
    }

    // Generic HTTP/HTTPS URL supported by yt-dlp / gallery-dl
    if parsed.scheme() == "http" || parsed.scheme() == "https" {
        let shortcode = parsed
            .path_segments()
            .and_then(|s| s.filter(|p| !p.is_empty()).next_back())
            .map(|s| s.to_string());
        return Ok(NormalizedMediaUrl {
            raw_url: trimmed.to_string(),
            canonical_url: parsed.to_string(),
            platform: Platform::Generic,
            url_type: MediaUrlType::Video,
            shortcode,
            username: Some(host),
            playlist_id: None,
            carousel_index_hint: None,
            requires_auth_hint: false,
            requires_redirect_resolution: false,
        });
    }

    Err(AppError::InvalidUrl(format!(
        "Unsupported domain '{}'.",
        host
    )))
}

fn normalize_youtube_url(
    raw_input: &str,
    parsed: &Url,
    host: &str,
) -> AppResult<NormalizedMediaUrl> {
    let segments: Vec<&str> = parsed
        .path_segments()
        .map(|c| c.filter(|s| !s.is_empty()).collect())
        .unwrap_or_default();

    let mut video_id: Option<String> = None;
    let mut playlist_id: Option<String> = None;

    for (k, v) in parsed.query_pairs() {
        if k == "v" && !v.is_empty() {
            video_id = Some(v.to_string());
        } else if k == "list" && !v.is_empty() {
            playlist_id = Some(v.to_string());
        }
    }

    // 1. Short link: https://youtu.be/{video_id}
    if host == "youtu.be" || host == "www.youtu.be" {
        let vid = segments
            .first()
            .copied()
            .ok_or_else(|| AppError::InvalidUrl("youtu.be link is missing a video ID.".into()))?;
        let canonical = format!("https://www.youtube.com/watch?v={}", vid);
        return Ok(NormalizedMediaUrl {
            raw_url: raw_input.to_string(),
            canonical_url: canonical,
            platform: Platform::YouTube,
            url_type: MediaUrlType::Video,
            shortcode: Some(vid.to_string()),
            username: None,
            playlist_id,
            carousel_index_hint: None,
            requires_auth_hint: false,
            requires_redirect_resolution: false,
        });
    }

    // 2. YouTube Shorts: /shorts/{video_id}
    if segments.first().copied() == Some("shorts") {
        let vid = segments
            .get(1)
            .copied()
            .ok_or_else(|| AppError::InvalidUrl("YouTube Shorts link is missing a video ID.".into()))?;
        return Ok(NormalizedMediaUrl {
            raw_url: raw_input.to_string(),
            canonical_url: format!("https://www.youtube.com/shorts/{}", vid),
            platform: Platform::YouTube,
            url_type: MediaUrlType::Shorts,
            shortcode: Some(vid.to_string()),
            username: None,
            playlist_id: None,
            carousel_index_hint: None,
            requires_auth_hint: false,
            requires_redirect_resolution: false,
        });
    }

    // 3. Pure Playlist: /playlist?list={playlist_id}
    if segments.first().copied() == Some("playlist") {
        let list = playlist_id.ok_or_else(|| {
            AppError::InvalidUrl("YouTube playlist URL is missing the ?list= parameter.".into())
        })?;
        return Ok(NormalizedMediaUrl {
            raw_url: raw_input.to_string(),
            canonical_url: format!("https://www.youtube.com/playlist?list={}", list),
            platform: Platform::YouTube,
            url_type: MediaUrlType::Playlist,
            shortcode: Some(list.clone()),
            username: None,
            playlist_id: Some(list),
            carousel_index_hint: None,
            requires_auth_hint: false,
            requires_redirect_resolution: false,
        });
    }

    // 4. Live / Embed URLs: /live/{id} or /embed/{id}
    if matches!(segments.first().copied(), Some("live") | Some("embed")) {
        let vid = segments
            .get(1)
            .copied()
            .ok_or_else(|| AppError::InvalidUrl("YouTube URL is missing a video ID.".into()))?;
        return Ok(NormalizedMediaUrl {
            raw_url: raw_input.to_string(),
            canonical_url: format!("https://www.youtube.com/watch?v={}", vid),
            platform: Platform::YouTube,
            url_type: MediaUrlType::Video,
            shortcode: Some(vid.to_string()),
            username: None,
            playlist_id,
            carousel_index_hint: None,
            requires_auth_hint: false,
            requires_redirect_resolution: false,
        });
    }

    // 5. Standard /watch?v={id} (or music.youtube.com/watch?v={id})
    // Always treat /watch?v={id} as a single Video/Music item even if a &list=RD... radio mix
    // parameter is present, so copying a playing video never triggers a 400+ item Radio Mix hang.
    if let Some(vid) = video_id {
        let is_music = host == "music.youtube.com";
        let base_host = if is_music {
            "https://music.youtube.com"
        } else {
            "https://www.youtube.com"
        };
        let canonical = format!("{}/watch?v={}", base_host, vid);
        let url_type = if is_music {
            MediaUrlType::Music
        } else {
            MediaUrlType::Video
        };
        return Ok(NormalizedMediaUrl {
            raw_url: raw_input.to_string(),
            canonical_url: canonical,
            platform: Platform::YouTube,
            url_type,
            shortcode: Some(vid),
            username: None,
            playlist_id,
            carousel_index_hint: None,
            requires_auth_hint: false,
            requires_redirect_resolution: false,
        });
    }

    Err(AppError::InvalidUrl(
        "Unrecognized YouTube URL format. Supported: Videos, Shorts, Playlists, and YouTube Music."
            .into(),
    ))
}

fn normalize_instagram_url(raw_input: &str, parsed: &Url) -> AppResult<NormalizedMediaUrl> {
    let mut carousel_index_hint: Option<usize> = None;
    for (k, v) in parsed.query_pairs() {
        if k == "img_index" {
            if let Ok(idx) = v.parse::<usize>() {
                if idx >= 1 {
                    carousel_index_hint = Some(idx);
                }
            }
        }
    }

    let segments: Vec<&str> = parsed
        .path_segments()
        .map(|c| c.filter(|s| !s.is_empty()).collect())
        .unwrap_or_default();

    if segments.is_empty() {
        return Err(AppError::InvalidUrl(
            "Instagram URL does not point to any media or profile.".into(),
        ));
    }

    let (url_type, shortcode, username, canonical_path, requires_auth_hint) =
        match segments.as_slice() {
            ["p", code, ..] => (
                MediaUrlType::Post,
                Some((*code).to_string()),
                None,
                format!("/p/{}/", code),
                false,
            ),
            ["reel", code, ..] | ["reels", code, ..] => (
                MediaUrlType::Reel,
                Some((*code).to_string()),
                None,
                format!("/reel/{}/", code),
                false,
            ),
            ["tv", code, ..] => (
                MediaUrlType::Tv,
                Some((*code).to_string()),
                None,
                format!("/tv/{}/", code),
                false,
            ),
            ["stories", "highlights", highlight_id, ..] => (
                MediaUrlType::Highlight,
                Some((*highlight_id).to_string()),
                None,
                format!("/stories/highlights/{}/", highlight_id),
                true,
            ),
            ["s", highlight_code, ..] => (
                MediaUrlType::Highlight,
                Some((*highlight_code).to_string()),
                None,
                format!("/s/{}/", highlight_code),
                true,
            ),
            ["stories", user, story_id, ..] => (
                MediaUrlType::Stories,
                Some((*story_id).to_string()),
                Some((*user).to_string()),
                format!("/stories/{}/{}/", user, story_id),
                true,
            ),
            ["stories", user] => (
                MediaUrlType::Stories,
                None,
                Some((*user).to_string()),
                format!("/stories/{}/", user),
                true,
            ),
            [user, "p", code, ..] => (
                MediaUrlType::Post,
                Some((*code).to_string()),
                Some((*user).to_string()),
                format!("/p/{}/", code),
                false,
            ),
            [user, "reel", code, ..] | [user, "reels", code, ..] => (
                MediaUrlType::Reel,
                Some((*code).to_string()),
                Some((*user).to_string()),
                format!("/reel/{}/", code),
                false,
            ),
            [user] | [user, "reels"] | [user, "tagged"] => {
                let clean_user = *user;
                if RESERVED_IG_SEGMENTS.contains(&clean_user.to_lowercase().as_str()) {
                    return Err(AppError::InvalidUrl(format!(
                        "'{}' is a reserved Instagram path, not a downloadable resource.",
                        clean_user
                    )));
                }
                (
                    MediaUrlType::Profile,
                    None,
                    Some(clean_user.to_string()),
                    format!("/{}/", clean_user),
                    true,
                )
            }
            _ => {
                return Err(AppError::InvalidUrl(
                    "Unrecognized Instagram URL structure.".into(),
                ));
            }
        };

    let canonical_url = format!("https://www.instagram.com{}", canonical_path);

    Ok(NormalizedMediaUrl {
        raw_url: raw_input.to_string(),
        canonical_url,
        platform: Platform::Instagram,
        url_type,
        shortcode,
        username,
        playlist_id: None,
        carousel_index_hint,
        requires_auth_hint,
        requires_redirect_resolution: false,
    })
}

fn normalize_facebook_url(
    raw_input: &str,
    parsed: &Url,
    host: &str,
) -> AppResult<NormalizedMediaUrl> {
    let segments: Vec<&str> = parsed
        .path_segments()
        .map(|c| c.filter(|s| !s.is_empty()).collect())
        .unwrap_or_default();

    // 1. Shortlink: fb.watch/{token}
    if host == "fb.watch" || host == "www.fb.watch" {
        let token = segments
            .first()
            .copied()
            .ok_or_else(|| AppError::InvalidUrl("fb.watch shortlink is missing a video token.".into()))?;
        return Ok(NormalizedMediaUrl {
            raw_url: raw_input.to_string(),
            canonical_url: format!("https://fb.watch/{}/", token),
            platform: Platform::Facebook,
            url_type: MediaUrlType::Shortlink,
            shortcode: Some(token.to_string()),
            username: None,
            playlist_id: None,
            carousel_index_hint: None,
            requires_auth_hint: false,
            requires_redirect_resolution: true,
        });
    }

    let mut query_v: Option<String> = None;
    let mut query_story_fbid: Option<String> = None;
    let mut query_id: Option<String> = None;
    let mut query_fbid: Option<String> = None;
    let mut query_set: Option<String> = None;

    for (k, v) in parsed.query_pairs() {
        match k.as_ref() {
            "v" if !v.is_empty() => query_v = Some(v.to_string()),
            "story_fbid" if !v.is_empty() => query_story_fbid = Some(v.to_string()),
            "id" if !v.is_empty() => query_id = Some(v.to_string()),
            "fbid" if !v.is_empty() => query_fbid = Some(v.to_string()),
            "set" if !v.is_empty() => query_set = Some(v.to_string()),
            _ => {}
        }
    }

    if matches!(segments.first().copied(), Some("watch") | Some("video.php")) {
        if let Some(vid) = query_v.clone().or_else(|| segments.get(1).map(|s| (*s).to_string())) {
            if vid != "live" {
                return Ok(NormalizedMediaUrl {
                    raw_url: raw_input.to_string(),
                    canonical_url: format!("https://www.facebook.com/watch/?v={}", vid),
                    platform: Platform::Facebook,
                    url_type: MediaUrlType::Watch,
                    shortcode: Some(vid),
                    username: None,
                    playlist_id: None,
                    carousel_index_hint: None,
                    requires_auth_hint: false,
                    requires_redirect_resolution: false,
                });
            }
        }
        if segments.get(1).copied() == Some("live") {
            if let Some(vid) = query_v.clone() {
                return Ok(NormalizedMediaUrl {
                    raw_url: raw_input.to_string(),
                    canonical_url: format!("https://www.facebook.com/watch/?v={}", vid),
                    platform: Platform::Facebook,
                    url_type: MediaUrlType::Watch,
                    shortcode: Some(vid),
                    username: None,
                    playlist_id: None,
                    carousel_index_hint: None,
                    requires_auth_hint: false,
                    requires_redirect_resolution: false,
                });
            }
        }
    }

    if matches!(segments.first().copied(), Some("story.php") | Some("permalink.php")) {
        let fbid = query_story_fbid.ok_or_else(|| {
            AppError::InvalidUrl("Facebook story.php/permalink.php URL is missing story_fbid.".into())
        })?;
        let canonical = if let Some(ref page_id) = query_id {
            format!(
                "https://www.facebook.com/story.php?story_fbid={}&id={}",
                fbid, page_id
            )
        } else {
            format!("https://www.facebook.com/story.php?story_fbid={}", fbid)
        };
        return Ok(NormalizedMediaUrl {
            raw_url: raw_input.to_string(),
            canonical_url: canonical,
            platform: Platform::Facebook,
            url_type: MediaUrlType::Post,
            shortcode: Some(fbid),
            username: query_id,
            playlist_id: None,
            carousel_index_hint: None,
            requires_auth_hint: false,
            requires_redirect_resolution: false,
        });
    }

    if matches!(segments.first().copied(), Some("photo") | Some("photo.php")) {
        let fbid = query_fbid
            .or_else(|| segments.get(1).map(|s| (*s).to_string()))
            .ok_or_else(|| AppError::InvalidUrl("Facebook photo URL is missing fbid.".into()))?;
        let canonical = if let Some(ref set_id) = query_set {
            format!(
                "https://www.facebook.com/photo/?fbid={}&set={}",
                fbid, set_id
            )
        } else {
            format!("https://www.facebook.com/photo/?fbid={}", fbid)
        };
        return Ok(NormalizedMediaUrl {
            raw_url: raw_input.to_string(),
            canonical_url: canonical,
            platform: Platform::Facebook,
            url_type: MediaUrlType::Photo,
            shortcode: Some(fbid),
            username: None,
            playlist_id: None,
            carousel_index_hint: None,
            requires_auth_hint: false,
            requires_redirect_resolution: false,
        });
    }

    if segments.first().copied() == Some("media") && segments.get(1).copied() == Some("set") {
        let set_id = query_set
            .ok_or_else(|| AppError::InvalidUrl("Facebook media album URL is missing set ID.".into()))?;
        return Ok(NormalizedMediaUrl {
            raw_url: raw_input.to_string(),
            canonical_url: format!("https://www.facebook.com/media/set/?set={}", set_id),
            platform: Platform::Facebook,
            url_type: MediaUrlType::Photo,
            shortcode: Some(set_id),
            username: None,
            playlist_id: None,
            carousel_index_hint: None,
            requires_auth_hint: false,
            requires_redirect_resolution: false,
        });
    }

    if segments.is_empty() {
        return Err(AppError::InvalidUrl(
            "Facebook URL does not point to a downloadable video, reel, or post.".into(),
        ));
    }

    match segments.as_slice() {
        ["reel", id, ..] | ["reels", id, ..] => Ok(NormalizedMediaUrl {
            raw_url: raw_input.to_string(),
            canonical_url: format!("https://www.facebook.com/reel/{}", id),
            platform: Platform::Facebook,
            url_type: MediaUrlType::Reel,
            shortcode: Some((*id).to_string()),
            username: None,
            playlist_id: None,
            carousel_index_hint: None,
            requires_auth_hint: false,
            requires_redirect_resolution: false,
        }),
        ["share", kind @ ("v" | "r" | "p"), token, ..] => Ok(NormalizedMediaUrl {
            raw_url: raw_input.to_string(),
            canonical_url: format!("https://www.facebook.com/share/{}/{}/", kind, token),
            platform: Platform::Facebook,
            url_type: MediaUrlType::Share,
            shortcode: Some((*token).to_string()),
            username: None,
            playlist_id: None,
            carousel_index_hint: None,
            requires_auth_hint: false,
            requires_redirect_resolution: true,
        }),
        ["share", token] => Ok(NormalizedMediaUrl {
            raw_url: raw_input.to_string(),
            canonical_url: format!("https://www.facebook.com/share/{}/", token),
            platform: Platform::Facebook,
            url_type: MediaUrlType::Share,
            shortcode: Some((*token).to_string()),
            username: None,
            playlist_id: None,
            carousel_index_hint: None,
            requires_auth_hint: false,
            requires_redirect_resolution: true,
        }),
        ["stories", user, story_id, ..] => Ok(NormalizedMediaUrl {
            raw_url: raw_input.to_string(),
            canonical_url: format!("https://www.facebook.com/stories/{}/{}/", user, story_id),
            platform: Platform::Facebook,
            url_type: MediaUrlType::Story,
            shortcode: Some((*story_id).to_string()),
            username: Some((*user).to_string()),
            playlist_id: None,
            carousel_index_hint: None,
            requires_auth_hint: true,
            requires_redirect_resolution: false,
        }),
        ["stories", story_id] => Ok(NormalizedMediaUrl {
            raw_url: raw_input.to_string(),
            canonical_url: format!("https://www.facebook.com/stories/{}/", story_id),
            platform: Platform::Facebook,
            url_type: MediaUrlType::Story,
            shortcode: Some((*story_id).to_string()),
            username: None,
            playlist_id: None,
            carousel_index_hint: None,
            requires_auth_hint: true,
            requires_redirect_resolution: false,
        }),
        ["groups", group_id, "posts" | "permalink", post_id, ..] => Ok(NormalizedMediaUrl {
            raw_url: raw_input.to_string(),
            canonical_url: format!(
                "https://www.facebook.com/groups/{}/posts/{}/",
                group_id, post_id
            ),
            platform: Platform::Facebook,
            url_type: MediaUrlType::Post,
            shortcode: Some((*post_id).to_string()),
            username: Some((*group_id).to_string()),
            playlist_id: None,
            carousel_index_hint: None,
            requires_auth_hint: false,
            requires_redirect_resolution: false,
        }),
        ["groups", group_id, "videos", video_id, ..] => Ok(NormalizedMediaUrl {
            raw_url: raw_input.to_string(),
            canonical_url: format!(
                "https://www.facebook.com/groups/{}/videos/{}/",
                group_id, video_id
            ),
            platform: Platform::Facebook,
            url_type: MediaUrlType::Video,
            shortcode: Some((*video_id).to_string()),
            username: Some((*group_id).to_string()),
            playlist_id: None,
            carousel_index_hint: None,
            requires_auth_hint: false,
            requires_redirect_resolution: false,
        }),
        [user, "videos", _slug, video_id, ..] => Ok(NormalizedMediaUrl {
            raw_url: raw_input.to_string(),
            canonical_url: format!("https://www.facebook.com/{}/videos/{}/", user, video_id),
            platform: Platform::Facebook,
            url_type: MediaUrlType::Video,
            shortcode: Some((*video_id).to_string()),
            username: Some((*user).to_string()),
            playlist_id: None,
            carousel_index_hint: None,
            requires_auth_hint: false,
            requires_redirect_resolution: false,
        }),
        [user, "videos", video_id] => Ok(NormalizedMediaUrl {
            raw_url: raw_input.to_string(),
            canonical_url: format!("https://www.facebook.com/{}/videos/{}/", user, video_id),
            platform: Platform::Facebook,
            url_type: MediaUrlType::Video,
            shortcode: Some((*video_id).to_string()),
            username: Some((*user).to_string()),
            playlist_id: None,
            carousel_index_hint: None,
            requires_auth_hint: false,
            requires_redirect_resolution: false,
        }),
        [user, "reel" | "reels", reel_id, ..] => Ok(NormalizedMediaUrl {
            raw_url: raw_input.to_string(),
            canonical_url: format!("https://www.facebook.com/reel/{}", reel_id),
            platform: Platform::Facebook,
            url_type: MediaUrlType::Reel,
            shortcode: Some((*reel_id).to_string()),
            username: Some((*user).to_string()),
            playlist_id: None,
            carousel_index_hint: None,
            requires_auth_hint: false,
            requires_redirect_resolution: false,
        }),
        [user, "posts", post_id, ..] => Ok(NormalizedMediaUrl {
            raw_url: raw_input.to_string(),
            canonical_url: format!("https://www.facebook.com/{}/posts/{}", user, post_id),
            platform: Platform::Facebook,
            url_type: MediaUrlType::Post,
            shortcode: Some((*post_id).to_string()),
            username: Some((*user).to_string()),
            playlist_id: None,
            carousel_index_hint: None,
            requires_auth_hint: false,
            requires_redirect_resolution: false,
        }),
        [user, "photos", album_or_id, photo_id, ..] => Ok(NormalizedMediaUrl {
            raw_url: raw_input.to_string(),
            canonical_url: format!(
                "https://www.facebook.com/{}/photos/{}/{}/",
                user, album_or_id, photo_id
            ),
            platform: Platform::Facebook,
            url_type: MediaUrlType::Photo,
            shortcode: Some((*photo_id).to_string()),
            username: Some((*user).to_string()),
            playlist_id: None,
            carousel_index_hint: None,
            requires_auth_hint: false,
            requires_redirect_resolution: false,
        }),
        [user, "photos", photo_id] => Ok(NormalizedMediaUrl {
            raw_url: raw_input.to_string(),
            canonical_url: format!("https://www.facebook.com/{}/photos/{}/", user, photo_id),
            platform: Platform::Facebook,
            url_type: MediaUrlType::Photo,
            shortcode: Some((*photo_id).to_string()),
            username: Some((*user).to_string()),
            playlist_id: None,
            carousel_index_hint: None,
            requires_auth_hint: false,
            requires_redirect_resolution: false,
        }),
        [first, ..] => {
            if RESERVED_FB_SEGMENTS.contains(&first.to_lowercase().as_str()) {
                return Err(AppError::InvalidUrl(format!(
                    "'{}' is a Facebook navigation path, not a downloadable media link.",
                    first
                )));
            }
            if let Some(vid) = query_v {
                return Ok(NormalizedMediaUrl {
                    raw_url: raw_input.to_string(),
                    canonical_url: format!("https://www.facebook.com/watch/?v={}", vid),
                    platform: Platform::Facebook,
                    url_type: MediaUrlType::Watch,
                    shortcode: Some(vid),
                    username: Some((*first).to_string()),
                    playlist_id: None,
                    carousel_index_hint: None,
                    requires_auth_hint: false,
                    requires_redirect_resolution: false,
                });
            }
            Err(AppError::InvalidUrl(
                "Unrecognized Facebook URL structure.".into(),
            ))
        }
        [] => Err(AppError::InvalidUrl("Empty Facebook path.".into())),
    }
}

/// Parses a multi-line or whitespace-separated list of mixed YouTube, Instagram, and Facebook URLs.
pub fn parse_batch_urls(raw_text: &str) -> BatchUrlParseResult {
    let mut seen = HashSet::new();
    let mut valid_urls = Vec::new();
    let mut duplicate_count = 0usize;
    let mut invalid_lines = Vec::new();

    for line in raw_text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        for token in trimmed.split_whitespace() {
            match normalize_media_url(token) {
                Ok(norm) => {
                    if seen.insert(norm.canonical_url.clone()) {
                        valid_urls.push(norm);
                    } else {
                        duplicate_count += 1;
                    }
                }
                Err(_) => {
                    invalid_lines.push(token.to_string());
                }
            }
        }
    }

    BatchUrlParseResult {
        valid_urls,
        duplicate_count,
        invalid_lines,
    }
}

const WINDOWS_RESERVED_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

pub fn sanitize_filename_component(input: &str, max_chars: usize) -> String {
    let mut cleaned: String = input
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => ' ',
            other => other,
        })
        .collect();

    cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    cleaned = cleaned.trim_matches(|c| c == '.' || c == ' ').to_string();

    if cleaned.is_empty() {
        cleaned = "media".to_string();
    }

    let upper = cleaned.to_uppercase();
    let base_upper = upper.split('.').next().unwrap_or("");
    if WINDOWS_RESERVED_NAMES.contains(&base_upper) {
        cleaned = format!("_{}", cleaned);
    }

    if cleaned.chars().count() > max_chars {
        cleaned = cleaned.chars().take(max_chars).collect();
        cleaned = cleaned.trim_end_matches(|c| c == '.' || c == ' ').to_string();
    }

    if cleaned.is_empty() {
        cleaned = "media".to_string();
    }

    let post_upper = cleaned.to_uppercase();
    let post_base = post_upper.split('.').next().unwrap_or("");
    if WINDOWS_RESERVED_NAMES.contains(&post_base) {
        cleaned = format!("_{}", cleaned);
    }

    cleaned
}

pub fn resolve_output_path(
    base_dir: &Path,
    platform: &str,
    username: Option<&str>,
    shortcode: Option<&str>,
    date_yyyy_mm_dd: &str,
    slide_idx: usize,
    ext: &str,
    organize_by_platform: bool,
    organize_by_uploader: bool,
) -> PathBuf {
    let safe_platform = sanitize_filename_component(platform, 24);
    let safe_user = sanitize_filename_component(username.unwrap_or("creator"), 40);
    let safe_code = sanitize_filename_component(shortcode.unwrap_or("post"), 32);
    let safe_date = sanitize_filename_component(date_yyyy_mm_dd, 16);
    let safe_ext = ext.trim_start_matches('.');

    let mut target_dir = base_dir.to_path_buf();
    if organize_by_platform {
        target_dir = target_dir.join(&safe_platform);
    }
    if organize_by_uploader {
        target_dir = target_dir.join(&safe_user);
    }

    let stem = format!(
        "{}_{}_{}_{:02}",
        safe_user, safe_date, safe_code, slide_idx
    );
    let file_name = format!("{}.{}", stem, safe_ext);
    let mut candidate = target_dir.join(&file_name);

    if candidate.to_string_lossy().len() > 235 {
        let short_stem = format!("{}_{:02}", safe_code, slide_idx);
        candidate = target_dir.join(format!("{}.{}", short_stem, safe_ext));
    }

    if !candidate.exists() {
        return candidate;
    }

    for counter in 1..1000 {
        let numbered_name = format!("{} ({}).{}", stem, counter, safe_ext);
        let alt = target_dir.join(numbered_name);
        if !alt.exists() {
            return alt;
        }
    }

    candidate
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_youtube_urls() {
        let watch = normalize_media_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ&si=abc").unwrap();
        assert_eq!(watch.platform, Platform::YouTube);
        assert_eq!(watch.url_type, MediaUrlType::Video);
        assert_eq!(watch.shortcode.as_deref(), Some("dQw4w9WgXcQ"));
        assert_eq!(watch.canonical_url, "https://www.youtube.com/watch?v=dQw4w9WgXcQ");

        let shorts = normalize_media_url("https://youtube.com/shorts/abc123XYZ?feature=share").unwrap();
        assert_eq!(shorts.platform, Platform::YouTube);
        assert_eq!(shorts.url_type, MediaUrlType::Shorts);
        assert_eq!(shorts.shortcode.as_deref(), Some("abc123XYZ"));

        let playlist = normalize_media_url("https://www.youtube.com/playlist?list=PL1234567890").unwrap();
        assert_eq!(playlist.platform, Platform::YouTube);
        assert_eq!(playlist.url_type, MediaUrlType::Playlist);
        assert_eq!(playlist.playlist_id.as_deref(), Some("PL1234567890"));
    }

    #[test]
    fn test_normalize_instagram_urls() {
        let post = normalize_media_url("https://www.instagram.com/p/C8abc123/?igsh=xyz&img_index=3").unwrap();
        assert_eq!(post.platform, Platform::Instagram);
        assert_eq!(post.url_type, MediaUrlType::Post);
        assert_eq!(post.shortcode.as_deref(), Some("C8abc123"));
        assert_eq!(post.carousel_index_hint, Some(3));

        let reel = normalize_media_url("instagram.com/reels/D9xyz789").unwrap();
        assert_eq!(reel.platform, Platform::Instagram);
        assert_eq!(reel.url_type, MediaUrlType::Reel);
        assert_eq!(reel.canonical_url, "https://www.instagram.com/reel/D9xyz789/");
    }

    #[test]
    fn test_normalize_facebook_urls() {
        let watch = normalize_media_url("https://www.facebook.com/watch/?v=9876543210&rdid=abc").unwrap();
        assert_eq!(watch.platform, Platform::Facebook);
        assert_eq!(watch.url_type, MediaUrlType::Watch);
        assert_eq!(watch.shortcode.as_deref(), Some("9876543210"));

        let short = normalize_media_url("https://fb.watch/xYz123AbC/").unwrap();
        assert_eq!(short.platform, Platform::Facebook);
        assert_eq!(short.url_type, MediaUrlType::Shortlink);
        assert!(short.requires_redirect_resolution);
    }

    #[test]
    fn test_mixed_platform_batch_parsing() {
        let raw = "
            https://youtu.be/dQw4w9WgXcQ
            https://www.instagram.com/reel/C8abc123/
            https://fb.watch/xYz123AbC/
            https://www.youtube.com/watch?v=dQw4w9WgXcQ
        ";
        let res = parse_batch_urls(raw);
        assert_eq!(res.valid_urls.len(), 3);
        assert_eq!(res.duplicate_count, 1);
    }
}
