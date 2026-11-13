import React from "react";
import { AppUpdateBanner } from "./components/AppUpdateBanner";
import { Sidebar } from "./components/Sidebar";
import { TitleBar } from "./components/TitleBar";
import { ToastContainer } from "./components/ToastContainer";
import { WelcomeSetupModal } from "./components/WelcomeSetupModal";
import { useKeyboardShortcuts } from "./hooks/useKeyboardShortcuts";
import { DownloadsPage } from "./pages/DownloadsPage";
import { HistoryPage } from "./pages/HistoryPage";
import { HomePage } from "./pages/HomePage";
import { SettingsPage } from "./pages/SettingsPage";
import { AppProvider, useApp } from "./stores/AppContext";

const AppShell: React.FC = () => {
  const { activePage, appUpdateInfo, dismissAppUpdate } = useApp();
  useKeyboardShortcuts();

  return (
    <div className="h-screen w-screen flex flex-col bg-surface text-zinc-900 dark:text-zinc-100 overflow-hidden">
      <TitleBar />
      <div className="flex-1 flex min-h-0 overflow-hidden">
        <Sidebar />
        <div className="flex-1 flex flex-col min-w-0 overflow-hidden">
          <AppUpdateBanner
            updateInfo={appUpdateInfo}
            onDismiss={dismissAppUpdate}
          />
          <main className="flex-1 overflow-y-auto p-6">
            {activePage === "home" && <HomePage />}
            {activePage === "downloads" && <DownloadsPage />}
            {activePage === "history" && <HistoryPage />}
            {activePage === "settings" && <SettingsPage />}
          </main>
        </div>
      </div>
      <WelcomeSetupModal />
      <ToastContainer />
    </div>
  );
};

export const App: React.FC = () => {
  return (
    <AppProvider>
      <AppShell />
    </AppProvider>
  );
};

export default App;
