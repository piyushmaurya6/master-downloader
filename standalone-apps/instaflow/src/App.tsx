import React, { useCallback } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
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
import { ToastContainer } from "./components/ui/ToastContainer";
import { useClipboardMonitor } from "./hooks/useClipboardMonitor";
import { useKeyboardShortcuts } from "./hooks/useKeyboardShortcuts";
import { useTauriEvents } from "./hooks/useTauriEvents";
import { ipc, parseIpcError } from "./lib/ipc";
import { classifyInstagramUrlClient } from "./lib/utils";
import { useUIStore } from "./stores/useUIStore";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 1,
      refetchOnWindowFocus: false,
    },
  },
});

const InstaFlowMain: React.FC = () => {
  const activeView = useUIStore((s) => s.activeView);
  const setIsInspecting = useUIStore((s) => s.setIsInspecting);
  const setInspectedMedia = useUIStore((s) => s.setInspectedMedia);
  const setInspectorError = useUIStore((s) => s.setInspectorError);

  const handleAnalyzeUrl = useCallback(
    async (rawUrl: string) => {
      const check = classifyInstagramUrlClient(rawUrl);
      if (!check.valid) return;

      setIsInspecting(true);
      setInspectorError(null);
      try {
        const result = await ipc.analyzeInstagramUrl(rawUrl);
        setInspectedMedia(result, check.imgIndex);
      } catch (err) {
        setInspectorError(parseIpcError(err));
      } finally {
        setIsInspecting(false);
      }
    },
    [setIsInspecting, setInspectedMedia, setInspectorError]
  );

  useTauriEvents();
  useClipboardMonitor(handleAnalyzeUrl);
  useKeyboardShortcuts(handleAnalyzeUrl);

  return (
    <div className="h-screen w-screen flex flex-col bg-[#09090B] text-[#FAFAFA] overflow-hidden">
      {/* Custom Frameless Titlebar */}
      <Titlebar />

      {/* Main Split Workspace */}
      <div className="flex-1 flex min-h-0 overflow-hidden">
        <Sidebar />

        <main className="flex-1 flex flex-col min-w-0 min-h-0 p-5 overflow-hidden">
          {activeView === "workspace" && (
            <div className="h-full flex flex-col space-y-4 min-h-0">
              <SmartOmnibar onAnalyzeUrl={handleAnalyzeUrl} />
              <MediaInspectorCard />
              <ActiveQueueView />
            </div>
          )}

          {activeView === "history" && <HistoryLibraryView />}

          {activeView === "settings" && <SettingsView />}
        </main>
      </div>

      {/* Bottom Telemetry & Engine Footer */}
      <StatusFooter />

      {/* Global Overlays & Modals */}
      <CommandPalette onAnalyzeUrl={handleAnalyzeUrl} />
      <BatchUrlModal />
      <AuthVaultModal />
      <BinaryBootstrapModal />
      <ToastContainer />
    </div>
  );
};

export function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <InstaFlowMain />
    </QueryClientProvider>
  );
}

export default App;
