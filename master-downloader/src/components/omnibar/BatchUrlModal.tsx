import React, { useEffect, useState } from "react";
import {
  CheckCircle2,
  FolderPlus,
  Layers,
  Loader2,
  PlusCircle,
  X,
} from "lucide-react";
import { ipc, isTauriRuntime, parseIpcError } from "../../lib/ipc";
import { classifyMediaUrlClient } from "../../lib/utils";
import { useQueueStore } from "../../stores/useQueueStore";
import { useUIStore } from "../../stores/useUIStore";
import type { BatchUrlParseResult } from "../../types";

export const BatchUrlModal: React.FC = () => {
  const batchModalOpen = useUIStore((s) => s.batchModalOpen);
  const setBatchModalOpen = useUIStore((s) => s.setBatchModalOpen);
  const addToast = useUIStore((s) => s.addToast);
  const setJobs = useQueueStore((s) => s.setJobs);

  const [rawText, setRawText] = useState("");
  const [qualityPreset, setQualityPreset] = useState("best");
  const [subdirectory, setSubdirectory] = useState("");
  const [parseResult, setParseResult] = useState<BatchUrlParseResult | null>(
    null
  );
  const [isSubmitting, setIsSubmitting] = useState(false);

  useEffect(() => {
    if (!batchModalOpen) return;
    const timer = setTimeout(async () => {
      if (!rawText.trim()) {
        setParseResult(null);
        return;
      }
      if (isTauriRuntime()) {
        try {
          const res = await ipc.parseBatchLinks(rawText);
          setParseResult(res);
        } catch {
          // ignore
        }
      } else {
        const seen = new Set<string>();
        const valid_urls = [];
        let duplicate_count = 0;
        const invalid_lines = [];

        for (const line of rawText.split(/\r?\n/)) {
          const t = line.trim();
          if (!t) continue;
          const c = classifyMediaUrlClient(t);
          if (c.valid && c.type && c.platform) {
            if (seen.has(t)) {
              duplicate_count++;
            } else {
              seen.add(t);
              valid_urls.push({
                raw_url: t,
                canonical_url: t,
                platform: c.platform,
                url_type: c.type,
                shortcode: c.shortcode,
                username: null,
                playlist_id: null,
                carousel_index_hint: null,
                requires_auth_hint: false,
                requires_redirect_resolution: false,
              });
            }
          } else {
            invalid_lines.push(t);
          }
        }
        setParseResult({ valid_urls, duplicate_count, invalid_lines });
      }
    }, 180);

    return () => clearTimeout(timer);
  }, [rawText, batchModalOpen]);

  if (!batchModalOpen) return null;

  const handleEnqueueAll = async () => {
    if (!parseResult || parseResult.valid_urls.length === 0) return;
    setIsSubmitting(true);
    try {
      const urls = parseResult.valid_urls.map((u) => u.canonical_url);
      await ipc.enqueueBatchJobs({
        urls,
        qualityPreset,
        subdirectory: subdirectory.trim() || null,
        namingTemplate: null,
      });
      const updated = await ipc.listQueue();
      setJobs(updated);
      addToast({
        title: "Multi-Platform Batch Enqueued",
        description: `Added ${urls.length} YouTube/Instagram/Facebook links to the download queue.`,
        variant: "success",
      });
      setRawText("");
      setBatchModalOpen(false);
    } catch (err) {
      const serialized = parseIpcError(err);
      addToast({
        title: "Batch Import Failed",
        description: serialized.user_message,
        variant: "error",
      });
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="w-full max-w-2xl rounded-2xl bg-[#0F131D] border border-white/[0.1] shadow-2xl overflow-hidden">
        <div className="flex items-center justify-between px-5 py-4 border-b border-white/[0.08]">
          <div className="flex items-center gap-2.5">
            <div className="w-8 h-8 rounded-lg bg-violet-500/15 border border-violet-500/30 flex items-center justify-center">
              <Layers className="w-4 h-4 text-violet-400" />
            </div>
            <div>
              <h3 className="text-sm font-semibold text-slate-100">
                Multi-Platform Batch URL Importer
              </h3>
              <p className="text-[11px] text-slate-400">
                Paste mixed YouTube, Instagram, and Facebook links — auto-detected &amp; deduplicated
              </p>
            </div>
          </div>
          <button
            onClick={() => setBatchModalOpen(false)}
            className="p-1.5 rounded-lg text-slate-400 hover:text-slate-200 hover:bg-white/[0.06]"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <div className="p-5 space-y-4">
          <textarea
            value={rawText}
            onChange={(e) => setRawText(e.target.value)}
            rows={7}
            placeholder={
              "https://www.youtube.com/watch?v=dQw4w9WgXcQ\nhttps://www.instagram.com/reel/C8abc123/\nhttps://www.facebook.com/watch/?v=123456789\nhttps://fb.watch/abcXYZ123/"
            }
            className="w-full rounded-xl bg-[#07090E] border border-white/[0.08] focus:border-violet-500/50 p-3 text-xs font-mono text-slate-200 placeholder:text-slate-600 focus:outline-none resize-none"
          />

          <div className="flex items-center gap-3 text-xs">
            <span className="px-2.5 py-1 rounded-md bg-emerald-500/10 border border-emerald-500/25 text-emerald-300 font-mono">
              {parseResult?.valid_urls.length ?? 0} Valid Links
            </span>
            <span className="px-2.5 py-1 rounded-md bg-amber-500/10 border border-amber-500/25 text-amber-300 font-mono">
              {parseResult?.duplicate_count ?? 0} Duplicates Skipped
            </span>
            <span className="px-2.5 py-1 rounded-md bg-rose-500/10 border border-rose-500/25 text-rose-300 font-mono">
              {parseResult?.invalid_lines.length ?? 0} Invalid
            </span>
          </div>

          <div className="grid grid-cols-2 gap-3 pt-1">
            <div>
              <label className="block text-[11px] font-medium text-slate-400 mb-1">
                Quality Preset for Batch
              </label>
              <select
                value={qualityPreset}
                onChange={(e) => setQualityPreset(e.target.value)}
                className="w-full px-3 py-2 rounded-lg bg-[#07090E] border border-white/[0.08] text-xs text-slate-200 focus:outline-none"
              >
                <option value="best">Best Available (Up to 4K / 60fps)</option>
                <option value="1080p60">1080p 60fps Full HD</option>
                <option value="1080p">1080p Full HD</option>
                <option value="720p">720p HD</option>
                <option value="480p">480p SD</option>
                <option value="audio_mp3">Audio Only (MP3 320kbps)</option>
                <option value="audio_m4a">Audio Only (M4A AAC)</option>
                <option value="audio_flac">Audio Only (FLAC Lossless)</option>
              </select>
            </div>

            <div>
              <label className="block text-[11px] font-medium text-slate-400 mb-1">
                Optional Subfolder Name
              </label>
              <div className="flex items-center px-3 py-2 rounded-lg bg-[#07090E] border border-white/[0.08]">
                <FolderPlus className="w-3.5 h-3.5 text-slate-500 mr-2 shrink-0" />
                <input
                  type="text"
                  value={subdirectory}
                  onChange={(e) => setSubdirectory(e.target.value)}
                  placeholder="e.g. Creator_Pack_2026"
                  className="w-full bg-transparent text-xs text-slate-200 placeholder:text-slate-600 focus:outline-none"
                />
              </div>
            </div>
          </div>
        </div>

        <div className="flex items-center justify-between px-5 py-3.5 bg-[#07090E] border-t border-white/[0.08]">
          <div className="flex items-center gap-1.5 text-[11px] text-slate-400">
            <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400" />
            <span>Smart anti-ban jitter &amp; platform routing applied automatically</span>
          </div>

          <div className="flex items-center gap-2">
            <button
              onClick={() => setBatchModalOpen(false)}
              className="px-3.5 py-1.5 rounded-lg text-xs text-slate-400 hover:text-slate-200"
            >
              Cancel
            </button>
            <button
              onClick={handleEnqueueAll}
              disabled={
                !parseResult ||
                parseResult.valid_urls.length === 0 ||
                isSubmitting
              }
              className="flex items-center gap-1.5 px-4 py-1.5 rounded-lg master-gradient-bg hover:opacity-95 disabled:opacity-40 text-xs font-semibold text-white transition-all"
            >
              {isSubmitting ? (
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
              ) : (
                <PlusCircle className="w-3.5 h-3.5" />
              )}
              <span>
                Enqueue {parseResult?.valid_urls.length ?? 0} Downloads
              </span>
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
