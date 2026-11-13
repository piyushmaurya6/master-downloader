import React, { useEffect, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import {
  AlertCircle,
  CheckCircle2,
  FileUp,
  Layers,
  PlusCircle,
  X,
} from "lucide-react";
import { ipc, isTauriRuntime, parseIpcError } from "../../lib/ipc";
import { classifyInstagramUrlClient } from "../../lib/utils";
import { useQueueStore } from "../../stores/useQueueStore";
import { useSettingsStore } from "../../stores/useSettingsStore";
import { useUIStore } from "../../stores/useUIStore";
import type { BatchUrlParseResult } from "../../types";

export const BatchUrlModal: React.FC = () => {
  const open = useUIStore((s) => s.batchModalOpen);
  const setOpen = useUIStore((s) => s.setBatchModalOpen);
  const addToast = useUIStore((s) => s.addToast);

  const settings = useSettingsStore((s) => s.settings);
  const setJobs = useQueueStore((s) => s.setJobs);

  const [rawText, setRawText] = useState("");
  const [subdirectory, setSubdirectory] = useState("");
  const [qualityPreset, setQualityPreset] = useState(
    settings?.default_quality ?? "best"
  );
  const [namingTemplate, setNamingTemplate] = useState(
    settings?.naming_template ?? "{username}/{date}_{shortcode}_{index}.{ext}"
  );
  const [parsedResult, setParsedResult] = useState<BatchUrlParseResult>({
    valid_urls: [],
    invalid_lines: [],
    duplicate_count: 0,
  });

  useEffect(() => {
    if (!rawText.trim()) {
      setParsedResult({ valid_urls: [], invalid_lines: [], duplicate_count: 0 });
      return;
    }

    if (isTauriRuntime()) {
      ipc
        .parseBatchLinks(rawText)
        .then(setParsedResult)
        .catch(() => {});
      return;
    }

    // Client-side fallback for non-Tauri / unit test environment
    const seen = new Set<string>();
    const valid_urls: BatchUrlParseResult["valid_urls"] = [];
    const invalid_lines: string[] = [];
    let duplicate_count = 0;

    for (const rawLine of rawText.split(/\r?\n/)) {
      const line = rawLine.trim();
      if (!line || line.startsWith("#")) continue;
      const c = classifyInstagramUrlClient(line);
      if (c.valid && c.badge) {
        if (seen.has(c.canonicalUrl)) {
          duplicate_count++;
        } else {
          seen.add(c.canonicalUrl);
          valid_urls.push({
            canonical_url: c.canonicalUrl,
            original_url: line,
            url_type: { kind: "post", value: c.shortcode ?? "" },
            type_badge: c.badge,
            shortcode: c.shortcode,
            username_hint: null,
            img_index_hint: c.imgIndex,
          });
        }
      } else {
        invalid_lines.push(line);
      }
    }

    setParsedResult({ valid_urls, invalid_lines, duplicate_count });
  }, [rawText]);

  const handleFileImport = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => {
      const textResult = reader.result;
      if (typeof textResult === "string") {
        setRawText((prev) => (prev ? `${prev}\n${textResult}` : textResult));
      }
    };
    reader.readAsText(file);
  };

  const handleEnqueueBatch = async () => {
    if (parsedResult.valid_urls.length === 0) return;
    try {
      await ipc.enqueueBatchJobs({
        urls: parsedResult.valid_urls.map((u) => u.canonical_url),
        qualityPreset,
        subdirectory: subdirectory.trim() || null,
        namingTemplate: namingTemplate.trim() || null,
      });
      const updatedQueue = await ipc.listQueue();
      setJobs(updatedQueue);
      addToast({
        title: `Enqueued ${parsedResult.valid_urls.length} Instagram Links`,
        description:
          "Smart rate-limit jitter pacing active between consecutive requests.",
        variant: "success",
      });
      setRawText("");
      setOpen(false);
    } catch (err) {
      addToast({
        title: "Batch Enqueue Error",
        description: parseIpcError(err).user_message,
        variant: "error",
      });
    }
  };

  return (
    <AnimatePresence>
      {open && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          className="fixed inset-0 z-50 bg-black/75 backdrop-blur-md flex items-center justify-center p-4"
          onClick={() => setOpen(false)}
        >
          <motion.div
            initial={{ opacity: 0, scale: 0.96, y: 12 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.96, y: 12 }}
            onClick={(e) => e.stopPropagation()}
            className="w-full max-w-2xl rounded-2xl bg-[#121215] border border-white/10 shadow-2xl overflow-hidden"
          >
            {/* Header */}
            <div className="px-6 py-4 border-b border-white/[0.07] flex items-center justify-between">
              <div className="flex items-center gap-3">
                <div className="p-2 rounded-xl bg-violet-500/15 border border-violet-500/30 text-violet-400">
                  <Layers className="w-5 h-5" />
                </div>
                <div>
                  <h2 className="text-sm font-semibold text-zinc-100">
                    Batch Instagram URL Downloader
                  </h2>
                  <p className="text-xs text-zinc-400">
                    Paste multiple links or import a <code className="text-zinc-300">.txt</code> /{" "}
                    <code className="text-zinc-300">.csv</code> list with automatic deduplication
                  </p>
                </div>
              </div>
              <button
                onClick={() => setOpen(false)}
                className="p-1.5 rounded-lg text-zinc-400 hover:text-zinc-100 hover:bg-white/[0.06]"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <div className="p-6 space-y-4">
              <div className="flex items-center justify-between">
                <span className="text-xs font-medium text-zinc-300">
                  Instagram Links (one per line)
                </span>
                <label className="cursor-pointer inline-flex items-center gap-1.5 px-3 py-1 rounded-lg text-xs font-medium bg-white/[0.06] hover:bg-white/[0.12] text-zinc-200 transition-colors">
                  <FileUp className="w-3.5 h-3.5 text-indigo-400" />
                  <span>Import .txt / .csv</span>
                  <input
                    type="file"
                    accept=".txt,.csv"
                    onChange={handleFileImport}
                    className="hidden"
                  />
                </label>
              </div>

              <textarea
                rows={6}
                value={rawText}
                onChange={(e) => setRawText(e.target.value)}
                placeholder="https://www.instagram.com/reel/C8abc_123/&#10;https://www.instagram.com/p/C9xyz_987/&#10;https://www.instagram.com/stories/highlights/17945829102938/"
                className="w-full p-3 rounded-xl bg-[#09090B] border border-white/[0.08] text-xs font-mono text-zinc-100 placeholder-zinc-600 focus:outline-none focus:border-indigo-500"
              />

              {/* Live Validation Summary */}
              <div className="flex flex-wrap items-center gap-3 text-xs">
                <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-emerald-500/10 text-emerald-300 border border-emerald-500/20">
                  <CheckCircle2 className="w-3.5 h-3.5" />
                  {parsedResult.valid_urls.length} Valid Links
                </span>

                {parsedResult.duplicate_count > 0 && (
                  <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-indigo-500/10 text-indigo-300 border border-indigo-500/20">
                    {parsedResult.duplicate_count} Duplicates Stripped
                  </span>
                )}

                {parsedResult.invalid_lines.length > 0 && (
                  <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-rose-500/10 text-rose-300 border border-rose-500/20">
                    <AlertCircle className="w-3.5 h-3.5" />
                    {parsedResult.invalid_lines.length} Invalid Lines Ignored
                  </span>
                )}
              </div>

              {/* Unified Naming & Subdirectory Options */}
              <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 pt-2">
                <div>
                  <label className="block text-[11px] font-medium text-zinc-400 mb-1">
                    Quality Preset
                  </label>
                  <select
                    value={qualityPreset}
                    onChange={(e) => setQualityPreset(e.target.value)}
                    className="w-full px-3 py-2 rounded-lg bg-[#09090B] border border-white/[0.08] text-xs text-zinc-100 focus:outline-none focus:border-indigo-500"
                  >
                    <option value="best">Highest Available (Best Video + Audio)</option>
                    <option value="1080p">1080p Full HD MP4</option>
                    <option value="720p">720p HD MP4</option>
                    <option value="audio_only">Audio Only (M4A)</option>
                  </select>
                </div>

                <div>
                  <label className="block text-[11px] font-medium text-zinc-400 mb-1">
                    Output Subfolder (Optional)
                  </label>
                  <input
                    type="text"
                    value={subdirectory}
                    onChange={(e) => setSubdirectory(e.target.value)}
                    placeholder="e.g. Campaign_Archive_Nov"
                    className="w-full px-3 py-2 rounded-lg bg-[#09090B] border border-white/[0.08] text-xs text-zinc-100 placeholder-zinc-600 focus:outline-none focus:border-indigo-500"
                  />
                </div>

                <div>
                  <label className="block text-[11px] font-medium text-zinc-400 mb-1">
                    Filename Template
                  </label>
                  <input
                    type="text"
                    value={namingTemplate}
                    onChange={(e) => setNamingTemplate(e.target.value)}
                    className="w-full px-3 py-2 rounded-lg bg-[#09090B] border border-white/[0.08] text-xs font-mono text-zinc-100 focus:outline-none focus:border-indigo-500"
                  />
                </div>
              </div>
            </div>

            {/* Footer */}
            <div className="px-6 py-4 bg-[#09090B]/60 border-t border-white/[0.07] flex items-center justify-end gap-2">
              <button
                onClick={() => setOpen(false)}
                className="px-4 py-2 rounded-lg text-xs font-medium text-zinc-400 hover:text-zinc-100 bg-white/[0.05]"
              >
                Cancel
              </button>
              <button
                onClick={handleEnqueueBatch}
                disabled={parsedResult.valid_urls.length === 0}
                className="inline-flex items-center gap-2 px-5 py-2 rounded-lg text-xs font-semibold text-white bg-gradient-to-r from-indigo-500 to-violet-600 hover:from-indigo-400 hover:to-violet-500 disabled:opacity-40 shadow-lg shadow-indigo-500/20 transition-all"
              >
                <PlusCircle className="w-4 h-4" />
                Enqueue {parsedResult.valid_urls.length} Items with Jitter Pacing
              </button>
            </div>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
};
