import React, { useEffect, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  AlertCircle,
  AlertTriangle,
  ArrowRight,
  ClipboardPaste,
  Link2,
  Loader2,
  Sparkles,
  X,
} from "lucide-react";
import { DownloadCard } from "../components/DownloadCard";
import { VideoPreviewCard } from "../components/VideoPreviewCard";
import { useApp } from "../stores/AppContext";
import { isValidMediaUrl } from "../utils/formatters";

export const HomePage: React.FC = () => {
  const {
    urlInput,
    setUrlInput,
    analyzing,
    analysisError,
    clearAnalysisError,
    analyzedMedia,
    clearAnalyzedMedia,
    analyzeUrl,
    queue,
    setActivePage,
    dependencies,
    dependenciesDeclined,
    setShowWelcomeSetup,
    settings,
  } = useApp();

  const [clipboardSuggestion, setClipboardSuggestion] = useState<string | null>(
    null
  );

  // Optional clipboard detection only when explicitly permitted in Settings
  useEffect(() => {
    if (!settings.clipboardDetectionEnabled) {
      setClipboardSuggestion(null);
      return;
    }
    const checkClipboard = async () => {
      try {
        const text = await navigator.clipboard.readText();
        const trimmed = text.trim();
        if (
          trimmed &&
          trimmed !== urlInput.trim() &&
          isValidMediaUrl(trimmed)
        ) {
          setClipboardSuggestion(trimmed);
        }
      } catch {
        // Ignore clipboard read errors
      }
    };
    window.addEventListener("focus", checkClipboard);
    return () => window.removeEventListener("focus", checkClipboard);
  }, [settings.clipboardDetectionEnabled, urlInput]);

  const handlePasteClick = async () => {
    try {
      const text = await navigator.clipboard.readText();
      if (text && text.trim()) {
        const clean = text.trim();
        setUrlInput(clean);
        setClipboardSuggestion(null);
      }
    } catch {
      // Fallback if clipboard API blocked
    }
  };

  const handleClear = () => {
    setUrlInput("");
    clearAnalysisError();
    clearAnalyzedMedia();
  };

  const handleFormSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    await analyzeUrl();
  };

  const recentDownloads = queue.slice(0, 3);

  return (
    <motion.div
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.3, ease: [0.2, 0, 0, 1] }}
      className="max-w-4xl mx-auto space-y-6 pb-10"
    >
      {/* Missing dependency / declined setup banner */}
      {(!dependencies.ready || dependenciesDeclined) && (
        <div className="p-4 rounded-2xl bg-amber-500/10 border border-amber-500/25 flex items-center justify-between gap-4">
          <div className="flex items-start gap-3">
            <AlertTriangle className="w-5 h-5 text-amber-500 shrink-0 mt-0.5" />
            <div className="text-xs text-amber-800 dark:text-amber-200 space-y-0.5">
              <div className="font-semibold text-sm">
                Download engine setup is incomplete
              </div>
              <p className="opacity-90">
                {!dependencies.ytdlp.functional &&
                  "yt-dlp is required to analyze links and download videos. "}
                {!dependencies.ffmpeg.functional &&
                  "FFmpeg is required to merge 1080p+/4K streams and convert audio to MP3."}
              </p>
            </div>
          </div>
          <button
            type="button"
            onClick={() => setShowWelcomeSetup(true)}
            className="px-4 py-2 rounded-xl text-xs font-semibold bg-amber-500 hover:bg-amber-400 text-zinc-950 transition-colors shrink-0"
          >
            Complete Setup
          </button>
        </div>
      )}

      {/* Hero Header */}
      <div className="space-y-1.5 pt-1">
        <h1 className="text-2xl sm:text-3xl font-bold tracking-tight text-zinc-900 dark:text-zinc-100">
          Download something great.
        </h1>
        <p className="text-sm text-zinc-500 dark:text-zinc-400">
          Paste a link, choose your format, and we'll handle the rest.
        </p>
      </div>

      {/* Main URL Input Card */}
      <form
        onSubmit={handleFormSubmit}
        className="p-4 rounded-2xl bg-surface-card border border-border-strong shadow-card space-y-3"
      >
        <div className="relative flex items-center">
          <Link2 className="w-5 h-5 text-zinc-400 absolute left-4 pointer-events-none" />
          <input
            type="text"
            value={urlInput}
            onChange={(e) => {
              setUrlInput(e.target.value);
              if (analysisError) clearAnalysisError();
            }}
            placeholder="Paste a YouTube or supported video link..."
            aria-label="Video or audio URL"
            className="w-full pl-12 pr-28 py-3.5 rounded-xl bg-surface-subtle border border-border text-sm text-zinc-900 dark:text-zinc-100 placeholder:text-zinc-400 dark:placeholder:text-zinc-500 focus:outline-none focus:ring-2 focus:ring-indigo-500 transition-all"
          />
          {urlInput && (
            <button
              type="button"
              onClick={handleClear}
              aria-label="Clear URL"
              className="absolute right-3 p-1.5 rounded-lg text-zinc-400 hover:text-zinc-700 dark:hover:text-zinc-200 hover:bg-surface-hover transition-colors"
            >
              <X className="w-4 h-4" />
            </button>
          )}
        </div>

        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={handlePasteClick}
              className="inline-flex items-center gap-2 px-3.5 py-2 rounded-xl text-xs font-medium bg-surface-subtle hover:bg-surface-hover border border-border text-zinc-700 dark:text-zinc-300 transition-colors"
            >
              <ClipboardPaste className="w-3.5 h-3.5 text-indigo-500" />
              Paste
            </button>
            {urlInput && (
              <button
                type="button"
                onClick={handleClear}
                className="px-3 py-2 rounded-xl text-xs font-medium text-zinc-500 hover:text-zinc-800 dark:hover:text-zinc-200 transition-colors"
              >
                Clear
              </button>
            )}
          </div>

          <button
            type="submit"
            disabled={analyzing || !urlInput.trim()}
            className="inline-flex items-center gap-2 px-5 py-2.5 rounded-xl text-xs sm:text-sm font-semibold bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 text-white shadow-sm shadow-indigo-500/25 transition-all"
          >
            {analyzing ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin" />
                Analyzing...
              </>
            ) : (
              <>
                <Sparkles className="w-4 h-4" />
                Analyze
              </>
            )}
          </button>
        </div>

        {/* One-click clipboard suggestion when enabled in Settings */}
        {clipboardSuggestion && (
          <div className="pt-2 border-t border-border flex items-center justify-between gap-2 text-xs">
            <span className="text-zinc-500 dark:text-zinc-400 truncate">
              Detected link in clipboard:{" "}
              <span className="font-mono text-zinc-700 dark:text-zinc-300">
                {clipboardSuggestion}
              </span>
            </span>
            <button
              type="button"
              onClick={() => {
                setUrlInput(clipboardSuggestion);
                analyzeUrl(clipboardSuggestion);
                setClipboardSuggestion(null);
              }}
              className="px-2.5 py-1 rounded-lg font-semibold bg-indigo-500/15 text-indigo-600 dark:text-indigo-300 hover:bg-indigo-500/25 shrink-0"
            >
              Paste & Analyze
            </button>
          </div>
        )}
      </form>

      {/* Skeleton Loading State during URL Analysis */}
      <AnimatePresence mode="wait">
        {analyzing && (
          <motion.div
            key="skeleton"
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -10 }}
            className="p-5 rounded-2xl bg-surface-card border border-border space-y-5"
            aria-label="Analyzing link metadata"
          >
            <div className="flex flex-col md:flex-row gap-5">
              <div className="w-full md:w-64 aspect-video rounded-xl bg-surface-subtle animate-pulse" />
              <div className="flex-1 space-y-3 py-1">
                <div className="h-5 w-3/4 rounded-lg bg-surface-subtle animate-pulse" />
                <div className="h-4 w-1/3 rounded-lg bg-surface-subtle animate-pulse" />
                <div className="h-3 w-full rounded-lg bg-surface-subtle animate-pulse" />
                <div className="h-3 w-2/3 rounded-lg bg-surface-subtle animate-pulse" />
              </div>
            </div>
            <div className="grid grid-cols-2 gap-4 pt-2 border-t border-border">
              <div className="h-10 rounded-xl bg-surface-subtle animate-pulse" />
              <div className="h-10 rounded-xl bg-surface-subtle animate-pulse" />
            </div>
          </motion.div>
        )}

        {/* Analysis Error Card */}
        {!analyzing && analysisError && (
          <motion.div
            key="error"
            initial={{ opacity: 0, x: -8 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0 }}
            className="p-4 rounded-2xl bg-rose-500/10 border border-rose-500/25 flex items-center justify-between gap-4"
          >
            <div className="flex items-start gap-3">
              <AlertCircle className="w-5 h-5 text-rose-500 shrink-0 mt-0.5" />
              <div>
                <div className="text-sm font-semibold text-rose-600 dark:text-rose-300">
                  {analysisError.message}
                </div>
                <div className="text-xs text-rose-500/80 dark:text-rose-400/80 mt-0.5">
                  Verify the URL is publicly accessible and supported by yt-dlp.
                </div>
              </div>
            </div>
            <button
              type="button"
              onClick={() => analyzeUrl()}
              className="px-3.5 py-2 rounded-xl text-xs font-semibold bg-rose-600 hover:bg-rose-500 text-white transition-colors shrink-0"
            >
              Retry
            </button>
          </motion.div>
        )}

        {/* Video Preview & Format Selector Card */}
        {!analyzing && analyzedMedia && (
          <VideoPreviewCard key={analyzedMedia.id} metadata={analyzedMedia} />
        )}
      </AnimatePresence>

      {/* Active / Recent Downloads Queue Preview */}
      {recentDownloads.length > 0 && (
        <div className="space-y-3 pt-2">
          <div className="flex items-center justify-between">
            <h2 className="text-sm font-semibold text-zinc-800 dark:text-zinc-200">
              Recent Queue Activity
            </h2>
            <button
              type="button"
              onClick={() => setActivePage("downloads")}
              className="inline-flex items-center gap-1 text-xs font-medium text-indigo-600 dark:text-indigo-400 hover:underline"
            >
              Open Full Queue ({queue.length})
              <ArrowRight className="w-3.5 h-3.5" />
            </button>
          </div>
          <div className="space-y-2.5">
            {recentDownloads.map((item) => (
              <DownloadCard key={item.id} item={item} />
            ))}
          </div>
        </div>
      )}
    </motion.div>
  );
};
