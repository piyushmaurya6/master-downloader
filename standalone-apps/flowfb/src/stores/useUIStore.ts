import { create } from "zustand";
import { createJSONStorage, persist, type StateStorage } from "zustand/middleware";
import type { MediaInspectionResult, SerializedAppError } from "../types";

const memoryStorage = new Map<string, string>();
const safeStorage: StateStorage = {
  getItem: (name: string) => {
    try {
      if (
        typeof window !== "undefined" &&
        window.localStorage &&
        typeof window.localStorage.getItem === "function"
      ) {
        return window.localStorage.getItem(name);
      }
    } catch {
      // ignore
    }
    return memoryStorage.get(name) ?? null;
  },
  setItem: (name: string, value: string) => {
    try {
      if (
        typeof window !== "undefined" &&
        window.localStorage &&
        typeof window.localStorage.setItem === "function"
      ) {
        window.localStorage.setItem(name, value);
        return;
      }
    } catch {
      // ignore
    }
    memoryStorage.set(name, value);
  },
  removeItem: (name: string) => {
    try {
      if (
        typeof window !== "undefined" &&
        window.localStorage &&
        typeof window.localStorage.removeItem === "function"
      ) {
        window.localStorage.removeItem(name);
        return;
      }
    } catch {
      // ignore
    }
    memoryStorage.delete(name);
  },
};

export type ActiveNavView = "workspace" | "downloads" | "history" | "settings";

export interface ToastNotification {
  id: string;
  title: string;
  description?: string;
  message?: string;
  variant?: "default" | "success" | "warning" | "error";
  type?: "default" | "success" | "warning" | "error" | "info";
  actionLabel?: string;
  onAction?: () => void;
}

interface UIState {
  activeView: ActiveNavView;
  activeTab: "downloads" | "history" | "settings";
  theme: "dark" | "light";
  commandPaletteOpen: boolean;
  batchModalOpen: boolean;
  isBatchModalOpen: boolean;
  authModalOpen: boolean;
  isAuthModalOpen: boolean;
  onboardingModalOpen: boolean;
  isBootstrapModalOpen: boolean;
  omnibarUrl: string;
  omnibarInput: string;
  inspectedMedia: MediaInspectionResult | null;
  selectedSlideIndices: number[];
  selectedQuality: string;
  saveCaption: boolean;
  saveMetadata: boolean;
  embedThumbnail: boolean;
  isInspecting: boolean;
  inspectorError: SerializedAppError | null;
  toasts: ToastNotification[];

  setActiveView: (view: ActiveNavView) => void;
  setActiveTab: (tab: "downloads" | "history" | "settings") => void;
  setTheme: (theme: "dark" | "light") => void;
  toggleTheme: () => void;
  setCommandPaletteOpen: (open: boolean) => void;
  setBatchModalOpen: (open: boolean) => void;
  setAuthModalOpen: (open: boolean) => void;
  setOnboardingModalOpen: (open: boolean) => void;
  setBootstrapModalOpen: (open: boolean) => void;
  setOmnibarUrl: (url: string) => void;
  setOmnibarInput: (url: string) => void;
  setInspectedMedia: (media: MediaInspectionResult | null, hintIndex?: number | null) => void;
  toggleSlideIndex: (index: number) => void;
  selectAllSlides: () => void;
  selectPhotosOnly: () => void;
  selectVideosOnly: () => void;
  setSelectedQuality: (q: string) => void;
  setSaveCaption: (v: boolean) => void;
  setSaveMetadata: (v: boolean) => void;
  setEmbedThumbnail: (v: boolean) => void;
  setIsInspecting: (loading: boolean) => void;
  setInspectorError: (err: SerializedAppError | null) => void;
  addToast: (toast: Omit<ToastNotification, "id">) => void;
  dismissToast: (id: string) => void;
}

