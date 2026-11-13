import React, { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  Layers,
  X,
  CheckCircle2,
  AlertCircle,
  Download,
  Loader2,
  Upload,
} from "lucide-react";
import { useUIStore } from "../../stores/useUIStore";
import { useQueueStore } from "../../stores/useQueueStore";
import { useSettingsStore } from "../../stores/useSettingsStore";
import { ipc, parseSerializedError } from "../../lib/ipc";
import type { BatchUrlParseResult } from "../../types";

const QUALITY_PRESETS = [
  { id: "best", label: "Max Available (4K / 1080p60 DASH)" },
  { id: "4k", label: "4K Ultra HD (2160p)" },
  { id: "1440p", label: "2K QHD (1440p)" },
  { id: "1080p", label: "1080p Full HD" },
  { id: "720p", label: "720p HD" },
  { id: "480p", label: "480p SD" },
  { id: "audio_mp3", label: "Audio Only — MP3 (320kbps)" },
  { id: "audio_m4a", label: "Audio Only — M4A (AAC)" },
];

export const BatchUrlModal: React.FC = () => {
  const { isBatchModalOpen, setBatchModalOpen, addToast } = useUIStore();
  const { upsertJob } = useQueueStore();
  const { settings } = useSettingsStore();

  const [rawText, setRawText] = useState("");
  const [parseResult, setParseResult] = useState<BatchUrlParseResult | null>(
    null
  );
  const [qualityPreset, setQualityPreset] = useState(
    settings?.default_quality || "best"
  );
  const [isValidating, setIsValidating] = useState(false);
  const [isEnqueueing, setIsEnqueueing] = useState(false);

  if (!isBatchModalOpen) return null;

  const handleValidate = async (input: string) => {
    setRawText(input);
    if (!input.trim()) {
      setParseResult(null);
      return;
    }
    setIsValidating(true);
    try {
      const res = await ipc.parseBatchUrls(input);
      setParseResult(res);
    } catch {
      // ignore validation error
    } finally {
      setIsValidating(false);
    }
  };

  const handleImportTxtFile = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = (ev) => {
      const content = String(ev.target?.result || "");
      handleValidate(content);
    };
    reader.readAsText(file);
  };

  const handleEnqueueAll = async () => {
    if (!parseResult || parseResult.valid_urls.length === 0) return;
    setIsEnqueueing(true);
    try {
      const requests = parseResult.valid_urls.map((u) => ({
        source_url: u.canonical_url,
        shortcode: u.shortcode,
        url_type: u.type_badge.toLowerCase(),
        author_username: u.username_hint,
        caption: null,
        thumbnail_url: null,
        selected_indices: [],
        quality_preset: qualityPreset,
        output_dir: settings?.output_dir ?? null,
        naming_template: settings?.naming_template ?? null,
        save_caption: settings?.save_caption_by_default ?? false,
        save_metadata: settings?.save_metadata_by_default ?? false,
        embed_thumbnail: settings?.embed_thumbnail_by_default ?? true,
      }));

      const jobs = await ipc.enqueueBatchJobs(requests);
      jobs.forEach((j) => upsertJob(j));

      addToast({
        type: "success",
        title: "Batch Queued",
        message: `Added ${jobs.length} Facebook downloads to the active queue.`,
      });
      setRawText("");
      setParseResult(null);
      setBatchModalOpen(false);
    } catch (err) {
      const parsed = parseSerializedError(err);
      addToast({
        type: "error",
        title: "Batch Enqueue Failed",
        message: parsed.user_message,
      });
    } finally {
      setIsEnqueueing(false);
    }
  };

  return (
    <AnimatePresence>
      <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-sm p-4">
        <motion.div
          initial={{ opacity: 0, scale: 0.96, y: 8 }}
          animate={{ opacity: 1, scale: 1, y: 0 }}
          exit={{ opacity: 0, scale: 0.96, y: 8 }}
          className="w-full max-w-2xl rounded-2xl bg-bg-surface border border-border-subtle shadow-2xl overflow-hidden flex flex-col max-h-[85vh]"
        >
          {/* Header */}
          <div className="flex items-center justify-between px-5 py-4 border-b border-border-subtle">
            <div className="flex items-center gap-2.5">
              <div className="w-8 h-8 rounded-lg fb-gradient-bg flex items-center justify-center text-white">
                <Layers className="w-4 h-4" />
              </div>
              <div>
                <h2 className="text-sm font-semibold text-text-primary">
                  Batch Multi-URL Downloader
                </h2>
                <p className="text-xs text-text-secondary">
                  Paste up to 100 Facebook Video, Reel, Watch, fb.watch, or Share URLs (one per line)
                </p>
              </div>
            </div>
            <button
              onClick={() => setBatchModalOpen(false)}
              className="p-1.5 rounded-lg text-text-muted hover:text-text-primary hover:bg-bg-elevated transition-colors"
            >
              <X className="w-4 h-4" />
            </button>
          </div>

          {/* Body */}
          <div className="p-5 space-y-4 overflow-y-auto flex-1">
            <div className="flex items-center justify-between">
              <label className="text-xs font-medium text-text-secondary">
                Facebook URLs (Reels, Watch, Videos, Shortlinks, Posts)
              </label>
              <label className="flex items-center gap-1.5 text-xs text-accent-primary hover:underline cursor-pointer">
                <Upload className="w-3.5 h-3.5" />
                <span>Import .txt file</span>
                <input
                  type="file"
                  accept=".txt"
                  onChange={handleImportTxtFile}
                  className="hidden"
                />
              </label>
            </div>

            <textarea
              value={rawText}
              onChange={(e) => handleValidate(e.target.value)}
              placeholder={
                "https://www.facebook.com/reel/123456789012345\nhttps://www.facebook.com/watch/?v=987654321098765\nhttps://fb.watch/xYz123AbC/\nhttps://www.facebook.com/share/r/1AbCdEfGh/"
              }
              rows={7}
              className="w-full rounded-xl bg-bg-canvas border border-border-subtle p-3.5 text-xs font-mono text-text-primary placeholder:text-text-muted focus:outline-none focus:border-border-focus resize-none"
            />

            {/* Quality selector */}
            <div className="flex items-center justify-between gap-4 bg-bg-elevated/50 p-3 rounded-xl border border-border-subtle">
              <div>
                <div className="text-xs font-medium text-text-primary">
                  Target Quality Preset
                </div>
                <div className="text-[11px] text-text-secondary">
                  Applied to all queued items in this batch
                </div>
              </div>
              <select
                value={qualityPreset}
                onChange={(e) => setQualityPreset(e.target.value)}
                className="bg-bg-canvas border border-border-subtle rounded-lg px-3 py-1.5 text-xs text-text-primary focus:outline-none focus:border-border-focus"
              >
                {QUALITY_PRESETS.map((q) => (
                  <option key={q.id} value={q.id}>
                    {q.label}
                  </option>
                ))}
              </select>
            </div>

            {/* Validation summary */}
            {parseResult && (
              <div className="space-y-2">
                <div className="flex items-center gap-4 text-xs">
                  <span className="flex items-center gap-1.5 text-status-success font-medium">
                    <CheckCircle2 className="w-3.5 h-3.5" />
                    {parseResult.valid_urls.length} valid URLs
                  </span>
                  {parseResult.duplicate_count > 0 && (
                    <span className="text-text-secondary">
                      • {parseResult.duplicate_count} duplicates removed
                    </span>
                  )}
                  {parseResult.invalid_lines.length > 0 && (
                    <span className="flex items-center gap-1.5 text-status-warning">
                      <AlertCircle className="w-3.5 h-3.5" />
                      {parseResult.invalid_lines.length} invalid lines skipped
                    </span>
                  )}
                </div>

                {parseResult.valid_urls.length > 0 && (
                  <div className="max-h-36 overflow-y-auto rounded-lg bg-bg-canvas border border-border-subtle divide-y divide-border-subtle">
                    {parseResult.valid_urls.map((item, idx) => (
                      <div
                        key={idx}
                        className="flex items-center justify-between px-3 py-1.5 text-[11px]"
                      >
                        <span className="font-mono text-text-primary truncate max-w-[440px]">
                          {item.canonical_url}
                        </span>
                        <span className="px-1.5 py-0.5 rounded bg-blue-500/15 text-blue-400 font-mono text-[10px]">
                          {item.type_badge}
                        </span>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}
          </div>

          {/* Footer */}
          <div className="flex items-center justify-end gap-2.5 px-5 py-3.5 border-t border-border-subtle bg-bg-canvas/50">
            <button
              onClick={() => setBatchModalOpen(false)}
              className="px-4 py-2 rounded-lg text-xs font-medium text-text-secondary hover:text-text-primary hover:bg-bg-elevated transition-colors"
            >
              Cancel
            </button>
            <button
              onClick={handleEnqueueAll}
              disabled={
                isEnqueueing ||
                isValidating ||
                !parseResult ||
                parseResult.valid_urls.length === 0
              }
              className="flex items-center gap-2 px-4 py-2 rounded-lg text-xs font-semibold text-white fb-gradient-bg hover:opacity-95 disabled:opacity-40 transition-all"
            >
              {isEnqueueing ? (
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
              ) : (
                <Download className="w-3.5 h-3.5" />
              )}
              <span>
                Enqueue {parseResult?.valid_urls.length ?? 0} Downloads
              </span>
            </button>
          </div>
        </motion.div>
      </div>
    </AnimatePresence>
  );
};
