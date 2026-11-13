import React, { useEffect, useState, useCallback } from "react";
import {
  Search,
  LayoutGrid,
  List,
  FolderOpen,
  Trash2,
  ExternalLink,
  RotateCcw,
  Film,
  AlertTriangle,
  Play,
} from "lucide-react";
import { ipc, parseSerializedError } from "../../lib/ipc";
import { useUIStore } from "../../stores/useUIStore";
import { useQueueStore } from "../../stores/useQueueStore";
import { formatBytes, formatRelativeDate } from "../../lib/utils";
import { ProxiedThumbnail } from "../ui/ProxiedThumbnail";
import type { JobRecord } from "../../types";

export const HistoryLibraryView: React.FC = () => {
  const { addToast, setActiveTab } = useUIStore();
  const { upsertJob } = useQueueStore();

  const [items, setItems] = useState<JobRecord[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [mediaFilter, setMediaFilter] = useState("all");
  const [viewMode, setViewMode] = useState<"grid" | "list">("grid");
  const [isLoading, setIsLoading] = useState(false);

  const loadHistory = useCallback(async () => {
    setIsLoading(true);
    try {
      const records = await ipc.getHistory({
        search_query: searchQuery.trim() || null,
        media_type_filter: mediaFilter === "all" ? null : mediaFilter,
        limit: 200,
        offset: 0,
      });
      setItems(records);
    } catch (err) {
      addToast({
        type: "error",
        title: "Failed to Load History",
        message: parseSerializedError(err).user_message,
      });
    } finally {
      setIsLoading(false);
    }
  }, [searchQuery, mediaFilter, addToast]);

  useEffect(() => {
    loadHistory();
  }, [loadHistory]);

  const handleReveal = async (item: JobRecord) => {
    const targetPath = item.media_files?.[0]?.file_path || item.output_dir;
    try {
      await ipc.openInExplorer(targetPath);
    } catch (err) {
      addToast({
        type: "error",
        title: "File Not Found",
        message: parseSerializedError(err).user_message,
      });
    }
  };

  const handleOpenMedia = async (item: JobRecord) => {
    const file = item.media_files?.[0]?.file_path;
    if (!file) {
      handleReveal(item);
      return;
    }
    try {
      await ipc.openMediaFile(file);
    } catch (err) {
      addToast({
        type: "error",
        title: "Cannot Open Media",
        message: parseSerializedError(err).user_message,
      });
    }
  };

  const handleRedownload = async (item: JobRecord) => {
    try {
      const job = await ipc.enqueueJob({
        source_url: item.source_url,
        shortcode: item.shortcode,
        url_type: item.url_type,
        author_username: item.author_username,
        caption: item.caption,
        thumbnail_url: item.thumbnail_url,
        selected_indices: item.selected_indices,
        quality_preset: item.quality_preset,
        output_dir: item.output_dir,
        naming_template: item.naming_template,
        save_caption: item.save_caption,
        save_metadata: item.save_metadata,
        embed_thumbnail: item.embed_thumbnail,
      });
      upsertJob(job);
      addToast({
        type: "success",
        title: "Re-Queued Download",
        message: `Added ${item.author_username || item.shortcode} back to queue.`,
      });
      setActiveTab("downloads");
    } catch (err) {
      addToast({
        type: "error",
        title: "Re-Download Failed",
        message: parseSerializedError(err).user_message,
      });
    }
  };

  const handleDelete = async (item: JobRecord, deleteDisk: boolean) => {
    try {
      await ipc.deleteHistoryItem(item.id, deleteDisk);
      setItems((prev) => prev.filter((i) => i.id !== item.id));
      addToast({
        type: "info",
        title: "History Entry Removed",
        message: deleteDisk
          ? "Removed from history and deleted local files."
          : "Removed from history database.",
      });
    } catch (err) {
      addToast({
        type: "error",
        title: "Delete Failed",
        message: parseSerializedError(err).user_message,
      });
    }
  };

  const handleClearAll = async () => {
    try {
      await ipc.clearAllHistory();
      setItems([]);
      addToast({
        type: "info",
        title: "History Cleared",
        message: "All completed history records have been cleared.",
      });
    } catch {
      // ignore
    }
  };

  return (
    <div className="space-y-4">
      {/* Search & Filter Bar */}
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="relative flex-1 min-w-[240px]">
          <Search className="w-4 h-4 text-text-muted absolute left-3.5 top-1/2 -translate-y-1/2" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search by Facebook creator, page, title, caption, or video ID..."
            className="w-full h-10 pl-10 pr-4 rounded-xl bg-bg-surface border border-border-subtle text-xs text-text-primary placeholder:text-text-muted focus:outline-none focus:border-border-focus"
          />
        </div>

        <div className="flex items-center gap-2">
          <select
            value={mediaFilter}
            onChange={(e) => setMediaFilter(e.target.value)}
            className="h-10 px-3 rounded-xl bg-bg-surface border border-border-subtle text-xs text-text-primary focus:outline-none focus:border-border-focus"
          >
            <option value="all">All Media Types</option>
            <option value="reel">Reels</option>
            <option value="watch">Watch / Videos</option>
            <option value="post">Posts</option>
            <option value="photo">Photos / Albums</option>
            <option value="story">Stories</option>
          </select>

          <div className="flex items-center rounded-xl bg-bg-surface border border-border-subtle p-1">
            <button
              onClick={() => setViewMode("grid")}
              className={`p-1.5 rounded-lg transition-colors ${
                viewMode === "grid"
                  ? "bg-bg-elevated text-text-primary"
                  : "text-text-muted hover:text-text-primary"
              }`}
              title="Grid View"
            >
              <LayoutGrid className="w-4 h-4" />
            </button>
            <button
              onClick={() => setViewMode("list")}
              className={`p-1.5 rounded-lg transition-colors ${
                viewMode === "list"
                  ? "bg-bg-elevated text-text-primary"
                  : "text-text-muted hover:text-text-primary"
              }`}
              title="Table View"
            >
              <List className="w-4 h-4" />
            </button>
          </div>

          {items.length > 0 && (
            <button
              onClick={handleClearAll}
              className="h-10 px-3 rounded-xl bg-bg-surface hover:bg-bg-elevated border border-border-subtle text-xs text-text-secondary hover:text-status-error transition-colors"
            >
              Clear All
            </button>
          )}
        </div>
      </div>

      {/* Empty State */}
      {!isLoading && items.length === 0 && (
        <div className="flex flex-col items-center justify-center py-20 text-center rounded-2xl bg-bg-surface/40 border border-border-subtle">
          <Film className="w-8 h-8 text-text-muted mb-2" />
          <h3 className="text-sm font-semibold text-text-primary">
            No History Records Found
          </h3>
          <p className="text-xs text-text-secondary max-w-sm mt-1">
            Completed Facebook Video, Reel, and Post downloads will appear here with instant file preview and Windows Explorer integration.
          </p>
        </div>
      )}

      {/* Grid View */}
      {viewMode === "grid" && items.length > 0 && (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {items.map((item) => {
            const firstFile = item.media_files?.[0];
            const isMissing = firstFile && !firstFile.file_exists;

            return (
              <div
                key={item.id}
                className="group rounded-2xl bg-bg-surface border border-border-subtle overflow-hidden flex flex-col justify-between hover:border-border-focus/80 transition-all"
              >
                <div className="relative aspect-video bg-bg-canvas overflow-hidden">
                  <ProxiedThumbnail
                    src={item.thumbnail_url}
                    alt={item.author_username || "Media"}
                    className="w-full h-full object-cover"
                    fallbackIcon={<Film className="w-8 h-8 text-text-muted" />}
                  />
                  <div className="absolute inset-0 bg-gradient-to-t from-black/80 via-transparent to-black/30" />

                  <div className="absolute top-2.5 left-2.5 flex items-center gap-1.5">
                    <span className="px-2 py-0.5 text-[10px] font-mono font-bold uppercase rounded fb-gradient-bg text-white">
                      {item.url_type}
                    </span>
                    <span className="px-2 py-0.5 text-[10px] font-mono rounded bg-black/70 text-white">
                      {item.quality_preset.toUpperCase()}
                    </span>
                  </div>

                  {isMissing && (
                    <div className="absolute top-2.5 right-2.5 flex items-center gap-1 px-2 py-0.5 rounded bg-status-warning/90 text-black text-[10px] font-semibold">
                      <AlertTriangle className="w-3 h-3" />
                      <span>Moved/Deleted</span>
                    </div>
                  )}

                  {!isMissing && firstFile && (
                    <button
                      onClick={() => handleOpenMedia(item)}
                      className="absolute inset-0 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity bg-black/30"
                      title="Open Media"
                    >
                      <div className="w-10 h-10 rounded-full fb-gradient-bg flex items-center justify-center text-white shadow-lg">
                        <Play className="w-4 h-4 fill-current" />
                      </div>
                    </button>
                  )}

                  <div className="absolute bottom-2.5 left-2.5 right-2.5 flex items-center justify-between text-xs text-white">
                    <span className="font-semibold truncate">
                      {item.author_username || item.shortcode || "Facebook"}
                    </span>
                    <span className="text-[11px] font-mono text-white/80">
                      {formatBytes(item.downloaded_bytes)}
                    </span>
                  </div>
                </div>

                <div className="p-3.5 space-y-3">
                  <p className="text-xs text-text-secondary line-clamp-2 min-h-[32px]">
                    {item.caption || item.source_url}
                  </p>

                  <div className="flex items-center justify-between pt-2 border-t border-border-subtle">
                    <span className="text-[11px] text-text-muted">
                      {formatRelativeDate(item.completed_at || item.created_at)}
                    </span>

                    <div className="flex items-center gap-1">
                      {isMissing ? (
                        <button
                          onClick={() => handleRedownload(item)}
                          title="Re-Download Missing File"
                          className="p-1.5 rounded-lg text-status-warning hover:bg-bg-elevated transition-colors"
                        >
                          <RotateCcw className="w-3.5 h-3.5" />
                        </button>
                      ) : (
                        <button
                          onClick={() => handleReveal(item)}
                          title="Show in Explorer"
                          className="p-1.5 rounded-lg text-text-secondary hover:text-text-primary hover:bg-bg-elevated transition-colors"
                        >
                          <FolderOpen className="w-3.5 h-3.5" />
                        </button>
                      )}

                      <a
                        href={item.source_url}
                        target="_blank"
                        rel="noreferrer"
                        title="Open Original Facebook URL"
                        className="p-1.5 rounded-lg text-text-secondary hover:text-text-primary hover:bg-bg-elevated transition-colors"
                      >
                        <ExternalLink className="w-3.5 h-3.5" />
                      </a>

                      <button
                        onClick={() => handleDelete(item, false)}
                        title="Remove from History"
                        className="p-1.5 rounded-lg text-text-muted hover:text-status-error hover:bg-bg-elevated transition-colors"
                      >
                        <Trash2 className="w-3.5 h-3.5" />
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {/* List View */}
      {viewMode === "list" && items.length > 0 && (
        <div className="rounded-2xl bg-bg-surface border border-border-subtle overflow-hidden divide-y divide-border-subtle">
          {items.map((item) => (
            <div
              key={item.id}
              className="flex items-center justify-between px-4 py-3 hover:bg-bg-elevated/40 transition-colors gap-4"
            >
              <div className="flex items-center gap-3 min-w-0">
                <span className="px-2 py-0.5 text-[10px] font-mono font-bold uppercase rounded bg-blue-500/15 text-blue-400 shrink-0">
                  {item.url_type}
                </span>
                <span className="text-xs font-semibold text-text-primary shrink-0">
                  {item.author_username || item.shortcode || "Facebook"}
                </span>
                <span className="text-xs text-text-secondary truncate">
                  {item.caption || item.source_url}
                </span>
              </div>

              <div className="flex items-center gap-3 shrink-0">
                <span className="text-xs font-mono text-text-secondary">
                  {formatBytes(item.downloaded_bytes)}
                </span>
                <button
                  onClick={() => handleReveal(item)}
                  className="p-1.5 rounded-lg text-text-secondary hover:text-text-primary hover:bg-bg-elevated"
                  title="Show in Explorer"
                >
                  <FolderOpen className="w-4 h-4" />
                </button>
                <button
                  onClick={() => handleDelete(item, false)}
                  className="p-1.5 rounded-lg text-text-muted hover:text-status-error hover:bg-bg-elevated"
                  title="Remove"
                >
                  <Trash2 className="w-4 h-4" />
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};
