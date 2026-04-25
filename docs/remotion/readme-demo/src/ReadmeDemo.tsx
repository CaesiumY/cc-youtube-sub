import { AbsoluteFill, interpolate, useCurrentFrame } from "remotion";
import { AppPreview } from "./AppPreview";

export function ReadmeDemo() {
  const frame = useCurrentFrame();
  const progress = interpolate(frame, [180, 690], [0.18, 0.9], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  const mode = frame < 150 ? "home" : frame < 610 ? "player" : "cache";
  const showOriginal = frame > 330;

  return (
    <AbsoluteFill style={{ background: "#030712" }}>
      <AppPreview mode={mode} progress={progress} showOriginal={showOriginal} />
      <div
        style={{
          position: "absolute",
          left: 110,
          top: 70,
          fontFamily:
            "Pretendard, Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, sans-serif",
          color: "#fafafa",
        }}
      >
        <div style={{ fontSize: 50, fontWeight: 900, letterSpacing: 0 }}>
          YouTube Subtitle Translator
        </div>
        <div style={{ marginTop: 12, fontSize: 22, color: "#cbd5e1" }}>
          Claude Code CLI로 YouTube 자막을 실시간 한국어 오버레이로 번역
        </div>
      </div>
    </AbsoluteFill>
  );
}
