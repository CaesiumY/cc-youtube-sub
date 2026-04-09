/**
 * Tauri IPC 경계 — 모든 백엔드 호출이 이 모듈을 통과한다.
 *
 * Tauri 환경이면 실제 invoke()를 호출하고,
 * 브라우저(개발 서버)에서는 mock 구현으로 자동 전환된다.
 */

// ── 타입 정의 (Rust 타입과 1:1 매핑) ──────────────────

export interface SubtitleLine {
  text: string;
  start: number;
  end: number;
}

export interface SubtitleChunk {
  index: number;
  start_time: number;
  end_time: number;
  lines: SubtitleLine[];
}

export interface VideoInfo {
  title: string;
  description: string;
}

export interface TranslationEntry {
  original: string;
  translated: string;
  start: number;
  end: number;
}

export interface AppError {
  kind:
    | "CaptionFetch"
    | "Translation"
    | "Database"
    | "EnvironmentCheck"
    | "Process";
  message: string;
}

export type EnvErrorKind = "not_installed" | "execution_failed";

// ── Tauri 환경 감지 ──────────────────────────────────

export function isTauri(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

async function getInvoke() {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke;
}

// ── 커맨드 래퍼 ──────────────────────────────────────

export async function checkEnvironment(): Promise<string> {
  if (!isTauri()) {
    const mock = await import("./mock-tauri");
    return mock.checkEnvironment();
  }
  const invoke = await getInvoke();
  return invoke<string>("check_environment");
}

export async function fetchSubtitles(
  videoId: string,
): Promise<SubtitleChunk[]> {
  if (!isTauri()) {
    const mock = await import("./mock-tauri");
    return mock.fetchSubtitles(videoId);
  }
  const invoke = await getInvoke();
  return invoke<SubtitleChunk[]>("fetch_subtitles", { videoId });
}

export async function fetchVideoInfo(videoId: string): Promise<VideoInfo> {
  if (!isTauri()) {
    const mock = await import("./mock-tauri");
    return mock.fetchVideoInfo(videoId);
  }
  const invoke = await getInvoke();
  return invoke<VideoInfo>("fetch_video_info", { videoId });
}

export async function translateChunk(
  chunk: SubtitleChunk,
  videoInfo?: VideoInfo,
  previousContext?: SubtitleLine[],
  model?: string,
): Promise<TranslationEntry[]> {
  if (!isTauri()) {
    const mock = await import("./mock-tauri");
    return mock.translateChunk(chunk, videoInfo, previousContext);
  }
  const invoke = await getInvoke();
  return invoke<TranslationEntry[]>("translate_chunk", {
    chunk,
    videoInfo: videoInfo ?? null,
    previousContext: previousContext ?? null,
    model: model ?? null,
  });
}

export async function queryCache(
  videoId: string,
  chunkHash: string,
): Promise<string | null> {
  if (!isTauri()) {
    const mock = await import("./mock-tauri");
    return mock.queryCache(videoId, chunkHash);
  }
  const invoke = await getInvoke();
  return invoke<string | null>("query_cache", { videoId, chunkHash });
}

export async function saveToCache(
  videoId: string,
  chunkHash: string,
  translatedJson: string,
): Promise<void> {
  if (!isTauri()) {
    const mock = await import("./mock-tauri");
    return mock.saveToCache(videoId, chunkHash, translatedJson);
  }
  const invoke = await getInvoke();
  return invoke<void>("save_to_cache", {
    videoId,
    chunkHash,
    translatedJson,
  });
}

export async function getChunkHash(lines: SubtitleLine[]): Promise<string> {
  if (!isTauri()) {
    const mock = await import("./mock-tauri");
    return mock.getChunkHash(lines);
  }
  const invoke = await getInvoke();
  return invoke<string>("get_chunk_hash", { lines });
}

export async function batchQueryCache(
  videoId: string,
  chunkHashes: string[],
): Promise<Record<string, string>> {
  if (!isTauri()) {
    const mock = await import("./mock-tauri");
    return mock.batchQueryCache(videoId, chunkHashes);
  }
  const invoke = await getInvoke();
  return invoke<Record<string, string>>("batch_query_cache", {
    videoId,
    chunkHashes,
  });
}

// ── 버퍼 매니저 커맨드 (Phase 3) ────────────────────

export async function initBuffer(
  videoId: string,
  chunks: SubtitleChunk[],
  videoInfo: VideoInfo | null,
  cachedIndices: number[],
  model?: string,
): Promise<void> {
  if (!isTauri()) return;
  const invoke = await getInvoke();
  return invoke<void>("init_buffer", {
    videoId,
    chunks,
    videoInfo,
    cachedIndices,
    model: model ?? null,
  });
}

export async function updatePlaybackPosition(
  currentTime: number,
): Promise<void> {
  if (!isTauri()) return;
  const invoke = await getInvoke();
  return invoke<void>("update_playback_position", { currentTime });
}

export async function onSeek(targetTime: number): Promise<void> {
  if (!isTauri()) return;
  const invoke = await getInvoke();
  return invoke<void>("on_seek", { targetTime });
}

export async function cancelBuffering(): Promise<void> {
  if (!isTauri()) return;
  const invoke = await getInvoke();
  return invoke<void>("cancel_buffering");
}
