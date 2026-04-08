import { useEffect } from "react";
import { usePlayerStore } from "../stores/player-store";

/**
 * 자막 관련 키보드 단축키 훅
 *
 * | 키       | 동작             |
 * |----------|------------------|
 * | T        | 원본 자막 토글   |
 * | + / =    | 폰트 크기 증가   |
 * | -        | 폰트 크기 감소   |
 * | Space    | 재생/일시정지     |
 *
 * Space는 playerRef를 직접 제어하므로 ref를 외부에서 전달받는다.
 */
export function useKeyboardShortcuts(
  playerRef?: React.RefObject<YT.Player | null>,
) {
  const toggleOriginal = usePlayerStore((s) => s.toggleOriginal);
  const increaseSize = usePlayerStore((s) => s.increaseSubtitleSize);
  const decreaseSize = usePlayerStore((s) => s.decreaseSubtitleSize);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      // input/textarea에서는 동작하지 않음
      const target = e.target as HTMLElement;
      if (
        target.tagName === "INPUT" ||
        target.tagName === "TEXTAREA" ||
        target.isContentEditable
      ) {
        return;
      }

      switch (e.key) {
        case "t":
        case "T":
          e.preventDefault();
          toggleOriginal();
          break;
        case "+":
        case "=":
          e.preventDefault();
          increaseSize();
          break;
        case "-":
          e.preventDefault();
          decreaseSize();
          break;
        case " ":
          e.preventDefault();
          if (playerRef?.current) {
            const state = playerRef.current.getPlayerState();
            if (state === 1) {
              playerRef.current.pauseVideo();
            } else {
              playerRef.current.playVideo();
            }
          }
          break;
      }
    };

    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [toggleOriginal, increaseSize, decreaseSize, playerRef]);
}
