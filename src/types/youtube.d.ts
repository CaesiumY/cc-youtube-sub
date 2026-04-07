// YouTube IFrame API 글로벌 타입 선언
declare namespace YT {
  interface Player {
    getCurrentTime(): number;
    getDuration(): number;
    getPlayerState(): number;
    playVideo(): void;
    pauseVideo(): void;
    seekTo(seconds: number, allowSeekAhead: boolean): void;
    destroy(): void;
  }

  enum PlayerState {
    UNSTARTED = -1,
    ENDED = 0,
    PLAYING = 1,
    PAUSED = 2,
    BUFFERING = 3,
    CUED = 5,
  }
}
