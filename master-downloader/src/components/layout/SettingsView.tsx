import React, { useEffect, useState } from "react";
import {
  CheckCircle2,
  Cpu,
  FileArchive,
  FolderOpen,
  RefreshCw,
  Save,
  Shield,
  Volume2,
  Zap,
} from "lucide-react";
import { ipc, parseIpcError } from "../../lib/ipc";
import { formatBytes } from "../../lib/utils";
import { useSettingsStore } from "../../stores/useSettingsStore";
import { useUIStore } from "../../stores/useUIStore";

export const SettingsView: React.FC = () => {
  const settings = useSettingsStore((s) => s.settings);
  const setSettings = useSettingsStore((s) => s.setSettings);
  const binaryStatus = useSettingsStore((s) => s.binaryStatus);
  const setBinaryStatus = useSettingsStore((s) => s.setBinaryStatus);
  const addToast = useUIStore((s) => s.addToast);

  const [draft, setDraft] = useState(
    settings ?? {
      output_dir: "Downloads\\MasterDownloader",
      default_quality: "best",
      default_container: "mp4",
      default_audio_format: "mp3",
      default_audio_bitrate: "320",
      audio_normalization: false,
      concurrency: 3,
      jitter_min_ms: 250,
      jitter_max_ms: 600,
      speed_limit_kbps: null,
      clipboard_monitor: true,
      auto_download_on_detect: true,
      organize_by_platform: true,
      organize_by_uploader: true,
      embed_thumbnail: true,
      save_caption_sidecar: false,
      save_metadata_json: false,
      write_subtitles: false,
      embed_chapters: true,
      naming_template: "{username}_{date}_{shortcode}_{index}",
      browser_cookie_source: null,
    }
  );
  const [updatingBins, setUpdatingBins] = useState(false);

  useEffect(() => {
    if (settings) {
      setDraft(settings);
    }
  }, [settings]);

  const handlePickFolder = async () => {
    try {
      const picked = await ipc.pickDirectoryDialog(draft.output_dir);
      if (picked) {
        setDraft({ ...draft, output_dir: picked });
      }
    } catch {
      // ignore
    }
  };

  const handleSave = async () => {
    try {
      const saved = await ipc.updateAppSettings(draft);
      setSettings(saved);
      addToast({
        title: "Settings Saved",
        description: "Configuration persisted to SQLite.",
        variant: "success",
      });
    } catch (err) {
      const s = parseIpcError(err);
      addToast({
        title: "Failed to Save Settings",
        description: s.user_message,
        variant: "error",
      });
    }
  };

  const handleUpdateBinaries = async () => {
    setUpdatingBins(true);
    try {
      const report = await ipc.bootstrapOrUpdateBinaries(true);
      setBinaryStatus(report);
      addToast({
        title: "Engines Updated & Verified",
        description: "yt-dlp, gallery-dl, Deno, and FFmpeg are up to date.",
        variant: "success",
      });
    } catch (err) {
      const s = parseIpcError(err);
      addToast({
        title: "Engine Update Failed",
        description: s.user_message,
        variant: "error",
      });
    } finally {
      setUpdatingBins(false);
    }
  };

  const handleExportDiagnostics = async () => {
    try {
      const zipPath = await ipc.exportDiagnosticBundle();
      addToast({
        title: "Diagnostic Bundle Exported",
        description: `Saved redacted bundle to ${zipPath}`,
        variant: "success",
      });
    } catch (err) {
      const s = parseIpcError(err);
      addToast({
        title: "Diagnostic Export Failed",
        description: s.user_message,
        variant: "error",
      });
    }
  };

  return (
    <div className="flex-1 overflow-y-auto space-y-5 pr-1">
      {/* Auto-Detection & Output Directory Card */}
      <div className="p-5 rounded-2xl bg-[#0F131D] border border-white/[0.08] space-y-4">
        <div className="flex items-center justify-between">
          <h3 className="text-xs font-semibold uppercase tracking-wider text-slate-400">
            Auto-Detect, Auto-Download &amp; Output Path
          </h3>
          <span className="flex items-center gap-1 text-[11px] text-emerald-400 font-medium">
            <Zap className="w-3.5 h-3.5" />
            <span>Universal Platform Router</span>
          </span>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div className="md:col-span-2">
            <label className="block text-xs text-slate-300 mb-1.5">
              Base Download Directory
            </label>
            <div className="flex items-center gap-2">
              <input
                type="text"
                value={draft.output_dir}
                onChange={(e) =>
                  setDraft({ ...draft, output_dir: e.target.value })
                }
                className="flex-1 px-3 py-2 rounded-xl bg-[#07090E] border border-white/[0.08] text-xs font-mono text-slate-200"
              />
              <button
                type="button"
                onClick={handlePickFolder}
                className="flex items-center gap-1.5 px-3.5 py-2 rounded-xl bg-white/[0.06] hover:bg-white/[0.12] border border-white/[0.08] text-xs text-slate-200"
              >
                <FolderOpen className="w-3.5 h-3.5 text-violet-400" />
                <span>Browse</span>
              </button>
            </div>
          </div>

          <div>
            <label className="block text-xs text-slate-300 mb-1.5">
              Default Video Quality Preset
            </label>
            <select
              value={draft.default_quality}
              onChange={(e) =>
                setDraft({ ...draft, default_quality: e.target.value })
              }
              className="w-full px-3 py-2 rounded-xl bg-[#07090E] border border-white/[0.08] text-xs text-slate-200"
            >
              <option value="best">Best Available (Up to 4K / 60fps)</option>
              <option value="4k">4K UHD (2160p)</option>
              <option value="1440p">2K QHD (1440p)</option>
              <option value="1080p60">1080p 60fps</option>
              <option value="1080p">1080p Full HD</option>
              <option value="720p">720p HD</option>
              <option value="480p">480p SD</option>
              <option value="audio_mp3">Audio Only (MP3 320kbps)</option>
              <option value="audio_m4a">Audio Only (M4A AAC)</option>
              <option value="audio_flac">Audio Only (FLAC Lossless)</option>
            </select>
          </div>

          <div>
            <label className="block text-xs text-slate-300 mb-1.5">
              Default Video Container &amp; Audio Format
            </label>
            <div className="grid grid-cols-2 gap-2">
              <select
                value={draft.default_container}
                onChange={(e) =>
                  setDraft({ ...draft, default_container: e.target.value })
                }
                className="w-full px-3 py-2 rounded-xl bg-[#07090E] border border-white/[0.08] text-xs text-slate-200"
              >
                <option value="mp4">MP4 Container</option>
                <option value="mkv">MKV Container</option>
                <option value="webm">WebM Container</option>
                <option value="auto">Auto Container</option>
              </select>

              <select
                value={draft.default_audio_format}
                onChange={(e) =>
                  setDraft({ ...draft, default_audio_format: e.target.value })
                }
                className="w-full px-3 py-2 rounded-xl bg-[#07090E] border border-white/[0.08] text-xs text-slate-200"
              >
                <option value="mp3">MP3 (320k)</option>
                <option value="m4a">M4A (AAC)</option>
                <option value="flac">FLAC (Lossless)</option>
                <option value="wav">WAV (PCM)</option>
                <option value="opus">OPUS</option>
              </select>
            </div>
          </div>
        </div>

        <div className="grid grid-cols-2 sm:grid-cols-3 gap-3 pt-2 text-xs text-slate-300">
          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              checked={draft.clipboard_monitor}
              onChange={(e) =>
                setDraft({ ...draft, clipboard_monitor: e.target.checked })
              }
              className="rounded accent-violet-500"
            />
            <span>Smart Clipboard Monitor</span>
          </label>

          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              checked={draft.auto_download_on_detect}
              onChange={(e) =>
                setDraft({
                  ...draft,
                  auto_download_on_detect: e.target.checked,
                })
              }
              className="rounded accent-violet-500"
            />
            <span className="font-semibold text-emerald-300">
              Auto-Download on Link Detect
            </span>
          </label>

          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              checked={draft.organize_by_platform}
              onChange={(e) =>
                setDraft({ ...draft, organize_by_platform: e.target.checked })
              }
              className="rounded accent-violet-500"
            />
            <span>Organize by Platform Folder</span>
          </label>

          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              checked={draft.organize_by_uploader}
              onChange={(e) =>
                setDraft({ ...draft, organize_by_uploader: e.target.checked })
              }
              className="rounded accent-violet-500"
            />
            <span>Organize by Creator / Channel</span>
          </label>

          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              checked={draft.audio_normalization}
              onChange={(e) =>
                setDraft({ ...draft, audio_normalization: e.target.checked })
              }
              className="rounded accent-violet-500"
            />
            <Volume2 className="w-3.5 h-3.5 text-emerald-400" />
            <span>EBU R128 Audio Loudnorm</span>
          </label>

          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              checked={draft.write_subtitles}
              onChange={(e) =>
                setDraft({ ...draft, write_subtitles: e.target.checked })
              }
              className="rounded accent-violet-500"
            />
            <span>Auto-Download Subtitles</span>
          </label>

          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              checked={draft.embed_chapters}
              onChange={(e) =>
                setDraft({ ...draft, embed_chapters: e.target.checked })
              }
              className="rounded accent-violet-500"
            />
            <span>Embed Video Chapters</span>
          </label>

          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              checked={draft.save_caption_sidecar}
              onChange={(e) =>
                setDraft({ ...draft, save_caption_sidecar: e.target.checked })
              }
              className="rounded accent-violet-500"
            />
            <span>Save Caption (.txt)</span>
          </label>

          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              checked={draft.save_metadata_json}
              onChange={(e) =>
                setDraft({ ...draft, save_metadata_json: e.target.checked })
              }
              className="rounded accent-violet-500"
            />
            <span>Save Metadata (.json)</span>
          </label>
        </div>
      </div>

      {/* Anti-Ban Jitter, Concurrency & Bandwidth */}
      <div className="p-5 rounded-2xl bg-[#0F131D] border border-white/[0.08] space-y-4">
        <div className="flex items-center gap-2">
          <Shield className="w-4 h-4 text-emerald-400" />
          <h3 className="text-xs font-semibold uppercase tracking-wider text-slate-400">
            Queue Concurrency, Anti-Ban Jitter &amp; Bandwidth Control
          </h3>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-4 gap-4">
          <div>
            <label className="block text-xs text-slate-300 mb-1">
              Max Concurrent Jobs (1–5)
            </label>
            <input
              type="number"
              min={1}
              max={5}
              value={draft.concurrency}
              onChange={(e) =>
                setDraft({
                  ...draft,
                  concurrency: Math.min(
                    5,
                    Math.max(1, Number(e.target.value) || 2)
                  ),
                })
              }
              className="w-full px-3 py-2 rounded-xl bg-[#07090E] border border-white/[0.08] text-xs font-mono text-slate-200"
            />
          </div>

          <div>
            <label className="block text-xs text-slate-300 mb-1">
              Min Jitter Delay (ms)
            </label>
            <input
              type="number"
              min={0}
              max={10000}
              step={50}
              value={draft.jitter_min_ms}
              onChange={(e) =>
                setDraft({
                  ...draft,
                  jitter_min_ms: Math.max(0, Number(e.target.value) || 0),
                })
              }
              className="w-full px-3 py-2 rounded-xl bg-[#07090E] border border-white/[0.08] text-xs font-mono text-slate-200"
            />
          </div>

          <div>
            <label className="block text-xs text-slate-300 mb-1">
              Max Jitter Delay (ms)
            </label>
            <input
              type="number"
              min={100}
              max={20000}
              step={50}
              value={draft.jitter_max_ms}
              onChange={(e) =>
                setDraft({
                  ...draft,
                  jitter_max_ms: Math.max(100, Number(e.target.value) || 600),
                })
              }
              className="w-full px-3 py-2 rounded-xl bg-[#07090E] border border-white/[0.08] text-xs font-mono text-slate-200"
            />
          </div>

          <div>
            <label className="block text-xs text-slate-300 mb-1">
              Speed Limit (KB/s, 0 = Unlimited)
            </label>
            <input
              type="number"
              min={0}
              step={500}
              value={draft.speed_limit_kbps ?? 0}
              onChange={(e) => {
                const val = Number(e.target.value) || 0;
                setDraft({
                  ...draft,
                  speed_limit_kbps: val > 0 ? val : null,
                });
              }}
              className="w-full px-3 py-2 rounded-xl bg-[#07090E] border border-white/[0.08] text-xs font-mono text-slate-200"
            />
          </div>
        </div>

        <div className="flex justify-end pt-2">
          <button
            onClick={handleSave}
            className="flex items-center gap-1.5 px-4 py-2 rounded-xl master-gradient-bg hover:opacity-95 text-xs font-semibold text-white shadow-sm"
          >
            <Save className="w-3.5 h-3.5" />
            <span>Save Settings</span>
          </button>
        </div>
      </div>

      {/* Bundled Binary Engine Verification */}
      <div className="p-5 rounded-2xl bg-[#0F131D] border border-white/[0.08] space-y-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Cpu className="w-4 h-4 text-violet-400" />
            <h3 className="text-xs font-semibold uppercase tracking-wider text-slate-400">
              Self-Contained Sidecar Binaries ({binaryStatus?.bin_dir ?? "Managed Sidecars"})
            </h3>
          </div>

          <div className="flex items-center gap-2">
            <button
              onClick={handleExportDiagnostics}
              className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-white/[0.06] hover:bg-white/[0.1] text-xs text-slate-300"
            >
              <FileArchive className="w-3.5 h-3.5 text-sky-400" />
              <span>Export Diagnostic Bundle</span>
            </button>

            <button
              onClick={handleUpdateBinaries}
              disabled={updatingBins}
              className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-violet-500/20 hover:bg-violet-500/30 border border-violet-500/35 text-xs font-medium text-violet-200"
            >
              <RefreshCw
                className={`w-3.5 h-3.5 ${updatingBins ? "animate-spin" : ""}`}
              />
              <span>
                {updatingBins ? "Updating..." : "Self-Update / Repair Binaries"}
              </span>
            </button>
          </div>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-5 gap-3">
          {[
            binaryStatus?.yt_dlp,
            binaryStatus?.gallery_dl,
            binaryStatus?.ffmpeg,
            binaryStatus?.ffprobe,
            binaryStatus?.deno,
          ].map(
            (bin, i) =>
              bin && (
                <div
                  key={bin.name || i}
                  className="p-3 rounded-xl bg-[#07090E] border border-white/[0.08] space-y-1 text-xs"
                >
                  <div className="flex items-center justify-between">
                    <span className="font-mono font-semibold text-slate-200">
                      {bin.name}
                    </span>
                    <span
                      className={`flex items-center gap-1 text-[10px] font-mono ${
                        bin.installed ? "text-emerald-400" : "text-amber-400"
                      }`}
                    >
                      <CheckCircle2 className="w-3 h-3" />
                      {bin.installed ? "Ready" : "Missing"}
                    </span>
                  </div>
                  <div className="text-[11px] font-mono text-slate-400 truncate">
                    Version: {bin.version ?? "N/A"}
                  </div>
                  <div className="text-[10px] font-mono text-slate-500 truncate">
                    Size: {formatBytes(bin.size_bytes)}
                  </div>
                  {bin.sha256 && (
                    <div
                      className="text-[10px] font-mono text-slate-500 truncate"
                      title={bin.sha256}
                    >
                      SHA256: {bin.sha256.slice(0, 16)}…
                    </div>
                  )}
                </div>
              )
          )}
        </div>
      </div>
    </div>
  );
};
