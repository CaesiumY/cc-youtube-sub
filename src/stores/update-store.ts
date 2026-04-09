import { create } from "zustand";
import { isTauri } from "../lib/tauri-commands";

type UpdateStatus =
  | "idle"
  | "checking"
  | "available"
  | "downloading"
  | "ready"
  | "error";

interface UpdateState {
  status: UpdateStatus;
  version: string | null;
  progress: number;
  dismissed: boolean;
  error: string | null;

  checkForUpdate: () => Promise<void>;
  downloadAndInstall: () => Promise<void>;
  relaunch: () => Promise<boolean>;
  dismiss: () => void;
}

export const useUpdateStore = create<UpdateState>((set, get) => ({
  status: "idle",
  version: null,
  progress: 0,
  dismissed: false,
  error: null,

  checkForUpdate: async () => {
    if (!isTauri()) return;
    if (get().status === "checking") return;

    set({ status: "checking", error: null });

    try {
      const { check } = await import("@tauri-apps/plugin-updater");
      const update = await check();

      if (update) {
        set({ status: "available", version: update.version });
      } else {
        set({ status: "idle" });
      }
    } catch (e) {
      console.error("[Updater] 업데이트 확인 실패:", e);
      set({
        status: "error",
        error: e instanceof Error ? e.message : "업데이트 확인 실패",
      });
    }
  },

  downloadAndInstall: async () => {
    if (!isTauri()) return;

    set({ status: "downloading", progress: 0 });

    try {
      const { check } = await import("@tauri-apps/plugin-updater");
      const update = await check();

      if (!update) {
        set({ status: "idle" });
        return;
      }

      let totalBytes = 0;
      let downloadedBytes = 0;

      await update.downloadAndInstall((event) => {
        if (event.event === "Started" && event.data.contentLength) {
          totalBytes = event.data.contentLength;
        } else if (event.event === "Progress") {
          downloadedBytes += event.data.chunkLength;
          const progress =
            totalBytes > 0
              ? Math.round((downloadedBytes / totalBytes) * 100)
              : 0;
          set({ progress });
        } else if (event.event === "Finished") {
          set({ status: "ready", progress: 100 });
        }
      });

      set({ status: "ready", progress: 100 });
    } catch (e) {
      set({
        status: "error",
        error: e instanceof Error ? e.message : "다운로드 실패",
      });
    }
  },

  relaunch: async () => {
    if (!isTauri()) return false;

    // 번역 진행 중 여부 확인 — 호출자(UI)가 확인 다이얼로그를 표시할 수 있도록 boolean 리턴
    const { useTranslationStore } = await import("./translation-store");
    if (useTranslationStore.getState().isLoading) {
      return false;
    }

    const { relaunch } = await import("@tauri-apps/plugin-process");
    await relaunch();
    return true;
  },

  dismiss: () => set({ dismissed: true }),
}));
