import React, { useState, useEffect } from "react";
import {
  Folder,
  Save,
  RefreshCw,
  Shield,
  Sliders,
  Terminal,
  CheckCircle2,
  Loader2,
  Sun,
  Moon,
} from "lucide-react";
import { useSettingsStore } from "../../stores/useSettingsStore";
import { useUIStore } from "../../stores/useUIStore";
import { ipc, parseSerializedError } from "../../lib/ipc";
import type { AppSettings, DiagnosticLogEntry } from "../../types";

const NAMING_TOKENS = [
  "{uploader}",
  "{date}",
  "{id}",
  "{type}",
  "{ext}",
  "{title}",
];

export const SettingsView: React.FC = () => {
  const { settings, setSettings, binaryStatus, setBinaryStatus } =
    useSettingsStore();
  const { addToast, setAuthModalOpen, setBootstrapModalOpen } = useUIStore();

  const [draft, setDraft] = useState<AppSettings | null>(settings);
  const [logs, setLogs] = useState<DiagnosticLogEntry[]>([]);
  const [isSaving, setIsSaving] = useState(false);
  const [isUpdatingEngines, setIsUpdatingEngines] = useState(false);

  useEffect(() => {
    if (settings) setDraft(settings);
  }, [settings]);

  useEffect(() => {
    ipc
      .getDiagnosticLogs()
      .then(setLogs)
      .catch(() => {});
  }, []);

  if (!draft) return null;

  const handleBrowseFolder = async () => {
    try {
      const selected = await ipc.pickDirectoryDialog(draft.output_dir || null);
      if (typeof selected === "string" && selected.trim()) {
        setDraft({ ...draft, output_dir: selected });
      }
    } catch {
      // ignore dialog cancel
    }
  };

  const handleSave = async () => {
    setIsSaving(true);
    try {
      const updated = await ipc.updateSettings(draft);
      setSettings(updated);
      addToast({
        type: "success",
        title: "Settings Saved",
        message: "Your preferences have been persisted to SQLite.",
      });
    } catch (err) {
      addToast({
        type: "error",
        title: "Failed to Save Settings",
        message: parseSerializedError(err).user_message,
      });
    } finally {
      setIsSaving(false);
    }
  };

  const handleUpdateEngines = async () => {
    setIsUpdatingEngines(true);
    try {
      const report = await ipc.updateBinary("all");
      setBinaryStatus(report);
      addToast({
        type: "success",
        title: "Engines Updated",
        message: `yt-dlp (${report.yt_dlp.version || "latest"}) & gallery-dl (${
          report.gallery_dl.version || "latest"
        }) verified.`,
      });
    } catch (err) {
      addToast({
        type: "error",
        title: "Engine Update Failed",
        message: parseSerializedError(err).user_message,
      });
    } finally {
      setIsUpdatingEngines(false);
    }
  };

  return (
    <div className="space-y-6 pb-8">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-base font-semibold text-text-primary">
            Application Preferences
          </h2>
          <p className="text-xs text-text-secondary">
            Configure output paths, DASH quality defaults, rate-limit protection, and bundled engines
          </p>
        </div>

        <button
          onClick={handleSave}
          disabled={isSaving}
          className="flex items-center gap-2 px-4 py-2 rounded-xl text-xs font-semibold text-white fb-gradient-bg hover:opacity-95 disabled:opacity-40 transition-all shadow-sm"
        >
          {isSaving ? (
            <Loader2 className="w-3.5 h-3.5 animate-spin" />
          ) : (
            <Save className="w-3.5 h-3.5" />
          )}
          <span>Save Changes</span>
        </button>
      </div>

      {/* Section 1: Storage & Naming Template */}
      <div className="p-5 rounded-2xl bg-bg-surface border border-border-subtle space-y-4">
        <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-text-secondary">
          <Folder className="w-4 h-4 text-accent-primary" />
          <span>Storage & File Naming Template</span>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div className="space-y-1.5">
            <label className="text-xs font-medium text-text-primary">
              Download Directory
            </label>
            <div className="flex items-center gap-2">
              <input
                type="text"
                value={draft.output_dir}
                onChange={(e) =>
                  setDraft({ ...draft, output_dir: e.target.value })
                }
                className="flex-1 h-9 px-3 rounded-lg bg-bg-canvas border border-border-subtle text-xs font-mono text-text-primary focus:outline-none focus:border-border-focus"
              />
              <button
                onClick={handleBrowseFolder}
                className="h-9 px-3 rounded-lg bg-bg-elevated hover:bg-bg-canvas border border-border-subtle text-xs font-medium text-text-primary transition-colors"
              >
                Browse
              </button>
            </div>
          </div>

          <div className="space-y-1.5">
            <label className="text-xs font-medium text-text-primary">
              Default Quality Preset
            </label>
            <select
              value={draft.default_quality}
              onChange={(e) =>
                setDraft({ ...draft, default_quality: e.target.value })
              }
              className="w-full h-9 px-3 rounded-lg bg-bg-canvas border border-border-subtle text-xs text-text-primary focus:outline-none focus:border-border-focus"
            >
              <option value="best">Max Available (4K / 1080p60 DASH)</option>
              <option value="4k">4K Ultra HD (2160p)</option>
              <option value="1440p">2K QHD (1440p)</option>
              <option value="1080p">1080p Full HD</option>
              <option value="720p">720p HD</option>
              <option value="480p">480p SD</option>
              <option value="audio_mp3">Audio Only — MP3 (320kbps)</option>
              <option value="audio_m4a">Audio Only — M4A (AAC)</option>
            </select>
          </div>
        </div>

        <div className="space-y-1.5">
          <label className="text-xs font-medium text-text-primary">
            File Naming Template
          </label>
          <input
            type="text"
            value={draft.naming_template}
            onChange={(e) =>
              setDraft({ ...draft, naming_template: e.target.value })
            }
            className="w-full h-9 px-3 rounded-lg bg-bg-canvas border border-border-subtle text-xs font-mono text-text-primary focus:outline-none focus:border-border-focus"
          />
          <div className="flex items-center gap-1.5 flex-wrap pt-1">
            <span className="text-[11px] text-text-muted">Insert token:</span>
            {NAMING_TOKENS.map((tok) => (
              <button
                key={tok}
                type="button"
                onClick={() =>
                  setDraft({
                    ...draft,
                    naming_template: `${draft.naming_template}${tok}`,
                  })
                }
                className="px-2 py-0.5 rounded bg-bg-elevated hover:bg-bg-canvas border border-border-subtle text-[10px] font-mono text-text-secondary hover:text-text-primary"
              >
                {tok}
              </button>
            ))}
          </div>
        </div>

        <div className="flex flex-wrap items-center gap-5 pt-2">
          <label className="flex items-center gap-2 text-xs text-text-secondary cursor-pointer">
            <input
              type="checkbox"
              checked={draft.embed_thumbnail_by_default}
              onChange={(e) =>
                setDraft({
                  ...draft,
                  embed_thumbnail_by_default: e.target.checked,
                })
              }
              className="accent-accent-primary"
            />
            <span>Embed cover thumbnail in MP4/MP3</span>
          </label>

          <label className="flex items-center gap-2 text-xs text-text-secondary cursor-pointer">
            <input
              type="checkbox"
              checked={draft.save_caption_by_default}
              onChange={(e) =>
                setDraft({
                  ...draft,
                  save_caption_by_default: e.target.checked,
                })
              }
              className="accent-accent-primary"
            />
            <span>Save post caption (.txt)</span>
          </label>

          <label className="flex items-center gap-2 text-xs text-text-secondary cursor-pointer">
            <input
              type="checkbox"
              checked={draft.save_metadata_by_default}
              onChange={(e) =>
                setDraft({
                  ...draft,
                  save_metadata_by_default: e.target.checked,
                })
              }
              className="accent-accent-primary"
            />
            <span>Save raw JSON metadata (.info.json)</span>
          </label>

          <label className="flex items-center gap-2 text-xs text-text-secondary cursor-pointer">
            <input
              type="checkbox"
              checked={draft.clipboard_monitor}
              onChange={(e) =>
                setDraft({
                  ...draft,
                  clipboard_monitor: e.target.checked,
                })
              }
              className="accent-accent-primary"
            />
            <span>Smart Clipboard Auto-Detection</span>
          </label>
        </div>
      </div>

      {/* Section 2: Appearance & Concurrency */}
      <div className="p-5 rounded-2xl bg-bg-surface border border-border-subtle space-y-4">
        <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-text-secondary">
          <Sliders className="w-4 h-4 text-accent-secondary" />
          <span>Queue Concurrency, Jitter & Appearance</span>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div className="space-y-1.5">
            <label className="text-xs font-medium text-text-primary">
              Concurrent Active Downloads (1 – 5)
            </label>
            <input
              type="number"
              min={1}
              max={5}
              value={draft.concurrency}
              onChange={(e) =>
                setDraft({
                  ...draft,
                  concurrency: Math.min(5, Math.max(1, Number(e.target.value))),
                })
              }
              className="w-full h-9 px-3 rounded-lg bg-bg-canvas border border-border-subtle text-xs font-mono text-text-primary"
            />
            <p className="text-[11px] text-text-muted">
              2–3 recommended to prevent Facebook HTTP 429 rate limits
            </p>
          </div>

          <div className="space-y-1.5">
            <label className="text-xs font-medium text-text-primary">
              Request Jitter Window (ms)
            </label>
            <div className="flex items-center gap-2">
              <input
                type="number"
                min={250}
                max={10000}
                value={draft.jitter_min_ms}
                onChange={(e) =>
                  setDraft({ ...draft, jitter_min_ms: Number(e.target.value) })
                }
                className="w-full h-9 px-2.5 rounded-lg bg-bg-canvas border border-border-subtle text-xs font-mono text-text-primary"
              />
              <span className="text-xs text-text-muted">to</span>
              <input
                type="number"
                min={500}
                max={20000}
                value={draft.jitter_max_ms}
                onChange={(e) =>
                  setDraft({ ...draft, jitter_max_ms: Number(e.target.value) })
                }
                className="w-full h-9 px-2.5 rounded-lg bg-bg-canvas border border-border-subtle text-xs font-mono text-text-primary"
              />
            </div>
          </div>

          <div className="space-y-1.5">
            <label className="text-xs font-medium text-text-primary">
              Theme Mode
            </label>
            <div className="flex items-center gap-2">
              <button
                type="button"
                onClick={() => setDraft({ ...draft, theme: "dark" })}
                className={`flex-1 h-9 rounded-lg border text-xs font-medium flex items-center justify-center gap-1.5 transition-colors ${
                  draft.theme === "dark"
                    ? "bg-bg-elevated border-accent-primary text-text-primary"
                    : "bg-bg-canvas border-border-subtle text-text-secondary"
                }`}
              >
                <Moon className="w-3.5 h-3.5" />
                <span>Dark Obsidian</span>
              </button>
              <button
                type="button"
                onClick={() => setDraft({ ...draft, theme: "light" })}
                className={`flex-1 h-9 rounded-lg border text-xs font-medium flex items-center justify-center gap-1.5 transition-colors ${
                  draft.theme === "light"
                    ? "bg-bg-elevated border-accent-primary text-text-primary"
                    : "bg-bg-canvas border-border-subtle text-text-secondary"
                }`}
              >
                <Sun className="w-3.5 h-3.5" />
                <span>Alabaster Light</span>
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* Section 3: Engine Health & Auth Vault */}
      <div className="p-5 rounded-2xl bg-bg-surface border border-border-subtle space-y-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-text-secondary">
            <Shield className="w-4 h-4 text-status-success" />
            <span>Managed Media Engines & Auth Vault</span>
          </div>

          <div className="flex items-center gap-2">
            <button
              onClick={() => setAuthModalOpen(true)}
              className="px-3 py-1.5 rounded-lg bg-bg-elevated hover:bg-bg-canvas border border-border-subtle text-xs font-medium text-text-primary transition-colors"
            >
              Open Auth Vault
            </button>
            <button
              onClick={() => setBootstrapModalOpen(true)}
              className="px-3 py-1.5 rounded-lg bg-bg-elevated hover:bg-bg-canvas border border-border-subtle text-xs font-medium text-text-primary transition-colors"
            >
              Inspect Binaries
            </button>
            <button
              onClick={handleUpdateEngines}
              disabled={isUpdatingEngines}
              className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-accent-primary/15 text-accent-primary hover:bg-accent-primary/25 text-xs font-medium transition-colors"
            >
              <RefreshCw
                className={`w-3.5 h-3.5 ${
                  isUpdatingEngines ? "animate-spin" : ""
                }`}
              />
              <span>Update yt-dlp & gallery-dl</span>
            </button>
          </div>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
          {[
            { label: "yt-dlp", info: binaryStatus?.yt_dlp },
            { label: "gallery-dl", info: binaryStatus?.gallery_dl },
            { label: "ffmpeg", info: binaryStatus?.ffmpeg },
          ].map((b) => (
            <div
              key={b.label}
              className="p-3 rounded-xl bg-bg-canvas border border-border-subtle flex items-center justify-between"
            >
              <div>
                <div className="text-xs font-semibold text-text-primary">
                  {b.label}
                </div>
                <div className="text-[11px] font-mono text-text-muted truncate max-w-[180px]">
                  {b.info?.version || "Bundled"}
                </div>
              </div>
              <CheckCircle2
                className={`w-4 h-4 ${
                  b.info?.installed ? "text-status-success" : "text-text-muted"
                }`}
              />
            </div>
          ))}
        </div>
      </div>

      {/* Section 4: Diagnostic Logs */}
      <div className="p-5 rounded-2xl bg-bg-surface border border-border-subtle space-y-3">
        <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-text-secondary">
          <Terminal className="w-4 h-4 text-text-muted" />
          <span>Sanitized Runtime Diagnostics</span>
        </div>
        <div className="rounded-xl bg-bg-canvas border border-border-subtle p-3 max-h-40 overflow-y-auto font-mono text-[11px] text-text-secondary space-y-1">
          {logs.map((l, i) => (
            <div key={i}>
              <span className="text-text-muted">[{l.timestamp}]</span>{" "}
              <span className="text-accent-primary">[{l.component}]</span>{" "}
              <span>{l.message}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};
