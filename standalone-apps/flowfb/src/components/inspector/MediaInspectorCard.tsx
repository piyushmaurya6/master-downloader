import React, { useState, useEffect } from "react";
import { motion } from "framer-motion";
import {
  Download,
  CheckSquare,
  Square,
  Film,
  Image as ImageIcon,
  Music,
  User,
  Calendar,
  X,
  Sparkles,
  Loader2,
  Eye,
  ThumbsUp,
  Clock,
  Cpu,
} from "lucide-react";
import { useUIStore } from "../../stores/useUIStore";
import { useQueueStore } from "../../stores/useQueueStore";
import { useSettingsStore } from "../../stores/useSettingsStore";
import { ipc, parseSerializedError } from "../../lib/ipc";
import {
  formatDuration,
  formatRelativeDate,
  formatCompactNumber,
  formatBytes,
} from "../../lib/utils";
import { ProxiedThumbnail } from "../ui/ProxiedThumbnail";

export const MediaInspectorCard: React.FC = () => {
  const { inspectedMedia, setInspectedMedia, addToast } = useUIStore();
  const { upsertJob } = useQueueStore();
  const { settings } = useSettingsStore();

  const [selectedIndices, setSelectedIndices] = useState<number[]>([]);
  const [qualityPreset, setQualityPreset] = useState<string>("best");
  const [saveCaption, setSaveCaption] = useState<boolean>(false);
  const [saveMetadata, setSaveMetadata] = useState<boolean>(false);
  const [embedThumbnail, setEmbedThumbnail] = useState<boolean>(true);
  const [isEnqueueing, setIsEnqueueing] = useState<boolean>(false);

  useEffect(() => {
    if (inspectedMedia) {
      setSelectedIndices(inspectedMedia.slides.map((s) => s.index));
      setQualityPreset(settings?.default_quality || "best");
      setSaveCaption(settings?.save_caption_by_default ?? false);
      setSaveMetadata(settings?.save_metadata_by_default ?? false);
      setEmbedThumbnail(settings?.embed_thumbnail_by_default ?? true);
    }
  }, [inspectedMedia, settings]);

  if (!inspectedMedia) return null;

  const isMultiItem = inspectedMedia.total_items > 1;
  const isVerticalReel =
    inspectedMedia.aspect_ratio_label === "9:16" ||
    inspectedMedia.type_badge === "REEL";

  const toggleSlide = (idx: number) => {
    setSelectedIndices((prev) =>
      prev.includes(idx) ? prev.filter((i) => i !== idx) : [...prev, idx].sort((a, b) => a - b)
    );
  };

  const selectAll = () => {
    setSelectedIndices(inspectedMedia.slides.map((s) => s.index));
  };

  const deselectAll = () => {
    setSelectedIndices([]);
  };

  const selectVideosOnly = () => {
    setSelectedIndices(
      inspectedMedia.slides
        .filter((s) => s.media_type === "video")
        .map((s) => s.index)
    );
  };

  const handleDownload = async () => {
    if (isMultiItem && selectedIndices.length === 0) {
      addToast({
        type: "warning",
        title: "No Slides Selected",
        message: "Select at least one item to download.",
      });
      return;
    }

    setIsEnqueueing(true);
    try {
      const job = await ipc.enqueueJob({
        source_url: inspectedMedia.canonical_url,
        shortcode: inspectedMedia.shortcode,
        url_type: inspectedMedia.url_type,
        author_username: inspectedMedia.owner_username,
        caption: inspectedMedia.caption || inspectedMedia.title,
        thumbnail_url:
          inspectedMedia.thumbnail_url ||
          inspectedMedia.slides[0]?.display_url ||
          null,
        selected_indices: isMultiItem ? selectedIndices : [1],
        quality_preset: qualityPreset,
        output_dir: settings?.output_dir ?? null,
        naming_template: settings?.naming_template ?? null,
        save_caption: saveCaption,
        save_metadata: saveMetadata,
        embed_thumbnail: embedThumbnail,
      });

      upsertJob(job);
      addToast({
        type: "success",
        title: "Added to Download Queue",
        message: `Queued ${inspectedMedia.owner_username} (${inspectedMedia.type_badge})`,
      });
      setInspectedMedia(null);
    } catch (err) {
      const parsed = parseSerializedError(err);
      addToast({
        type: "error",
        title: "Failed to Queue Download",
        message: parsed.user_message,
      });
    } finally {
      setIsEnqueueing(false);
    }
  };

  return (
    <motion.div
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: -10 }}
      className="rounded-2xl bg-bg-surface border border-blue-500/30 shadow-[0_10px_40px_-15px_rgba(24,119,242,0.28)] overflow-hidden"
    >
      {/* Top Bar */}
      <div className="flex items-center justify-between px-5 py-3.5 border-b border-border-subtle bg-bg-elevated/40">
        <div className="flex items-center gap-3 flex-wrap">
          <span className="px-2.5 py-0.5 text-[10px] font-mono font-bold uppercase tracking-wider rounded-md fb-gradient-bg text-white">
            {inspectedMedia.type_badge}
          </span>
          <div className="flex items-center gap-1.5 text-xs font-semibold text-text-primary">
            <User className="w-3.5 h-3.5 text-accent-primary" />
            <span>{inspectedMedia.owner_username}</span>
            {inspectedMedia.owner_full_name &&
              inspectedMedia.owner_full_name !== inspectedMedia.owner_username && (
                <span className="text-text-muted font-normal">
                  ({inspectedMedia.owner_full_name})
                </span>
              )}
          </div>
          {inspectedMedia.timestamp && (
            <div className="flex items-center gap-1 text-[11px] text-text-muted">
              <Calendar className="w-3 h-3" />
              <span>{formatRelativeDate(inspectedMedia.timestamp)}</span>
            </div>
          )}
          {inspectedMedia.duration_seconds && inspectedMedia.duration_seconds > 0 && (
            <div className="flex items-center gap-1 text-[11px] text-sky-400 font-mono bg-sky-500/10 px-2 py-0.5 rounded-md border border-sky-500/20">
              <Clock className="w-3 h-3" />
              <span>{formatDuration(inspectedMedia.duration_seconds)}</span>
            </div>
          )}
          {inspectedMedia.view_count !== null && inspectedMedia.view_count > 0 && (
            <div className="flex items-center gap-1 text-[11px] text-text-secondary font-mono">
              <Eye className="w-3 h-3 text-text-muted" />
              <span>{formatCompactNumber(inspectedMedia.view_count)} views</span>
            </div>
          )}
          {inspectedMedia.like_count !== null && inspectedMedia.like_count > 0 && (
            <div className="flex items-center gap-1 text-[11px] text-text-secondary font-mono">
              <ThumbsUp className="w-3 h-3 text-text-muted" />
              <span>{formatCompactNumber(inspectedMedia.like_count)}</span>
            </div>
          )}
        </div>

        <button
          onClick={() => setInspectedMedia(null)}
          className="p-1 rounded-lg text-text-muted hover:text-text-primary hover:bg-bg-elevated transition-colors"
        >
          <X className="w-4 h-4" />
        </button>
      </div>

      {/* Main Content */}
      <div className="p-5 space-y-5">
        {/* Preview + Title/Caption + Format Matrix */}
        <div className="flex flex-col lg:flex-row gap-5">
          {/* Single Media Preview (if not multi-item album) */}
          {!isMultiItem && (
            <div
              className={`relative rounded-xl overflow-hidden bg-bg-canvas border border-border-subtle shrink-0 ${
                isVerticalReel ? "w-36 aspect-[9/16]" : "w-64 aspect-video"
              }`}
            >
              <ProxiedThumbnail
                src={
                  inspectedMedia.thumbnail_url ||
                  inspectedMedia.slides[0]?.display_url
                }
                alt={inspectedMedia.title}
                className="w-full h-full object-cover"
              />
              <div className="absolute top-2 left-2 px-1.5 py-0.5 rounded bg-black/75 text-[10px] font-mono text-white">
                {inspectedMedia.aspect_ratio_label}
              </div>
              {inspectedMedia.duration_seconds && (
                <div className="absolute bottom-2 right-2 px-1.5 py-0.5 rounded bg-black/80 text-[10px] font-mono text-white">
                  {formatDuration(inspectedMedia.duration_seconds)}
                </div>
              )}
            </div>
          )}

          {/* Right Column: Title, Caption, & DASH Stream Format Matrix */}
          <div className="flex-1 space-y-4 min-w-0">
            <div>
              <h3 className="text-sm font-semibold text-text-primary line-clamp-1">
                {inspectedMedia.title || "Facebook Media"}
              </h3>
              {inspectedMedia.caption && (
                <p className="text-xs text-text-secondary line-clamp-2 mt-1 leading-relaxed">
                  {inspectedMedia.caption}
                </p>
              )}
            </div>

            {/* DASH Video & Audio Format Matrix */}
            {inspectedMedia.format_options &&
              inspectedMedia.format_options.length > 0 && (
                <div className="space-y-2">
                  <div className="flex items-center justify-between">
                    <span className="text-[11px] font-semibold uppercase tracking-wider text-text-secondary flex items-center gap-1.5">
                      <Cpu className="w-3.5 h-3.5 text-accent-primary" />
                      Available Facebook DASH Streams & Audio Formats
                    </span>
                    <span className="text-[10px] font-mono text-text-muted">
                      FFmpeg Lossless Remux Enabled
                    </span>
                  </div>

                  <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 gap-2">
                    {inspectedMedia.format_options.map((fmt) => {
                      const active = qualityPreset === fmt.quality_key;
                      return (
                        <button
                          key={fmt.quality_key}
                          type="button"
                          onClick={() => setQualityPreset(fmt.quality_key)}
                          className={`flex items-center justify-between p-2.5 rounded-xl border text-left transition-all ${
                            active
                              ? "bg-accent-primary/15 border-accent-primary text-text-primary shadow-sm"
                              : "bg-bg-canvas/70 border-border-subtle text-text-secondary hover:border-border-focus hover:text-text-primary"
                          }`}
                        >
                          <div className="min-w-0 pr-2">
                            <div className="flex items-center gap-1.5">
                              {fmt.is_audio_only ? (
                                <Music className="w-3.5 h-3.5 text-sky-400 shrink-0" />
                              ) : (
                                <Film className="w-3.5 h-3.5 text-accent-primary shrink-0" />
                              )}
                              <span className="text-xs font-semibold truncate">
                                {fmt.resolution_label}
                              </span>
                            </div>
                            <div className="text-[10px] font-mono text-text-muted mt-0.5 flex items-center gap-1.5">
                              <span>
                                {fmt.is_audio_only
                                  ? `${fmt.acodec} • .${fmt.ext}`
                                  : `${fmt.vcodec}+${fmt.acodec} • .${fmt.ext}`}
                              </span>
                              {fmt.filesize_bytes && fmt.filesize_bytes > 0 && (
                                <span>• ~{formatBytes(fmt.filesize_bytes)}</span>
                              )}
                            </div>
                          </div>
                          <span
                            className={`px-1.5 py-0.5 text-[9px] font-mono font-bold rounded shrink-0 ${
                              active
                                ? "fb-gradient-bg text-white"
                                : "bg-bg-elevated text-text-muted"
                            }`}
                          >
                            {fmt.badge}
                          </span>
                        </button>
                      );
                    })}
                  </div>
                </div>
              )}
          </div>
        </div>

        {/* Multi-Item Album / Post Grid */}
        {isMultiItem && (
          <div className="space-y-3 pt-2 border-t border-border-subtle">
            <div className="flex items-center justify-between flex-wrap gap-2">
              <div className="text-xs font-medium text-text-primary">
                Select Post Items ({selectedIndices.length} of{" "}
                {inspectedMedia.total_items} selected)
              </div>
              <div className="flex items-center gap-2">
                <button
                  onClick={selectAll}
                  className="px-2.5 py-1 rounded-md text-[11px] font-medium bg-bg-elevated hover:bg-bg-canvas text-text-secondary hover:text-text-primary border border-border-subtle transition-colors"
                >
                  Select All
                </button>
                <button
                  onClick={deselectAll}
                  className="px-2.5 py-1 rounded-md text-[11px] font-medium bg-bg-elevated hover:bg-bg-canvas text-text-secondary hover:text-text-primary border border-border-subtle transition-colors"
                >
                  Deselect All
                </button>
                <button
                  onClick={selectVideosOnly}
                  className="px-2.5 py-1 rounded-md text-[11px] font-medium bg-bg-elevated hover:bg-bg-canvas text-text-secondary hover:text-text-primary border border-border-subtle transition-colors"
                >
                  Videos Only
                </button>
              </div>
            </div>

            <div className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-5 gap-3 max-h-64 overflow-y-auto p-1">
              {inspectedMedia.slides.map((slide) => {
                const isSelected = selectedIndices.includes(slide.index);
                return (
                  <div
                    key={slide.index}
                    onClick={() => toggleSlide(slide.index)}
                    className={`group relative aspect-square rounded-xl overflow-hidden border-2 cursor-pointer transition-all ${
                      isSelected
                        ? "border-accent-primary ring-2 ring-accent-primary/20"
                        : "border-border-subtle opacity-60 hover:opacity-90"
                    }`}
                  >
                    <ProxiedThumbnail
                      src={slide.display_url}
                      alt={`Item ${slide.index}`}
                      className="w-full h-full object-cover"
                    />
                    <div className="absolute inset-0 bg-gradient-to-t from-black/70 via-transparent to-black/30" />

                    <div className="absolute top-2 left-2 flex items-center gap-1 px-1.5 py-0.5 rounded bg-black/70 text-[10px] font-mono text-white">
                      <span>#{slide.index}</span>
                      {slide.media_type === "video" ? (
                        <Film className="w-3 h-3 text-sky-400" />
                      ) : (
                        <ImageIcon className="w-3 h-3 text-emerald-400" />
                      )}
                    </div>

                    <div className="absolute top-2 right-2">
                      {isSelected ? (
                        <CheckSquare className="w-4 h-4 text-accent-primary bg-black/60 rounded" />
                      ) : (
                        <Square className="w-4 h-4 text-white/70 bg-black/40 rounded" />
                      )}
                    </div>

                    <div className="absolute bottom-2 left-2 right-2 flex items-center justify-between text-[10px] font-mono text-white/90">
                      <span>
                        {slide.width > 0 ? `${slide.width}×${slide.height}` : slide.ext.toUpperCase()}
                      </span>
                      {slide.duration_seconds && (
                        <span>{formatDuration(slide.duration_seconds)}</span>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        )}

        {/* Sidecar Options & Download CTA */}
        <div className="flex flex-wrap items-center justify-between gap-4 pt-3 border-t border-border-subtle">
          <div className="flex flex-wrap items-center gap-4">
            <label className="flex items-center gap-1.5 text-xs text-text-secondary cursor-pointer">
              <input
                type="checkbox"
                checked={embedThumbnail}
                onChange={(e) => setEmbedThumbnail(e.target.checked)}
                className="rounded border-border-subtle accent-accent-primary"
              />
              <span>Embed Cover Thumbnail</span>
            </label>

            <label className="flex items-center gap-1.5 text-xs text-text-secondary cursor-pointer">
              <input
                type="checkbox"
                checked={saveCaption}
                onChange={(e) => setSaveCaption(e.target.checked)}
                className="rounded border-border-subtle accent-accent-primary"
              />
              <span>Save Caption (.txt)</span>
            </label>

            <label className="flex items-center gap-1.5 text-xs text-text-secondary cursor-pointer">
              <input
                type="checkbox"
                checked={saveMetadata}
                onChange={(e) => setSaveMetadata(e.target.checked)}
                className="rounded border-border-subtle accent-accent-primary"
              />
              <span>Save Metadata (.json)</span>
            </label>
          </div>

          <button
            onClick={handleDownload}
            disabled={isEnqueueing}
            className="flex items-center gap-2 px-5 py-2.5 rounded-xl text-xs font-semibold text-white fb-gradient-bg hover:opacity-95 disabled:opacity-40 transition-all shadow-md"
          >
            {isEnqueueing ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <Download className="w-4 h-4" />
            )}
            <span>
              {isMultiItem
                ? `Download Selected (${selectedIndices.length})`
                : qualityPreset.startsWith("audio_")
                ? "Extract Audio Now"
                : "Download Now"}
            </span>
            <Sparkles className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>
    </motion.div>
  );
};
