import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { SubtitleChunk, VideoInfo } from "../lib/tauri-commands";
import { useHistoryStore } from "../stores/history-store";
import { useTranslationStore } from "../stores/translation-store";
import { useTranslationPipeline } from "./use-translation-pipeline";

const mockChunks: SubtitleChunk[] = [
  {
    index: 0,
    start_time: 0,
    end_time: 30,
    lines: [{ text: "hello world", start: 0, end: 2 }],
  },
];

const mockVideoInfo: VideoInfo = {
  title: "Mock Video Title",
  description: "",
};

// vi.mock 팩토리는 hoist되므로, 테스트별로 제어할 mock 함수만 vi.hoisted로 끌어올린다.
const mocks = vi.hoisted(() => ({
  fetchSubtitles: vi.fn(),
  fetchVideoInfo: vi.fn(),
}));

vi.mock("../lib/tauri-commands", () => ({
  isTauri: () => false,
  fetchSubtitles: mocks.fetchSubtitles,
  fetchVideoInfo: mocks.fetchVideoInfo,
  getChunkHash: vi.fn(async () => "chunkhash"),
  batchQueryCache: vi.fn(async () => ({})),
  translateChunk: vi.fn(async () => []),
  saveToCache: vi.fn(async () => undefined),
  initBuffer: vi.fn(async () => undefined),
}));

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
}

beforeEach(() => {
  useHistoryStore.getState().clearAll();
  useTranslationStore.getState().reset();
  mocks.fetchSubtitles.mockReset();
  mocks.fetchVideoInfo.mockReset();
  mocks.fetchSubtitles.mockResolvedValue(mockChunks);
  mocks.fetchVideoInfo.mockResolvedValue(mockVideoInfo);
});

describe("useTranslationPipeline 히스토리 기록", () => {
  it("자막 로드 성공 시 영상을 히스토리에 기록한다", async () => {
    renderHook(() => useTranslationPipeline("testvideo01"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      const entries = useHistoryStore.getState().entries;
      expect(entries).toHaveLength(1);
      expect(entries[0]?.videoId).toBe("testvideo01");
      expect(entries[0]?.title).toBe("Mock Video Title");
    });
  });

  it("videoInfo가 자막보다 늦게 도착해도 제목이 채워진다", async () => {
    let resolveInfo!: (v: VideoInfo) => void;
    mocks.fetchVideoInfo.mockReturnValueOnce(
      new Promise<VideoInfo>((resolve) => {
        resolveInfo = resolve;
      }),
    );

    renderHook(() => useTranslationPipeline("testvideo02"), {
      wrapper: createWrapper(),
    });

    // 자막이 먼저 도착 → 제목 없이 우선 기록된다
    await waitFor(() => {
      expect(useHistoryStore.getState().entries).toHaveLength(1);
    });
    expect(useHistoryStore.getState().entries[0]?.title).toBe("");

    // videoInfo 도착 → effect 재실행으로 제목이 채워진다
    resolveInfo(mockVideoInfo);
    await waitFor(() => {
      expect(useHistoryStore.getState().entries[0]?.title).toBe(
        "Mock Video Title",
      );
    });
  });
});
