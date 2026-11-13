import React, { useState } from "react";
import { motion } from "framer-motion";
import {
  CheckCircle2,
  Cpu,
  FolderOpen,
  HardDrive,
  Loader2,
  RefreshCw,
  Shield,
  Sliders,
  Terminal,
  Trash2,
  Wrench,
  AlertTriangle,
} from "lucide-react";
import { tauriBridge } from "../services/tauriBridge";
import { useApp } from "../stores/AppContext";
import type {
  AudioFormat,
  AudioQualityPreset,
  DiagnosticLogEntry,
  VideoContainer,
  VideoQualityPreset,
} from "../types";

export const SettingsPage: React.FC = () => {
  const {
    settings,
    updateSettings,
    dependencies,
    checkDependenciesNow,
    runDependencySetup,
    checkAppUpdateNow,
    clearAllHistory,
    addToast,
  } = useApp();

  const [updatingYtdlp, setUpdatingYtdlp] = useState(false);
  const [updatingFfmpeg, setUpdatingFfmpeg] = useState(false);
  const [checkingUpdates, setCheckingUpdates] = useState(false);
  const [checkingAppUpdate, setCheckingAppUpdate] = useState(false);
  const [logs, setLogs] = useState<DiagnosticLogEntry[] | null>(null);
  const [diagnosticReportMd, setDiagnosticReportMd] = useState<string | null>(
    null
  );

  const handleCheckAppUpdate = async () => {
    setCheckingAppUpdate(true);
    try {
      const info = await checkAppUpdateNow();
      addToast({
        type: info.updateAvailable ? "info" : "success",
        title: info.updateAvailable
          ? `FlowDL ${info.latestVersion} is available`
          : "FlowDL is up to date",
        description: info.updateAvailable
          ? "Use the notification banner at the top to view release notes or update."
          : `You are running the latest version (v${info.currentVersion}).`,
      });
    } catch {
      addToast({
        type: "error",
        title: "Couldn't check for app updates",
        description: "Check your internet connection and try again.",
      });
    } finally {
      setCheckingAppUpdate(false);
    }
  };

  const handleCopyDiagnosticReport = async () => {
    try {
      const report = await tauriBridge.generateDiagnosticReport();
      setDiagnosticReportMd(report.markdownReport);
      if (navigator.clipboard && navigator.clipboard.writeText) {
        await navigator.clipboard.writeText(report.markdownReport);
      }
      addToast({
        type: "success",
        title: "Sanitized diagnostic report copied",
        description:
          "Personal paths, tokens, and cookies were automatically redacted. You can paste this into a GitHub issue.",
      });
    } catch {
      addToast({
        type: "error",
        title: "Could not generate report",
        description: "Please try viewing diagnostic logs directly.",
      });
    }
  };

  const handleBrowseDefaultDir = async () => {
    const picked = await tauriBridge.pickFolder(settings.downloadDir);
    if (picked) {
      await updateSettings({ downloadDir: picked });
    }
  };

  const handleCheckUpdates = async () => {
    setCheckingUpdates(true);
    try {
      const state = await checkDependenciesNow(true);
      addToast({
        type: "info",
        title: "Engine check complete",
        description: state.ytdlp.updateAvailable
          ? `Update available: yt-dlp ${state.ytdlp.latestVersion}`
          : "All download engines are up to date.",
      });
    } catch {
      addToast({
        type: "error",
        title: "Couldn't check for updates",
        description: "Check your internet connection and try again.",
      });
    } finally {
      setCheckingUpdates(false);
    }
  };

  const handleUpdateYtdlp = async () => {
    setUpdatingYtdlp(true);
    try {
      const updated = await tauriBridge.updateYtdlpEngine();
      await checkDependenciesNow(false);
      addToast({
        type: "success",
        title: "yt-dlp updated",
        description: `Active version is now ${updated.version}`,
      });
    } catch (e: unknown) {
      addToast({
        type: "error",
        title: "Update failed",
        description:
          "Couldn't update the download engine. Your existing working version is still available.",
      });
    } finally {
      setUpdatingYtdlp(false);
    }
  };

  const handleUpdateFfmpeg = async () => {
    setUpdatingFfmpeg(true);
    try {
      await tauriBridge.updateFfmpegEngine();
      await checkDependenciesNow(false);
      addToast({
        type: "success",
        title: "FFmpeg updated",
        description: "Managed FFmpeg & ffprobe binaries verified.",
      });
    } catch {
      addToast({
        type: "error",
        title: "Update failed",
        description:
          "Couldn't update FFmpeg. Your existing working version is still available.",
      });
    } finally {
      setUpdatingFfmpeg(false);
    }
  };

  const handleToggleLogs = async () => {
    if (logs) {
      setLogs(null);
    } else {
      const entries = await tauriBridge.getDiagnosticLogs();
      setLogs(entries);
    }
  };

  const handleClearTemp = async () => {
    await tauriBridge.clearTemporaryFiles();
    addToast({
      type: "info",
      title: "Temporary files cleared",
      description: "Cleaned FlowDL temporary workspace.",
    });
  };

  return (
    <motion.div
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.25, ease: [0.2, 0, 0, 1] }}
      className="max-w-4xl mx-auto space-y-6 pb-12"
    >
      <div>
        <h1 className="text-2xl font-bold tracking-tight text-zinc-900 dark:text-zinc-100">
          Settings
        </h1>
        <p className="text-sm text-zinc-500 dark:text-zinc-400">
          Configure default formats, queue behavior, download engines, and
          privacy.
        </p>
      </div>

      {/* 1. General Settings */}
      <section className="p-5 rounded-2xl bg-surface-card border border-border space-y-4">
        <div className="flex items-center gap-2.5 pb-2 border-b border-border">
          <Sliders className="w-4 h-4 text-indigo-500" />
          <h2 className="text-sm font-semibold text-zinc-900 dark:text-zinc-100">
            General
          </h2>
        </div>

        <div className="space-y-4">
          {/* Default Download Folder */}
          <div>
            <label className="text-xs font-medium text-zinc-700 dark:text-zinc-300 block mb-1.5">
              Default Download Folder
            </label>
            <div className="flex items-center gap-2">
              <input
                type="text"
                value={settings.downloadDir}
                onChange={(e) =>
                  updateSettings({ downloadDir: e.target.value })
                }
                className="flex-1 px-3.5 py-2 rounded-xl text-xs font-mono bg-surface-subtle border border-border text-zinc-900 dark:text-zinc-100"
              />
              <button
                type="button"
                onClick={handleBrowseDefaultDir}
                className="inline-flex items-center gap-1.5 px-3.5 py-2 rounded-xl text-xs font-medium bg-surface-subtle hover:bg-surface-hover border border-border text-zinc-700 dark:text-zinc-300 transition-colors"
              >
                <FolderOpen className="w-3.5 h-3.5 text-indigo-500" />
                Browse
              </button>
            </div>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-4 gap-3">
            <div>
              <label className="text-xs font-medium text-zinc-600 dark:text-zinc-400 block mb-1">
                Default Video Format
              </label>
              <select
                value={settings.defaultVideoContainer}
                onChange={(e) =>
                  updateSettings({
                    defaultVideoContainer: e.target.value as VideoContainer,
                  })
                }
                className="w-full px-3 py-2 rounded-xl text-xs bg-surface-subtle border border-border text-zinc-900 dark:text-zinc-100"
              >
                <option value="mp4">MP4</option>
                <option value="mkv">MKV</option>
                <option value="webm">WebM</option>
                <option value="auto">Automatic</option>
              </select>
            </div>

            <div>
              <label className="text-xs font-medium text-zinc-600 dark:text-zinc-400 block mb-1">
                Default Video Resolution
              </label>
              <select
                value={settings.defaultVideoQuality}
                onChange={(e) =>
                  updateSettings({
                    defaultVideoQuality: e.target.value as VideoQualityPreset,
                  })
                }
                className="w-full px-3 py-2 rounded-xl text-xs bg-surface-subtle border border-border text-zinc-900 dark:text-zinc-100"
              >
                <option value="best">Best available</option>
                <option value="2160">4K — 2160p</option>
                <option value="1440">1440p</option>
                <option value="1080">1080p</option>
                <option value="720">720p</option>
                <option value="480">480p</option>
              </select>
            </div>

            <div>
              <label className="text-xs font-medium text-zinc-600 dark:text-zinc-400 block mb-1">
                Default Audio Format
              </label>
              <select
                value={settings.defaultAudioFormat}
                onChange={(e) =>
                  updateSettings({
                    defaultAudioFormat: e.target.value as AudioFormat,
                  })
                }
                className="w-full px-3 py-2 rounded-xl text-xs bg-surface-subtle border border-border text-zinc-900 dark:text-zinc-100"
              >
                <option value="mp3">MP3</option>
                <option value="m4a">M4A</option>
                <option value="best">Original Best</option>
                <option value="flac">FLAC</option>
                <option value="wav">WAV</option>
                <option value="opus">OPUS</option>
              </select>
            </div>

            <div>
              <label className="text-xs font-medium text-zinc-600 dark:text-zinc-400 block mb-1">
                Default Audio Quality
              </label>
              <select
                value={settings.defaultAudioQuality}
                onChange={(e) =>
                  updateSettings({
                    defaultAudioQuality: e.target.value as AudioQualityPreset,
                  })
                }
                className="w-full px-3 py-2 rounded-xl text-xs bg-surface-subtle border border-border text-zinc-900 dark:text-zinc-100"
              >
                <option value="best">Best source</option>
                <option value="320">320 kbps</option>
                <option value="256">256 kbps</option>
                <option value="192">192 kbps</option>
                <option value="128">128 kbps</option>
              </select>
            </div>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 pt-1">
            <div>
              <label className="text-xs font-medium text-zinc-600 dark:text-zinc-400 block mb-1">
                Appearance Theme
              </label>
              <select
                value={settings.theme}
                onChange={(e) =>
                  updateSettings({
                    theme: e.target.value as "dark" | "light" | "system",
                  })
                }
                className="w-full px-3 py-2 rounded-xl text-xs bg-surface-subtle border border-border text-zinc-900 dark:text-zinc-100"
              >
                <option value="dark">Dark Mode</option>
                <option value="light">Light Mode</option>
                <option value="system">Follow Windows System</option>
              </select>
            </div>

            <label className="flex items-center gap-2.5 text-xs text-zinc-700 dark:text-zinc-300 pt-5 cursor-pointer">
              <input
                type="checkbox"
                checked={settings.launchOnStartup}
                onChange={(e) =>
                  updateSettings({ launchOnStartup: e.target.checked })
                }
                className="rounded border-zinc-400 text-indigo-600"
              />
              Launch on Windows startup
            </label>

            <label className="flex items-center gap-2.5 text-xs text-zinc-700 dark:text-zinc-300 pt-5 cursor-pointer">
              <input
                type="checkbox"
                checked={settings.minimizeToTray}
                onChange={(e) =>
                  updateSettings({ minimizeToTray: e.target.checked })
                }
                className="rounded border-zinc-400 text-indigo-600"
              />
              Minimize to system tray
            </label>
          </div>
        </div>
      </section>

      {/* 2. Downloads Behavior */}
      <section className="p-5 rounded-2xl bg-surface-card border border-border space-y-4">
        <div className="flex items-center gap-2.5 pb-2 border-b border-border">
          <HardDrive className="w-4 h-4 text-indigo-500" />
          <h2 className="text-sm font-semibold text-zinc-900 dark:text-zinc-100">
            Downloads & Queue
          </h2>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-4 gap-3">
          <div>
            <label className="text-xs font-medium text-zinc-600 dark:text-zinc-400 block mb-1">
              Simultaneous Downloads
            </label>
            <select
              value={settings.maxConcurrentDownloads}
              onChange={(e) =>
                updateSettings({
                  maxConcurrentDownloads: Number(e.target.value),
                })
              }
              className="w-full px-3 py-2 rounded-xl text-xs bg-surface-subtle border border-border text-zinc-900 dark:text-zinc-100"
            >
              <option value={1}>1 (Sequential mode)</option>
              <option value={2}>2 (Recommended default)</option>
              <option value={3}>3 simultaneous</option>
              <option value={4}>4 simultaneous</option>
            </select>
          </div>

          <div>
            <label className="text-xs font-medium text-zinc-600 dark:text-zinc-400 block mb-1">
              Filename Template
            </label>
            <input
              type="text"
              value={settings.filenameTemplate}
              onChange={(e) =>
                updateSettings({ filenameTemplate: e.target.value })
              }
              className="w-full px-3 py-2 rounded-xl text-xs font-mono bg-surface-subtle border border-border text-zinc-900 dark:text-zinc-100"
            />
          </div>

          <div>
            <label className="text-xs font-medium text-zinc-600 dark:text-zinc-400 block mb-1">
              Speed Limit (KB/s, 0 = Off)
            </label>
            <input
              type="number"
              min={0}
              value={settings.speedLimitKbps}
              onChange={(e) =>
                updateSettings({
                  speedLimitKbps: Math.max(0, Number(e.target.value) || 0),
                })
              }
              className="w-full px-3 py-2 rounded-xl text-xs font-mono bg-surface-subtle border border-border text-zinc-900 dark:text-zinc-100"
            />
          </div>

          <div>
            <label className="text-xs font-medium text-zinc-600 dark:text-zinc-400 block mb-1">
              Automatic Retry Count
            </label>
            <input
              type="number"
              min={0}
              max={10}
              value={settings.retryCount}
              onChange={(e) =>
                updateSettings({
                  retryCount: Math.max(0, Math.min(10, Number(e.target.value) || 0)),
                })
              }
              className="w-full px-3 py-2 rounded-xl text-xs font-mono bg-surface-subtle border border-border text-zinc-900 dark:text-zinc-100"
            />
          </div>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-4 gap-3 pt-1">
          <label className="flex items-center gap-2 text-xs text-zinc-700 dark:text-zinc-300 cursor-pointer">
            <input
              type="checkbox"
              checked={settings.createPlaylistFolders}
              onChange={(e) =>
                updateSettings({ createPlaylistFolders: e.target.checked })
              }
              className="rounded border-zinc-400 text-indigo-600"
            />
            Create folders for playlists
          </label>

          <label className="flex items-center gap-2 text-xs text-zinc-700 dark:text-zinc-300 cursor-pointer">
            <input
              type="checkbox"
              checked={settings.askBeforeOverwrite}
              onChange={(e) =>
                updateSettings({ askBeforeOverwrite: e.target.checked })
              }
              className="rounded border-zinc-400 text-indigo-600"
            />
            Prevent overwriting existing files
          </label>

          <label className="flex items-center gap-2 text-xs text-zinc-700 dark:text-zinc-300 cursor-pointer">
            <input
              type="checkbox"
              checked={settings.downloadSubtitlesByDefault}
              onChange={(e) =>
                updateSettings({
                  downloadSubtitlesByDefault: e.target.checked,
                })
              }
              className="rounded border-zinc-400 text-indigo-600"
            />
            Download subtitles by default
          </label>

          <label className="flex items-center gap-2 text-xs text-zinc-700 dark:text-zinc-300 cursor-pointer">
            <input
              type="checkbox"
              checked={settings.preserveIntermediateFiles}
              onChange={(e) =>
                updateSettings({
                  preserveIntermediateFiles: e.target.checked,
                })
              }
              className="rounded border-zinc-400 text-indigo-600"
            />
            Preserve intermediate streams
          </label>
        </div>
      </section>

      {/* 3. Download Engine (yt-dlp & FFmpeg) */}
      <section className="p-5 rounded-2xl bg-surface-card border border-border space-y-4">
        <div className="flex flex-wrap items-center justify-between gap-2 pb-2 border-b border-border">
          <div className="flex items-center gap-2.5">
            <Cpu className="w-4 h-4 text-indigo-500" />
            <h2 className="text-sm font-semibold text-zinc-900 dark:text-zinc-100">
              Download Engine
            </h2>
          </div>

          <div className="flex items-center gap-2">
            <button
              type="button"
              disabled={checkingUpdates}
              onClick={handleCheckUpdates}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-medium bg-surface-subtle hover:bg-surface-hover border border-border text-zinc-700 dark:text-zinc-300 transition-colors"
            >
              {checkingUpdates ? (
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
              ) : (
                <RefreshCw className="w-3.5 h-3.5" />
              )}
              Check for Updates
            </button>

            <button
              type="button"
              onClick={() => runDependencySetup(false)}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-medium bg-surface-subtle hover:bg-surface-hover border border-border text-zinc-700 dark:text-zinc-300 transition-colors"
            >
              <Wrench className="w-3.5 h-3.5 text-indigo-500" />
              Repair Engines
            </button>
          </div>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {/* yt-dlp status card */}
          <div className="p-4 rounded-xl bg-surface-subtle border border-border space-y-2.5">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <span className="text-sm font-semibold text-zinc-900 dark:text-zinc-100">
                  yt-dlp
                </span>
                {dependencies.ytdlp.functional ? (
                  <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] font-medium bg-emerald-500/15 text-emerald-600 dark:text-emerald-400">
                    <CheckCircle2 className="w-3 h-3" />
                    Healthy
                  </span>
                ) : (
                  <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] font-medium bg-amber-500/15 text-amber-600 dark:text-amber-400">
                    <AlertTriangle className="w-3 h-3" />
                    Missing
                  </span>
                )}
              </div>

              <button
                type="button"
                disabled={updatingYtdlp}
                onClick={handleUpdateYtdlp}
                className="px-3 py-1 rounded-lg text-xs font-semibold bg-indigo-600 hover:bg-indigo-500 text-white transition-colors disabled:opacity-50"
              >
                {updatingYtdlp
                  ? "Updating..."
                  : dependencies.ytdlp.updateAvailable
                    ? "Update Now"
                    : "Reinstall / Update"}
              </button>
            </div>

            <div className="text-xs text-zinc-500 dark:text-zinc-400 space-y-1 font-mono">
              <div>
                Installed: {dependencies.ytdlp.version || "Not detected"}
              </div>
              {dependencies.ytdlp.latestVersion && (
                <div>Latest release: {dependencies.ytdlp.latestVersion}</div>
              )}
              <div className="truncate" title={dependencies.ytdlp.path || ""}>
                Path: {dependencies.ytdlp.path || "—"}
              </div>
            </div>
          </div>

          {/* FFmpeg status card */}
          <div className="p-4 rounded-xl bg-surface-subtle border border-border space-y-2.5">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <span className="text-sm font-semibold text-zinc-900 dark:text-zinc-100">
                  FFmpeg & ffprobe
                </span>
                {dependencies.ffmpeg.functional ? (
                  <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] font-medium bg-emerald-500/15 text-emerald-600 dark:text-emerald-400">
                    <CheckCircle2 className="w-3 h-3" />
                    Healthy
                  </span>
                ) : (
                  <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] font-medium bg-amber-500/15 text-amber-600 dark:text-amber-400">
                    <AlertTriangle className="w-3 h-3" />
                    Missing
                  </span>
                )}
              </div>

              <button
                type="button"
                disabled={updatingFfmpeg}
                onClick={handleUpdateFfmpeg}
                className="px-3 py-1 rounded-lg text-xs font-semibold bg-indigo-600 hover:bg-indigo-500 text-white transition-colors disabled:opacity-50"
              >
                {updatingFfmpeg ? "Updating..." : "Reinstall / Update"}
              </button>
            </div>

            <div className="text-xs text-zinc-500 dark:text-zinc-400 space-y-1 font-mono">
              <div>
                Installed: {dependencies.ffmpeg.version || "Not detected"}
              </div>
              <div>
                ffprobe:{" "}
                {dependencies.ffprobe.functional ? "Verified" : "Not detected"}
              </div>
              <div className="truncate" title={dependencies.ffmpeg.path || ""}>
                Path: {dependencies.ffmpeg.path || "—"}
              </div>
            </div>
          </div>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 pt-1">
          <div>
            <label className="text-xs font-medium text-zinc-600 dark:text-zinc-400 block mb-1">
              yt-dlp Update Channel
            </label>
            <select
              value={settings.ytdlpChannel || "stable"}
              onChange={(e) =>
                updateSettings({ ytdlpChannel: e.target.value })
              }
              className="w-full px-3 py-2 rounded-xl text-xs bg-surface-subtle border border-border text-zinc-900 dark:text-zinc-100"
            >
              <option value="stable">Stable Releases (Recommended)</option>
              <option value="nightly">Nightly Builds (Advanced — Experimental)</option>
            </select>
            {settings.ytdlpChannel === "nightly" && (
              <p className="mt-1 text-[11px] text-amber-600 dark:text-amber-400">
                Warning: Nightly builds include bleeding-edge extractor fixes but may occasionally contain regressions.
              </p>
            )}
          </div>

          <div className="flex flex-col justify-end gap-2">
            <label className="flex items-center gap-2 text-xs text-zinc-700 dark:text-zinc-300 cursor-pointer">
              <input
                type="checkbox"
                checked={settings.autoUpdateEngines}
                onChange={(e) =>
                  updateSettings({ autoUpdateEngines: e.target.checked })
                }
                className="rounded border-zinc-400 text-indigo-600"
              />
              Automatically update download engines in the background
            </label>

            <label className="flex items-center gap-2 text-xs text-zinc-700 dark:text-zinc-300 cursor-pointer">
              <input
                type="checkbox"
                checked={settings.checkAppUpdatesOnStartup !== false}
                onChange={(e) =>
                  updateSettings({
                    checkAppUpdatesOnStartup: e.target.checked,
                  })
                }
                className="rounded border-zinc-400 text-indigo-600"
              />
              Check for FlowDL application updates on startup
            </label>
          </div>
        </div>

        <div className="flex flex-wrap items-center justify-between gap-3 pt-1 border-t border-border">
          <div className="flex items-center gap-3">
            <button
              type="button"
              disabled={checkingAppUpdate}
              onClick={handleCheckAppUpdate}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-medium bg-surface-subtle hover:bg-surface-hover border border-border text-zinc-700 dark:text-zinc-300 transition-colors"
            >
              {checkingAppUpdate ? (
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
              ) : (
                <RefreshCw className="w-3.5 h-3.5" />
              )}
              Check for App Updates
            </button>

            <button
              type="button"
              onClick={handleCopyDiagnosticReport}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-medium bg-surface-subtle hover:bg-surface-hover border border-border text-zinc-700 dark:text-zinc-300 transition-colors"
            >
              <Terminal className="w-3.5 h-3.5 text-indigo-500" />
              Copy Sanitized Diagnostic Report
            </button>
          </div>

          <button
            type="button"
            onClick={handleToggleLogs}
            className="inline-flex items-center gap-1.5 text-xs font-medium text-indigo-600 dark:text-indigo-400 hover:underline"
          >
            <Terminal className="w-3.5 h-3.5" />
            {logs ? "Hide Diagnostic Logs" : "View Diagnostic Logs"}
          </button>
        </div>

        {diagnosticReportMd && (
          <div className="p-3 rounded-xl bg-zinc-950 text-zinc-300 font-mono text-xs max-h-40 overflow-y-auto border border-zinc-800 whitespace-pre-wrap">
            {diagnosticReportMd}
          </div>
        )}

        {logs && (
          <div className="p-3 rounded-xl bg-zinc-950 text-zinc-300 font-mono text-xs max-h-48 overflow-y-auto space-y-1 border border-zinc-800">
            {logs.length === 0 ? (
              <div className="text-zinc-500">No diagnostic log entries.</div>
            ) : (
              logs.map((entry, idx) => (
                <div key={idx} className="leading-relaxed">
                  <span className="text-zinc-500">[{entry.timestamp}]</span>{" "}
                  <span
                    className={
                      entry.level === "ERROR"
                        ? "text-rose-400 font-semibold"
                        : entry.level === "WARN"
                          ? "text-amber-400"
                          : "text-indigo-400"
                    }
                  >
                    [{entry.level}]
                  </span>{" "}
                  <span className="text-zinc-400">[{entry.component}]</span>{" "}
                  {entry.message}
                </div>
              ))
            )}
          </div>
        )}
      </section>

      {/* 4. Privacy */}
      <section className="p-5 rounded-2xl bg-surface-card border border-border space-y-4">
        <div className="flex items-center gap-2.5 pb-2 border-b border-border">
          <Shield className="w-4 h-4 text-indigo-500" />
          <h2 className="text-sm font-semibold text-zinc-900 dark:text-zinc-100">
            Privacy & Local Data
          </h2>
        </div>

        <p className="text-xs text-zinc-500 dark:text-zinc-400">
          FlowDL operates 100% locally on your Windows machine. There is no
          cloud account, no telemetry, and your download history never leaves
          your computer.
        </p>

        <div className="flex flex-wrap items-center justify-between gap-4">
          <label className="flex items-center gap-2 text-xs text-zinc-700 dark:text-zinc-300 cursor-pointer">
            <input
              type="checkbox"
              checked={settings.clipboardDetectionEnabled}
              onChange={(e) =>
                updateSettings({ clipboardDetectionEnabled: e.target.checked })
              }
              className="rounded border-zinc-400 text-indigo-600"
            />
            Detect supported links on clipboard when FlowDL window gains focus
          </label>

          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={handleClearTemp}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-medium bg-surface-subtle hover:bg-surface-hover border border-border text-zinc-700 dark:text-zinc-300 transition-colors"
            >
              <Trash2 className="w-3.5 h-3.5" />
              Clear Temporary Files
            </button>
            <button
              type="button"
              onClick={clearAllHistory}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-medium text-rose-600 dark:text-rose-400 bg-rose-500/10 hover:bg-rose-500/20 border border-rose-500/20 transition-colors"
            >
              <Trash2 className="w-3.5 h-3.5" />
              Clear Local History
            </button>
          </div>
        </div>
      </section>
    </motion.div>
  );
};
