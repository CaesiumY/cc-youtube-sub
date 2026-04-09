import { create } from "zustand";

interface PlayerState {
  currentTime: number;
  isFullscreen: boolean;
  playerState: number; // YT.PlayerState: -1 unstarted, 0 ended, 1 playing, 2 paused, 3 buffering, 5 cued
  showOriginal: boolean; // T키 토글: 원본 자막 표시
  subtitleSize: number; // 자막 폰트 크기 (rem)

  setCurrentTime: (time: number) => void;
  setFullscreen: (value: boolean) => void;
  setPlayerState: (state: number) => void;
  toggleOriginal: () => void;
  increaseSubtitleSize: () => void;
  decreaseSubtitleSize: () => void;
}

const SUBTITLE_SIZE_MIN = 0.875;
const SUBTITLE_SIZE_MAX = 2.0;
const SUBTITLE_SIZE_STEP = 0.125;

export const usePlayerStore = create<PlayerState>((set) => ({
  currentTime: 0,
  isFullscreen: false,
  playerState: -1,
  showOriginal: false,
  subtitleSize: 1.25, // --subtitle-size 기본값 (20px)

  setCurrentTime: (time) => set({ currentTime: time }),
  setFullscreen: (value) => set({ isFullscreen: value }),
  setPlayerState: (state) => set({ playerState: state }),
  toggleOriginal: () => set((s) => ({ showOriginal: !s.showOriginal })),
  increaseSubtitleSize: () =>
    set((s) => ({
      subtitleSize: Math.min(
        s.subtitleSize + SUBTITLE_SIZE_STEP,
        SUBTITLE_SIZE_MAX,
      ),
    })),
  decreaseSubtitleSize: () =>
    set((s) => ({
      subtitleSize: Math.max(
        s.subtitleSize - SUBTITLE_SIZE_STEP,
        SUBTITLE_SIZE_MIN,
      ),
    })),
}));
