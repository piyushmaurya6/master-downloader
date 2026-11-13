export type FacebookUrlKind =
  | { kind: "reel"; value: string }
  | { kind: "watch"; value: string }
  | { kind: "video"; value: string }
  | { kind: "share"; value: { share_type: string; token: string } }
  | { kind: "shortlink"; value: string }
  | { kind: "post"; value: string }
  | { kind: "photo"; value: string }
  | { kind: "story"; value: string };

export interface NormalizedFacebookUrl {
  canonical_url: string;
  original_url: string;
  url_type: FacebookUrlKind;
  type_badge: "REEL" | "WATCH" | "VIDEO" | "FB.WATCH" | "SHARE" | "POST" | "PHOTO" | "STORY" | string;
  shortcode: string | null;
  username_hint: string | null;
  img_index_hint: number | null;
}

export interface BatchUrlParseResult {
  valid_urls: NormalizedFacebookUrl[];
  invalid_lines: string[];
  duplicate_count: number;
}

export interface CarouselSlideItem {
  index: number;
  media_type: "image" | "video";
  width: number;
  height: number;
  display_url: string;
  video_url: string | null;
  duration_seconds: number | null;
  ext: string;
}

export interface VideoFormatOption {
  quality_key: string;      // e.g. "best", "4k", "1440p", "1080p", "720p", "480p", "audio_mp3", "audio_m4a"
  format_id: string;        // yt-dlp format selector or format_id
  resolution_label: string; // e.g. "1080p Full HD", "Audio Only — MP3 (320kbps)"
  badge: string;            // e.g. "DASH 1080P", "AUTO MAX", "MP3 320K"
  vcodec: string;           // "H.264" | "VP9" | "AV1" | "none"
  acodec: string;           // "AAC" | "MP3"
  ext: string;              // "mp4" | "mp3" | "m4a"
  height: number | null;
  fps: number | null;
  filesize_bytes: number | null;
  is_dash: boolean;
  is_audio_only: boolean;
}

export interface MediaInspectionResult {
  canonical_url: string;
  shortcode: string | null;
  url_type: string;
  type_badge: string;
  owner_username: string;
  owner_full_name: string | null;
  title: string;
  caption: string;
  timestamp: number | null;
  thumbnail_url: string | null;
  duration_seconds: number | null;
  view_count: number | null;
  like_count: number | null;
  aspect_ratio_label: "9:16" | "16:9" | "1:1" | "4:5" | string;
  total_items: number;
  slides: CarouselSlideItem[];
  available_qualities: string[];
  format_options: VideoFormatOption[];
}

export interface MediaFileRecord {
  id: string;
  job_id: string;
  slide_index: number;
  media_type: "image" | "video" | "audio" | string;
  file_path: string;
  file_size_bytes: number | null;
  width: number | null;
  height: number | null;
  duration_seconds: number | null;
  file_exists: boolean;
}

export type QueueJobStatus =
  | "queued"
  | "running"
  | "paused"
  | "completed"
  | "failed"
  | "cancelled";

export interface JobRecord {
  id: string;
  source_url: string;
  shortcode: string | null;
  url_type: string;
  author_username: string | null;
  caption: string | null;
  thumbnail_url: string | null;
  status: QueueJobStatus;
  selected_indices: number[];
  quality_preset: string;
  output_dir: string;
  naming_template: string;
  save_caption: boolean;
  save_metadata: boolean;
  embed_thumbnail: boolean;
  error_message: string | null;
  progress_percent: number;
  downloaded_bytes: number;
  total_bytes: number | null;
  created_at: number;
  completed_at: number | null;
  media_files: MediaFileRecord[];
}

export interface NewJobRequest {
  source_url: string;
  shortcode?: string | null;
  url_type: string;
  author_username?: string | null;
  caption?: string | null;
  thumbnail_url?: string | null;
  selected_indices: number[];
  quality_preset: string;
  output_dir?: string | null;
  naming_template?: string | null;
  save_caption?: boolean;
  save_metadata?: boolean;
  embed_thumbnail?: boolean;
}

export type TelemetryJobStatus =
  | "inspecting"
  | "downloading_video"
  | "downloading_audio"
  | "merging"
  | "completed"
  | "failed"
  | "paused"
  | "rate_limited";

export interface ProgressPayload {
  job_id: string;
  item_index: number;
  total_items: number;
  status: TelemetryJobStatus;
  stage_label: string;
  percentage: number;
  downloaded_bytes: number;
  total_bytes: number | null;
  speed_bps: number | null;
  eta_seconds: number | null;
}

export interface RateLimitTrippedPayload {
  reason: string;
  cooldown_seconds: number;
  requires_reauth: boolean;
  timestamp: number;
}

export interface BinaryInfo {
  name: string;
  executable: string;
  installed: boolean;
  path: string;
  version: string | null;
  sha256: string | null;
  size_bytes: number | null;
}

export interface BinaryStatusReport {
  bin_dir: string;
  all_ready: boolean;
  yt_dlp: BinaryInfo;
  gallery_dl: BinaryInfo;
  ffmpeg: BinaryInfo;
  ffprobe: BinaryInfo;
}

export interface BinaryBootstrapProgress {
  binary_name: string;
  downloaded_bytes: number;
  total_bytes: number | null;
  percentage: number;
  status: "checking" | "downloading" | "verifying" | "extracting" | "ready" | "error";
  sha256: string | null;
  version: string | null;
}

export interface AuthSessionStatus {
  mode: "anonymous" | "webview_vault" | "browser_cookies" | "cookie_file" | string;
  authenticated: boolean;
  username: string | null;
  fb_user_id: string | null;
  selected_browser: string | null;
  vault_present: boolean;
}

export interface AppSettings {
  output_dir: string;
  naming_template: string;
  default_quality: string;
  concurrency: number;
  jitter_min_ms: number;
  jitter_max_ms: number;
  proxy_url: string | null;
  clipboard_monitor: boolean;
  auto_update_engines: boolean;
  save_caption_by_default: boolean;
  save_metadata_by_default: boolean;
  embed_thumbnail_by_default: boolean;
  auth_mode: string;
  selected_browser: string | null;
  connected_username: string | null;
  theme: "dark" | "light";
}

export interface HistoryFilter {
  search_query?: string | null;
  media_type_filter?: string | null;
  date_from?: number | null;
  date_to?: number | null;
  limit?: number;
  offset?: number;
}

export interface DiagnosticLogEntry {
  timestamp: string;
  level: string;
  component: string;
  message: string;
}

export interface SerializedAppError {
  code: string;
  user_message: string;
  technical_details: string;
  remediation_action: string;
  retry_after_secs?: number | null;
}
