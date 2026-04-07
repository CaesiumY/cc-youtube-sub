import { useCallback, useEffect, useRef } from "react";
import YouTube, { type YouTubeEvent } from "react-youtube";
import { usePlayerStore } from "../stores/player-store";

interface YouTubePlayerProps {
  videoId: string;
}

export function YouTubePlayer({ videoId }: YouTubePlayerProps) {
  const playerRef = useRef<YT.Player | null>(null);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const setCurrentTime = usePlayerStore((s) => s.setCurrentTime);
  const setPlayerState = usePlayerStore((s) => s.setPlayerState);

  // 500ms 폴링으로 재생 시간 추적
  const startPolling = useCallback(() => {
    if (intervalRef.current) return;
    intervalRef.current = setInterval(() => {
      const time = playerRef.current?.getCurrentTime();
      if (time !== undefined) {
        setCurrentTime(time);
      }
    }, 500);
  }, [setCurrentTime]);

  const stopPolling = useCallback(() => {
    if (intervalRef.current) {
      clearInterval(intervalRef.current);
      intervalRef.current = null;
    }
  }, []);

  // cleanup on unmount
  useEffect(() => {
    return () => stopPolling();
  }, [stopPolling]);

  const onReady = (event: YouTubeEvent) => {
    playerRef.current = event.target;
    startPolling();
  };

  const onStateChange = (event: YouTubeEvent) => {
    setPlayerState(event.data as number);

    // 재생 중일 때만 폴링
    if (event.data === 1) {
      startPolling();
    } else if (event.data === 2 || event.data === 0) {
      stopPolling();
    }
  };

  const onError = (event: YouTubeEvent) => {
    console.error("YouTube player error:", event.data);
  };

  return (
    <div className="flex h-full w-full items-center justify-center bg-black">
      <YouTube
        videoId={videoId}
        opts={{
          width: "100%",
          height: "100%",
          playerVars: {
            fs: 0, // YouTube 자체 풀스크린 비활성화
            autoplay: 0,
            enablejsapi: 1,
            rel: 0,
            modestbranding: 1,
          },
        }}
        onReady={onReady}
        onStateChange={onStateChange}
        onError={onError}
        className="h-full w-full"
        iframeClassName="h-full w-full"
      />
    </div>
  );
}
