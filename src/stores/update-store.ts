import { create } from "zustand";
import { isTauri } from "../lib/tauri-commands";
import { translateUpdateError } from "../lib/update-error";

type UpdateStatus =
  | "idle"
  | "checking"
  | "available"
  | "downloading"
  | "ready"
  | "error";

type UpdateTrigger = "auto" | "manual";

/**
 * 수동 확인(Home "업데이트 확인" 버튼) 시 로딩 상태가 최소 이 시간 이상 보이도록
 * 보장한다. 404 같은 즉시 실패가 수십 ms 만에 끝나면 버튼 → 배너로 갑자기
 * 전환되어 "클릭이 반영됐는지" 인지가 안 되는 문제를 해결. 자동 확인에는 적용
 * 안 함(UI에 노출 자체가 없음).
 */
const MIN_CHECKING_MS_FOR_MANUAL = 500;

interface UpdateState {
  status: UpdateStatus;
  version: string | null;
  progress: number;
  dismissed: boolean;
  error: string | null;
  lastTriggeredBy: UpdateTrigger;

  checkForUpdate: (trigger?: UpdateTrigger) => Promise<void>;
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
  lastTriggeredBy: "auto",

  checkForUpdate: async (trigger = "auto") => {
    if (!isTauri()) return;
    if (get().status === "checking") return;

    const startedAt = Date.now();

    set({
      status: "checking",
      error: null,
      dismissed: false,
      lastTriggeredBy: trigger,
    });

    const ensureMinCheckingTime = async () => {
      if (trigger !== "manual") return;
      const elapsed = Date.now() - startedAt;
      if (elapsed < MIN_CHECKING_MS_FOR_MANUAL) {
        await new Promise((r) =>
          setTimeout(r, MIN_CHECKING_MS_FOR_MANUAL - elapsed),
        );
      }
    };

    try {
      const { check } = await import("@tauri-apps/plugin-updater");
      const update = await check();

      await ensureMinCheckingTime();

      if (update) {
        set({ status: "available", version: update.version });
      } else {
        set({ status: "idle" });
      }
    } catch (e) {
      console.error("[Updater] 업데이트 확인 실패:", e);
      await ensureMinCheckingTime();
      set({
        status: "error",
        error: translateUpdateError(e),
      });
    }
  },

  downloadAndInstall: async () => {
    if (!isTauri()) return;

    // 다운로드는 사용자가 명시적으로 트리거하므로 manual로 기록한다.
    set({ status: "downloading", progress: 0, lastTriggeredBy: "manual" });

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
      console.error("[Updater] 다운로드 실패:", e);
      set({
        status: "error",
        error: translateUpdateError(e),
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
