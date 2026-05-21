import { useQuery } from "@tanstack/react-query";
import { useCallback, useEffect, useRef } from "react";
import {
  batchQueryCache,
  fetchSubtitles,
  fetchVideoInfo,
  getChunkHash,
  initBuffer,
  isTauri,
  saveToCache,
  translateChunk,
} from "../lib/tauri-commands";
import type { SubtitleChunk, SubtitleLine } from "../lib/tauri-commands";
import { useHistoryStore } from "../stores/history-store";
import { getActiveModel, useSettingsStore } from "../stores/settings-store";
import { useTranslationStore } from "../stores/translation-store";

const MAX_CONCURRENT = 2;

/**
 * 번역 파이프라인 훅 — Phase 2의 핵심 오케스트레이터
 *
 * TQ: 자막/영상정보 fetch (캐싱 + 로딩 상태)
 * Zustand: 번역 큐 관리, 진행 상태, seek 세션
 *
 * 플로우:
 * 1. useQuery로 자막 + 영상정보 fetch
 * 2. 청크별 해시 계산 → batchQueryCache로 캐시 조회
 * 3. cache hit → store에 즉시 추가 / miss → 번역 큐
 * 4. 큐에서 max 2개 동시 번역 실행
 * 5. 완료 시 store 업데이트 + 캐시 저장
 */
