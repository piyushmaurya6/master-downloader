export type Platform = "youtube" | "instagram" | "facebook" | "generic";

export type MediaUrlType =
  | "video"
  | "reel"
  | "post"
  | "story"
  | "photo"
  | "shorts"
  | "playlist"
  | "music"
  | "tv"
  | "stories"
  | "highlight"
  | "profile"
  | "watch"
  | "share"
  | "shortlink";

export type SlideMediaType = "image" | "video" | "audio";

export type QueueJobStatus =
  | "queued"
  | "running"
  | "paused"
  | "completed"
  | "failed"
  | "cancelled";

export interface NormalizedMediaUrl {
  raw_url: string;
  canonical_url: string;
  platform: Platform;
  url_type: MediaUrlType;
  shortcode: string | null;
  username: string | null;
  playlist_id: string | null;
  carousel_index_hint: number | null;
  requires_auth_hint: boolean;
  requires_redirect_resolution: boolean;
}

export interface BatchUrlParseResult {
  valid_urls: NormalizedMediaUrl[];
  duplicate_count: number;
  invalid_lines: string[];
}

export interface MediaSlide {
  index: number;
  media_type: SlideMediaType;
  thumbnail_url: string | null;
  direct_url: string | null;
  width: number | null;
  height: number | null;
  duration_seconds: number | null;
  filesize_approx: number | null;
}

export interface FormatItem {
  format_id: string;
  ext: string;
  resolution: string;
  height: number | null;
  fps: number | null;
  vcodec: string | null;
  acodec: string | null;
  filesize_bytes: number | null;
  has_video: boolean;
  has_audio: boolean;
}

export interface SubtitleTrack {
  lang: string;
  name: string;
  is_auto: boolean;
}

export interface ChapterItem {
  title: string;
  start_time: number;
  end_time: number;
}

export interface PlaylistEntryPreview {
  index: number;
  id: string;
  title: string;
  duration_seconds: number | null;
  uploader: string | null;
}

export interface MediaInspectionResult {
  source_url: string;
  platform: Platform;
  shortcode: string | null;
  url_type: string;
  author_username: string;
  author_full_name: string | null;
  caption: string | null;
  timestamp: number | null;
  like_count: number | null;
  view_count: number | null;
  comment_count: number | null;
  slide_count: number;
  slides: MediaSlide[];
  available_qualities: string[];
  formats: FormatItem[];
  subtitles: SubtitleTrack[];
  chapters: ChapterItem[];
  is_playlist: boolean;
  playlist_title: string | null;
  playlist_count: number | null;
  playlist_entries: PlaylistEntryPreview[];
  engine_used: string;
}

export interface AdvancedJobOptions {
  container?: string | null;
  preferred_codec?: string | null;
  preferred_fps?: number | null;
  exact_resolution?: boolean;
  audio_format?: string | null;
  audio_bitrate?: string | null;
  audio_normalization?: boolean;
  write_subtitles?: boolean;
  subtitle_langs?: string | null;
  embed_chapters?: boolean;
  playlist_mode?: string | null;
  speed_limit_kbps?: number | null;
}

export interface MediaFileRecord {
  id: string;
  job_id: string;
  slide_index: number;
  media_type: SlideMediaType;
  file_path: string;
  file_size_bytes: number | null;
  width: number | null;
  height: number | null;
  duration_seconds: number | null;
}

export interface JobRecord {
  id: string;
  source_url: string;
  platform: Platform;
  shortcode: string | null;
  url_type: string;
  author_username: string | null;
  caption: string | null;
  thumbnail_url: string | null;
  status: QueueJobStatus;
  selected_indices: number[];
  quality_preset: string;
  output_dir: string;
  error_message: string | null;
  save_caption: boolean;
  save_metadata: boolean;
  embed_thumbnail: boolean;
  naming_template: string | null;
  options: AdvancedJobOptions;
  progress_percent: number;
  downloaded_bytes: number;
  total_bytes: number | null;
  created_at: number;
  completed_at: number | null;
  media_files: MediaFileRecord[];
}

export interface NewJobRequest {
  source_url: string;
  platform?: Platform | string | null;
  shortcode?: string | null;
  url_type: string;
  author_username?: string | null;
  caption?: string | null;
  thumbnail_url?: string | null;
  selected_indices: number[];
  quality_preset: string;
  output_dir?: string | null;
  save_caption: boolean;
  save_metadata: boolean;
  embed_thumbnail: boolean;
  naming_template?: string | null;
  options?: AdvancedJobOptions;
}

export interface ProgressPayload {
  job_id: string;
  slide_index: number;
  status: string;
  downloaded_bytes: number;
  total_bytes: number | null;
  speed_bps: number | null;
  eta_seconds: number | null;
  percentage: number;
}

export interface RateLimitTrippedPayload {
  job_id: string;
  reason: string;
  cooldown_seconds: number;
  requires_reauth: boolean;
}

export interface SingleBinaryStatus {
  name: string;
  installed: boolean;
  path: string;
  version: string | null;
  sha256: string | null;
  size_bytes: number | null;
}

export interface BinaryStatusReport {
  all_ready: boolean;
  bin_dir: string;
  yt_dlp: SingleBinaryStatus;
  gallery_dl: SingleBinaryStatus;
  ffmpeg: SingleBinaryStatus;
  ffprobe: SingleBinaryStatus;
  deno: SingleBinaryStatus;
}

export interface BinaryBootstrapProgress {
  binary_name: string;
  status: string;
  downloaded_bytes: number;
  total_bytes: number | null;
  percentage: number;
}

export interface AuthSessionStatus {
  authenticated: boolean;
  mode: "none" | "cookies_txt" | "webview" | "browser" | string;
  username: string | null;
  browser_source: string | null;
  instagram_authenticated: boolean;
  instagram_username: string | null;
  facebook_authenticated: boolean;
  facebook_username: string | null;
  youtube_authenticated: boolean;
  cookie_count: number;
  updated_at: number | null;
}

export interface AppSettings {
  output_dir: string;
  default_quality: string;
  default_container: string;
  default_audio_format: string;
  default_audio_bitrate: string;
  audio_normalization: boolean;
  concurrency: number;
  jitter_min_ms: number;
  jitter_max_ms: number;
  speed_limit_kbps: number | null;
  clipboard_monitor: boolean;
  auto_download_on_detect: boolean;
  organize_by_platform: boolean;
  organize_by_uploader: boolean;
  embed_thumbnail: boolean;
  save_caption_sidecar: boolean;
  save_metadata_json: boolean;
  write_subtitles: boolean;
  embed_chapters: boolean;
  naming_template: string;
  browser_cookie_source: string | null;
}

export interface HistoryFilter {
  search_query?: string | null;
  platform?: string | null;
  media_type?: string | null;
  status?: string | null;
  limit?: number;
  offset?: number;
}

export interface DiagnosticLogEntry {
  line: string;
}

export interface SerializedAppError {
  code: string;
  user_message: string;
  technical_details: string;
  remediation_action: string | null;
}
