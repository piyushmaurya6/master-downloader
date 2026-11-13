import React, { useCallback, useEffect, useState } from "react";
import {
  Download,
  ExternalLink,
  FolderOpen,
  Search,
  Trash2,
} from "lucide-react";
import { ipc, isTauriRuntime } from "../../lib/ipc";
import { formatBytes } from "../../lib/utils";
import { useUIStore } from "../../stores/useUIStore";
import type { JobRecord } from "../../types";

export const HistoryLibraryView: React.FC = () => {
  const addToast = useUIStore((s) => s.addToast);
  const [items, setItems] = useState<JobRecord[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [platformFilter, setPlatformFilter] = useState("all");
  const [mediaTypeFilter, setMediaTypeFilter] = useState("all");
  const [loading, setLoading] = useState(false);

  const fetchHistory = useCallback(async () => {
    if (!isTauriRuntime()) return;
    setLoading(true);
    try {
      const records = await ipc.queryDownloadHistory({
        search_query: searchQuery.trim() || null,
        platform: platformFilter === "all" ? null : platformFilter,
        media_type: mediaTypeFilter === "all" ? null : mediaTypeFilter,
        status: "completed",
        limit: 200,
        offset: 0,
      });
      setItems(records);
    } catch {
      // ignore
    } finally {
      setLoading(false);
    }
  }, [searchQuery, platformFilter, mediaTypeFilter]);

  useEffect(() => {
    const t = setTimeout(fetchHistory, 150);
    return () => clearTimeout(t);
  }, [fetchHistory]);

  const handleExportJson = async () => {
    try {
      const dir = await ipc.pickDirectoryDialog(null);
      if (!dir) return;
      const exportPath = `${dir}\\MasterDownloader_History_Export.json`;
      const count = await ipc.exportHistoryJson(exportPath);
      addToast({
        title: "History Exported",
        description: `Exported ${count} records to ${exportPath}`,
        variant: "success",
      });
    } catch {
      // ignore
    }
  };

  return (
    <div className="flex-1 flex flex-col min-h-0 space-y-4">
      {/* Search & Filter Header */}
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-2 flex-1 max-w-md px-3 py-2 rounded-xl bg-[#0F131D] border border-white/[0.08]">
          <Search className="w-4 h-4 text-slate-400" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search by creator, channel, video title, shortcode, or URL..."
            className="w-full bg-transparent text-xs text-slate-100 placeholder:text-slate-500 focus:outline-none"
          />
        </div>

        <div className="flex items-center gap-2 flex-wrap">
          {/* Platform Filter Pills */}
          {(["all", "youtube", "instagram", "facebook"] as const).map((plat) => (
            <button
              key={plat}
              onClick={() => setPlatformFilter(plat)}
              className={`px-2.5 py-1.5 rounded-lg text-xs font-medium capitalize transition-colors ${
                platformFilter === plat
                  ? "bg-violet-500/20 text-violet-200 border border-violet-500/35"
                  : "bg-[#0F131D] text-slate-400 hover:text-slate-200 border border-white/[0.08]"
              }`}
            >
              {plat === "all" ? "All Platforms" : plat}
            </button>
          ))}

          {/* Media Type Filter Select */}
          <select
            value={mediaTypeFilter}
            onChange={(e) => setMediaTypeFilter(e.target.value)}
            className="px-2.5 py-1.5 rounded-lg bg-[#0F131D] border border-white/[0.08] text-xs text-slate-300"
          >
            <option value="all">All Media Types</option>
            <option value="video">Video</option>
            <option value="reel">Reels / Shorts</option>
            <option value="audio">Audio</option>
            <option value="image">Photos / Carousels</option>
          </select>

          <button
            onClick={handleExportJson}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-[#0F131D] hover:bg-white/[0.06] border border-white/[0.08] text-xs text-slate-300"
          >
            <Download className="w-3.5 h-3.5 text-violet-400" />
            <span>Export JSON</span>
          </button>
        </div>
      </div>

      {/* Media Grid */}
      <div className="flex-1 overflow-y-auto pr-1">
        {loading ? (
          <div className="h-48 flex items-center justify-center text-xs text-slate-400">
            Loading library...
          </div>
        ) : items.length === 0 ? (
          <div className="h-60 rounded-2xl border border-dashed border-white/[0.08] bg-[#0F131D]/40 flex flex-col items-center justify-center text-center p-6">
            <p className="text-xs font-medium text-slate-300">
              No completed downloads match your filter
            </p>
            <p className="text-[11px] text-slate-500 mt-1">
              Completed YouTube, Instagram, and Facebook downloads appear here
              with instant file preview and Explorer reveal.
            </p>
          </div>
        ) : (
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
            {items.map((item) => {
              const primaryFile = item.media_files[0];
              const totalSize = item.media_files.reduce(
                (acc, f) => acc + (f.file_size_bytes ?? 0),
                0
              );

              return (
                <div
                  key={item.id}
                  className="rounded-xl bg-[#0F131D] border border-white/[0.08] hover:border-white/[0.15] overflow-hidden flex flex-col justify-between p-3.5 space-y-3"
                >
                  <div className="flex items-start gap-3">
                    <div className="w-16 h-16 rounded-lg bg-[#07090E] border border-white/[0.08] overflow-hidden shrink-0 flex items-center justify-center">
                      {item.thumbnail_url ? (
                        <img
                          src={item.thumbnail_url}
                          alt=""
                          className="w-full h-full object-cover"
                        />
                      ) : (
                        <span className="text-[10px] font-mono uppercase text-violet-400 font-bold">
                          {item.platform}
                        </span>
                      )}
                    </div>

                    <div className="min-w-0 flex-1">
                      <div className="flex items-center justify-between gap-2">
                        <span className="text-xs font-semibold text-slate-100 truncate">
                          @{item.author_username ?? "creator"}
                        </span>
                        <span className="text-[10px] font-mono uppercase px-1.5 py-0.5 rounded bg-violet-500/15 text-violet-300">
                          {item.platform} · {item.quality_preset}
                        </span>
                      </div>
                      <p className="text-[11px] text-slate-400 line-clamp-2 mt-1">
                        {item.caption || item.source_url}
                      </p>
                    </div>
                  </div>

                  <div className="flex items-center justify-between pt-2 border-t border-white/[0.06] text-[11px] text-slate-400">
                    <span className="font-mono">
                      {item.media_files.length} file(s) ·{" "}
                      {formatBytes(totalSize || item.downloaded_bytes)}
                    </span>

                    <div className="flex items-center gap-1">
                      {primaryFile && (
                        <button
                          onClick={() =>
                            ipc.openMediaFile(primaryFile.file_path)
                          }
                          className="p-1.5 rounded-lg hover:bg-white/[0.06] text-slate-300"
                          title="Play / Open Media"
                        >
                          <ExternalLink className="w-3.5 h-3.5 text-violet-400" />
                        </button>
                      )}
                      <button
                        onClick={() =>
                          ipc.revealInExplorer(
                            primaryFile?.file_path ?? item.output_dir
                          )
                        }
                        className="p-1.5 rounded-lg hover:bg-white/[0.06] text-slate-300"
                        title="Show in Windows Explorer"
                      >
                        <FolderOpen className="w-3.5 h-3.5" />
                      </button>
                      <button
                        onClick={async () => {
                          await ipc.deleteHistoryItem(item.id, false);
                          fetchHistory();
                        }}
                        className="p-1.5 rounded-lg hover:bg-rose-500/15 text-slate-400 hover:text-rose-300"
                        title="Remove from History"
                      >
                        <Trash2 className="w-3.5 h-3.5" />
                      </button>
                    </div>
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