export function useTranslationPipeline(videoId: string) {
  const store = useTranslationStore();
  const activeTranslations = useRef(0);
  const sessionRef = useRef(store.sessionId);
  const initializedRef = useRef(false);
  const chunkHashesRef = useRef<Map<number, string>>(new Map());

  // TQ: 자막 fetch
  const {
    data: chunks,
    isLoading: isLoadingSubtitles,
    error: subtitleError,
  } = useQuery({
    queryKey: ["subtitles", videoId],
    queryFn: () => fetchSubtitles(videoId),
    staleTime: 1000 * 60 * 30, // 30분
  });

  // TQ: 영상 정보 fetch
  const { data: videoInfo } = useQuery({
    queryKey: ["videoInfo", videoId],
    queryFn: () => fetchVideoInfo(videoId),
    staleTime: 1000 * 60 * 30,
  });

  // 세션 ID 동기화
  useEffect(() => {
    sessionRef.current = store.sessionId;
  }, [store.sessionId]);

  // 번역 큐에서 다음 청크를 처리
  const processQueue = useCallback(async () => {
    const { chunks, chunkStatuses, videoInfo, sessionId } =
      useTranslationStore.getState();
    const currentSession = sessionRef.current;

    if (activeTranslations.current >= MAX_CONCURRENT) return;
    if (sessionId !== currentSession) return;

    // pending 상태인 청크를 index 순으로 찾기
    const nextChunk = chunks.find((c) => chunkStatuses[c.index] === "pending");
    if (!nextChunk) return;

    activeTranslations.current++;
    useTranslationStore
      .getState()
      .markChunkStatus(nextChunk.index, "translating");

    try {
      // 이전 청크의 마지막 5줄을 context로 전달
      const prevContext = getPreviousContext(chunks, nextChunk.index);

      const settings = useSettingsStore.getState();
      const activeModel = getActiveModel(settings);
      const entries = await translateChunk(
        nextChunk,
        nextChunk.index === 0 ? (videoInfo ?? undefined) : undefined,
        prevContext,
        activeModel,
        settings.backend,
      );

      // 세션이 바뀌었으면 결과 폐기
      if (sessionRef.current !== currentSession) return;

      useTranslationStore.getState().addTranslations(entries);
      useTranslationStore.getState().markChunkStatus(nextChunk.index, "done");

      // 백그라운드 캐시 저장
      const hash = chunkHashesRef.current.get(nextChunk.index);
      if (hash) {
        saveToCache(videoId, hash, JSON.stringify(entries)).catch(() => {
          // 캐시 저장 실패는 로그만
          console.warn("캐시 저장 실패:", nextChunk.index);
        });
      }
    } catch (err) {
      if (sessionRef.current === currentSession) {
        useTranslationStore
          .getState()
          .markChunkStatus(nextChunk.index, "error");
        console.error("번역 실패:", nextChunk.index, err);
      }
    } finally {
      activeTranslations.current--;
      // 다음 청크 처리 시도
      if (sessionRef.current === currentSession) {
        processQueue();
      }
    }
  }, [videoId]);

  // 초기화: 청크 도착 시 캐시 조회 → 큐 시작
  useEffect(() => {
    if (!chunks || chunks.length === 0 || initializedRef.current) return;
    initializedRef.current = true;

    const init = async () => {
      const s = useTranslationStore.getState();
      s.startLoading(videoId);
      s.setChunks(chunks);
      if (videoInfo) s.setVideoInfo(videoInfo);

      // 모든 청크의 해시 계산
      const hashes = await Promise.all(
        chunks.map(async (chunk) => {
          const hash = await getChunkHash(chunk.lines);
          return { index: chunk.index, hash };
        }),
      );

      const hashMap = new Map<number, string>();
      const hashArray: string[] = [];
      for (const { index, hash } of hashes) {
        hashMap.set(index, hash);
        hashArray.push(hash);
      }
      chunkHashesRef.current = hashMap;

      // 캐시 일괄 조회
      const cached = await batchQueryCache(videoId, hashArray);

      // 캐시 hit 처리
      const cachedIndices: number[] = [];
      for (const { index, hash } of hashes) {
        const cachedJson = cached[hash];
        if (cachedJson) {
          try {
            const entries = JSON.parse(cachedJson);
            useTranslationStore.getState().addTranslations(entries);
            useTranslationStore.getState().markChunkStatus(index, "cached");
            cachedIndices.push(index);
          } catch {
            // 파싱 실패 → pending으로 유지 (재번역)
          }
        }
      }

      if (isTauri()) {
        // Phase 3: Rust 버퍼 매니저가 재생 위치 기반 번역 스케줄링
        const settings = useSettingsStore.getState();
        const activeModel = getActiveModel(settings);
        await initBuffer(
          videoId,
          chunks,
          videoInfo ?? null,
          cachedIndices,
          activeModel,
          settings.backend,
        );
      } else {
        // 브라우저 개발 모드: 기존 프론트엔드 큐
        for (let i = 0; i < MAX_CONCURRENT; i++) {
          processQueue();
        }
      }
    };

    init().catch((err) => {
      console.error("[translation-pipeline] init failed:", err);
      useTranslationStore.getState().setError(String(err));
    });
  }, [chunks, videoInfo, videoId, processQueue]);

  // videoInfo 늦게 도착 시 store 업데이트
  useEffect(() => {
    if (videoInfo && useTranslationStore.getState().videoId === videoId) {
      useTranslationStore.getState().setVideoInfo(videoInfo);
    }
  }, [videoInfo, videoId]);

  // 자막 로드 성공 시 시청 히스토리에 기록.
  // addEntry는 멱등 — videoInfo가 자막보다 늦게 도착하면 effect가 재실행되어
  // 제목이 채워진다.
  useEffect(() => {
    if (chunks && chunks.length > 0) {
      useHistoryStore.getState().addEntry(videoId, videoInfo?.title ?? "");
    }
  }, [chunks, videoInfo, videoId]);

  // cleanup on videoId change
  useEffect(() => {
    const cleanupVideoId = videoId;

    return () => {
      void cleanupVideoId;
      initializedRef.current = false;
      chunkHashesRef.current = new Map();
      activeTranslations.current = 0;
    };
  }, [videoId]);

  return {
    isLoading: isLoadingSubtitles || store.isLoading,
    error: subtitleError ? String(subtitleError) : store.error,
    progress:
      store.totalChunks > 0 ? store.completedChunks / store.totalChunks : 0,
    totalChunks: store.totalChunks,
    completedChunks: store.completedChunks,
    cachedChunks: store.cachedChunks,
  };
}

/** 이전 청크의 마지막 5줄을 context로 추출 */
function getPreviousContext(
  chunks: SubtitleChunk[],
  currentIndex: number,
): SubtitleLine[] | undefined {
  if (currentIndex === 0) return undefined;
  const prev = chunks.find((c) => c.index === currentIndex - 1);
  if (!prev) return undefined;
  return prev.lines.slice(-5);
}
