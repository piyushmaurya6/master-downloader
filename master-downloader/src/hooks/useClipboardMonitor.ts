import { useEffect, useRef } from "react";
import { classifyMediaUrlClient } from "../lib/utils";
import { useQueueStore } from "../stores/useQueueStore";
import { useSettingsStore } from "../stores/useSettingsStore";
import { useUIStore } from "../stores/useUIStore";

export function useClipboardMonitor(
  onInspectUrl: (url: string) => void,
  onAutoDownloadUrl: (url: string) => void
) {
  const settings = useSettingsStore((s) => s.settings);
  const lastClipboardUrl = useUIStore((s) => s.lastClipboardUrl);
  const setLastClipboardUrl = useUIStore((s) => s.setLastClipboardUrl);
  const setOmnibarUrl = useUIStore((s) => s.setOmnibarUrl);
  const addToast = useUIStore((s) => s.addToast);
  const primedOnMountRef = useRef(false);

  useEffect(() => {
    if (!settings?.clipboard_monitor) return;

    const checkClipboard = async () => {
      try {
        if (!navigator.clipboard?.readText || !document.hasFocus()) return;
        const text = (await navigator.clipboard.readText()).trim();

        // Prime initial clipboard contents on first focus so pre-existing clipboard text
        // does not trigger an unsolicited download when the user opens the app.
        if (!primedOnMountRef.current) {
          primedOnMountRef.current = true;
          if (text) {
            setLastClipboardUrl(text);
          }
          return;
        }

        if (!text || text === lastClipboardUrl) return;

        const check = classifyMediaUrlClient(text);
        if (check.valid) {
          setLastClipboardUrl(text);

          // Skip duplicate auto-enqueue if this exact item is already queued or running
          const activeDuplicate = useQueueStore
            .getState()
            .jobs.some(
              (j) =>
                (j.status === "queued" || j.status === "running") &&
                (j.source_url === text ||
                  (Boolean(check.shortcode) &&
                    j.platform === check.platform &&
                    j.shortcode === check.shortcode))
            );
          if (activeDuplicate) return;

          setOmnibarUrl(text);

          if (settings.auto_download_on_detect) {
            onAutoDownloadUrl(text);
          } else {
            addToast({
              title: `${check.badgeLabel ?? "Media"} Link Detected`,
              description: text.slice(0, 68),
              variant: "info",
              actionLabel: "Inspect",
              onAction: () => onInspectUrl(text),
            });
          }
        }
      } catch {
        // Clipboard read permission denied or window unfocused
      }
    };

    window.addEventListener("focus", checkClipboard);
    const interval = window.setInterval(checkClipboard, 2200);
    return () => {
      window.removeEventListener("focus", checkClipboard);
      window.clearInterval(interval);
    };
  }, [
    settings?.clipboard_monitor,
    settings?.auto_download_on_detect,
    lastClipboardUrl,
    setLastClipboardUrl,
    setOmnibarUrl,
    addToast,
    onInspectUrl,
    onAutoDownloadUrl,
  ]);
}