export const useUIStore = create<UIState>()(
  persist(
    (set, get) => ({
      activeView: "workspace",
      activeTab: "downloads",
      theme: "dark",
      commandPaletteOpen: false,
      batchModalOpen: false,
      isBatchModalOpen: false,
      authModalOpen: false,
      isAuthModalOpen: false,
      onboardingModalOpen: false,
      isBootstrapModalOpen: false,
      omnibarUrl: "",
      omnibarInput: "",
      inspectedMedia: null,
      selectedSlideIndices: [],
      selectedQuality: "best",
      saveCaption: false,
      saveMetadata: false,
      embedThumbnail: true,
      isInspecting: false,
      inspectorError: null,
      toasts: [],

      setActiveView: (view) =>
        set({
          activeView: view === "downloads" ? "workspace" : view,
          activeTab: view === "workspace" ? "downloads" : (view as "downloads" | "history" | "settings"),
        }),

      setActiveTab: (tab) =>
        set({
          activeTab: tab,
          activeView: tab === "downloads" ? "workspace" : tab,
        }),

      setTheme: (theme) => {
        if (typeof document !== "undefined") {
          document.documentElement.classList.toggle("light", theme === "light");
          document.documentElement.classList.toggle("dark", theme === "dark");
        }
        set({ theme });
      },

      toggleTheme: () => {
        const next = get().theme === "dark" ? "light" : "dark";
        get().setTheme(next);
      },

      setCommandPaletteOpen: (commandPaletteOpen) => set({ commandPaletteOpen }),
      setBatchModalOpen: (open) => set({ batchModalOpen: open, isBatchModalOpen: open }),
      setAuthModalOpen: (open) => set({ authModalOpen: open, isAuthModalOpen: open }),
      setOnboardingModalOpen: (open) =>
        set({ onboardingModalOpen: open, isBootstrapModalOpen: open }),
      setBootstrapModalOpen: (open) =>
        set({ onboardingModalOpen: open, isBootstrapModalOpen: open }),

      setOmnibarUrl: (url) => set({ omnibarUrl: url, omnibarInput: url }),
      setOmnibarInput: (url) => set({ omnibarUrl: url, omnibarInput: url }),

      setInspectedMedia: (media, hintIndex) => {
        if (!media) {
          set({ inspectedMedia: null, selectedSlideIndices: [], inspectorError: null });
          return;
        }
        const indices =
          hintIndex && hintIndex >= 1 && hintIndex <= media.slides.length
            ? [hintIndex]
            : media.slides.map((s) => s.index);
        set({
          inspectedMedia: media,
          selectedSlideIndices: indices,
          inspectorError: null,
        });
      },

      toggleSlideIndex: (index) => {
        const current = get().selectedSlideIndices;
        if (current.includes(index)) {
          set({ selectedSlideIndices: current.filter((i) => i !== index) });
        } else {
          set({ selectedSlideIndices: [...current, index].sort((a, b) => a - b) });
        }
      },

      selectAllSlides: () => {
        const media = get().inspectedMedia;
        if (!media) return;
        set({ selectedSlideIndices: media.slides.map((s) => s.index) });
      },

      selectPhotosOnly: () => {
        const media = get().inspectedMedia;
        if (!media) return;
        set({
          selectedSlideIndices: media.slides
            .filter((s) => s.media_type === "image")
            .map((s) => s.index),
        });
      },

      selectVideosOnly: () => {
        const media = get().inspectedMedia;
        if (!media) return;
        set({
          selectedSlideIndices: media.slides
            .filter((s) => s.media_type === "video")
            .map((s) => s.index),
        });
      },

      setSelectedQuality: (selectedQuality) => set({ selectedQuality }),
      setSaveCaption: (saveCaption) => set({ saveCaption }),
      setSaveMetadata: (saveMetadata) => set({ saveMetadata }),
      setEmbedThumbnail: (embedThumbnail) => set({ embedThumbnail }),
      setIsInspecting: (isInspecting) => set({ isInspecting }),
      setInspectorError: (inspectorError) => set({ inspectorError }),

      addToast: (toast) => {
        const id = `${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;
        const mappedVariant =
          toast.variant ||
          (toast.type === "info" ? "default" : toast.type) ||
          "default";
        const mappedDescription = toast.description ?? toast.message;
        set((state) => ({
          toasts: [
            ...state.toasts.slice(-4),
            {
              ...toast,
              id,
              variant: mappedVariant,
              description: mappedDescription,
            },
          ],
        }));
        setTimeout(() => {
          get().dismissToast(id);
        }, 5000);
      },

      dismissToast: (id) =>
        set((state) => ({
          toasts: state.toasts.filter((t) => t.id !== id),
        })),
    }),
    {
      name: "flowfb-ui-preferences",
      storage: createJSONStorage(() => safeStorage),
      partialize: (state) => ({
        theme: state.theme,
        selectedQuality: state.selectedQuality,
        saveCaption: state.saveCaption,
        saveMetadata: state.saveMetadata,
        embedThumbnail: state.embedThumbnail,
      }),
    }
  )
);
