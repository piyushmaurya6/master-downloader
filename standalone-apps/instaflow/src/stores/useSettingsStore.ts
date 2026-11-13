import { create } from "zustand";
import type {
  AppSettings,
  AuthSessionStatus,
  BinaryBootstrapProgress,
  BinaryStatusReport,
} from "../types";

interface SettingsState {
  settings: AppSettings | null;
  authStatus: AuthSessionStatus | null;
  binaryStatus: BinaryStatusReport | null;
  bootstrapProgressMap: Record<string, BinaryBootstrapProgress>;
  isBootstrapping: boolean;

  setSettings: (settings: AppSettings) => void;
  setAuthStatus: (status: AuthSessionStatus) => void;
  setBinaryStatus: (status: BinaryStatusReport) => void;
  updateBootstrapProgress: (progress: BinaryBootstrapProgress) => void;
  setIsBootstrapping: (val: boolean) => void;
}

export const useSettingsStore = create<SettingsState>((set) => ({
  settings: null,
  authStatus: null,
  binaryStatus: null,
  bootstrapProgressMap: {},
  isBootstrapping: false,

  setSettings: (settings) => set({ settings }),
  setAuthStatus: (authStatus) => set({ authStatus }),
  setBinaryStatus: (binaryStatus) => set({ binaryStatus }),
  updateBootstrapProgress: (progress) =>
    set((state) => ({
      bootstrapProgressMap: {
        ...state.bootstrapProgressMap,
        [progress.binary_name]: progress,
      },
    })),
  setIsBootstrapping: (isBootstrapping) => set({ isBootstrapping }),
}));
