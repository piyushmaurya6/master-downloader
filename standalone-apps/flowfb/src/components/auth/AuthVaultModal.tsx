import React, { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  ShieldCheck,
  KeyRound,
  Globe,
  FileText,
  X,
  CheckCircle2,
  LogOut,
  Loader2,
  Lock,
  AlertTriangle,
} from "lucide-react";
import { useUIStore } from "../../stores/useUIStore";
import { useSettingsStore } from "../../stores/useSettingsStore";
import { ipc, parseSerializedError } from "../../lib/ipc";

const BROWSERS = [
  { id: "firefox", label: "Mozilla Firefox (Recommended for Direct Import)" },
  { id: "edge", label: "Microsoft Edge" },
  { id: "chrome", label: "Google Chrome" },
  { id: "brave", label: "Brave Browser" },
];

export const AuthVaultModal: React.FC = () => {
  const { isAuthModalOpen, setAuthModalOpen, addToast } = useUIStore();
  const { authStatus, setAuthStatus } = useSettingsStore();

  const [activeTab, setActiveTab] = useState<"webview" | "cookies" | "browser">(
    "webview"
  );
  const [selectedBrowser, setSelectedBrowser] = useState(
    authStatus?.selected_browser || "firefox"
  );
  const [cookieText, setCookieText] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);

  if (!isAuthModalOpen) return null;

  const handleOpenWebviewLogin = async () => {
    setIsSubmitting(true);
    try {
      await ipc.openFacebookLoginWebview();
      addToast({
        type: "info",
        title: "Facebook Login Window Opened",
        message:
          "Sign in to your Facebook account in the popup window. It will close automatically once c_user & xs cookies are captured.",
      });
    } catch (err) {
      addToast({
        type: "error",
        title: "Failed to Open Login Window",
        message: parseSerializedError(err).user_message,
      });
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleImportCookieText = async () => {
    if (!cookieText.trim()) return;
    setIsSubmitting(true);
    try {
      const status = await ipc.importCookiesTxt(cookieText);
      setAuthStatus(status);
      setCookieText("");
      addToast({
        type: "success",
        title: "Cookies Encrypted in Vault",
        message: "Netscape cookies.txt validated and encrypted with AES-256-GCM.",
      });
    } catch (err) {
      addToast({
        type: "error",
        title: "Invalid Cookie Format",
        message: parseSerializedError(err).user_message,
      });
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleUploadCookieFile = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = (ev) => {
      setCookieText(String(ev.target?.result || ""));
    };
    reader.readAsText(file);
  };

  const handleConnectBrowser = async () => {
    setIsSubmitting(true);
    try {
      const status = await ipc.setBrowserCookieMode(selectedBrowser);
      setAuthStatus(status);
      addToast({
        type: "success",
        title: "Browser Cookie Delegation Active",
        message: `FlowFB will now use your local ${selectedBrowser.toUpperCase()} session for authorized Facebook media.`,
      });
    } catch (err) {
      addToast({
        type: "error",
        title: "Failed to Configure Browser",
        message: parseSerializedError(err).user_message,
      });
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleLogout = async () => {
    setIsSubmitting(true);
    try {
      const status = await ipc.logoutAuthSession();
      setAuthStatus(status);
      addToast({
        type: "info",
        title: "Session Disconnected",
        message:
          "Encrypted vault and OS Credential Manager keys have been wiped.",
      });
    } catch (err) {
      addToast({
        type: "error",
        title: "Logout Error",
        message: parseSerializedError(err).user_message,
      });
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <AnimatePresence>
      <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-sm p-4">
        <motion.div
          initial={{ opacity: 0, scale: 0.96, y: 8 }}
          animate={{ opacity: 1, scale: 1, y: 0 }}
          exit={{ opacity: 0, scale: 0.96, y: 8 }}
          className="w-full max-w-xl rounded-2xl bg-bg-surface border border-border-subtle shadow-2xl overflow-hidden"
        >
          {/* Header */}
          <div className="flex items-center justify-between px-5 py-4 border-b border-border-subtle">
            <div className="flex items-center gap-2.5">
              <div className="w-8 h-8 rounded-lg fb-gradient-bg flex items-center justify-center text-white">
                <ShieldCheck className="w-4 h-4" />
              </div>
              <div>
                <h2 className="text-sm font-semibold text-text-primary">
                  Facebook Authentication Vault
                </h2>
                <p className="text-xs text-text-secondary">
                  AES-256-GCM + Windows Credential Manager Session Storage
                </p>
              </div>
            </div>
            <button
              onClick={() => setAuthModalOpen(false)}
              className="p-1.5 rounded-lg text-text-muted hover:text-text-primary hover:bg-bg-elevated transition-colors"
            >
              <X className="w-4 h-4" />
            </button>
          </div>

          {/* Current Session Banner */}
          <div className="px-5 py-3 bg-bg-elevated/50 border-b border-border-subtle flex items-center justify-between">
            <div className="flex items-center gap-2 text-xs">
              {authStatus?.authenticated ? (
                <>
                  <CheckCircle2 className="w-4 h-4 text-status-success" />
                  <span className="text-text-primary font-medium">
                    Connected:{" "}
                    <span className="font-mono text-status-success">
                      {authStatus.username ||
                        authStatus.selected_browser ||
                        authStatus.mode}
                    </span>
                  </span>
                </>
              ) : (
                <>
                  <Lock className="w-4 h-4 text-text-muted" />
                  <span className="text-text-secondary">
                    Anonymous Guest Mode (Public Facebook Videos & Reels Only)
                  </span>
                </>
              )}
            </div>

            {authStatus?.authenticated && (
              <button
                onClick={handleLogout}
                disabled={isSubmitting}
                className="flex items-center gap-1 px-2.5 py-1 rounded-md text-xs font-medium bg-status-error/15 text-status-error hover:bg-status-error/25 transition-colors"
              >
                <LogOut className="w-3.5 h-3.5" />
                <span>Disconnect & Wipe Vault</span>
              </button>
            )}
          </div>

          {/* Mode Tabs */}
          <div className="grid grid-cols-3 border-b border-border-subtle bg-bg-canvas/40">
            <button
              onClick={() => setActiveTab("webview")}
              className={`flex items-center justify-center gap-1.5 py-2.5 text-xs font-medium border-b-2 transition-colors ${
                activeTab === "webview"
                  ? "border-accent-primary text-text-primary bg-bg-surface"
                  : "border-transparent text-text-secondary hover:text-text-primary"
              }`}
            >
              <KeyRound className="w-3.5 h-3.5" />
              <span>1. Built-In Login</span>
            </button>
            <button
              onClick={() => setActiveTab("cookies")}
              className={`flex items-center justify-center gap-1.5 py-2.5 text-xs font-medium border-b-2 transition-colors ${
                activeTab === "cookies"
                  ? "border-accent-primary text-text-primary bg-bg-surface"
                  : "border-transparent text-text-secondary hover:text-text-primary"
              }`}
            >
              <FileText className="w-3.5 h-3.5" />
              <span>2. cookies.txt</span>
            </button>
            <button
              onClick={() => setActiveTab("browser")}
              className={`flex items-center justify-center gap-1.5 py-2.5 text-xs font-medium border-b-2 transition-colors ${
                activeTab === "browser"
                  ? "border-accent-primary text-text-primary bg-bg-surface"
                  : "border-transparent text-text-secondary hover:text-text-primary"
              }`}
            >
              <Globe className="w-3.5 h-3.5" />
              <span>3. Browser Profile</span>
            </button>
          </div>

          {/* Tab Content */}
          <div className="p-5 space-y-4">
            {activeTab === "webview" && (
              <div className="space-y-4">
                <p className="text-xs text-text-secondary leading-relaxed">
                  Sign in directly through an isolated WebView2 window. FlowFB
                  never sees your password—once Facebook sets your{" "}
                  <code className="font-mono text-sky-400">c_user</code> &{" "}
                  <code className="font-mono text-sky-400">xs</code> session
                  cookies, they are encrypted with AES-256-GCM and locked inside
                  your Windows Credential Manager.
                </p>
                <div className="p-3.5 rounded-xl bg-bg-canvas border border-border-subtle text-[11px] text-text-secondary space-y-1">
                  <div className="font-semibold text-text-primary">
                    What this unlocks:
                  </div>
                  <div>
                    • Videos and Reels visible to your Facebook account (Friends /
                    Group posts you are a member of)
                  </div>
                  <div>
                    • Higher-bitrate 1080p60 / 4K DASH streams when Facebook
                    throttles anonymous guest sessions
                  </div>
                </div>
                <button
                  onClick={handleOpenWebviewLogin}
                  disabled={isSubmitting}
                  className="w-full flex items-center justify-center gap-2 py-2.5 rounded-xl text-xs font-semibold text-white fb-gradient-bg hover:opacity-95 disabled:opacity-40 transition-all"
                >
                  {isSubmitting ? (
                    <Loader2 className="w-4 h-4 animate-spin" />
                  ) : (
                    <KeyRound className="w-4 h-4" />
                  )}
                  <span>Launch Isolated Facebook Sign-In Window</span>
                </button>
              </div>
            )}

            {activeTab === "cookies" && (
              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <span className="text-xs text-text-secondary">
                    Paste or import Netscape HTTP Cookie File (
                    <code className="font-mono text-sky-400">.facebook.com</code>)
                  </span>
                  <label className="text-xs text-accent-primary hover:underline cursor-pointer">
                    <span>Import File</span>
                    <input
                      type="file"
                      accept=".txt"
                      onChange={handleUploadCookieFile}
                      className="hidden"
                    />
                  </label>
                </div>

                <textarea
                  value={cookieText}
                  onChange={(e) => setCookieText(e.target.value)}
                  placeholder={
                    "# Netscape HTTP Cookie File\n.facebook.com\tTRUE\t/\tTRUE\t1800000000\tc_user\t1000123456789\n.facebook.com\tTRUE\t/\tTRUE\t1800000000\txs\t42%3Aexample..."
                  }
                  rows={5}
                  className="w-full rounded-xl bg-bg-canvas border border-border-subtle p-3 text-xs font-mono text-text-primary placeholder:text-text-muted focus:outline-none focus:border-border-focus resize-none"
                />

                <button
                  onClick={handleImportCookieText}
                  disabled={isSubmitting || !cookieText.trim()}
                  className="w-full py-2.5 rounded-xl text-xs font-semibold text-white fb-gradient-bg hover:opacity-95 disabled:opacity-40 transition-all"
                >
                  Validate & Encrypt Cookies in Vault
                </button>
              </div>
            )}

            {activeTab === "browser" && (
              <div className="space-y-4">
                <p className="text-xs text-text-secondary leading-relaxed">
                  Delegate authentication directly to an installed desktop
                  browser where you are already logged into Facebook.
                </p>

                {selectedBrowser !== "firefox" && (
                  <div className="flex items-start gap-2.5 p-3 rounded-xl bg-status-warning/10 border border-status-warning/30 text-[11px] text-text-secondary">
                    <AlertTriangle className="w-4 h-4 text-status-warning shrink-0 mt-0.5" />
                    <div>
                      <span className="font-semibold text-status-warning">
                        Chromium App-Bound Encryption Notice (v127+):
                      </span>{" "}
                      Chrome, Edge, and Brave lock cookies with Windows DPAPI v20
                      while the browser is open. If extraction fails, use{" "}
                      <strong>Tab 1 (Built-In Login)</strong> or{" "}
                      <strong>Tab 2 (cookies.txt)</strong>.
                    </div>
                  </div>
                )}

                <select
                  value={selectedBrowser}
                  onChange={(e) => setSelectedBrowser(e.target.value)}
                  className="w-full h-10 px-3 rounded-xl bg-bg-canvas border border-border-subtle text-xs text-text-primary focus:outline-none focus:border-border-focus"
                >
                  {BROWSERS.map((b) => (
                    <option key={b.id} value={b.id}>
                      {b.label}
                    </option>
                  ))}
                </select>

                <button
                  onClick={handleConnectBrowser}
                  disabled={isSubmitting}
                  className="w-full py-2.5 rounded-xl text-xs font-semibold text-white fb-gradient-bg hover:opacity-95 disabled:opacity-40 transition-all"
                >
                  Enable {selectedBrowser.toUpperCase()} Cookie Delegation
                </button>
              </div>
            )}
          </div>
        </motion.div>
      </div>
    </AnimatePresence>
  );
};
