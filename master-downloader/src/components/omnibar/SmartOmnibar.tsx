import React, { useMemo } from "react";
import {
  AlertCircle,
  ClipboardPaste,
  Layers,
  Link2,
  Loader2,
  Sparkles,
  X,
  Zap,
} from "lucide-react";
import { ipc } from "../../lib/ipc";
import { classifyMediaUrlClient } from "../../lib/utils";
import { useSettingsStore } from "../../stores/useSettingsStore";
import { useUIStore } from "../../stores/useUIStore";

interface SmartOmnibarProps {
  onAnalyze: (url: string) => void;
  onAutoDownload: (url: string) => void;
}

export const SmartOmnibar: React.FC<SmartOmnibarProps> = ({
  onAnalyze,
  onAutoDownload,
}) => {
  const omnibarUrl = useUIStore((s) => s.omnibarUrl);
  const setOmnibarUrl = useUIStore((s) => s.setOmnibarUrl);
  const isAnalyzing = useUIStore((s) => s.isAnalyzing);
  const analyzeError = useUIStore((s) => s.analyzeError);
  const setAnalyzeError = useUIStore((s) => s.setAnalyzeError);
  const setBatchModalOpen = useUIStore((s) => s.setBatchModalOpen);
  const setAuthModalOpen = useUIStore((s) => s.setAuthModalOpen);

  const settings = useSettingsStore((s) => s.settings);
  const setSettings = useSettingsStore((s) => s.setSettings);

  const autoDownloadEnabled = settings?.auto_download_on_detect ?? true;

  const classification = useMemo(
    () => classifyMediaUrlClient(omnibarUrl),
    [omnibarUrl]
  );

  const handleToggleAutoDownload = async () => {
    if (!settings) return;
    const updated = {
      ...settings,
      auto_download_on_detect: !settings.auto_download_on_detect,
    };
    setSettings(updated);
    try {
      await ipc.updateAppSettings(updated);
    } catch {
      // ignore in non-tauri test env
    }
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!omnibarUrl.trim() || isAnalyzing) return;
    if (autoDownloadEnabled) {
      onAutoDownload(omnibarUrl.trim());
    } else {
      onAnalyze(omnibarUrl.trim());
    }
  };

  const handlePasteClick = async () => {
    try {
      const text = await navigator.clipboard.readText();
      if (text && text.trim()) {
        const clean = text.trim();
        setOmnibarUrl(clean);
        if (autoDownloadEnabled) {
          onAutoDownload(clean);
        } else {
          onAnalyze(clean);
        }
      }
    } catch {
      // Clipboard read denied
    }
  };

  const badgeStyle = useMemo(() => {
    if (!classification.valid) return "";
    switch (classification.platform) {
      case "youtube":
        return "bg-rose-500/20 text-rose-300 border-rose-500/35";
      case "instagram":
        return "bg-fuchsia-500/20 text-fuchsia-300 border-fuchsia-500/35";
      case "facebook":
        return "bg-blue-500/20 text-blue-300 border-blue-500/35";
      default:
        return "bg-violet-500/20 text-violet-300 border-violet-500/35";
    }
  }, [classification]);

  return (
    <div className="w-full space-y-2">
      <form
        onSubmit={handleSubmit}
        className={`relative flex items-center w-full h-13 rounded-xl bg-[#0F131D] border transition-all px-3.5 gap-2.5 shadow-lg ${
          analyzeError
            ? "border-rose-500/60 shadow-rose-500/5"
            : classification.valid
            ? "border-violet-500/60 shadow-violet-500/10"
            : "border-white/[0.08] focus-within:border-violet-500/50"
        }`}
      >
        <Link2
          className={`w-4 h-4 shrink-0 transition-colors ${
            classification.valid ? "text-violet-400" : "text-slate-500"
          }`}
        />

        {classification.valid && classification.badgeLabel && (
          <span
            className={`px-2 py-0.5 text-[10px] font-mono font-bold tracking-wider uppercase rounded-md border shrink-0 ${badgeStyle}`}
          >
            {classification.badgeLabel}
          </span>
        )}

        <input
          type="text"
          value={omnibarUrl}
          onChange={(e) => {
            setOmnibarUrl(e.target.value);
            if (analyzeError) setAnalyzeError(null);
          }}
          placeholder="Paste any YouTube, Instagram, or Facebook link (Video, Shorts, Reel, Carousel, Story, Playlist, Watch)..."
          className="w-full bg-transparent text-xs font-mono text-slate-100 placeholder:text-slate-500 focus:outline-none"
          aria-label="Universal Media URL Input"
        />

        {omnibarUrl && (
          <button
            type="button"
            onClick={() => {
              setOmnibarUrl("");
              setAnalyzeError(null);
            }}
            className="p-1 rounded-md text-slate-500 hover:text-slate-300 hover:bg-white/[0.06] transition-colors"
            title="Clear URL"
          >
            <X className="w-3.5 h-3.5" />
          </button>
        )}

        <div className="flex items-center gap-1.5 shrink-0">
          {/* Auto-Download Mode Toggle Pill */}
          <button
            type="button"
            onClick={handleToggleAutoDownload}
            className={`flex items-center gap-1 px-2.5 py-1.5 rounded-lg text-[11px] font-semibold border transition-all ${
              autoDownloadEnabled
                ? "bg-emerald-500/15 text-emerald-300 border-emerald-500/35 hover:bg-emerald-500/25"
                : "bg-white/[0.04] text-slate-400 border-white/[0.08] hover:text-slate-200"
            }`}
            title="When enabled, detected or pasted links immediately auto-download into your queue using your default quality settings"
          >
            <Zap className="w-3 h-3" />
            <span>{autoDownloadEnabled ? "Auto-DL: ON" : "Auto-DL: OFF"}</span>
          </button>

          <button
            type="button"
            onClick={handlePasteClick}
            className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg bg-white/[0.05] hover:bg-white/[0.1] border border-white/[0.08] text-[11px] font-medium text-slate-300 transition-colors"
            title="Paste from Clipboard"
          >
            <ClipboardPaste className="w-3.5 h-3.5 text-violet-400" />
            <span>Paste</span>
          </button>

          <button
            type="button"
            onClick={() => setBatchModalOpen(true)}
            className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg bg-white/[0.05] hover:bg-white/[0.1] border border-white/[0.08] text-[11px] font-medium text-slate-300 transition-colors"
            title="Open Multi-Platform Batch URL Importer (Ctrl+B)"
          >
            <Layers className="w-3.5 h-3.5 text-sky-400" />
            <span>Batch</span>
          </button>

          <button
            type="button"
            disabled={!omnibarUrl.trim() || isAnalyzing}
            onClick={() => omnibarUrl.trim() && onAnalyze(omnibarUrl.trim())}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-white/[0.07] hover:bg-white/[0.13] disabled:opacity-40 border border-white/[0.1] text-xs font-semibold text-slate-200 transition-all"
            title="Inspect formats, carousels, playlists, subtitles, and codecs before downloading"
          >
            {isAnalyzing ? (
              <Loader2 className="w-3.5 h-3.5 animate-spin" />
            ) : (
              <Sparkles className="w-3.5 h-3.5 text-violet-400" />
            )}
            <span>{isAnalyzing ? "Inspecting..." : "Inspect"}</span>
          </button>

          <button
            type="submit"
            disabled={!omnibarUrl.trim() || isAnalyzing}
            className="flex items-center gap-1.5 px-3.5 py-1.5 rounded-lg master-gradient-bg hover:opacity-95 disabled:opacity-40 text-xs font-semibold text-white shadow-sm transition-all"
          >
            <Zap className="w-3.5 h-3.5" />
            <span>{autoDownloadEnabled ? "Auto-Download" : "Quick Download"}</span>
          </button>
        </div>
      </form>

      {/* Supported URL Quick Chips */}
      <div className="flex items-center justify-between px-1 text-[11px] text-slate-500">
        <div className="flex items-center gap-2 flex-wrap">
          <span className="text-slate-400 font-medium">Auto-Detects:</span>
          <span className="px-1.5 py-0.5 rounded bg-rose-500/10 text-rose-300 border border-rose-500/20 font-mono text-[10px]">
            youtube.com / youtu.be / shorts / playlist
          </span>
          <span className="px-1.5 py-0.5 rounded bg-fuchsia-500/10 text-fuchsia-300 border border-fuchsia-500/20 font-mono text-[10px]">
            instagram.com / reel / p / stories / highlights
          </span>
          <span className="px-1.5 py-0.5 rounded bg-blue-500/10 text-blue-300 border border-blue-500/20 font-mono text-[10px]">
            facebook.com / watch / reel / share / fb.watch
          </span>
        </div>
      </div>

      {/* Inline Error Banner */}
      {analyzeError && (
        <div className="flex items-center justify-between px-3.5 py-2.5 rounded-xl bg-rose-500/10 border border-rose-500/30 text-xs text-rose-200">
          <div className="flex items-center gap-2">
            <AlertCircle className="w-4 h-4 text-rose-400 shrink-0" />
            <span>{analyzeError}</span>
          </div>
          {(analyzeError.toLowerCase().includes("login") ||
            analyzeError.toLowerCase().includes("auth") ||
            analyzeError.toLowerCase().includes("session") ||
            analyzeError.toLowerCase().includes("private") ||
            analyzeError.toLowerCase().includes("age")) && (
            <button
              onClick={() => setAuthModalOpen(true)}
              className="px-2.5 py-1 rounded-md bg-rose-500/20 hover:bg-rose-500/30 text-rose-100 font-medium text-[11px] shrink-0 transition-colors"
            >
              Open Session Vault
            </button>
          )}
        </div>
      )}
    </div>
  );
};
