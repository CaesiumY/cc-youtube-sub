import { beforeEach, describe, expect, it } from "vitest";
import type { SubtitleChunk, TranslationEntry } from "../lib/tauri-commands";
import { useTranslationStore } from "./translation-store";

function makeChunks(n: number): SubtitleChunk[] {
  return Array.from({ length: n }, (_, i) => ({
    index: i,
    start_time: i * 30,
    end_time: (i + 1) * 30,
    lines: [{ text: `line ${i}`, start: i * 30, end: (i + 1) * 30 }],
  }));
}

function entry(start: number): TranslationEntry {
  return {
    original: `orig ${start}`,
    translated: `번역 ${start}`,
    start,
    end: start + 2,
  };
}

describe("translation-store isLoading 생명주기", () => {
  beforeEach(() => {
    useTranslationStore.getState().reset();
  });

  it("startLoading은 isLoading을 true로 설정한다", () => {
    useTranslationStore.getState().startLoading("vid1");
    expect(useTranslationStore.getState().isLoading).toBe(true);
  });

  it("첫 addTranslations가 초기 로딩(isLoading)을 종료한다", () => {
    const s = useTranslationStore.getState();
    s.startLoading("vid1");
    s.setChunks(makeChunks(10));
    expect(useTranslationStore.getState().isLoading).toBe(true);

    s.addTranslations([entry(0)]);
    expect(useTranslationStore.getState().isLoading).toBe(false);
  });

  it("회귀: markChunkStatus는 일부 청크만 처리돼도 isLoading을 되살리지 않는다", () => {
    // 버그 시나리오: BufferManager는 LOOK_AHEAD 청크만 lazy 번역하므로
    // 영상 뒷부분은 pending으로 남는다. 과거 markChunkStatus는 호출될 때마다
    // isLoading=true로 설정하고 "모든 청크 done"일 때만 해제 → 긴 영상에서 영구 true.
    const s = useTranslationStore.getState();
    s.startLoading("vid1");
    s.setChunks(makeChunks(10));
    s.addTranslations([entry(0)]); // 첫 자막 도착 → isLoading false

    // 10개 중 일부만 상태 전이 (뒷부분은 계속 pending)
    s.markChunkStatus(0, "done");
    s.markChunkStatus(1, "translating");
    s.markChunkStatus(2, "translating");

    expect(useTranslationStore.getState().isLoading).toBe(false);
  });

  it("회귀: 첫 청크가 에러로 끝나도 초기 로딩(isLoading)이 해제된다", () => {
    // browser mock 경로에서 첫 translateChunk가 실패하면 markChunkStatus(error)만
    // 호출된다. 종료 상태(error 포함)에서 isLoading을 끄지 않으면 '번역 준비 중'이
    // 영원히 표시된다.
    const s = useTranslationStore.getState();
    s.startLoading("vid1");
    s.setChunks(makeChunks(10));
    expect(useTranslationStore.getState().isLoading).toBe(true);

    s.markChunkStatus(0, "translating"); // 종료 상태 아님 — 여전히 로딩
    expect(useTranslationStore.getState().isLoading).toBe(true);

    s.markChunkStatus(0, "error"); // 종료 상태 — 초기 로딩 해제
    expect(useTranslationStore.getState().isLoading).toBe(false);
  });

  it("markChunkStatus는 일단 꺼진 isLoading을 다시 켜지 않는다", () => {
    const s = useTranslationStore.getState();
    s.startLoading("vid1");
    s.setChunks(makeChunks(10));
    s.markChunkStatus(0, "done"); // isLoading false
    expect(useTranslationStore.getState().isLoading).toBe(false);

    s.markChunkStatus(1, "translating"); // 다시 켜지면 안 됨
    expect(useTranslationStore.getState().isLoading).toBe(false);
  });

  it("markChunkStatus는 completedChunks/cachedChunks를 계속 추적한다", () => {
    const s = useTranslationStore.getState();
    s.startLoading("vid1");
    s.setChunks(makeChunks(5));

    s.markChunkStatus(0, "done");
    s.markChunkStatus(1, "cached");
    s.markChunkStatus(2, "cached");

    const state = useTranslationStore.getState();
    expect(state.completedChunks).toBe(3); // done + cached 2개
    expect(state.cachedChunks).toBe(2);
  });

  it("같은 청크를 done으로 중복 마킹해도 completedChunks가 이중 집계되지 않는다", () => {
    const s = useTranslationStore.getState();
    s.startLoading("vid1");
    s.setChunks(makeChunks(5));

    s.markChunkStatus(0, "done");
    s.markChunkStatus(0, "done");

    expect(useTranslationStore.getState().completedChunks).toBe(1);
  });

  it("setError는 isLoading을 해제한다", () => {
    const s = useTranslationStore.getState();
    s.startLoading("vid1");
    s.setError("번역 실패");

    const state = useTranslationStore.getState();
    expect(state.isLoading).toBe(false);
    expect(state.error).toBe("번역 실패");
  });
});
