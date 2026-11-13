import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { ipc, isTauriRuntime } from "../lib/ipc";
import { useQueueStore } from "../stores/useQueueStore";
import { useSettingsStore } from "../stores/useSettingsStore";
import { useUIStore } from "../stores/useUIStore";
import type {
  BinaryBootstrapProgress,
  ProgressPayload,
  RateLimitTrippedPayload,
} from "../types";

export function useTauriEvents() {
  const setJobs = useQueueStore((s) => s.setJobs);
  const applyProgressTelemetry = useQueueStore((s) => s.applyProgressTelemetry);
  const setRateLimitAlert = useQueueStore((s) => s.setRateLimitAlert);

  const setSettings = useSettingsStore((s) => s.setSettings);
  const setAuthStatus = useSettingsStore((s) => s.setAuthStatus);
  const setBinaryStatus = useSettingsStore((s) => s.setBinaryStatus);
  const updateBootstrapProgress = useSettingsStore((s) => s.updateBootstrapProgress);

  const setOnboardingModalOpen = useUIStore((s) => s.setOnboardingModalOpen);
  const setAuthModalOpen = useUIStore((s) => s.setAuthModalOpen);
  const addToast = useUIStore((s) => s.addToast);

  useEffect(() => {
    if (!isTauriRuntime()) return;

    const init = async () => {
      try {
        const [settings, auth, binaries, queue] = await Promise.all([
          ipc.getAppSettings(),
          ipc.getAuthStatus(),
          ipc.checkBinariesStatus(),
          ipc.listQueue(),
        ]);
        setSettings(settings);
        setAuthStatus(auth);
        setBinaryStatus(binaries);
        setJobs(queue);

        if (!binaries.all_ready) {
          setOnboardingModalOpen(true);
        }
      } catch (err) {
        console.error("Failed initial IPC hydration:", err);
      }
    };

    init();

    const unlisteners: Array<() => void> = [];

    const setupListeners = async () => {
      const u1 = await listen<ProgressPayload>("download-progress", (event) => {
        applyProgressTelemetry(event.payload);
      });
      unlisteners.push(u1);

      const u2 = await listen<string>("queue-updated", async () => {
        try {
          const updated = await ipc.listQueue();
          setJobs(updated);
        } catch {
          // ignore
        }
      });
      unlisteners.push(u2);

      const u3 = await listen<RateLimitTrippedPayload>("rate-limit-tripped", (event) => {
        setRateLimitAlert(event.payload);
        addToast({
          title: event.payload.requires_reauth
            ? "Facebook Authentication Required"
            : "Facebook Rate-Limit Circuit Breaker Tripped",
          description: event.payload.reason,
          variant: "warning",
          actionLabel: event.payload.requires_reauth ? "Connect Account" : "Resume Queue",
          onAction: () => {
            if (event.payload.requires_reauth) {
              setAuthModalOpen(true);
            } else {
              ipc.resumeAllJobs();
              setRateLimitAlert(null);
            }
          },
        });
      });
      unlisteners.push(u3);

      const u4 = await listen<BinaryBootstrapProgress>(
        "binary-bootstrap-progress",
        (event) => {
          updateBootstrapProgress(event.payload);
        }
      );
      unlisteners.push(u4);

      const u5 = await listen<string>("auth-status-changed", async () => {
        try {
          const [auth, settings] = await Promise.all([
            ipc.getAuthStatus(),
            ipc.getAppSettings(),
          ]);
          setAuthStatus(auth);
          setSettings(settings);
        } catch {
          // ignore
        }
      });
      unlisteners.push(u5);
    };

    setupListeners();

    return () => {
      for (const unlisten of unlisteners) {
        unlisten();
      }
    };
  }, [
    setJobs,
    applyProgressTelemetry,
    setRateLimitAlert,
    setSettings,
    setAuthStatus,
    setBinaryStatus,
    updateBootstrapProgress,
    setOnboardingModalOpen,
    setAuthModalOpen,
    addToast,
  ]);
}
