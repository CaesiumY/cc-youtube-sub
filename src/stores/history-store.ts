import { create } from "zustand";
import { persist } from "zustand/middleware";

/** 홈 화면 히스토리에 유지하는 최근 영상 최대 개수. */
const MAX_HISTORY = 20;

/** 시청 히스토리 항목 1개. 썸네일 URL은 videoId에서 파생하므로 저장하지 않는다. */
export type HistoryEntry = {
  videoId: string;
  title: string;
  /** 마지막으로 본 시각 (epoch ms) — 정렬용 보조 정보. */
  addedAt: number;
};

interface HistoryState {
  /** addedAt 내림차순 (최신이 먼저). */
  entries: HistoryEntry[];
  addEntry: (videoId: string, title: string) => void;
  removeEntry: (videoId: string) => void;
  clearAll: () => void;
}

/**
 * 시청 히스토리 스토어.
 *
 * `settings-store`와 동일한 zustand `persist` 패턴으로 localStorage에 영속한다.
 * 신규 스토어이므로 `migrate`는 없다.
 */
export const useHistoryStore = create<HistoryState>()(
  persist(
    (set) => ({
      entries: [],
      // 같은 videoId를 제거한 뒤 맨 앞에 다시 넣어 중복 제거 + 재방문 시 상단
      // 이동 + 제목 갱신을 동시에 처리하고, slice로 상한을 적용한다.
      addEntry: (videoId, title) =>
        set((state) => {
          const withoutDup = state.entries.filter((e) => e.videoId !== videoId);
          const entry: HistoryEntry = { videoId, title, addedAt: Date.now() };
          return { entries: [entry, ...withoutDup].slice(0, MAX_HISTORY) };
        }),
      removeEntry: (videoId) =>
        set((state) => ({
          entries: state.entries.filter((e) => e.videoId !== videoId),
        })),
      clearAll: () => set({ entries: [] }),
    }),
    {
      name: "yt-subtitle-history",
      version: 1,
    },
  ),
);
