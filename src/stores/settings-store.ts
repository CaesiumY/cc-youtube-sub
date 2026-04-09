import { create } from "zustand";
import { persist } from "zustand/middleware";

export type ModelAlias = "haiku" | "sonnet";

interface SettingsState {
  selectedModel: ModelAlias;
  setSelectedModel: (model: ModelAlias) => void;
}

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set) => ({
      selectedModel: "haiku",
      setSelectedModel: (model) => set({ selectedModel: model }),
    }),
    { name: "yt-subtitle-settings" },
  ),
);
