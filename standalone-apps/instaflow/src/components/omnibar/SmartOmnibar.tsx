import React from "react";
import {
  Clipboard,
  Download,
  Layers,
  Link2,
  Loader2,
  Search,
  Sparkles,
  X,
} from "lucide-react";
import { ipc, parseIpcError } from "../../lib/ipc";
import { classifyInstagramUrlClient } from "../../lib/utils";
import { useQueueStore } from "../../stores/useQueueStore";
import { useSettingsStore } from "../../stores/useSettingsStore";
import { useUIStore } from "../../stores/useUIStore";

export const SmartOmnibar: React.FC<{
  onAnalyzeUrl: (url: string) => Promise<void>;
}> = ({ onAnalyzeUrl }) => {
  const omnibarUrl = useUIStore((s) => s.omnibarUrl);
  const setOmnibarUrl = useUIStore((s) => s.setOmnibarUrl);
  const isInspecting = useUIStore((s) => s.isInspecting);
  const setBatchModalOpen = useUIStore((s) => s.setBatchModalOpen);
  const addToast = useUIStore((s) => s.addToast);

  const settings = useSettingsStore((s) => s.settings);
  const setSettings = useSettingsStore((s) => s.setSettings);
  const upsertJob = useQueueStore((s) => s.upsertJob);

  const classification = classifyInstagramUrlClient(omnibarUrl);

  const handleQuickDownload = async () => {
    if (!classification.valid) return;
    try {
      const job = await ipc.enqueueJob({
        source_url: classification.canonicalUrl,
        shortcode: classification.shortcode,
        url_type: (classification.badge ?? "post").toLowerCase(),
        selected_indices: classification.imgIndex ? [classification.imgIndex] : [],
        quality_preset: settings?.default_quality ?? "best",
      });
      upsertJob(job);
      addToast({
        title: `Enqueued ${classification.badge}`,
        description: classification.canonicalUrl,
        variant: "success",
      });
      setOmnibarUrl("");
    } catch (err) {
      addToast({
        title: "Failed to Enqueue URL",
        description: parseIpcError(err).user_message,
        variant: "error",
      });
    }
  };

  const handleToggleClipboardMonitor = async () => {
    if (!settings) return;
    const next = { ...settings, clipboard_monitor: !settings.clipboard_monitor };
    setSettings(next);
    try {
      await ipc.updateAppSettings(next);
    } catch {
      // ignore
    }
  };

  return (
    <div className="w-full rounded-2xl bg-[#121215] border border-white/[0.08] p-2.5 shadow-xl">
      <div className="flex items-center gap-2">
        <div className="pl-2.5 flex items-center gap-2 shrink-0">
          <Link2
            className={`w-4 h-4 ${
              classification.valid ? "text-indigo-400" : "text-zinc-500"
            }`}
          />
          {classification.badge && (
            <span className="px-2 py-0.5 text-[10px] font-mono font-bold tracking-wider rounded-md bg-indigo-500/20 text-indigo-300 border border-indigo-500/30">
              {classification.badge}
            </span>
          )}
        </div>

        <input
          type="text"
          value={omnibarUrl}
          onChange={(e) => setOmnibarUrl(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && classification.valid) {
              onAnalyzeUrl(omnibarUrl);
            }
          }}
          placeholder="Paste Instagram URL (Reel, Carousel, Post, Story, Highlight, or @Profile)..."
          aria-label="Instagram URL Input"
          className="flex-1 bg-transparent text-xs font-mono text-zinc-100 placeholder-zinc-500 focus:outline-none py-2"
        />

        {omnibarUrl && (
          <button
            onClick={() => setOmnibarUrl("")}
            className="p-1.5 rounded-lg text-zinc-500 hover:text-zinc-300 hover:bg-white/[0.05]"
            aria-label="Clear URL input"
          >
            <X className="w-3.5 h-3.5" />
          </button>
        )}

        {/* Clipboard Monitor Toggle Pill */}
        <button
          onClick={handleToggleClipboardMonitor}
          className={`hidden sm:inline-flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-[11px] font-medium border transition-colors ${
            settings?.clipboard_monitor ?? true
              ? "bg-emerald-500/10 text-emerald-300 border-emerald-500/25"
              : "bg-white/[0.03] text-zinc-500 border-white/[0.07]"
          }`}
          title="Automatically detect copied Instagram links on clipboard"
        >
          <Clipboard className="w-3 h-3" />
          <span>Auto-Detect</span>
        </button>

        {/* Batch Modal Button */}
        <button
          onClick={() => setBatchModalOpen(true)}
          className="inline-flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-xs font-medium text-zinc-300 hover:text-white bg-white/[0.05] hover:bg-white/[0.1] border border-white/[0.07] transition-colors"
          title="Batch URL Import (Ctrl+Shift+B)"
        >
          <Layers className="w-3.5 h-3.5 text-violet-400" />
          <span className="hidden md:inline">Batch</span>
        </button>

        {/* Inspect Button */}
        <button
          onClick={() => onAnalyzeUrl(omnibarUrl)}
          disabled={!classification.valid || isInspecting}
          className="inline-flex items-center gap-1.5 px-3.5 py-1.5 rounded-lg text-xs font-semibold text-zinc-100 bg-white/[0.08] hover:bg-white/[0.14] border border-white/10 disabled:opacity-40 transition-all"
        >
          {isInspecting ? (
            <>
              <Loader2 className="w-3.5 h-3.5 animate-spin text-indigo-400" />
              <span>Inspecting...</span>
            </>
          ) : (
            <>
              <Search className="w-3.5 h-3.5 text-indigo-400" />
              <span>Inspect</span>
            </>
          )}
        </button>

        {/* Quick Download Button */}
        <button
          onClick={handleQuickDownload}
          disabled={!classification.valid}
          className="inline-flex items-center gap-1.5 px-4 py-1.5 rounded-lg text-xs font-semibold text-white bg-gradient-to-r from-indigo-500 to-violet-600 hover:from-indigo-400 hover:to-violet-500 shadow-md shadow-indigo-500/20 disabled:opacity-40 transition-all"
        >
          <Download className="w-3.5 h-3.5" />
          <span>Quick Download</span>
          <Sparkles className="w-3 h-3 opacity-80" />
        </button>
      </div>
    </div>
  );
};
