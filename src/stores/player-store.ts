import { create } from "zustand";

interface PlayerState {
  currentTime: number;
  isFullscreen: boolean;
  playerState: number; // YT.PlayerState: -1 unstarted, 0 ended, 1 playing, 2 paused, 3 buffering, 5 cued

  setCurrentTime: (time: number) => void;
  setFullscreen: (value: boolean) => void;
  setPlayerState: (state: number) => void;
}

export const usePlayerStore = create<PlayerState>((set) => ({
  currentTime: 0,
  isFullscreen: false,
  playerState: -1,

  setCurrentTime: (time) => set({ currentTime: time }),
  setFullscreen: (value) => set({ isFullscreen: value }),
  setPlayerState: (state) => set({ playerState: state }),
}));
