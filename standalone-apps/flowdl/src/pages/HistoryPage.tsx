import React, { useMemo, useState } from "react";
import { motion } from "framer-motion";
import {
  AlertTriangle,
  Clock,
  Copy,
  ExternalLink,
  Film,
  FolderOpen,
  Music,
  RotateCcw,
  Search,
  Trash2,
} from "lucide-react";
import { tauriBridge } from "../services/tauriBridge";
import { useApp } from "../stores/AppContext";
import type { HistoryItem } from "../types";
import { formatBytes } from "../utils/formatters";

type FilterType = "all" | "video" | "audio" | "completed" | "failed";
type SortType = "newest" | "oldest" | "size";

export const HistoryPage: React.FC = () => {
  const {
    history,
    removeHistoryItem,
    clearAllHistory,
    startDownload,
    addToast,
  } = useApp();

  const [searchQuery, setSearchQuery] = useState("");
  const [filter, setFilter] = useState<FilterType>("all");
  const [sortBy, setSortBy] = useState<SortType>("newest");
  const [confirmClearAll, setConfirmClearAll] = useState(false);

  const filteredHistory = useMemo(() => {
    return history
      .filter((item) => {
        if (filter === "video" && item.mode !== "video") return false;
        if (filter === "audio" && item.mode !== "audio") return false;
        if (filter === "completed" && item.status !== "completed") return false;
        if (filter === "failed" && item.status !== "failed") return false;

        if (searchQuery.trim()) {
          const q = searchQuery.toLowerCase();
          const matchTitle = item.title.toLowerCase().includes(q);
          const matchUrl = item.sourceUrl.toLowerCase().includes(q);
          const matchUploader =
            item.uploader?.toLowerCase().includes(q) ?? false;
          return matchTitle || matchUrl || matchUploader;
        }
        return true;
      })
      .sort((a, b) => {
        if (sortBy === "newest") {
          return (
            new Date(b.downloadedAt).getTime() -
            new Date(a.downloadedAt).getTime()
          );
        }
        if (sortBy === "oldest") {
          return (
            new Date(a.downloadedAt).getTime() -
            new Date(b.downloadedAt).getTime()
          );
        }
        return (b.fileSize ?? 0) - (a.fileSize ?? 0);
      });
  }, [history, filter, searchQuery, sortBy]);

  const handleCopyUrl = async (url: string) => {
    try {
      await navigator.clipboard.writeText(url);
      addToast({
        type: "info",
        title: "Copied link to clipboard",
        description: url,
      });
    } catch {
      // ignore
    }
  };

  const handleDownloadAgain = async (item: HistoryItem) => {
    await startDownload(item.request);
  };

  return (
    <motion.div
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.25, ease: [0.2, 0, 0, 1] }}
      className="max-w-4xl mx-auto space-y-6 pb-10"
    >
      {/* Header */}
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-bold tracking-tight text-zinc-900 dark:text-zinc-100">
            Download History
          </h1>
          <p className="text-sm text-zinc-500 dark:text-zinc-400">
            Search and revisit your previous downloads stored locally.
          </p>
        </div>

        {history.length > 0 && (
          <div>
            {!confirmClearAll ? (
              <button
                type="button"
                onClick={() => setConfirmClearAll(true)}
                className="inline-flex items-center gap-1.5 px-3.5 py-2 rounded-xl text-xs font-medium text-rose-600 dark:text-rose-400 bg-rose-500/10 hover:bg-rose-500/20 border border-rose-500/20 transition-colors"
              >
                <Trash2 className="w-3.5 h-3.5" />
                Clear History
              </button>
            ) : (
              <div className="inline-flex items-center gap-2 p-1.5 rounded-xl bg-rose-500/10 border border-rose-500/25">
                <span className="text-xs text-rose-600 dark:text-rose-300 px-2 font-medium">
                  Clear records? (Files on disk are kept)
                </span>
                <button
                  type="button"
                  onClick={() => {
                    clearAllHistory();
                    setConfirmClearAll(false);
                  }}
                  className="px-2.5 py-1 rounded-lg bg-rose-600 text-white text-xs font-semibold"
                >
                  Confirm
                </button>
                <button
                  type="button"
                  onClick={() => setConfirmClearAll(false)}
                  className="px-2.5 py-1 rounded-lg bg-surface-elevated text-xs text-zinc-600 dark:text-zinc-300"
                >
                  Cancel
                </button>
              </div>
            )}
          </div>
        )}
      </div>

      {/* Search, Filter Pills & Sort Controls */}
      <div className="p-3.5 rounded-2xl bg-surface-card border border-border flex flex-col md:flex-row items-stretch md:items-center justify-between gap-3">
        <div className="relative flex-1">
          <Search className="w-4 h-4 text-zinc-400 absolute left-3.5 top-1/2 -translate-y-1/2" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search by title, creator, or URL..."
            aria-label="Search download history"
            className="w-full pl-9 pr-3 py-2 rounded-xl text-xs bg-surface-subtle border border-border text-zinc-900 dark:text-zinc-100 focus:outline-none focus:ring-2 focus:ring-indigo-500"
          />
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <div className="inline-flex p-1 rounded-xl bg-surface-subtle border border-border">
            {(
              [
                ["all", "All"],
                ["video", "Video"],
                ["audio", "Audio"],
                ["completed", "Completed"],
                ["failed", "Failed"],
              ] as [FilterType, string][]
            ).map(([key, label]) => (
              <button
                key={key}
                type="button"
                onClick={() => setFilter(key)}
                className={`px-2.5 py-1 rounded-lg text-xs font-medium transition-colors ${
                  filter === key
                    ? "bg-indigo-600 text-white"
                    : "text-zinc-600 dark:text-zinc-400 hover:text-zinc-900 dark:hover:text-zinc-100"
                }`}
              >
                {label}
              </button>
            ))}
          </div>

          <select
            aria-label="Sort history by"
            value={sortBy}
            onChange={(e) => setSortBy(e.target.value as SortType)}
            className="px-3 py-2 rounded-xl text-xs bg-surface-subtle border border-border text-zinc-800 dark:text-zinc-200 focus:outline-none"
          >
            <option value="newest">Newest first</option>
            <option value="oldest">Oldest first</option>
            <option value="size">File size</option>
          </select>
        </div>
      </div>

      {/* History List */}
      {filteredHistory.length === 0 ? (
        <div className="p-12 rounded-2xl bg-surface-card border border-border text-center space-y-2">
          <Clock className="w-8 h-8 text-zinc-400 mx-auto" />
          <div className="text-sm font-semibold text-zinc-800 dark:text-zinc-200">
            No matching history records
          </div>
          <p className="text-xs text-zinc-500 dark:text-zinc-400">
            Completed and failed downloads will appear here automatically.
          </p>
        </div>
      ) : (
        <div className="space-y-2.5">
          {filteredHistory.map((item) => (
            <div
              key={item.id}
              className="p-4 rounded-2xl bg-surface-card border border-border hover:border-border-strong transition-colors flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4"
            >
              <div className="flex items-start gap-3.5 min-w-0 flex-1">
                <div className="relative w-24 aspect-video rounded-xl overflow-hidden bg-zinc-900 shrink-0 border border-border">
                  {item.thumbnail ? (
                    <img
                      src={item.thumbnail}
                      alt={item.title}
                      className="w-full h-full object-cover"
                    />
                  ) : (
                    <div className="w-full h-full flex items-center justify-center text-zinc-600">
                      {item.mode === "audio" ? (
                        <Music className="w-5 h-5" />
                      ) : (
                        <Film className="w-5 h-5" />
                      )}
                    </div>
                  )}
                </div>

                <div className="min-w-0 flex-1 space-y-1">
                  <div className="flex items-center gap-2">
                    <h4 className="text-sm font-semibold text-zinc-900 dark:text-zinc-100 truncate">
                      {item.title}
                    </h4>
                  </div>

                  <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-zinc-500 dark:text-zinc-400">
                    <span className="font-mono font-medium text-indigo-600 dark:text-indigo-400">
                      {item.formatSummary}
                    </span>
                    {item.fileSize && (
                      <span className="font-mono">
                        {formatBytes(item.fileSize)}
                      </span>
                    )}
                    <span>
                      {new Date(item.downloadedAt).toLocaleDateString(undefined, {
                        month: "short",
                        day: "numeric",
                        hour: "2-digit",
                        minute: "2-digit",
                      })}
                    </span>
                    {item.status === "completed" && !item.fileExists && (
                      <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-amber-500/15 text-amber-600 dark:text-amber-400 text-[11px] font-medium">
                        <AlertTriangle className="w-3 h-3" />
                        File moved or deleted
                      </span>
                    )}
                    {item.status === "failed" && (
                      <span className="px-2 py-0.5 rounded-md bg-rose-500/15 text-rose-600 dark:text-rose-400 text-[11px] font-medium">
                        Failed
                      </span>
                    )}
                  </div>

                  {item.outputPath && (
                    <div
                      className="text-[11px] font-mono text-zinc-400 dark:text-zinc-500 truncate"
                      title={item.outputPath}
                    >
                      {item.outputPath}
                    </div>
                  )}
                </div>
              </div>

              {/* Actions */}
              <div className="flex flex-wrap items-center gap-1.5 shrink-0">
                {item.status === "completed" &&
                  item.fileExists &&
                  item.outputPath && (
                    <button
                      type="button"
                      onClick={() => tauriBridge.openFile(item.outputPath!)}
                      className="inline-flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-xs font-semibold bg-indigo-600 hover:bg-indigo-500 text-white transition-colors"
                    >
                      <ExternalLink className="w-3.5 h-3.5" />
                      Open
                    </button>
                  )}

                <button
                  type="button"
                  onClick={() =>
                    tauriBridge.showInFolder(item.outputPath || item.outputDir)
                  }
                  title="Open containing folder"
                  className="p-2 rounded-lg text-zinc-500 hover:text-zinc-900 dark:hover:text-zinc-100 hover:bg-surface-hover border border-border transition-colors"
                >
                  <FolderOpen className="w-3.5 h-3.5" />
                </button>

                <button
                  type="button"
                  onClick={() => handleDownloadAgain(item)}
                  title="Download again"
                  className="p-2 rounded-lg text-zinc-500 hover:text-zinc-900 dark:hover:text-zinc-100 hover:bg-surface-hover border border-border transition-colors"
                >
                  <RotateCcw className="w-3.5 h-3.5" />
                </button>

                <button
                  type="button"
                  onClick={() => handleCopyUrl(item.sourceUrl)}
                  title="Copy source URL"
                  className="p-2 rounded-lg text-zinc-500 hover:text-zinc-900 dark:hover:text-zinc-100 hover:bg-surface-hover border border-border transition-colors"
                >
                  <Copy className="w-3.5 h-3.5" />
                </button>

                <button
                  type="button"
                  onClick={() => removeHistoryItem(item.id)}
                  title="Remove history entry (preserves file on disk)"
                  className="p-2 rounded-lg text-zinc-400 hover:text-rose-500 hover:bg-rose-500/10 transition-colors"
                >
                  <Trash2 className="w-3.5 h-3.5" />
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </motion.div>
  );
};
