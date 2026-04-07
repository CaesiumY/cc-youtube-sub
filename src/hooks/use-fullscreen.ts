import { getCurrentWindow } from "@tauri-apps/api/window";
import { useCallback, useEffect } from "react";
import { usePlayerStore } from "../stores/player-store";

export function useFullscreen() {
  const setFullscreen = usePlayerStore((s) => s.setFullscreen);

  const toggleFullscreen = useCallback(async () => {
    try {
      const win = getCurrentWindow();
      const isFs = await win.isFullscreen();
      await win.setFullscreen(!isFs);
      setFullscreen(!isFs);
    } catch (e) {
      // Tauri API 없는 환경 (브라우저 개발 모드)에서는 무시
      console.warn("Fullscreen toggle failed:", e);
    }
  }, [setFullscreen]);

  useEffect(() => {
    const handleKeydown = (e: KeyboardEvent) => {
      // 입력 필드에서는 무시
      if (
        e.target instanceof HTMLInputElement ||
        e.target instanceof HTMLTextAreaElement
      ) {
        return;
      }

      if (e.key === "f" || e.key === "F") {
        toggleFullscreen();
      }
    };
    window.addEventListener("keydown", handleKeydown);
    return () => window.removeEventListener("keydown", handleKeydown);
  }, [toggleFullscreen]);
}
