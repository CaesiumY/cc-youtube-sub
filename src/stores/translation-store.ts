import { create } from "zustand";
import type {
  SubtitleChunk,
  TranslationEntry,
  VideoInfo,
} from "../lib/tauri-commands";

export type ChunkStatus =
  | "pending"
  | "translating"
  | "done"
  | "cached"
  | "error";

interface TranslationState {
  // 청크 데이터
  chunks: SubtitleChunk[];
  chunkStatuses: Record<number, ChunkStatus>;
  translations: TranslationEntry[]; // 시간순 정렬

  // 진행 상태
  totalChunks: number;
  completedChunks: number;
  cachedChunks: number;
  isLoading: boolean;
  error: string | null;

  // 영상 정보
  videoInfo: VideoInfo | null;
  videoId: string | null;

  // 세션 (seek 시 증가 → 이전 번역 폐기)
  sessionId: number;

  // Actions
  startLoading: (videoId: string) => void;
  setChunks: (chunks: SubtitleChunk[]) => void;
  setVideoInfo: (info: VideoInfo) => void;
  markChunkStatus: (index: number, status: ChunkStatus) => void;
  addTranslations: (entries: TranslationEntry[]) => void;
  setError: (error: string | null) => void;
  incrementSession: () => void;
  reset: () => void;
}

/**
 * 번역 진행 상태를 관리하는 Zustand store.
 * 청크별 상태 추적, 번역 결과 시간순 병합, seek 세션 관리.
 */
export const useTranslationStore = create<TranslationState>((set, get) => ({
  chunks: [],
  chunkStatuses: {},
  translations: [],
  totalChunks: 0,
  completedChunks: 0,
  cachedChunks: 0,
  isLoading: false,
  error: null,
  videoInfo: null,
  videoId: null,
  sessionId: 0,

  startLoading: (videoId) =>
    set({
      videoId,
      isLoading: true,
      error: null,
      chunks: [],
      chunkStatuses: {},
      translations: [],
      totalChunks: 0,
      completedChunks: 0,
      cachedChunks: 0,
    }),

  setChunks: (chunks) =>
    set({
      chunks,
      totalChunks: chunks.length,
      chunkStatuses: Object.fromEntries(
        chunks.map((c) => [c.index, "pending" as ChunkStatus]),
      ),
    }),

  setVideoInfo: (info) => set({ videoInfo: info }),

  markChunkStatus: (index, status) => {
    const { chunkStatuses, completedChunks, cachedChunks } = get();
    const prev = chunkStatuses[index];
    const wasDone = prev === "done" || prev === "cached";
    const isDone = status === "done" || status === "cached";

    set({
      chunkStatuses: { ...chunkStatuses, [index]: status },
      completedChunks:
        !wasDone && isDone ? completedChunks + 1 : completedChunks,
      cachedChunks: status === "cached" ? cachedChunks + 1 : cachedChunks,
      isLoading: true,
    });

    // 모든 청크 완료 시 isLoading 해제
    const updated = { ...chunkStatuses, [index]: status };
    const allDone = Object.values(updated).every(
      (s) => s === "done" || s === "cached" || s === "error",
    );
    if (allDone) {
      set({ isLoading: false });
    }
  },

  addTranslations: (entries) => {
    const { translations } = get();
    // 시간순으로 병합 (중복 방지: start 기준)
    const existingStarts = new Set(translations.map((t) => t.start));
    const newEntries = entries.filter((e) => !existingStarts.has(e.start));
    const merged = [...translations, ...newEntries].sort(
      (a, b) => a.start - b.start,
    );
    set({ translations: merged });
  },

  setError: (error) => set({ error, isLoading: false }),

  incrementSession: () => set((state) => ({ sessionId: state.sessionId + 1 })),

  reset: () =>
    set({
      chunks: [],
      chunkStatuses: {},
      translations: [],
      totalChunks: 0,
      completedChunks: 0,
      cachedChunks: 0,
      isLoading: false,
      error: null,
      videoInfo: null,
      videoId: null,
      sessionId: 0,
    }),
}));
