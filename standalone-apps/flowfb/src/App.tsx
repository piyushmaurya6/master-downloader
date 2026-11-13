import React, { useEffect, useCallback } from "react";
import { AnimatePresence } from "framer-motion";
import { Titlebar } from "./components/layout/Titlebar";
import { Sidebar } from "./components/layout/Sidebar";
import { StatusFooter } from "./components/layout/StatusFooter";
import { CommandPalette } from "./components/layout/CommandPalette";
import { SmartOmnibar } from "./components/omnibar/SmartOmnibar";
import { BatchUrlModal } from "./components/omnibar/BatchUrlModal";
import { MediaInspectorCard } from "./components/inspector/MediaInspectorCard";
import { ActiveQueueView } from "./components/queue/ActiveQueueView";
import { HistoryLibraryView } from "./components/history/HistoryLibraryView";
import { SettingsView } from "./components/layout/SettingsView";
import { AuthVaultModal } from "./components/auth/AuthVaultModal";
import { BinaryBootstrapModal } from "./components/onboarding/BinaryBootstrapModal";
import { ToastContainer } from "./components/ui/ToastContainer";
import { useUIStore } from "./stores/useUIStore";
import { useQueueStore } from "./stores/useQueueStore";
import { useSettingsStore } from "./stores/useSettingsStore";
import { useTauriEvents } from "./hooks/useTauriEvents";
import { useClipboardMonitor } from "./hooks/useClipboardMonitor";
import { useKeyboardShortcuts } from "./hooks/useKeyboardShortcuts";
import { ipc, parseSerializedError } from "./lib/ipc";

export const App: React.FC = () => {
  const {
    activeTab,
    inspectedMedia,
    setInspectedMedia,
    setIsInspecting,
    setBootstrapModalOpen,
    setAuthModalOpen,
    addToast,
  } = useUIStore();
  const { setJobs } = useQueueStore();
  const { setSettings, setBinaryStatus, setAuthStatus } = useSettingsStore();

  const handleAnalyzeUrl = useCallback(
    async (url: string) => {
      const target = url.trim();
      if (!target) return;
      setIsInspecting(true);
      try {
        const result = await ipc.analyzeFacebookUrl(target);
        setInspectedMedia(result);
      } catch (err) {
        const parsed = parseSerializedError(err);
        if (parsed.code === "AUTH_REQUIRED") {
          addToast({
            type: "warning",
            title: "Authentication Required",
            message: parsed.user_message,
            actionLabel: "Connect Session",
            onAction: () => setAuthModalOpen(true),
          });
        } else {
          addToast({
            type: "error",
            title: "Analysis Failed",
            message: parsed.user_message,
          });
        }
      } finally {
        setIsInspecting(false);
      }
    },
    [setIsInspecting, setInspectedMedia, addToast, setAuthModalOpen]
  );

  useTauriEvents();
  useClipboardMonitor(handleAnalyzeUrl);
  useKeyboardShortcuts(handleAnalyzeUrl);

  useEffect(() => {
    const init = async () => {
      try {
        const [appSettings, queueJobs, binReport, authSession] =
          await Promise.all([
            ipc.getSettings(),
            ipc.getActiveQueue(),
            ipc.getBinariesStatus(),
            ipc.getAuthStatus(),
          ]);

        setSettings(appSettings);
        setJobs(queueJobs);
        setBinaryStatus(binReport);
        setAuthStatus(authSession);

        if (!binReport.all_ready) {
          try {
            const provisioned = await ipc.bootstrapBinaries();
            setBinaryStatus(provisioned);
            if (!provisioned.all_ready) {
              setBootstrapModalOpen(true);
            }
          } catch {
            setBootstrapModalOpen(true);
          }
        }
      } catch {
        // Fallback when running in pure browser dev mode outside Tauri
      }
    };

    init();
  }, [
    setSettings,
    setJobs,
    setBinaryStatus,
    setAuthStatus,
    setBootstrapModalOpen,
  ]);

  return (
    <div className="flex flex-col w-screen h-screen bg-bg-canvas text-text-primary overflow-hidden select-none">
      {/* Custom Frameless Titlebar */}
      <Titlebar />

      {/* Main Workspace */}
      <div className="flex flex-1 min-h-0 overflow-hidden">
        <Sidebar />

        <main className="flex-1 overflow-y-auto px-6 py-5">
          <div className="max-w-5xl mx-auto space-y-5">
            {activeTab === "downloads" && (
              <>
                <SmartOmnibar />
                <AnimatePresence>
                  {inspectedMedia && <MediaInspectorCard />}
                </AnimatePresence>
                <ActiveQueueView />
              </>
            )}

            {activeTab === "history" && <HistoryLibraryView />}

            {activeTab === "settings" && <SettingsView />}
          </div>
        </main>
      </div>

      {/* Persistent Telemetry Footer */}
      <StatusFooter />

      {/* Modals & Overlays */}
      <BatchUrlModal />
      <AuthVaultModal />
      <BinaryBootstrapModal />
      <CommandPalette onAnalyzeUrl={handleAnalyzeUrl} />
      <ToastContainer />
    </div>
  );
};

export default App;
