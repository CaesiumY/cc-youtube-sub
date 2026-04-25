import type { CSSProperties } from "react";
import { appName, sampleLines, valueProps } from "./copy";

type PreviewMode = "home" | "player" | "cache";

type AppPreviewProps = {
  mode: PreviewMode;
  progress: number;
  showOriginal?: boolean;
};

const shellStyle: CSSProperties = {
  width: 1280,
  height: 760,
  borderRadius: 22,
  background: "#09090b",
  border: "1px solid rgba(255,255,255,0.12)",
  boxShadow: "0 34px 90px rgba(0,0,0,0.45)",
  overflow: "hidden",
  color: "#fafafa",
  fontFamily:
    "Pretendard, Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, sans-serif",
};

const topBarStyle: CSSProperties = {
  height: 42,
  display: "flex",
  alignItems: "center",
  gap: 8,
  padding: "0 18px",
  background: "#18181b",
  borderBottom: "1px solid rgba(255,255,255,0.08)",
};

const dotStyle = (background: string): CSSProperties => ({
  width: 11,
  height: 11,
  borderRadius: 999,
  background,
});

function WindowChrome() {
  return (
    <div style={topBarStyle}>
      <div style={dotStyle("#ef4444")} />
      <div style={dotStyle("#f59e0b")} />
      <div style={dotStyle("#22c55e")} />
      <div style={{ marginLeft: 14, fontSize: 13, color: "#a1a1aa" }}>
        {appName}
      </div>
    </div>
  );
}

function HomePreview() {
  return (
    <div style={{ display: "grid", placeItems: "center", height: 718 }}>
      <div style={{ width: 620, textAlign: "center" }}>
        <div style={{ marginBottom: 26, fontSize: 20, color: "#d4d4d8" }}>
          YouTube URL을 붙여넣으면 번역 자막 시청을 시작합니다
        </div>
        <div
          style={{
            display: "flex",
            alignItems: "center",
            height: 58,
            borderRadius: 12,
            border: "1px solid rgba(255,255,255,0.18)",
            background: "#111113",
            padding: "0 20px",
            color: "#f4f4f5",
            fontSize: 17,
            boxShadow: "0 18px 50px rgba(0,0,0,0.28)",
          }}
        >
          https://www.youtube.com/watch?v=demo
        </div>
        <div style={{ marginTop: 20, fontSize: 14, color: "#71717a" }}>
          Model: claude-sonnet-4.5
        </div>
      </div>
    </div>
  );
}

function PlayerPreview({
  progress,
  showOriginal,
  mode,
}: {
  progress: number;
  showOriginal?: boolean;
  mode: PreviewMode;
}) {
  const line = mode === "cache" ? sampleLines[1] : sampleLines[0];

  return (
    <div style={{ position: "relative", height: 718, background: "#050505" }}>
      <div
        style={{
          position: "absolute",
          inset: 0,
          background:
            "linear-gradient(140deg, #111827 0%, #18181b 42%, #312e81 100%)",
        }}
      />
      <div
        style={{
          position: "absolute",
          inset: 0,
          opacity: 0.36,
          backgroundImage:
            "linear-gradient(rgba(255,255,255,0.08) 1px, transparent 1px), linear-gradient(90deg, rgba(255,255,255,0.08) 1px, transparent 1px)",
          backgroundSize: "54px 54px",
        }}
      />
      <div
        style={{
          position: "absolute",
          left: 22,
          top: 22,
          width: 44,
          height: 44,
          borderRadius: 10,
          display: "grid",
          placeItems: "center",
          background: "rgba(0,0,0,0.42)",
          color: "#fafafa",
          fontSize: 24,
        }}
      >
        ←
      </div>
      <div
        style={{
          position: "absolute",
          left: "50%",
          bottom: 118,
          transform: "translateX(-50%)",
          width: 860,
          borderRadius: 14,
          background: "rgba(0,0,0,0.82)",
          padding: "26px 34px",
          textAlign: "center",
          boxShadow: "0 20px 70px rgba(0,0,0,0.35)",
        }}
      >
        <div style={{ fontSize: 30, lineHeight: 1.55, fontWeight: 700 }}>
          {line.translated}
        </div>
        {showOriginal && (
          <div
            style={{
              marginTop: 14,
              fontSize: 17,
              lineHeight: 1.45,
              color: "#a1a1aa",
            }}
          >
            {line.original}
          </div>
        )}
      </div>
      <div
        style={{
          position: "absolute",
          left: 0,
          right: 0,
          bottom: 0,
          height: 10,
          background: "#27272a",
        }}
      >
        <div
          style={{
            width: `${Math.round(progress * 100)}%`,
            height: "100%",
            background: "#e4e4e7",
          }}
        />
      </div>
      <div
        style={{
          position: "absolute",
          right: 24,
          bottom: 28,
          borderRadius: 9,
          background: "rgba(0,0,0,0.58)",
          color: "#d4d4d8",
          padding: "10px 14px",
          fontSize: 14,
          fontFamily: "JetBrains Mono, ui-monospace, SFMono-Regular, monospace",
        }}
      >
        {mode === "cache" ? "cached: 8 / 8" : "translated: 5 / 8"}
      </div>
    </div>
  );
}

function Callout({
  title,
  body,
  left,
  top,
}: {
  title: string;
  body: string;
  left: number;
  top: number;
}) {
  return (
    <div
      style={{
        position: "absolute",
        left,
        top,
        width: 300,
        borderRadius: 14,
        border: "1px solid rgba(255,255,255,0.15)",
        background: "rgba(24,24,27,0.9)",
        padding: "18px 20px",
        boxShadow: "0 22px 60px rgba(0,0,0,0.36)",
      }}
    >
      <div style={{ fontSize: 18, fontWeight: 800 }}>{title}</div>
      <div
        style={{
          marginTop: 7,
          fontSize: 14,
          color: "#c4c4cc",
          lineHeight: 1.45,
        }}
      >
        {body}
      </div>
    </div>
  );
}

export function AppPreview({ mode, progress, showOriginal }: AppPreviewProps) {
  return (
    <div style={{ position: "relative", width: 1600, height: 1000 }}>
      <div style={{ position: "absolute", left: 160, top: 92, ...shellStyle }}>
        <WindowChrome />
        {mode === "home" ? (
          <HomePreview />
        ) : (
          <PlayerPreview
            mode={mode}
            progress={progress}
            showOriginal={showOriginal}
          />
        )}
      </div>
      {mode !== "home" && (
        <>
          <Callout {...valueProps[0]} left={1050} top={150} />
          <Callout {...valueProps[1]} left={88} top={660} />
          <Callout {...valueProps[2]} left={1030} top={700} />
        </>
      )}
    </div>
  );
}
