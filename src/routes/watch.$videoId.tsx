import { useNavigate, useParams } from "@tanstack/react-router";
import { ArrowLeft } from "lucide-react";
import { useRef } from "react";
import { ProgressBar } from "../components/progress-bar";
import { SubtitleOverlay } from "../components/subtitle-overlay";
import { YouTubePlayer } from "../components/youtube-player";
import { useFullscreen } from "../hooks/use-fullscreen";
import { useKeyboardShortcuts } from "../hooks/use-keyboard-shortcuts";
import { useTranslationPipeline } from "../hooks/use-translation-pipeline";
import { usePlayerStore } from "../stores/player-store";

export function PlayerView() {
  const { videoId } = useParams({ from: "/watch/$videoId" });
  const navigate = useNavigate();
  const currentTime = usePlayerStore((s) => s.currentTime);
  const playerState = usePlayerStore((s) => s.playerState);
  const playerRef = useRef<YT.Player | null>(null);

  // 핵심 훅
  useFullscreen();
  useKeyboardShortcuts(playerRef);
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
              {" "}| {pipeline.completedChunks}/{pipeline.totalChunks}
              {pipeline.cachedChunks > 0 &&
                ` (cached: ${pipeline.cachedChunks})`}
            </>
          )}
          {pipeline.error && <span className="text-red-400"> | ERR</span>}
        </div>
      )}
    </div>
  );
}
