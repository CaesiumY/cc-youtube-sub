import { useEffect, useRef } from "react";
import type { TranslationEntry } from "../lib/tauri-commands";
import {
  cancelBuffering,
  isTauri,
  onSeek,
  updatePlaybackPosition,
} from "../lib/tauri-commands";
import { usePlayerStore } from "../stores/player-store";
import type { ChunkStatus } from "../stores/translation-store";
import { useTranslationStore } from "../stores/translation-store";

const SEEK_THRESHOLD = 2.0;
const POLL_INTERVAL = 500;

// ── Rust 이벤트 페이로드 타입 ───────────────────────

interface SubtitleUpdateEvent {
  chunk_index: number;
  entries: TranslationEntry[];
  session_id: number;
}

interface BufferStatusEvent {
  chunk_index: number;
  status: string;
  session_id: number;
}

interface BufferErrorEvent {
  chunk_index: number;
  error: string;
  error_kind: string;
  retryable: boolean;
  session_id: number;
}

function mapBufferStatus(status: string): ChunkStatus | null {
  switch (status) {
    case "translating":
      return "translating";
    case "done":
      return "done";
    case "error":
      return "error";
    default:
      return null;
  }
}

/**
 * Rust 버퍼 매니저와 연동하는 훅 (Tauri 환경 전용)
 *
 * 역할:
 * - 500ms 간격으로 재생 위치를 Rust에 전달 → 사전 버퍼링 트리거
 * - Tauri 이벤트 수신: subtitle-update → store에 번역 추가
 * - Seek 감지: 시간 점프 > 2초 → on_seek IPC 호출
 * - 컴포넌트 언마운트 시 cancel_buffering 호출
 *
 * 브라우저 개발 모드에서는 전부 no-op (기존 JS 큐가 대신 동작).
 */
export function useBufferManager() {
  const prevTimeRef = useRef(0);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  useEffect(() => {
    if (!isTauri()) return;

    const unlisteners: Promise<() => void>[] = [];

    const setup = async () => {
      const { listen } = await import("@tauri-apps/api/event");

      unlisteners.push(
        listen<SubtitleUpdateEvent>("subtitle-update", (event) => {
          const { entries } = event.payload;
          console.log("[subtitle-update] chunk entries:", entries.length);
          useTranslationStore.getState().addTranslations(entries);
        }),
      );

      unlisteners.push(
        listen<BufferStatusEvent>("buffer-status", (event) => {
          const { chunk_index, status } = event.payload;
          const mapped = mapBufferStatus(status);
          if (mapped) {
            useTranslationStore.getState().markChunkStatus(chunk_index, mapped);
          }
        }),
      );

      unlisteners.push(
        listen<BufferErrorEvent>("buffer-error", (event) => {
          const { chunk_index, error, error_kind, retryable } = event.payload;
          console.error("[buffer-error]", { chunk_index, error, error_kind, retryable });
          if (!retryable) {
            useTranslationStore.getState().setError(error);
          }
        }),
      );
    };

    setup();

    // 500ms 폴링: 재생 위치 → Rust 버퍼 매니저
    intervalRef.current = setInterval(() => {
      const currentTime = usePlayerStore.getState().currentTime;
      const timeDelta = Math.abs(currentTime - prevTimeRef.current);

      if (timeDelta > SEEK_THRESHOLD && prevTimeRef.current > 0) {
        onSeek(currentTime).catch(() => {});
      } else {
        updatePlaybackPosition(currentTime).catch(() => {});
      }

      prevTimeRef.current = currentTime;
    }, POLL_INTERVAL);

    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
      cancelBuffering().catch(() => {});
      Promise.all(unlisteners).then((fns) => {
        for (const fn of fns) fn();
      });
    };
  }, []);
}
