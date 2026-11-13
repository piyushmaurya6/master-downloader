import React, { useEffect, useState } from "react";
import {
  CheckSquare,
  Download,
  Eye,
  Film,
  Heart,
  Image as ImageIcon,
  ListVideo,
  Music,
  Sliders,
  Square,
  Subtitles,
  User,
  Volume2,
  X,
} from "lucide-react";
import { ipc, isTauriRuntime, parseIpcError } from "../../lib/ipc";
import { formatCount, formatDuration } from "../../lib/utils";
import { useQueueStore } from "../../stores/useQueueStore";
import { useSettingsStore } from "../../stores/useSettingsStore";
import { useUIStore } from "../../stores/useUIStore";

export const MediaInspectorCard: React.FC = () => {
  const media = useUIStore((s) => s.inspectedMedia);
  const setInspectedMedia = useUIStore((s) => s.setInspectedMedia);
  const selectedSlides = useUIStore((s) => s.selectedSlides);
  const toggleSlideSelection = useUIStore((s) => s.toggleSlideSelection);
  const selectAllSlides = useUIStore((s) => s.selectAllSlides);
  const deselectAllSlides = useUIStore((s) => s.deselectAllSlides);
  const selectSlidesByType = useUIStore((s) => s.selectSlidesByType);

  const selectedQuality = useUIStore((s) => s.selectedQuality);
  const setSelectedQuality = useUIStore((s) => s.setSelectedQuality);
  const selectedContainer = useUIStore((s) => s.selectedContainer);
  const setSelectedContainer = useUIStore((s) => s.setSelectedContainer);
  const selectedCodec = useUIStore((s) => s.selectedCodec);
  const setSelectedCodec = useUIStore((s) => s.setSelectedCodec);
  const selectedAudioBitrate = useUIStore((s) => s.selectedAudioBitrate);
  const setSelectedAudioBitrate = useUIStore((s) => s.setSelectedAudioBitrate);
  const exactResolution = useUIStore((s) => s.exactResolution);
  const setExactResolution = useUIStore((s) => s.setExactResolution);
  const audioNormalization = useUIStore((s) => s.audioNormalization);
  const setAudioNormalization = useUIStore((s) => s.setAudioNormalization);
  const writeSubtitles = useUIStore((s) => s.writeSubtitles);
  const setWriteSubtitles = useUIStore((s) => s.setWriteSubtitles);
  const embedChapters = useUIStore((s) => s.embedChapters);
  const setEmbedChapters = useUIStore((s) => s.setEmbedChapters);
  const playlistMode = useUIStore((s) => s.playlistMode);
  const setPlaylistMode = useUIStore((s) => s.setPlaylistMode);
  const saveCaption = useUIStore((s) => s.saveCaption);
  const setSaveCaption = useUIStore((s) => s.setSaveCaption);
  const saveMetadata = useUIStore((s) => s.saveMetadata);
  const setSaveMetadata = useUIStore((s) => s.setSaveMetadata);
  const embedThumbnail = useUIStore((s) => s.embedThumbnail);
  const setEmbedThumbnail = useUIStore((s) => s.setEmbedThumbnail);
  const addToast = useUIStore((s) => s.addToast);

  const upsertJob = useQueueStore((s) => s.upsertJob);
  const settings = useSettingsStore((s) => s.settings);

  const [proxiedThumbs, setProxiedThumbs] = useState<Record<number, string>>({});
  const [isEnqueuing, setIsEnqueuing] = useState(false);
  const [showAdvanced, setShowAdvanced] = useState(false);

  useEffect(() => {
    if (!media || !isTauriRuntime()) return;
    let cancelled = false;

    const loadThumbs = async () => {
      const next: Record<number, string> = {};
      for (const slide of media.slides) {
        if (slide.thumbnail_url) {
          try {
            const dataUri = await ipc.proxyThumbnailUrl(slide.thumbnail_url);
            if (!cancelled) {
              next[slide.index] = dataUri;
              setProxiedThumbs((prev) => ({
                ...prev,
                [slide.index]: dataUri,
              }));
            }
          } catch {
            // fallback
          }
        }
      }
    };

    loadThumbs();
    return () => {
      cancelled = true;
    };
  }, [media]);

  if (!media) return null;

  const qualityOptions = [
    { id: "best", label: "Best Available (Up to 4K)" },
    { id: "4k", label: "4K UHD (2160p)" },
    { id: "1440p", label: "2K QHD (1440p)" },
    { id: "1080p60", label: "1080p 60fps" },
    { id: "1080p", label: "1080p Full HD" },
    { id: "720p", label: "720p HD" },
    { id: "480p", label: "480p SD" },
    { id: "360p", label: "360p Compact" },
    { id: "audio_mp3", label: "Audio MP3" },
    { id: "audio_m4a", label: "Audio M4A" },
    { id: "audio_flac", label: "Audio FLAC (Lossless)" },
    { id: "audio_wav", label: "Audio WAV" },
    { id: "audio_opus", label: "Audio OPUS" },
  ];

  const isAudioPreset = selectedQuality.startsWith("audio_");

  const handleEnqueue = async () => {
    if (selectedSlides.length === 0) return;
    setIsEnqueuing(true);
    try {
      const firstThumb =
        proxiedThumbs[1] ?? media.slides[0]?.thumbnail_url ?? null;

      const job = await ipc.enqueueJob({
        source_url: media.source_url,
        platform: media.platform,
        shortcode: media.shortcode,
        url_type: media.url_type,
        author_username: media.author_username,
        caption: media.caption,
        thumbnail_url: firstThumb,
        selected_indices: selectedSlides,
        quality_preset: selectedQuality,
        output_dir: settings?.output_dir ?? null,
        save_caption: saveCaption,
        save_metadata: saveMetadata,
        embed_thumbnail: embedThumbnail,
        naming_template: settings?.naming_template ?? null,
        options: {
          container: selectedContainer,
          preferred_codec: selectedCodec,
          preferred_fps: selectedQuality === "1080p60" ? 60 : null,
          exact_resolution: exactResolution,
          audio_format: isAudioPreset
            ? selectedQuality.replace("audio_", "")
            : settings?.default_audio_format ?? "mp3",
          audio_bitrate: selectedAudioBitrate,
          audio_normalization: audioNormalization,
          write_subtitles: writeSubtitles,
          subtitle_langs: "en,.*-orig",
          embed_chapters: embedChapters,
          playlist_mode: playlistMode,
          speed_limit_kbps: settings?.speed_limit_kbps ?? null,
        },
      });

      upsertJob(job);
      addToast({
        title: "Added to Download Queue",
        description: `@${media.author_username} (${selectedQuality.toUpperCase()})`,
        variant: "success",
      });
      setInspectedMedia(null);
    } catch (err) {
      const serialized = parseIpcError(err);
      addToast({
        title: "Failed to Enqueue Download",
        description: serialized.user_message,
        variant: "error",
      });
    } finally {
      setIsEnqueuing(false);
    }
  };

  const platformBadgeClass =
    media.platform === "youtube"
      ? "bg-rose-500/20 text-rose-300 border-rose-500/35"
      : media.platform === "instagram"
      ? "bg-fuchsia-500/20 text-fuchsia-300 border-fuchsia-500/35"
      : media.platform === "facebook"
      ? "bg-blue-500/20 text-blue-300 border-blue-500/35"
      : "bg-violet-500/20 text-violet-300 border-violet-500/35";

  return (
    <div className="w-full rounded-2xl bg-[#0F131D] border border-violet-500/30 shadow-xl overflow-hidden animate-in fade-in duration-200">
      {/* Top Bar: Platform & Creator Metadata */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-white/[0.08] bg-white/[0.02]">
        <div className="flex items-center gap-3 min-w-0">
          <div className="w-8 h-8 rounded-full master-gradient-bg flex items-center justify-center shrink-0">
            <User className="w-4 h-4 text-white" />
          </div>
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <span
                className={`text-[10px] font-mono uppercase px-1.5 py-0.5 rounded border font-bold ${platformBadgeClass}`}
              >
                {media.platform}
              </span>
              <span className="text-xs font-semibold text-slate-100 truncate">
                @{media.author_username}
              </span>
              <span className="text-[10px] font-mono uppercase px-1.5 py-0.5 rounded bg-white/[0.06] text-slate-300 border border-white/[0.08]">
                {media.url_type}
              </span>
              <span className="text-[10px] font-mono text-slate-500">
                via {media.engine_used}
              </span>
            </div>
            {media.caption && (
              <p className="text-[11px] text-slate-400 truncate max-w-xl mt-0.5">
                {media.caption}
              </p>
            )}
          </div>
        </div>

        <div className="flex items-center gap-4 shrink-0">
          {media.view_count !== null && (
            <span className="flex items-center gap-1 text-xs text-slate-400 font-mono">
              <Eye className="w-3.5 h-3.5 text-sky-400" />
              {formatCount(media.view_count)}
            </span>
          )}
          {media.like_count !== null && (
            <span className="flex items-center gap-1 text-xs text-slate-400 font-mono">
              <Heart className="w-3.5 h-3.5 text-rose-400" />
              {formatCount(media.like_count)}
            </span>
          )}
          <button
            onClick={() => setInspectedMedia(null)}
            className="p-1 rounded-lg text-slate-400 hover:text-slate-200 hover:bg-white/[0.06]"
            title="Dismiss Inspector"
          >
            <X className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* Main Content */}
      <div className="p-4 space-y-4">
        {/* YouTube Playlist Mode Selector */}
        {media.is_playlist && (
          <div className="p-3 rounded-xl bg-[#07090E] border border-rose-500/30 space-y-2">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <ListVideo className="w-4 h-4 text-rose-400" />
                <span className="text-xs font-semibold text-slate-200">
                  Playlist Detected: {media.playlist_title ?? "YouTube Playlist"}{" "}
                  ({media.playlist_count ?? media.playlist_entries.length} videos)
                </span>
              </div>
              <div className="flex items-center gap-1.5">
                <button
                  type="button"
                  onClick={() => setPlaylistMode("playlist")}
                  className={`px-2.5 py-1 rounded-md text-[11px] font-semibold transition-colors ${
                    playlistMode === "playlist"
                      ? "bg-rose-500 text-white"
                      : "bg-white/[0.05] text-slate-400 hover:text-slate-200"
                  }`}
                >
                  Full Playlist
                </button>
                <button
                  type="button"
                  onClick={() => setPlaylistMode("single")}
                  className={`px-2.5 py-1 rounded-md text-[11px] font-semibold transition-colors ${
                    playlistMode === "single"
                      ? "bg-rose-500 text-white"
                      : "bg-white/[0.05] text-slate-400 hover:text-slate-200"
                  }`}
                >
                  Single Video Only
                </button>
              </div>
            </div>
            {playlistMode === "playlist" && media.playlist_entries.length > 0 && (
              <div className="max-h-28 overflow-y-auto space-y-1 pr-1 pt-1">
                {media.playlist_entries.slice(0, 15).map((entry) => (
                  <div
                    key={entry.index}
                    className="flex items-center justify-between text-[11px] px-2 py-1 rounded bg-white/[0.02] text-slate-300"
                  >
                    <span className="truncate max-w-md">
                      <strong className="font-mono text-slate-500 mr-2">
                        #{entry.index}
                      </strong>
                      {entry.title}
                    </span>
                    {entry.duration_seconds && (
                      <span className="font-mono text-slate-500">
                        {formatDuration(entry.duration_seconds)}
                      </span>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {/* Carousel / Multi-Item Slide Selector */}
        {media.slides.length > 1 && (
          <div className="flex items-center justify-between text-xs">
            <span className="text-slate-400">
              Selected{" "}
              <strong className="text-slate-100 font-mono">
                {selectedSlides.length}
              </strong>{" "}
              of{" "}
              <strong className="text-slate-100 font-mono">
                {media.slides.length}
              </strong>{" "}
              carousel items
            </span>
            <div className="flex items-center gap-1.5">
              <button
                type="button"
                onClick={selectAllSlides}
                className="px-2 py-1 rounded bg-white/[0.05] hover:bg-white/[0.1] text-[11px] text-slate-300"
              >
                Select All
              </button>
              <button
                type="button"
                onClick={() => selectSlidesByType("image")}
                className="px-2 py-1 rounded bg-white/[0.05] hover:bg-white/[0.1] text-[11px] text-slate-300"
              >
                Photos Only
              </button>
              <button
                type="button"
                onClick={() => selectSlidesByType("video")}
                className="px-2 py-1 rounded bg-white/[0.05] hover:bg-white/[0.1] text-[11px] text-slate-300"
              >
                Videos Only
              </button>
              <button
                type="button"
                onClick={deselectAllSlides}
                className="px-2 py-1 rounded bg-white/[0.05] hover:bg-white/[0.1] text-[11px] text-slate-400"
              >
                Clear
              </button>
            </div>
          </div>
        )}

        {/* Slide Preview Grid */}
        <div
          className={`grid gap-2.5 max-h-56 overflow-y-auto pr-1 ${
            media.slides.length === 1
              ? "grid-cols-1 sm:grid-cols-2"
              : "grid-cols-3 sm:grid-cols-4 md:grid-cols-6"
          }`}
        >
          {media.slides.map((slide) => {
            const isSelected = selectedSlides.includes(slide.index);
            const thumbSrc = proxiedThumbs[slide.index] ?? slide.thumbnail_url;

            return (
              <div
                key={slide.index}
                onClick={() => toggleSlideSelection(slide.index)}
                className={`group relative rounded-xl overflow-hidden border cursor-pointer transition-all bg-[#07090E] ${
                  media.slides.length === 1 ? "h-36" : "aspect-square"
                } ${
                  isSelected
                    ? "border-violet-500 ring-2 ring-violet-500/30"
                    : "border-white/[0.08] opacity-60 hover:opacity-90"
                }`}
              >
                {thumbSrc ? (
                  <img
                    src={thumbSrc}
                    alt={`Item ${slide.index}`}
                    className="w-full h-full object-cover"
                  />
                ) : (
                  <div className="w-full h-full flex flex-col items-center justify-center text-slate-500 gap-1">
                    {slide.media_type === "video" ? (
                      <Film className="w-6 h-6 text-violet-400" />
                    ) : (
                      <ImageIcon className="w-6 h-6 text-sky-400" />
                    )}
                    <span className="text-[10px] font-mono">
                      #{slide.index}
                    </span>
                  </div>
                )}

                <div className="absolute top-2 left-2 flex items-center gap-1 px-1.5 py-0.5 rounded bg-black/75 backdrop-blur-sm text-[10px] font-mono text-white">
                  {slide.media_type === "video" ? (
                    <Film className="w-3 h-3 text-violet-400" />
                  ) : (
                    <ImageIcon className="w-3 h-3 text-sky-400" />
                  )}
                  <span>#{slide.index}</span>
                </div>

                <div className="absolute top-2 right-2">
                  {isSelected ? (
                    <CheckSquare className="w-4 h-4 text-violet-400 bg-black/70 rounded" />
                  ) : (
                    <Square className="w-4 h-4 text-slate-300 bg-black/60 rounded" />
                  )}
                </div>

                <div className="absolute bottom-1.5 inset-x-1.5 flex items-center justify-between text-[10px] font-mono text-slate-200 bg-black/75 px-1.5 py-0.5 rounded">
                  <span>
                    {slide.width && slide.height
                      ? `${slide.width}×${slide.height}`
                      : slide.media_type.toUpperCase()}
                  </span>
                  {slide.duration_seconds ? (
                    <span>{formatDuration(slide.duration_seconds)}</span>
                  ) : null}
                </div>
              </div>
            );
          })}
        </div>

        {/* Quality Preset Pills */}
        <div className="space-y-2 pt-2 border-t border-white/[0.08]">
          <div className="flex items-center justify-between">
            <span className="text-[11px] font-medium text-slate-400">
              Quality &amp; Format Matrix:
            </span>
            <button
              type="button"
              onClick={() => setShowAdvanced(!showAdvanced)}
              className="flex items-center gap-1 text-[11px] text-violet-300 hover:text-violet-200"
            >
              <Sliders className="w-3 h-3" />
              <span>
                {showAdvanced
                  ? "Hide Advanced Options"
                  : "Advanced Codec / Audio / Subtitles"}
              </span>
            </button>
          </div>

          <div className="flex items-center gap-1.5 flex-wrap">
            {qualityOptions.map((q) => {
              const active = selectedQuality === q.id;
              const isAudio = q.id.startsWith("audio_");
              return (
                <button
                  key={q.id}
                  type="button"
                  onClick={() => setSelectedQuality(q.id)}
                  className={`flex items-center gap-1 px-2.5 py-1 rounded-lg text-xs font-medium transition-all ${
                    active
                      ? "bg-violet-600 text-white shadow-sm"
                      : "bg-[#07090E] text-slate-400 hover:text-slate-200 border border-white/[0.08]"
                  }`}
                >
                  {isAudio && <Music className="w-3 h-3" />}
                  <span>{q.label}</span>
                </button>
              );
            })}
          </div>
        </div>

        {/* Advanced Options Drawer (FlowDL + InstaFlow + FlowFB converged controls) */}
        {showAdvanced && (
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 p-3 rounded-xl bg-[#07090E] border border-white/[0.08] text-xs">
            <div>
              <label className="block text-[10px] text-slate-400 mb-1">
                Video Container
              </label>
              <select
                value={selectedContainer}
                onChange={(e) => setSelectedContainer(e.target.value)}
                className="w-full px-2.5 py-1.5 rounded-lg bg-[#0F131D] border border-white/[0.08] text-xs text-slate-200"
              >
                <option value="mp4">MP4 (Universal)</option>
                <option value="mkv">MKV (Matroska)</option>
                <option value="webm">WebM</option>
                <option value="auto">Auto</option>
              </select>
            </div>

            <div>
              <label className="block text-[10px] text-slate-400 mb-1">
                Preferred Video Codec
              </label>
              <select
                value={selectedCodec}
                onChange={(e) => setSelectedCodec(e.target.value)}
                className="w-full px-2.5 py-1.5 rounded-lg bg-[#0F131D] border border-white/[0.08] text-xs text-slate-200"
              >
                <option value="auto">Auto (Best Quality)</option>
                <option value="h264">H.264 / AVC (Max Compatibility)</option>
                <option value="vp9">VP9 (4K / HDR)</option>
                <option value="av1">AV1 (Next-Gen)</option>
                <option value="h265">H.265 / HEVC</option>
              </select>
            </div>

            <div>
              <label className="block text-[10px] text-slate-400 mb-1">
                Audio Bitrate
              </label>
              <select
                value={selectedAudioBitrate}
                onChange={(e) => setSelectedAudioBitrate(e.target.value)}
                className="w-full px-2.5 py-1.5 rounded-lg bg-[#0F131D] border border-white/[0.08] text-xs text-slate-200"
              >
                <option value="320">320 kbps (Studio)</option>
                <option value="256">256 kbps (High)</option>
                <option value="192">192 kbps (Standard)</option>
                <option value="128">128 kbps (Compact)</option>
              </select>
            </div>

            <div className="flex flex-col justify-center gap-1.5">
              <label className="flex items-center gap-2 text-[11px] text-slate-300 cursor-pointer">
                <input
                  type="checkbox"
                  checked={exactResolution}
                  onChange={(e) => setExactResolution(e.target.checked)}
                  className="rounded accent-violet-500"
                />
                <span>Exact Resolution Match</span>
              </label>
              <label className="flex items-center gap-2 text-[11px] text-slate-300 cursor-pointer">
                <input
                  type="checkbox"
                  checked={audioNormalization}
                  onChange={(e) => setAudioNormalization(e.target.checked)}
                  className="rounded accent-violet-500"
                />
                <Volume2 className="w-3 h-3 text-emerald-400" />
                <span>EBU R128 Loudnorm</span>
              </label>
            </div>
          </div>
        )}

        {/* Footer Sidecars & Enqueue Button */}
        <div className="flex flex-wrap items-center justify-between gap-3 pt-2 border-t border-white/[0.08]">
          <div className="flex items-center gap-4 text-xs text-slate-300 flex-wrap">
            <label className="flex items-center gap-1.5 cursor-pointer">
              <input
                type="checkbox"
                checked={writeSubtitles}
                onChange={(e) => setWriteSubtitles(e.target.checked)}
                className="rounded accent-violet-500"
              />
              <Subtitles className="w-3.5 h-3.5 text-sky-400" />
              <span>Subtitles (.srt/.vtt)</span>
            </label>

            <label className="flex items-center gap-1.5 cursor-pointer">
              <input
                type="checkbox"
                checked={embedChapters}
                onChange={(e) => setEmbedChapters(e.target.checked)}
                className="rounded accent-violet-500"
              />
              <span>Embed Chapters</span>
            </label>

            <label className="flex items-center gap-1.5 cursor-pointer">
              <input
                type="checkbox"
                checked={saveCaption}
                onChange={(e) => setSaveCaption(e.target.checked)}
                className="rounded accent-violet-500"
              />
              <span>Save Caption (.txt)</span>
            </label>

            <label className="flex items-center gap-1.5 cursor-pointer">
              <input
                type="checkbox"
                checked={saveMetadata}
                onChange={(e) => setSaveMetadata(e.target.checked)}
                className="rounded accent-violet-500"
              />
              <span>Save JSON Metadata</span>
            </label>

            <label className="flex items-center gap-1.5 cursor-pointer">
              <input
                type="checkbox"
                checked={embedThumbnail}
                onChange={(e) => setEmbedThumbnail(e.target.checked)}
                className="rounded accent-violet-500"
              />
              <span>Embed Thumbnail</span>
            </label>
          </div>

          <button
            type="button"
            onClick={handleEnqueue}
            disabled={selectedSlides.length === 0 || isEnqueuing}
            className="flex items-center gap-2 px-5 py-2 rounded-xl master-gradient-bg hover:opacity-95 disabled:opacity-40 text-xs font-semibold text-white shadow-lg transition-all"
          >
            <Download className="w-4 h-4" />
            <span>
              {media.is_playlist && playlistMode === "playlist"
                ? `Download Full Playlist (${media.playlist_count ?? "All"})`
                : `Download Selected (${selectedSlides.length})`}
            </span>
          </button>
        </div>
      </div>
    </div>
  );
};
