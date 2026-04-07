import { usePlayerStore } from "../stores/player-store";

export function ProgressBar() {
  const currentTime = usePlayerStore((s) => s.currentTime);

  // Phase 0에서는 placeholder — 실제 duration 연동은 Phase 1+
  // 임시로 currentTime 기반 시각적 표시만 제공
  const progress = Math.min((currentTime / 300) * 100, 100); // 5분 기준 임시

  return (
    <div
      className="h-[2px] w-full"
      style={{ background: "var(--progress-track)" }}
    >
      <div
        className="h-full transition-[width] duration-500 ease-linear"
        style={{
          width: `${progress}%`,
          background: "var(--progress-fill)",
        }}
      />
    </div>
  );
}
