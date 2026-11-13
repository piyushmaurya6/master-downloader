import React, { useCallback, useEffect, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import {
  AlertCircle,
  Copy,
  FolderOpen,
  LayoutGrid,
  List,
  Play,
  RefreshCw,
  Search,
  Trash2,
} from "lucide-react";
import { ipc, isTauriRuntime, parseIpcError } from "../../lib/ipc";
import { formatBytes, formatRelativeDate } from "../../lib/utils";
import { useUIStore } from "../../stores/useUIStore";
import type { JobRecord } from "../../types";
import { ProxiedThumbnail } from "../ui/ProxiedThumbnail";

export const HistoryLibraryView: React.FC = () => {
  const addToast = useUIStore((s) => s.addToast);
  const setActiveView = useUIStore((s) => s.setActiveView);

  const [records, setRecords] = useState<JobRecord[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [mediaFilter, setMediaFilter] = useState("all");
  const [viewMode, setViewMode] = useState<"table" | "grid">("table");
  const [loading, setLoading] = useState(false);

  const parentRef = useRef<HTMLDivElement>(null);

  const fetchHistory = useCallback(async () => {
    if (!isTauriRuntime()) return;
    setLoading(true);
    try {
      const data = await ipc.queryDownloadHistory({
        search_query: searchQuery.trim() || null,
        media_type_filter: mediaFilter,
        limit: 500,
      });
      setRecords(data);
    } catch (err) {
      addToast({
        title: "Failed to Load History",
        description: parseIpcError(err).user_message,
        variant: "error",
      });
    } finally {
      setLoading(false);
    }
  }, [searchQuery, mediaFilter, addToast]);

  useEffect(() => {
    fetchHistory();
  }, [fetchHistory]);

  const rowVirtualizer = useVirtualizer({
    count: records.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 74,
    overscan: 8,
  });

  const handleRedownload = async (job: JobRecord) => {
    try {
      await ipc.enqueueJob({
        source_url: job.source_url,
        shortcode: job.shortcode,
        url_type: job.url_type,
        author_username: job.author_username,
        caption: job.caption,
        thumbnail_url: job.thumbnail_url,
        selected_indices: job.selected_indices,
        quality_preset: job.quality_preset,
      });
      addToast({
        title: "Re-enqueued Missing Media",
        description: job.source_url,
        variant: "success",
      });
      setActiveView("workspace");
    } catch (err) {
      addToast({
        title: "Re-download Failed",
        description: parseIpcError(err).user_message,
        variant: "error",
      });
    }
  };

  const handleDelete = async (jobId: string, deleteFiles: boolean) => {
    try {
      await ipc.deleteHistoryItem(jobId, deleteFiles);
      setRecords((prev) => prev.filter((r) => r.id !== jobId));
      addToast({
        title: deleteFiles
          ? "Deleted Record & Disk Files"
          : "Removed History Record",
        variant: "default",
      });
    } catch (err) {
      addToast({
        title: "Delete Failed",
        description: parseIpcError(err).user_message,
        variant: "error",
      });
    }
  };

  return (
    <div className="h-full flex flex-col space-y-4">
      {/* Header & Search Controls */}
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="text-base font-semibold text-zinc-100">
            Download History &amp; Media Library
          </h1>
          <p className="text-xs text-zinc-400">
            {records.length} archived posts in SQLite with real-time disk verification
          </p>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <div className="relative">
            <Search className="w-3.5 h-3.5 text-zinc-400 absolute left-3 top-1/2 -translate-y-1/2" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Search @username, caption, shortcode..."
              className="pl-8 pr-3 py-1.5 w-64 rounded-lg bg-[#121215] border border-white/10 text-xs text-zinc-100 placeholder-zinc-500 focus:outline-none focus:border-indigo-500"
            />
          </div>

          <select
            value={mediaFilter}
            onChange={(e) => setMediaFilter(e.target.value)}
            className="px-3 py-1.5 rounded-lg bg-[#121215] border border-white/10 text-xs text-zinc-100 focus:outline-none focus:border-indigo-500"
          >
            <option value="all">All Media Types</option>
            <option value="reel">Reels &amp; Videos</option>
            <option value="post">Single Photos</option>
            <option value="carousel">Multi-Slide Carousels</option>
            <option value="stories">Stories &amp; Highlights</option>
          </select>

          <div className="flex items-center rounded-lg bg-[#121215] border border-white/10 p-0.5">
            <button
              onClick={() => setViewMode("table")}
              className={`p-1.5 rounded-md transition-colors ${
                viewMode === "table"
                  ? "bg-indigo-500/20 text-indigo-300"
                  : "text-zinc-400 hover:text-zinc-200"
              }`}
              title="Virtualized Table View"
            >
              <List className="w-3.5 h-3.5" />
            </button>
            <button
              onClick={() => setViewMode("grid")}
              className={`p-1.5 rounded-md transition-colors ${
                viewMode === "grid"
                  ? "bg-indigo-500/20 text-indigo-300"
                  : "text-zinc-400 hover:text-zinc-200"
              }`}
              title="Media Grid View"
            >
              <LayoutGrid className="w-3.5 h-3.5" />
            </button>
          </div>

          <button
            onClick={fetchHistory}
            className="p-2 rounded-lg bg-[#121215] border border-white/10 text-zinc-400 hover:text-zinc-100"
            title="Refresh Library"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${loading ? "animate-spin" : ""}`} />
          </button>
        </div>
      </div>

      {/* Library Content */}
      <div
        ref={parentRef}
        className="flex-1 overflow-y-auto rounded-2xl bg-[#121215] border border-white/[0.07] p-3"
      >
        {records.length === 0 ? (
          <div className="h-full flex flex-col items-center justify-center text-center p-8">
            <p className="text-xs font-medium text-zinc-400">
              No archived media matches your filter
            </p>
            <p className="text-[11px] text-zinc-500 mt-1">
              Completed downloads will appear here with instant Explorer and preview controls.
            </p>
          </div>
        ) : viewMode === "grid" ? (
          <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 gap-3">
            {records.map((job) => {
              const primaryFile = job.media_files[0];
              const missingOnDisk =
                job.media_files.length > 0 &&
                job.media_files.every((f) => !f.file_exists);

              return (
                <div
                  key={job.id}
                  className="rounded-xl bg-[#09090B] border border-white/[0.08] overflow-hidden flex flex-col justify-between"
                >
                  <div className="relative h-36">
                    <ProxiedThumbnail
                      src={job.thumbnail_url}
                      mediaType={primaryFile?.media_type ?? "video"}
                      className="w-full h-full"
                    />
                    <span className="absolute top-2 left-2 px-1.5 py-0.5 rounded bg-black/75 text-[10px] font-mono font-bold uppercase text-indigo-300">
                      {job.url_type}
                    </span>
                    {job.media_files.length > 1 && (
                      <span className="absolute top-2 right-2 px-1.5 py-0.5 rounded bg-black/75 text-[10px] font-mono text-zinc-200">
                        {job.media_files.length} files
                      </span>
                    )}
                  </div>

                  <div className="p-3 space-y-2">
                    <div className="flex items-center justify-between">
                      <span className="text-xs font-semibold text-zinc-100 truncate">
                        @{job.author_username ?? "instagram"}
                      </span>
                      <span className="text-[10px] font-mono text-zinc-500">
                        {formatBytes(job.downloaded_bytes)}
                      </span>
                    </div>

                    {missingOnDisk ? (
                      <button
                        onClick={() => handleRedownload(job)}
                        className="w-full py-1.5 px-2 rounded-lg text-[11px] font-medium bg-amber-500/15 text-amber-300 border border-amber-500/30 flex items-center justify-center gap-1"
                      >
                        <AlertCircle className="w-3 h-3" />
                        File Missing — Re-download
                      </button>
                    ) : (
                      <div className="flex items-center justify-between gap-1 pt-1">
                        {primaryFile && (
                          <button
                            onClick={() => ipc.openMediaFile(primaryFile.file_path)}
                            className="flex-1 py-1 rounded bg-indigo-500/15 hover:bg-indigo-500/25 text-indigo-300 text-[11px] font-medium flex items-center justify-center gap-1"
                          >
                            <Play className="w-3 h-3" />
                            Preview
                          </button>
                        )}
                        <button
                          onClick={() =>
                            ipc.revealInExplorer(
                              primaryFile?.file_path ?? job.output_dir
                            )
                          }
                          className="p-1.5 rounded bg-white/[0.05] hover:bg-white/[0.1] text-zinc-300"
                          title="Reveal in Windows Explorer"
                        >
                          <FolderOpen className="w-3.5 h-3.5" />
                        </button>
                        <button
                          onClick={() => handleDelete(job.id, false)}
                          className="p-1.5 rounded bg-white/[0.05] hover:bg-rose-500/20 text-zinc-400 hover:text-rose-300"
                          title="Remove Record"
                        >
                          <Trash2 className="w-3.5 h-3.5" />
                        </button>
                      </div>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        ) : (
          /* Virtualized 60fps Table View */
          <div
            style={{
              height: `${rowVirtualizer.getTotalSize()}px`,
              width: "100%",
              position: "relative",
            }}
          >
            {rowVirtualizer.getVirtualItems().map((virtualRow) => {
              const job = records[virtualRow.index];
              const primaryFile = job.media_files[0];
              const missingOnDisk =
                job.media_files.length > 0 &&
                job.media_files.every((f) => !f.file_exists);

              return (
                <div
                  key={job.id}
                  style={{
                    position: "absolute",
                    top: 0,
                    left: 0,
                    width: "100%",
                    height: `${virtualRow.size - 6}px`,
                    transform: `translateY(${virtualRow.start}px)`,
                  }}
                  className="rounded-xl bg-[#09090B] border border-white/[0.07] px-3.5 flex items-center justify-between gap-4"
                >
                  <div className="flex items-center gap-3 min-w-0">
                    <ProxiedThumbnail
                      src={job.thumbnail_url}
                      mediaType={primaryFile?.media_type ?? "video"}
                      className="w-12 h-12 rounded-lg shrink-0"
                    />
                    <div className="min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="text-xs font-semibold text-zinc-100">
                          @{job.author_username ?? "instagram"}
                        </span>
                        <span className="px-1.5 py-0.2 rounded text-[9px] font-mono uppercase bg-indigo-500/20 text-indigo-300">
                          {job.url_type}
                        </span>
                        <span className="text-[11px] font-mono text-zinc-500">
                          {formatBytes(job.downloaded_bytes)}
                        </span>
                        <span className="text-[11px] text-zinc-500">
                          · {formatRelativeDate(job.completed_at ?? job.created_at)}
                        </span>
                      </div>
                      <p className="text-[11px] text-zinc-400 truncate max-w-md mt-0.5">
                        {job.caption || job.source_url}
                      </p>
                    </div>
                  </div>

                  <div className="flex items-center gap-1.5 shrink-0">
                    {missingOnDisk ? (
                      <button
                        onClick={() => handleRedownload(job)}
                        className="px-2.5 py-1 rounded-lg text-[11px] font-medium bg-amber-500/15 text-amber-300 border border-amber-500/30 flex items-center gap-1"
                      >
                        <AlertCircle className="w-3.5 h-3.5" />
                        File Missing — Re-download
                      </button>
                    ) : (
                      <>
                        {primaryFile && (
                          <button
                            onClick={() => ipc.openMediaFile(primaryFile.file_path)}
                            className="px-2.5 py-1 rounded-lg text-xs font-medium bg-indigo-500/15 text-indigo-300 hover:bg-indigo-500/25 border border-indigo-500/30 flex items-center gap-1"
                          >
                            <Play className="w-3 h-3" />
                            Play
                          </button>
                        )}
                        <button
                          onClick={() =>
                            ipc.revealInExplorer(
                              primaryFile?.file_path ?? job.output_dir
                            )
                          }
                          className="p-1.5 rounded-lg text-zinc-400 hover:text-zinc-100 hover:bg-white/[0.06]"
                          title="Reveal in Windows Explorer"
                        >
                          <FolderOpen className="w-4 h-4" />
                        </button>
                      </>
                    )}

                    <button
                      onClick={() => {
                        navigator.clipboard.writeText(job.source_url);
                        addToast({
                          title: "Copied Original Instagram Link",
                          description: job.source_url,
                        });
                      }}
                      className="p-1.5 rounded-lg text-zinc-400 hover:text-zinc-100 hover:bg-white/[0.06]"
                      title="Copy Original Link"
                    >
                      <Copy className="w-4 h-4" />
                    </button>

                    <button
                      onClick={() => handleDelete(job.id, false)}
                      className="p-1.5 rounded-lg text-zinc-500 hover:text-rose-400 hover:bg-white/[0.06]"
                      title="Delete History Record"
                    >
                      <Trash2 className="w-4 h-4" />
                    </button>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
};
