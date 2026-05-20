import { useNavigate, useParams } from "@tanstack/react-router";
import { ArrowLeft } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { ErrorModal } from "../components/error-modal";
import { ProgressBar } from "../components/progress-bar";
import { SubtitleOverlay } from "../components/subtitle-overlay";
import { YouTubePlayer } from "../components/youtube-player";
import { useBufferManager } from "../hooks/use-buffer-manager";
import { useFullscreen } from "../hooks/use-fullscreen";
import { useKeyboardShortcuts } from "../hooks/use-keyboard-shortcuts";
import { useTranslationPipeline } from "../hooks/use-translation-pipeline";
import type { AppError, EnvErrorKind } from "../lib/tauri-commands";
import { checkEnvironment, isTauri } from "../lib/tauri-commands";
import { usePlayerStore } from "../stores/player-store";
import { useSettingsStore } from "../stores/settings-store";

type EnvError = { kind: EnvErrorKind };

export function PlayerView() {
  const { videoId } = useParams({ from: "/watch/$videoId" });
  const navigate = useNavigate();
  const currentTime = usePlayerStore((s) => s.currentTime);
  const playerState = usePlayerStore((s) => s.playerState);
  const playerRef = useRef<YT.Player | null>(null);
  const [envError, setEnvError] = useState<EnvError | null>(null);
  const backend = useSettingsStore((s) => s.backend);

  // 백엔드 CLI 환경 검증 (Tauri 환경에서만). 외부 링크로 Watch에 직접 진입하는 케이스
  // 대비 — Home에서 이미 검증을 통과한 사용자도 같은 백엔드면 즉시 통과한다.
  useEffect(() => {
    if (!isTauri()) return;
    checkEnvironment(backend).catch((err: unknown) => {
      // Tauri IPC 에러가 string으로 올 수 있으므로 방어적 파싱
      let parsed: unknown;
      try {
        parsed = typeof err === "string" ? JSON.parse(err) : err;
      } catch {
        parsed = err;
      }
      const appErr = parsed as AppError | undefined;
      if (appErr?.kind === "EnvironmentCheck") {
        if (appErr.message?.startsWith("NOT_INSTALLED")) {
          setEnvError({ kind: "not_installed" });
        } else {
          setEnvError({ kind: "execution_failed" });
        }
      } else {
        // EnvironmentCheck가 아닌 에러는 모달을 띄우지 않고 콘솔에만 기록
        console.error(
          "[PlayerView] unexpected error from checkEnvironment:",
          parsed,
        );
      }
    });
  }, [backend]);

  // 핵심 훅
  useFullscreen();
  useKeyboardShortcuts(playerRef);
  useBufferManager();
  const pipeline = useTranslationPipeline(videoId);

  const handleBack = () => {
    navigate({ to: "/" });
  };

  return (
    <div className="relative flex h-full flex-col bg-background">
      {/* 뒤로가기 버튼 */}
      <button
        type="button"
        onClick={handleBack}
        className="absolute top-3 left-3 z-20 flex h-9 w-9 items-center justify-center rounded-lg bg-black/40 text-white/80 transition-colors hover:bg-black/60 hover:text-white"
        aria-label="뒤로가기"
      >
        <ArrowLeft size={18} />
      </button>

      {/* YouTube 플레이어 + 자막 오버레이 */}
      <div className="relative flex-1">
        <YouTubePlayer videoId={videoId} playerRef={playerRef} />
        <SubtitleOverlay />
      </div>

      {/* 번역 진행률 */}
      <ProgressBar />

      {/* 디버그 정보 (개발용) */}
      {import.meta.env.DEV && (
        <div className="absolute right-3 bottom-12 z-20 rounded bg-black/60 px-2 py-1 font-mono text-xs text-white/60">
          {currentTime.toFixed(1)}s | state: {playerState}
          {pipeline.totalChunks > 0 && (
            <>
              {" "}
              | {pipeline.completedChunks}/{pipeline.totalChunks}
              {pipeline.cachedChunks > 0 &&
                ` (cached: ${pipeline.cachedChunks})`}
            </>
          )}
          {pipeline.error && <span className="text-red-400"> | ERR</span>}
        </div>
      )}
      {/* CLI 환경 에러 모달 — 현재 선택된 백엔드에 맞는 안내 표시 */}
      <ErrorModal
        open={envError !== null}
        errorKind={envError?.kind}
        backend={backend}
      />
    </div>
  );
}
