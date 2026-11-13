import React, { useCallback } from "react";
import { X } from "lucide-react";
import { AuthVaultModal } from "./components/auth/AuthVaultModal";
import { HistoryLibraryView } from "./components/history/HistoryLibraryView";
import { MediaInspectorCard } from "./components/inspector/MediaInspectorCard";
import { CommandPalette } from "./components/layout/CommandPalette";
import { SettingsView } from "./components/layout/SettingsView";
import { Sidebar } from "./components/layout/Sidebar";
import { StatusFooter } from "./components/layout/StatusFooter";
import { Titlebar } from "./components/layout/Titlebar";
import { BatchUrlModal } from "./components/omnibar/BatchUrlModal";
import { SmartOmnibar } from "./components/omnibar/SmartOmnibar";
import { BinaryBootstrapModal } from "./components/onboarding/BinaryBootstrapModal";
import { ActiveQueueView } from "./components/queue/ActiveQueueView";
import { useClipboardMonitor } from "./hooks/useClipboardMonitor";
import { useKeyboardShortcuts } from "./hooks/useKeyboardShortcuts";
import { useTauriEvents } from "./hooks/useTauriEvents";
import { ipc, parseIpcError } from "./lib/ipc";
import { classifyMediaUrlClient } from "./lib/utils";
import { useQueueStore } from "./stores/useQueueStore";
import { useUIStore } from "./stores/useUIStore";

export const App: React.FC = () => {
  const activeView = useUIStore((s) => s.activeView);
  const setIsAnalyzing = useUIStore((s) => s.setIsAnalyzing);
  const setAnalyzeError = useUIStore((s) => s.setAnalyzeError);
  const setInspectedMedia = useUIStore((s) => s.setInspectedMedia);
  const setOmnibarUrl = useUIStore((s) => s.setOmnibarUrl);
  const toasts = useUIStore((s) => s.toasts);
  const dismissToast = useUIStore((s) => s.dismissToast);
  const addToast = useUIStore((s) => s.addToast);

  const upsertJob = useQueueStore((s) => s.upsertJob);

  useTauriEvents();

  const handleAnalyzeUrl = useCallback(
    async (rawUrl: string) => {
      if (!rawUrl.trim()) return;
      setIsAnalyzing(true);
      setAnalyzeError(null);
      try {
        const normalized = await ipc.validateAndNormalizeUrl(rawUrl);
        const result = await ipc.analyzeMediaUrl(rawUrl);
        setInspectedMedia(result, normalized.carousel_index_hint);
      } catch (err) {
        const serialized = parseIpcError(err);
        setAnalyzeError(serialized.user_message);
      } finally {
        setIsAnalyzing(false);
      }
    },
    [setIsAnalyzing, setAnalyzeError, setInspectedMedia]
  );

  const handleAutoDownloadUrl = useCallback(
    async (rawUrl: string) => {
      if (!rawUrl.trim()) return;
      setAnalyzeError(null);
      try {
        const check = classifyMediaUrlClient(rawUrl);
        const job = await ipc.autoEnqueueUrl(rawUrl, null);
        upsertJob(job);
        setOmnibarUrl("");
        addToast({
          title: `Auto-Downloading ${check.badgeLabel ?? job.platform.toUpperCase()}`,
          description: `${job.source_url.slice(0, 64)} (${job.quality_preset.toUpperCase()})`,
          variant: "success",
          actionLabel: "Inspect Formats",
          onAction: () => handleAnalyzeUrl(rawUrl),
        });
      } catch (err) {
        const serialized = parseIpcError(err);
        setAnalyzeError(serialized.user_message);
      }
    },
    [setAnalyzeError, upsertJob, setOmnibarUrl, addToast, handleAnalyzeUrl]
  );

  useClipboardMonitor(handleAnalyzeUrl, handleAutoDownloadUrl);
  useKeyboardShortcuts(handleAnalyzeUrl, handleAutoDownloadUrl);

  return (
    <div className="h-screen w-screen flex flex-col bg-[#07090E] text-slate-100 overflow-hidden select-none">
      {/* Custom Frameless Titlebar */}
      <Titlebar />

      {/* Main Split Workspace */}
      <div className="flex-1 flex min-h-0 overflow-hidden">
        <Sidebar />

        <main className="flex-1 flex flex-col min-w-0 min-h-0 p-4 space-y-4 overflow-hidden">
          {activeView === "workspace" && (
            <>
              <SmartOmnibar
                onAnalyze={handleAnalyzeUrl}
                onAutoDownload={handleAutoDownloadUrl}
              />
              <MediaInspectorCard />
              <ActiveQueueView />
            </>
          )}

          {activeView === "history" && <HistoryLibraryView />}

          {activeView === "settings" && <SettingsView />}
        </main>
      </div>

      {/* Bottom Telemetry Status Bar */}
      <StatusFooter />

      {/* Modals & Overlays */}
      <CommandPalette />
      <BatchUrlModal />
      <AuthVaultModal />
      <BinaryBootstrapModal />

      {/* Floating Toast Stack */}
      <div className="fixed bottom-10 right-4 z-50 flex flex-col gap-2 max-w-sm w-full pointer-events-none">
        {toasts.map((t) => (
          <div
            key={t.id}
            className="pointer-events-auto p-3.5 rounded-xl bg-[#0F131D]/95 backdrop-blur-md border border-white/[0.12] shadow-2xl flex items-start justify-between gap-3 animate-in slide-in-from-bottom-2"
          >
            <div className="min-w-0 flex-1">
              <div className="text-xs font-semibold text-slate-100">
                {t.title}
              </div>
              {t.description && (
                <div className="text-[11px] text-slate-400 truncate mt-0.5">
                  {t.description}
                </div>
              )}
              {t.actionLabel && t.onAction && (
                <button
                  onClick={() => {
                    t.onAction?.();
                    dismissToast(t.id);
                  }}
                  className="mt-2 px-2.5 py-1 rounded-md bg-violet-500/20 hover:bg-violet-500/30 border border-violet-500/35 text-[11px] font-semibold text-violet-200"
                >
                  {t.actionLabel}
                </button>
              )}
            </div>
            <button
              onClick={() => dismissToast(t.id)}
              className="text-slate-500 hover:text-slate-300"
            >
              <X className="w-3.5 h-3.5" />
            </button>
          </div>
        ))}
      </div>
    </div>
  );
};

export default App;
