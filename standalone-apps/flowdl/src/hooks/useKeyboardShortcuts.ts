import { useEffect } from "react";
import { useApp } from "../stores/AppContext";
import { isValidMediaUrl } from "../utils/formatters";

export function useKeyboardShortcuts() {
  const {
    setActivePage,
    setUrlInput,
    analyzeUrl,
    showWelcomeSetup,
    setShowWelcomeSetup,
    dependencies,
  } = useApp();

  useEffect(() => {
    const handleKeyDown = async (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      const isEditable =
        target &&
        (target.tagName === "INPUT" ||
          target.tagName === "TEXTAREA" ||
          target.tagName === "SELECT" ||
          target.isContentEditable);

      // Esc: Close welcome modal if ready
      if (e.key === "Escape") {
        if (showWelcomeSetup && dependencies.ready) {
          setShowWelcomeSetup(false);
        }
        return;
      }

      // Ctrl + J -> Open Downloads page
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "j") {
        e.preventDefault();
        setActivePage("downloads");
        return;
      }

      // Ctrl + , -> Open Settings page
      if ((e.ctrlKey || e.metaKey) && e.key === ",") {
        e.preventDefault();
        setActivePage("settings");
        return;
      }

      // Ctrl + Enter -> Analyze current URL
      if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
        e.preventDefault();
        setActivePage("home");
        await analyzeUrl();
        return;
      }

      // Ctrl + V when NOT focused inside an input field -> Paste URL into input
      if (
        (e.ctrlKey || e.metaKey) &&
        e.key.toLowerCase() === "v" &&
        !isEditable
      ) {
        try {
          const text = await navigator.clipboard.readText();
          if (text && text.trim()) {
            e.preventDefault();
            setActivePage("home");
            setUrlInput(text.trim());
            if (isValidMediaUrl(text.trim())) {
              await analyzeUrl(text.trim());
            }
          }
        } catch {
          // Clipboard read requires user gesture or permission
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [
    setActivePage,
    setUrlInput,
    analyzeUrl,
    showWelcomeSetup,
    setShowWelcomeSetup,
    dependencies.ready,
  ]);
}
