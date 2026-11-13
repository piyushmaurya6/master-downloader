import React, { useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import {
  AlertTriangle,
  CheckSquare,
  Download,
  Film,
  Image as ImageIcon,
  KeyRound,
  Square,
  User,
  X,
} from "lucide-react";
import { ipc, parseIpcError } from "../../lib/ipc";
import { formatRelativeDate } from "../../lib/utils";
import { useQueueStore } from "../../stores/useQueueStore";
import { useUIStore } from "../../stores/useUIStore";
import { ProxiedThumbnail } from "../ui/ProxiedThumbnail";

export const MediaInspectorCard: React.FC = () => {
  const inspectedMedia = useUIStore((s) => s.inspectedMedia);
  const setInspectedMedia = useUIStore((s) => s.setInspectedMedia);
  const selectedSlideIndices = useUIStore((s) => s.selectedSlideIndices);
  const toggleSlideIndex = useUIStore((s) => s.toggleSlideIndex);
  const selectAllSlides = useUIStore((s) => s.selectAllSlides);
  const selectPhotosOnly = useUIStore((s) => s.selectPhotosOnly);
  const selectVideosOnly = useUIStore((s) => s.selectVideosOnly);
  const selectedQuality = useUIStore((s) => s.selectedQuality);
  const setSelectedQuality = useUIStore((s) => s.setSelectedQuality);
  const saveCaption = useUIStore((s) => s.saveCaption);
  const setSaveCaption = useUIStore((s) => s.setSaveCaption);
  const saveMetadata = useUIStore((s) => s.saveMetadata);
  const setSaveMetadata = useUIStore((s) => s.setSaveMetadata);
  const embedThumbnail = useUIStore((s) => s.embedThumbnail);
  const setEmbedThumbnail = useUIStore((s) => s.setEmbedThumbnail);
  const inspectorError = useUIStore((s) => s.inspectorError);
  const setInspectorError = useUIStore((s) => s.setInspectorError);
  const setAuthModalOpen = useUIStore((s) => s.setAuthModalOpen);
  const setOnboardingModalOpen = useUIStore((s) => s.setOnboardingModalOpen);
  const addToast = useUIStore((s) => s.addToast);

  const upsertJob = useQueueStore((s) => s.upsertJob);

  const [expandCaption, setExpandCaption] = useState(false);
  const [submitting, setSubmitting] = useState(false);

  const handleEnqueueInspected = async () => {
    if (!inspectedMedia || selectedSlideIndices.length === 0) return;
    setSubmitting(true);
    try {
      const job = await ipc.enqueueJob({
        source_url: inspectedMedia.canonical_url,
        shortcode: inspectedMedia.shortcode,
        url_type: inspectedMedia.url_type,
        author_username: inspectedMedia.owner_username,
        caption: inspectedMedia.caption,
        thumbnail_url: inspectedMedia.thumbnail_url,
        selected_indices: selectedSlideIndices,
        quality_preset: selectedQuality,
        save_caption: saveCaption,
        save_metadata: saveMetadata,
        embed_thumbnail: embedThumbnail,
      });
      upsertJob(job);
      addToast({
        title: `Enqueued ${selectedSlideIndices.length} Item(s) from @${inspectedMedia.owner_username}`,
        description: inspectedMedia.canonical_url,
        variant: "success",
      });
      setInspectedMedia(null);
    } catch (err) {
      addToast({
        title: "Failed to Enqueue Media",
        description: parseIpcError(err).user_message,
        variant: "error",
      });
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <AnimatePresence mode="wait">
      {inspectorError && (
        <motion.div
          key="inspector-error"
          initial={{ opacity: 0, y: -8 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: -8 }}
          className="rounded-2xl bg-[#121215] border border-rose-500/30 p-4 flex items-start justify-between gap-4"
        >
          <div className="flex items-start gap-3">
            <div className="p-2 rounded-xl bg-rose-500/15 text-rose-400 mt-0.5">
              <AlertTriangle className="w-4 h-4" />
            </div>
            <div>
              <h3 className="text-xs font-semibold text-zinc-100">
                Could Not Inspect Instagram Link ({inspectorError.code})
              </h3>
              <p className="text-xs text-zinc-400 mt-0.5">
                {inspectorError.user_message}
              </p>
              <div className="flex items-center gap-2 mt-3">
                {inspectorError.remediation_action === "CONNECT_ACCOUNT" && (
                  <button
                    onClick={() => setAuthModalOpen(true)}
                    className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium bg-indigo-500/20 text-indigo-300 hover:bg-indigo-500/30 border border-indigo-500/30"
                  >
                    <KeyRound className="w-3.5 h-3.5" />
                    Connect Instagram Account
                  </button>
                )}
                {inspectorError.remediation_action === "BOOTSTRAP_BINARIES" && (
                  <button
                    onClick={() => setOnboardingModalOpen(true)}
                    className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium bg-amber-500/20 text-amber-300 hover:bg-amber-500/30 border border-amber-500/30"
                  >
                    Setup Media Engines
                  </button>
                )}
              </div>
            </div>
          </div>
          <button
            onClick={() => setInspectorError(null)}
            className="p-1 text-zinc-500 hover:text-zinc-300"
          >
            <X className="w-4 h-4" />
          </button>
        </motion.div>
      )}

      {inspectedMedia && (
        <motion.div
          key={inspectedMedia.canonical_url}
          initial={{ opacity: 0, y: 12, scale: 0.99 }}
          animate={{ opacity: 1, y: 0, scale: 1 }}
          exit={{ opacity: 0, y: -8, scale: 0.99 }}
          transition={{ type: "spring", stiffness: 380, damping: 30 }}
          className="rounded-2xl bg-[#121215] border border-indigo-500/25 p-5 shadow-2xl space-y-4"
        >
          {/* Inspector Header */}
          <div className="flex items-start justify-between gap-4">
            <div className="flex items-center gap-3 min-w-0">
              <div className="w-10 h-10 rounded-full bg-gradient-to-tr from-indigo-500/30 to-violet-500/30 border border-indigo-500/30 flex items-center justify-center shrink-0">
                <User className="w-5 h-5 text-indigo-300" />
              </div>
              <div className="min-w-0">
                <div className="flex items-center gap-2">
                  <span className="text-sm font-semibold text-zinc-100 truncate">
                    @{inspectedMedia.owner_username}
                  </span>
                  <span className="px-2 py-0.5 text-[10px] font-mono font-bold rounded bg-indigo-500/20 text-indigo-300 border border-indigo-500/30">
                    {inspectedMedia.type_badge}
                  </span>
                  <span className="text-[11px] font-mono text-zinc-400">
                    {inspectedMedia.total_items}{" "}
                    {inspectedMedia.total_items === 1 ? "item" : "slides"}
                  </span>
                  {inspectedMedia.timestamp && (
                    <span className="text-[11px] text-zinc-500">
                      · {formatRelativeDate(inspectedMedia.timestamp)}
                    </span>
                  )}
                </div>

                {inspectedMedia.caption && (
                  <div className="mt-1">
                    <p
                      className={`text-xs text-zinc-400 ${
                        expandCaption ? "" : "line-clamp-1"
                      }`}
                    >
                      {inspectedMedia.caption}
                    </p>
                    {inspectedMedia.caption.length > 90 && (
                      <button
                        onClick={() => setExpandCaption((v) => !v)}
                        className="text-[11px] text-indigo-400 hover:underline mt-0.5"
                      >
                        {expandCaption ? "Show less" : "Expand caption"}
                      </button>
                    )}
                  </div>
                )}
              </div>
            </div>

            <button
              onClick={() => setInspectedMedia(null)}
              className="p-1.5 rounded-lg text-zinc-400 hover:text-zinc-100 hover:bg-white/[0.06]"
              aria-label="Close Inspector"
            >
              <X className="w-4 h-4" />
            </button>
          </div>

          {/* Carousel Selector Controls (if multi-item post) */}
          {inspectedMedia.slides.length > 1 && (
            <div className="flex flex-wrap items-center justify-between gap-2 pt-2 border-t border-white/[0.07]">
              <span className="text-xs font-medium text-zinc-300">
                Carousel Slide Selection ({selectedSlideIndices.length} of{" "}
                {inspectedMedia.slides.length} selected)
              </span>
              <div className="flex items-center gap-1.5">
                <button
                  onClick={selectAllSlides}
                  className="px-2.5 py-1 rounded-md text-[11px] font-medium bg-white/[0.06] hover:bg-white/[0.12] text-zinc-200 transition-colors"
                >
                  Select All
                </button>
                <button
                  onClick={selectPhotosOnly}
                  className="px-2.5 py-1 rounded-md text-[11px] font-medium bg-white/[0.06] hover:bg-white/[0.12] text-zinc-200 transition-colors"
                >
                  Photos Only
                </button>
                <button
                  onClick={selectVideosOnly}
                  className="px-2.5 py-1 rounded-md text-[11px] font-medium bg-white/[0.06] hover:bg-white/[0.12] text-zinc-200 transition-colors"
                >
                  Videos Only
                </button>
              </div>
            </div>
          )}

          {/* Slides Preview Grid */}
          <div className="grid grid-cols-2 sm:grid-cols-4 md:grid-cols-5 gap-2.5 max-h-56 overflow-y-auto pr-1">
            {inspectedMedia.slides.map((slide) => {
              const selected = selectedSlideIndices.includes(slide.index);
              return (
                <button
                  key={slide.index}
                  type="button"
                  onClick={() => toggleSlideIndex(slide.index)}
                  className={`group relative rounded-xl overflow-hidden border text-left transition-all ${
                    selected
                      ? "border-indigo-500 ring-2 ring-indigo-500/30"
                      : "border-white/10 opacity-60 hover:opacity-90"
                  }`}
                >
                  <ProxiedThumbnail
                    src={slide.display_url}
                    mediaType={slide.media_type}
                    className="w-full h-28"
                  />

                  {/* Top-left Slide Index + Checkbox */}
                  <div className="absolute top-1.5 left-1.5 flex items-center gap-1 px-1.5 py-0.5 rounded bg-black/75 backdrop-blur-sm text-[10px] font-mono text-white">
                    {selected ? (
                      <CheckSquare className="w-3 h-3 text-indigo-400" />
                    ) : (
                      <Square className="w-3 h-3 text-zinc-400" />
                    )}
                    <span>
                      {slide.index}/{inspectedMedia.slides.length}
                    </span>
                  </div>

                  {/* Top-right Media Type Badge */}
                  <div className="absolute top-1.5 right-1.5 px-1.5 py-0.5 rounded bg-black/75 backdrop-blur-sm text-[9px] font-mono font-semibold uppercase text-zinc-200 flex items-center gap-1">
                    {slide.media_type === "video" ? (
                      <>
                        <Film className="w-2.5 h-2.5 text-indigo-400" />
                        VIDEO
                      </>
                    ) : (
                      <>
                        <ImageIcon className="w-2.5 h-2.5 text-emerald-400" />
                        PHOTO
                      </>
                    )}
                  </div>

                  {/* Bottom Resolution */}
                  <div className="absolute bottom-1.5 right-1.5 px-1.5 py-0.5 rounded bg-black/75 text-[9px] font-mono text-zinc-300">
                    {slide.width}x{slide.height}
                  </div>
                </button>
              );
            })}
          </div>

          {/* Quality & Format Picker + Metadata Toggles */}
          <div className="pt-3 border-t border-white/[0.07] flex flex-wrap items-center justify-between gap-4">
            <div className="flex flex-wrap items-center gap-4">
              <div>
                <label className="block text-[10px] uppercase tracking-wider font-semibold text-zinc-400 mb-1">
                  Quality &amp; Stream Format
                </label>
                <select
                  value={selectedQuality}
                  onChange={(e) => setSelectedQuality(e.target.value)}
                  className="px-3 py-1.5 rounded-lg bg-[#09090B] border border-white/10 text-xs text-zinc-100 focus:outline-none focus:border-indigo-500"
                >
                  <option value="best">
                    Highest Available (Best Video + Audio)
                  </option>
                  <option value="1080p">1080p MP4</option>
                  <option value="720p">720p MP4</option>
                  <option value="audio_only">Audio Only (M4A)</option>
                </select>
              </div>

              <div className="flex items-center gap-3 pt-4">
                <label className="inline-flex items-center gap-1.5 text-xs text-zinc-300 cursor-pointer">
                  <input
                    type="checkbox"
                    checked={saveCaption}
                    onChange={(e) => setSaveCaption(e.target.checked)}
                    className="rounded border-white/20 bg-zinc-900 text-indigo-500 focus:ring-indigo-500"
                  />
                  <span>Save Caption (.txt)</span>
                </label>

                <label className="inline-flex items-center gap-1.5 text-xs text-zinc-300 cursor-pointer">
                  <input
                    type="checkbox"
                    checked={saveMetadata}
                    onChange={(e) => setSaveMetadata(e.target.checked)}
                    className="rounded border-white/20 bg-zinc-900 text-indigo-500 focus:ring-indigo-500"
                  />
                  <span>Save Metadata (.json)</span>
                </label>

                <label className="inline-flex items-center gap-1.5 text-xs text-zinc-300 cursor-pointer">
                  <input
                    type="checkbox"
                    checked={embedThumbnail}
                    onChange={(e) => setEmbedThumbnail(e.target.checked)}
                    className="rounded border-white/20 bg-zinc-900 text-indigo-500 focus:ring-indigo-500"
                  />
                  <span>Embed Thumbnail</span>
                </label>
              </div>
            </div>

            <button
              onClick={handleEnqueueInspected}
              disabled={selectedSlideIndices.length === 0 || submitting}
              className="inline-flex items-center gap-2 px-5 py-2.5 rounded-xl text-xs font-semibold text-white bg-gradient-to-r from-indigo-500 to-violet-600 hover:from-indigo-400 hover:to-violet-500 shadow-lg shadow-indigo-500/25 disabled:opacity-40 transition-all"
            >
              <Download className="w-4 h-4" />
              <span>
                Download Selected ({selectedSlideIndices.length}{" "}
                {selectedSlideIndices.length === 1 ? "Item" : "Items"})
              </span>
            </button>
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  );
};
