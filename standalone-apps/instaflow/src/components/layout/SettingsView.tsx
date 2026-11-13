import React, { useEffect, useState } from "react";
import {
  CheckCircle2,
  Copy,
  Cpu,
  FolderOpen,
  KeyRound,
  RefreshCw,
  Save,
  Shield,
  Terminal,
} from "lucide-react";
import { ipc, isTauriRuntime, parseIpcError } from "../../lib/ipc";
import { useSettingsStore } from "../../stores/useSettingsStore";
import { useUIStore } from "../../stores/useUIStore";
import type { AppSettings, DiagnosticLogEntry } from "../../types";

const TEMPLATE_TOKENS = [
  "{username}",
  "{date}",
  "{shortcode}",
  "{index}",
  "{ext}",
];

export const SettingsView: React.FC = () => {
  const settings = useSettingsStore((s) => s.settings);
  const setSettings = useSettingsStore((s) => s.setSettings);
  const authStatus = useSettingsStore((s) => s.authStatus);
  const binaryStatus = useSettingsStore((s) => s.binaryStatus);

  const setAuthModalOpen = useUIStore((s) => s.setAuthModalOpen);
  const setOnboardingModalOpen = useUIStore((s) => s.setOnboardingModalOpen);
  const addToast = useUIStore((s) => s.addToast);

  const [draft, setDraft] = useState<AppSettings>(() => ({
    output_dir: settings?.output_dir ?? "C:\\Users\\Downloads\\InstaFlow",
    naming_template:
      settings?.naming_template ??
      "{username}/{date}_{shortcode}_{index}.{ext}",
    default_quality: settings?.default_quality ?? "best",
    concurrency: settings?.concurrency ?? 2,
    jitter_min_ms: settings?.jitter_min_ms ?? 2000,
    jitter_max_ms: settings?.jitter_max_ms ?? 4500,
    proxy_url: settings?.proxy_url ?? null,
    clipboard_monitor: settings?.clipboard_monitor ?? true,
    auto_update_engines: settings?.auto_update_engines ?? true,
    save_caption_by_default: settings?.save_caption_by_default ?? false,
    save_metadata_by_default: settings?.save_metadata_by_default ?? false,
    embed_thumbnail_by_default: settings?.embed_thumbnail_by_default ?? true,
    auth_mode: settings?.auth_mode ?? "anonymous",
    selected_browser: settings?.selected_browser ?? null,
    connected_username: settings?.connected_username ?? null,
    theme: settings?.theme ?? "dark",
  }));

  const [logs, setLogs] = useState<DiagnosticLogEntry[]>([]);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (settings) {
      setDraft(settings);
    }
  }, [settings]);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    ipc
      .getDiagnosticLogs()
      .then(setLogs)
      .catch(() => {});
  }, []);

  const handleSave = async () => {
    setSaving(true);
    try {
      const saved = await ipc.updateAppSettings(draft);
      setSettings(saved);
      addToast({
        title: "Settings Saved",
        description: "Storage, rate-limit hygiene, and template preferences updated.",
        variant: "success",
      });
    } catch (err) {
      addToast({
        title: "Failed to Save Settings",
        description: parseIpcError(err).user_message,
        variant: "error",
      });
    } finally {
      setSaving(false);
    }
  };

  const handleBrowseFolder = async () => {
    try {
      const picked = await ipc.pickDirectoryDialog(draft.output_dir);
      if (picked) {
        setDraft((prev) => ({ ...prev, output_dir: picked }));
      }
    } catch {
      // ignore
    }
  };

  const handleCopyDiagnosticBundle = async () => {
    try {
      const bundleJson = await ipc.exportDiagnosticBundle();
      await navigator.clipboard.writeText(bundleJson);
      addToast({
        title: "Sanitized Diagnostic Bundle Copied",
        description:
          "All session tokens and user paths were scrubbed. Ready to paste into a GitHub Issue.",
        variant: "success",
      });
    } catch (err) {
      addToast({
        title: "Diagnostic Export Failed",
        description: parseIpcError(err).user_message,
        variant: "error",
      });
    }
  };

  return (
    <div className="h-full overflow-y-auto pr-1 space-y-5 pb-8">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-base font-semibold text-zinc-100">
            Settings, Rate-Limit Hygiene &amp; Diagnostics
          </h1>
          <p className="text-xs text-zinc-400">
            Configure storage templates, anti-ban pacing, standalone engines, and sanitized logs
          </p>
        </div>

        <button
          onClick={handleSave}
          disabled={saving}
          className="inline-flex items-center gap-2 px-4 py-2 rounded-xl text-xs font-semibold text-white bg-gradient-to-r from-indigo-500 to-violet-600 hover:from-indigo-400 hover:to-violet-500 shadow-lg shadow-indigo-500/20"
        >
          <Save className="w-3.5 h-3.5" />
          <span>{saving ? "Saving..." : "Save Changes"}</span>
        </button>
      </div>

      {/* 1. Storage & Naming Templates */}
      <section className="p-5 rounded-2xl bg-[#121215] border border-white/[0.07] space-y-4">
        <div className="flex items-center gap-2 text-xs font-semibold text-zinc-100">
          <FolderOpen className="w-4 h-4 text-indigo-400" />
          <span>Storage &amp; Filename Template Builder</span>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div>
            <label className="block text-[11px] font-medium text-zinc-400 mb-1">
              Default Download Directory
            </label>
            <div className="flex items-center gap-2">
              <input
                type="text"
                value={draft.output_dir}
                onChange={(e) =>
                  setDraft((prev) => ({ ...prev, output_dir: e.target.value }))
                }
                className="flex-1 px-3 py-2 rounded-lg bg-[#09090B] border border-white/10 text-xs font-mono text-zinc-100 focus:outline-none focus:border-indigo-500"
              />
              <button
                onClick={handleBrowseFolder}
                className="px-3 py-2 rounded-lg bg-white/[0.06] hover:bg-white/[0.12] text-xs font-medium text-zinc-200 border border-white/10"
              >
                Browse
              </button>
            </div>
          </div>

          <div>
            <label className="block text-[11px] font-medium text-zinc-400 mb-1">
              Interactive Naming Template
            </label>
            <input
              type="text"
              value={draft.naming_template}
              onChange={(e) =>
                setDraft((prev) => ({
                  ...prev,
                  naming_template: e.target.value,
                }))
              }
              className="w-full px-3 py-2 rounded-lg bg-[#09090B] border border-white/10 text-xs font-mono text-zinc-100 focus:outline-none focus:border-indigo-500"
            />
            <div className="flex flex-wrap items-center gap-1.5 mt-2">
              <span className="text-[10px] text-zinc-500">Insert token:</span>
              {TEMPLATE_TOKENS.map((tok) => (
                <button
                  key={tok}
                  type="button"
                  onClick={() =>
                    setDraft((prev) => ({
                      ...prev,
                      naming_template: `${prev.naming_template}${tok}`,
                    }))
                  }
                  className="px-2 py-0.5 rounded bg-indigo-500/15 hover:bg-indigo-500/25 text-indigo-300 text-[10px] font-mono border border-indigo-500/25"
                >
                  {tok}
                </button>
              ))}
            </div>
          </div>
        </div>
      </section>

      {/* 2. Network & Anti-Ban Rate Limiting */}
      <section className="p-5 rounded-2xl bg-[#121215] border border-white/[0.07] space-y-4">
        <div className="flex items-center gap-2 text-xs font-semibold text-zinc-100">
          <Shield className="w-4 h-4 text-emerald-400" />
          <span>Network, Concurrency &amp; Anti-Ban Jitter Pacing</span>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div>
            <label className="block text-[11px] font-medium text-zinc-400 mb-1">
              Max Concurrent Downloads ({draft.concurrency})
            </label>
            <input
              type="range"
              min={1}
              max={3}
              value={draft.concurrency}
              onChange={(e) =>
                setDraft((prev) => ({
                  ...prev,
                  concurrency: parseInt(e.target.value, 10),
                }))
              }
              className="w-full accent-indigo-500"
            />
            <p className="text-[10px] text-zinc-500 mt-1">
              Capped at 1–3 workers to prevent Instagram IP rate-limiting.
            </p>
          </div>

          <div>
            <label className="block text-[11px] font-medium text-zinc-400 mb-1">
              Randomized Jitter Range (ms)
            </label>
            <div className="flex items-center gap-2">
              <input
                type="number"
                min={500}
                max={10000}
                step={250}
                value={draft.jitter_min_ms}
                onChange={(e) =>
                  setDraft((prev) => ({
                    ...prev,
                    jitter_min_ms: parseInt(e.target.value, 10) || 2000,
                  }))
                }
                className="w-full px-2.5 py-1.5 rounded-lg bg-[#09090B] border border-white/10 text-xs font-mono text-zinc-100"
              />
              <span className="text-xs text-zinc-500">to</span>
              <input
                type="number"
                min={1000}
                max={15000}
                step={250}
                value={draft.jitter_max_ms}
                onChange={(e) =>
                  setDraft((prev) => ({
                    ...prev,
                    jitter_max_ms: parseInt(e.target.value, 10) || 5000,
                  }))
                }
                className="w-full px-2.5 py-1.5 rounded-lg bg-[#09090B] border border-white/10 text-xs font-mono text-zinc-100"
              />
            </div>
          </div>

          <div>
            <label className="block text-[11px] font-medium text-zinc-400 mb-1">
              HTTP / SOCKS5 Proxy (Optional)
            </label>
            <input
              type="text"
              value={draft.proxy_url ?? ""}
              onChange={(e) =>
                setDraft((prev) => ({
                  ...prev,
                  proxy_url: e.target.value.trim() || null,
                }))
              }
              placeholder="socks5://127.0.0.1:1080"
              className="w-full px-3 py-1.5 rounded-lg bg-[#09090B] border border-white/10 text-xs font-mono text-zinc-100 placeholder-zinc-600"
            />
          </div>
        </div>
      </section>

      {/* 3. Authentication & Standalone Engine Maintenance */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <section className="p-5 rounded-2xl bg-[#121215] border border-white/[0.07] space-y-3">
          <div className="flex items-center justify-between">
            <span className="flex items-center gap-2 text-xs font-semibold text-zinc-100">
              <KeyRound className="w-4 h-4 text-indigo-400" />
              Authentication &amp; Privacy Vault
            </span>
            <span className="text-[11px] font-mono text-zinc-400">
              {authStatus?.authenticated
                ? `Connected (${authStatus.username ?? authStatus.mode})`
                : "Anonymous Mode"}
            </span>
          </div>
          <p className="text-xs text-zinc-400">
            Session cookies are encrypted on disk with AES-256-GCM and unlocked via Windows
            Credential Manager.
          </p>
          <button
            onClick={() => setAuthModalOpen(true)}
            className="px-3.5 py-2 rounded-lg text-xs font-medium bg-indigo-500/20 text-indigo-200 hover:bg-indigo-500/30 border border-indigo-500/30"
          >
            Manage Session Vault &amp; Cookies
          </button>
        </section>

        <section className="p-5 rounded-2xl bg-[#121215] border border-white/[0.07] space-y-3">
          <div className="flex items-center justify-between">
            <span className="flex items-center gap-2 text-xs font-semibold text-zinc-100">
              <Cpu className="w-4 h-4 text-emerald-400" />
              Standalone Media Engines
            </span>
            <span className="inline-flex items-center gap-1 text-[11px] text-emerald-400">
              <CheckCircle2 className="w-3.5 h-3.5" />
              {binaryStatus?.all_ready ? "All Verified" : "Action Needed"}
            </span>
          </div>
          <div className="text-[11px] font-mono text-zinc-400 space-y-1">
            <div>yt-dlp: {binaryStatus?.yt_dlp.version ?? "Not installed"}</div>
            <div>gallery-dl: {binaryStatus?.gallery_dl.version ?? "Not installed"}</div>
            <div>ffmpeg: {binaryStatus?.ffmpeg.version ?? "Not installed"}</div>
          </div>
          <button
            onClick={() => setOnboardingModalOpen(true)}
            className="inline-flex items-center gap-1.5 px-3.5 py-2 rounded-lg text-xs font-medium bg-white/[0.06] hover:bg-white/[0.12] text-zinc-200 border border-white/10"
          >
            <RefreshCw className="w-3.5 h-3.5" />
            Check for Engine Updates Now
          </button>
        </section>
      </div>

      {/* 4. Diagnostic Console */}
      <section className="p-5 rounded-2xl bg-[#121215] border border-white/[0.07] space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2 text-xs font-semibold text-zinc-100">
            <Terminal className="w-4 h-4 text-violet-400" />
            <span>Sanitized Diagnostic Console (PII &amp; Cookies Auto-Redacted)</span>
          </div>

          <button
            onClick={handleCopyDiagnosticBundle}
            className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium bg-indigo-500/15 hover:bg-indigo-500/25 text-indigo-300 border border-indigo-500/30 transition-colors"
          >
            <Copy className="w-3.5 h-3.5" />
            Copy Diagnostic Bundle for GitHub Issue
          </button>
        </div>

        <div className="h-44 rounded-xl bg-[#09090B] border border-white/[0.07] p-3 overflow-y-auto font-mono text-[11px] space-y-1">
          {logs.length === 0 ? (
            <div className="text-zinc-500">
              No diagnostic events recorded in this session yet.
            </div>
          ) : (
            logs.map((entry, idx) => (
              <div key={idx} className="text-zinc-300 leading-relaxed">
                <span className="text-zinc-500">[{entry.timestamp}]</span>{" "}
                <span
                  className={
                    entry.level === "ERROR"
                      ? "text-rose-400 font-semibold"
                      : "text-indigo-400"
                  }
                >
                  [{entry.level}]
                </span>{" "}
                <span className="text-zinc-400">[{entry.component}]</span>{" "}
                <span>{entry.message}</span>
              </div>
            ))
          )}
        </div>
      </section>
    </div>
  );
};
