# README Showcase Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** README를 Claude Code CLI 기반 YouTube 자막 번역 앱의 쇼케이스형 소개 문서로 개선하고, README용 스크린샷과 Remotion 데모 영상을 추가한다.

**Architecture:** README는 제품 첫인상과 빠른 실행 경로를 상단에 두고, 상세 기술 설명은 하단과 `docs/` 링크로 이동한다. 시각 자산은 `docs/assets/readme/`에 저장하고, Remotion 소스는 앱 런타임과 분리된 `docs/remotion/readme-demo/` 워크스페이스 패키지로 관리한다. 실제 앱 캡처를 우선 시도하되 실패하면 Remotion 코드 기반 프리뷰로 같은 파일명을 생성한다.

**Tech Stack:** Markdown, pnpm workspace, Remotion 4, React 19, TypeScript 5.7, Vite mock-tauri browser mode, Biome.

---

## File Structure

- Modify: `.gitignore`
  - `.superpowers/` visual companion 임시 산출물을 Git에서 제외한다.
- Modify: `pnpm-workspace.yaml`
  - `docs/remotion/*` 문서용 Remotion 패키지를 pnpm workspace에 포함한다.
- Modify: `pnpm-lock.yaml`
  - Remotion workspace 의존성 설치 결과를 반영한다.
- Modify: `README.md`
  - 쇼케이스형 상단, 데모 섹션, 빠른 시작, 기능/작동 방식/기술 스택/문서 링크를 재구성한다.
- Create: `docs/assets/readme/hero.png`
  - README 상단 대표 이미지. 실제 캡처 성공 시 실제 캡처 기반, 실패 시 Remotion 프리뷰 still.
- Create: `docs/assets/readme/home.png`
  - URL 입력 화면. 실제 캡처 우선, 실패하면 Remotion 프리뷰 still.
- Create: `docs/assets/readme/player.png`
  - 플레이어와 자막 오버레이 화면. 실제 캡처 우선, 실패하면 Remotion 프리뷰 still.
- Create: `docs/assets/readme/demo.mp4`
  - 20-30초 README 데모 영상.
- Create: `docs/assets/readme/demo.gif`
  - GitHub README에서 바로 보이는 fallback 데모 영상.
- Create: `docs/remotion/readme-demo/package.json`
  - 문서용 Remotion 패키지와 렌더 스크립트.
- Create: `docs/remotion/readme-demo/tsconfig.json`
  - Remotion 패키지 TypeScript 설정.
- Create: `docs/remotion/readme-demo/src/index.ts`
  - Remotion root 등록 진입점.
- Create: `docs/remotion/readme-demo/src/Root.tsx`
  - 데모 영상과 still composition 등록.
- Create: `docs/remotion/readme-demo/src/copy.ts`
  - README와 영상이 공유할 제품 문구.
- Create: `docs/remotion/readme-demo/src/AppPreview.tsx`
  - 실제 캡처 실패 시 사용할 코드 기반 앱 화면 프리뷰.
- Create: `docs/remotion/readme-demo/src/ReadmeDemo.tsx`
  - 20-30초 데모 영상 composition.
- Create: `docs/remotion/readme-demo/src/StillFrames.tsx`
  - `hero.png`, `home.png`, `player.png` fallback still composition.

## Task 1: Ignore Local Brainstorm Artifacts

**Files:**
- Modify: `.gitignore`

- [ ] **Step 1: Confirm current untracked artifact**

Run:

```bash
git status --short
```

Expected: output includes `?? .superpowers/` if visual companion artifacts still exist.

- [ ] **Step 2: Add `.superpowers/` to `.gitignore`**

Append this block under the existing `# OMC` section:

```gitignore

# Superpowers local brainstorm artifacts
.superpowers/
```

- [ ] **Step 3: Verify `.superpowers/` is ignored**

Run:

```bash
git status --short
```

Expected: output no longer includes `?? .superpowers/`. Existing unrelated `?? .codex` and `?? AGENTS.md` may remain.

- [ ] **Step 4: Commit artifact ignore change**

Run:

```bash
git add .gitignore
git commit -m "chore: Superpowers 임시 산출물 ignore 추가" -m "README 쇼케이스 작업 중 생성되는 로컬 brainstorm 산출물을 Git에서 제외한다.

- .superpowers/ 디렉터리 ignore 추가
- 기존 미추적 AGENTS.md, .codex는 범위 밖이라 유지

검증: git status --short"
```

Expected: commit succeeds. If the repository hook runs Rust checks, wait for completion and verify exit code 0.

## Task 2: Scaffold The Remotion Workspace Package

**Files:**
- Modify: `pnpm-workspace.yaml`
- Create: `docs/remotion/readme-demo/package.json`
- Create: `docs/remotion/readme-demo/tsconfig.json`
- Create: `docs/remotion/readme-demo/src/index.ts`
- Create: `docs/remotion/readme-demo/src/Root.tsx`
- Create: `docs/remotion/readme-demo/src/copy.ts`
- Create: `docs/remotion/readme-demo/src/AppPreview.tsx`
- Create: `docs/remotion/readme-demo/src/ReadmeDemo.tsx`
- Create: `docs/remotion/readme-demo/src/StillFrames.tsx`

- [ ] **Step 1: Update workspace package list**

Replace `pnpm-workspace.yaml` with:

```yaml
packages:
  - docs/remotion/*
onlyBuiltDependencies:
  - '@biomejs/biome'
  - esbuild
```

- [ ] **Step 2: Create Remotion package manifest**

Create `docs/remotion/readme-demo/package.json`:

```json
{
  "name": "readme-demo",
  "private": true,
  "version": "0.0.0",
  "type": "module",
  "scripts": {
    "render:hero": "remotion still src/index.ts HeroStill ../../assets/readme/hero.png --overwrite",
    "render:home": "remotion still src/index.ts HomeStill ../../assets/readme/home.png --overwrite",
    "render:player": "remotion still src/index.ts PlayerStill ../../assets/readme/player.png --overwrite",
    "render:stills": "pnpm render:hero && pnpm render:home && pnpm render:player",
    "render:video": "remotion render src/index.ts ReadmeDemo ../../assets/readme/demo.mp4 --codec=h264 --overwrite",
    "render:gif": "remotion render src/index.ts ReadmeDemo ../../assets/readme/demo.gif --codec=gif --every-nth-frame=2 --overwrite",
    "render": "pnpm render:stills && pnpm render:video && pnpm render:gif"
  },
  "dependencies": {
    "react": "^19",
    "react-dom": "^19",
    "remotion": "4.0.350"
  },
  "devDependencies": {
    "@remotion/cli": "4.0.350",
    "@types/react": "^19",
    "@types/react-dom": "^19",
    "typescript": "^5.7"
  }
}
```

- [ ] **Step 3: Create TypeScript config**

Create `docs/remotion/readme-demo/tsconfig.json`:

```json
{
  "compilerOptions": {
    "allowSyntheticDefaultImports": true,
    "jsx": "react-jsx",
    "lib": ["DOM", "DOM.Iterable", "ES2022"],
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "noEmit": true,
    "strict": true,
    "target": "ES2022"
  },
  "include": ["src"]
}
```

- [ ] **Step 4: Create Remotion entrypoint**

Create `docs/remotion/readme-demo/src/index.ts`:

```ts
import { registerRoot } from "remotion";
import { RemotionRoot } from "./Root";

registerRoot(RemotionRoot);
```

- [ ] **Step 5: Create shared copy**

Create `docs/remotion/readme-demo/src/copy.ts`:

```ts
export const appName = "YouTube Subtitle Translator";

export const valueProps = [
  {
    title: "실시간 오버레이",
    body: "YouTube 플레이어 위에 한국어 번역 자막을 자연스럽게 표시",
  },
  {
    title: "Claude Code CLI",
    body: "구독 중인 CLI를 subprocess로 실행해 별도 API 키 없이 번역",
  },
  {
    title: "SQLite 캐시",
    body: "한 번 번역한 청크는 로컬에 저장해 재방문 시 즉시 재사용",
  },
] as const;

export const sampleLines = [
  {
    original: "Today we are going to look at how local-first apps can feel fast.",
    translated:
      "오늘은 로컬 우선 앱이 어떻게 빠르게 느껴질 수 있는지 살펴보겠습니다.",
  },
  {
    original: "The key is to translate ahead of the playback position.",
    translated: "핵심은 재생 위치보다 앞선 자막을 미리 번역해두는 것입니다.",
  },
] as const;
```

- [ ] **Step 6: Create app preview component**

Create `docs/remotion/readme-demo/src/AppPreview.tsx`:

```tsx
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
      <div style={{ marginTop: 7, fontSize: 14, color: "#c4c4cc", lineHeight: 1.45 }}>
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
```

- [ ] **Step 7: Create video composition**

Create `docs/remotion/readme-demo/src/ReadmeDemo.tsx`:

```tsx
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
      <AppPreview
        mode={mode}
        progress={progress}
        showOriginal={showOriginal}
      />
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
```

- [ ] **Step 8: Create still compositions**

Create `docs/remotion/readme-demo/src/StillFrames.tsx`:

```tsx
import { AbsoluteFill } from "remotion";
import { AppPreview } from "./AppPreview";

export function HeroStill() {
  return (
    <AbsoluteFill style={{ background: "#030712" }}>
      <AppPreview mode="player" progress={0.72} showOriginal />
    </AbsoluteFill>
  );
}

export function HomeStill() {
  return (
    <AbsoluteFill style={{ background: "#030712" }}>
      <AppPreview mode="home" progress={0} />
    </AbsoluteFill>
  );
}

export function PlayerStill() {
  return (
    <AbsoluteFill style={{ background: "#030712" }}>
      <AppPreview mode="player" progress={0.58} showOriginal />
    </AbsoluteFill>
  );
}
```

- [ ] **Step 9: Register compositions**

Create `docs/remotion/readme-demo/src/Root.tsx`:

```tsx
import { Composition } from "remotion";
import { ReadmeDemo } from "./ReadmeDemo";
import { HeroStill, HomeStill, PlayerStill } from "./StillFrames";

const width = 1600;
const height = 1000;
const fps = 30;

export function RemotionRoot() {
  return (
    <>
      <Composition
        id="ReadmeDemo"
        component={ReadmeDemo}
        durationInFrames={720}
        fps={fps}
        width={width}
        height={height}
      />
      <Composition
        id="HeroStill"
        component={HeroStill}
        durationInFrames={1}
        fps={fps}
        width={width}
        height={height}
      />
      <Composition
        id="HomeStill"
        component={HomeStill}
        durationInFrames={1}
        fps={fps}
        width={width}
        height={height}
      />
      <Composition
        id="PlayerStill"
        component={PlayerStill}
        durationInFrames={1}
        fps={fps}
        width={width}
        height={height}
      />
    </>
  );
}
```

- [ ] **Step 10: Install workspace dependencies**

Run:

```bash
pnpm install
```

Expected: `pnpm-lock.yaml` is updated and `docs/remotion/readme-demo` dependencies are installed. If the command fails due to network restrictions, rerun the same command with sandbox/network approval.

- [ ] **Step 11: Format Remotion package files**

Run:

```bash
pnpm exec biome check --write pnpm-workspace.yaml docs/remotion/readme-demo
```

Expected: Biome writes formatting changes if needed and exits 0.

- [ ] **Step 12: Type-check Remotion package**

Run:

```bash
pnpm --filter readme-demo exec tsc --noEmit
```

Expected: exit code 0.

- [ ] **Step 13: Commit Remotion scaffold**

Run:

```bash
git add pnpm-workspace.yaml pnpm-lock.yaml docs/remotion/readme-demo
git commit -m "docs: README 데모 Remotion 프로젝트 추가" -m "README 쇼케이스 영상과 fallback 스크린샷을 생성할 문서용 Remotion 패키지를 추가한다.

- docs/remotion/readme-demo 워크스페이스 패키지 생성
- 코드 기반 앱 프리뷰와 데모/still composition 추가
- 앱 런타임 의존성과 분리된 문서용 렌더 스크립트 구성

검증: pnpm exec biome check --write pnpm-workspace.yaml docs/remotion/readme-demo; pnpm --filter readme-demo exec tsc --noEmit"
```

Expected: commit succeeds. If the repository hook runs Rust checks, wait for completion and verify exit code 0.

## Task 3: Try Actual App Screenshots

**Files:**
- Create or overwrite: `docs/assets/readme/home.png`
- Create or overwrite: `docs/assets/readme/player.png`
- Create or overwrite: `docs/assets/readme/hero.png`

- [ ] **Step 1: Create README asset directory**

Run:

```bash
mkdir -p docs/assets/readme
```

Expected: directory exists.

- [ ] **Step 2: Start Vite mock-tauri dev server**

Run:

```bash
pnpm dev --host 127.0.0.1
```

Expected: Vite prints a local URL such as `http://127.0.0.1:5173/`. Keep the process running while taking screenshots.

- [ ] **Step 3: Capture Home screen if browser access works**

Use the in-app browser or browser automation to open:

```text
http://127.0.0.1:5173/#/
```

Save a 1600x1000 or larger screenshot to:

```text
docs/assets/readme/home.png
```

Expected: screenshot shows the URL input and model selector.

- [ ] **Step 4: Capture Player screen if browser access works**

Use the in-app browser or browser automation to open:

```text
http://127.0.0.1:5173/#/watch/dQw4w9WgXcQ
```

Save a 1600x1000 or larger screenshot to:

```text
docs/assets/readme/player.png
```

Expected: screenshot shows the app player route. If YouTube iframe or network loading is blocked, stop the dev server and continue to Task 4 fallback rendering.

- [ ] **Step 5: Copy or crop hero image from the best screenshot**

If `player.png` is a usable app screenshot, copy it to:

```text
docs/assets/readme/hero.png
```

Run:

```bash
cp docs/assets/readme/player.png docs/assets/readme/hero.png
```

Expected: `hero.png` exists and is the same visual state as `player.png`.

- [ ] **Step 6: Verify actual screenshot files**

Run:

```bash
test -f docs/assets/readme/home.png && test -f docs/assets/readme/player.png && test -f docs/assets/readme/hero.png
```

Expected: exit code 0 when actual capture succeeded. If exit code is non-zero, continue to Task 4 to generate fallback stills.

## Task 4: Render Fallback Assets And Demo Video

**Files:**
- Create or overwrite: `docs/assets/readme/hero.png`
- Create or overwrite: `docs/assets/readme/home.png`
- Create or overwrite: `docs/assets/readme/player.png`
- Create: `docs/assets/readme/demo.mp4`
- Create: `docs/assets/readme/demo.gif`

- [ ] **Step 1: Render fallback stills when actual screenshots are missing**

Run this only if Task 3 Step 6 failed:

```bash
pnpm --filter readme-demo render:stills
```

Expected: `hero.png`, `home.png`, and `player.png` are created under `docs/assets/readme/`.

- [ ] **Step 2: Render MP4 demo**

Run:

```bash
pnpm --filter readme-demo render:video
```

Expected: `docs/assets/readme/demo.mp4` is created.

- [ ] **Step 3: Render GIF demo**

Run:

```bash
pnpm --filter readme-demo render:gif
```

Expected: `docs/assets/readme/demo.gif` is created.

- [ ] **Step 4: Verify all README assets exist**

Run:

```bash
test -f docs/assets/readme/hero.png && test -f docs/assets/readme/home.png && test -f docs/assets/readme/player.png && test -f docs/assets/readme/demo.mp4 && test -f docs/assets/readme/demo.gif
```

Expected: exit code 0.

- [ ] **Step 5: Commit README assets**

Run:

```bash
git add docs/assets/readme
git commit -m "docs: README 쇼케이스 자산 추가" -m "README 상단과 데모 섹션에서 사용할 이미지와 영상 자산을 추가한다.

- hero/home/player 프리뷰 이미지 추가
- Remotion 기반 demo.mp4와 demo.gif 추가
- 실제 캡처가 실패한 경우 코드 기반 프리뷰로 동일 파일명 생성

검증: test -f docs/assets/readme/hero.png && test -f docs/assets/readme/home.png && test -f docs/assets/readme/player.png && test -f docs/assets/readme/demo.mp4 && test -f docs/assets/readme/demo.gif"
```

Expected: commit succeeds. If the repository hook runs Rust checks, wait for completion and verify exit code 0.

## Task 5: Rewrite README As A Product Showcase

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Replace README contents**

Replace `README.md` with:

```markdown
# YouTube Subtitle Translator

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Tauri v2](https://img.shields.io/badge/Tauri-v2-blue?logo=tauri)](https://v2.tauri.app)
[![Rust](https://img.shields.io/badge/Rust-2021-orange?logo=rust)](https://www.rust-lang.org)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.7-blue?logo=typescript)](https://www.typescriptlang.org)

YouTube 영상의 자막을 **Claude Code CLI**로 실시간 번역해 영상 위에 한국어 오버레이로 보여주는 Tauri 데스크톱 앱입니다.

Claude Code 구독을 활용하므로 별도 API 키나 API 과금 없이, 자막 fetch부터 번역 캐시까지 로컬 앱 안에서 처리합니다.

![App preview](docs/assets/readme/hero.png)

## 왜 이 앱인가

| 가치 | 설명 |
|------|------|
| 실시간 오버레이 | YouTube 플레이어 위에 번역 자막을 자연스럽게 표시합니다 |
| 청크 기반 번역 | 긴 영상도 30초-1분 단위로 나누어 빠르게 번역을 시작합니다 |
| 로컬 캐시 | 한 번 번역한 자막은 SQLite에 저장해 동일 영상 재방문 시 재사용합니다 |
| 별도 API 키 불필요 | Claude Code CLI 로그인 상태를 사용하므로 추가 API 키를 요구하지 않습니다 |

## 데모

![Demo preview](docs/assets/readme/demo.gif)

[MP4 데모 보기](docs/assets/readme/demo.mp4)

| Home | Player |
|------|--------|
| ![Home screenshot](docs/assets/readme/home.png) | ![Player screenshot](docs/assets/readme/player.png) |

## 빠른 시작

### 사전 요구사항

- [Node.js](https://nodejs.org) 18+
- [pnpm](https://pnpm.io)
- [Rust](https://rustup.rs)
- [Tauri 사전 요구사항](https://v2.tauri.app/start/prerequisites/)
- [Claude Code CLI](https://docs.anthropic.com/en/docs/claude-code) 설치 및 로그인 완료

### 실행

```bash
git clone https://github.com/CaesiumY/cc-youtube-sub.git
cd cc-youtube-sub
pnpm install
pnpm tauri dev
```

브라우저 모드에서 프론트엔드만 확인하려면 다음 명령을 사용합니다.

```bash
pnpm dev
```

## 주요 기능

- **실시간 자막 번역**: YouTube 재생 중 Claude Code CLI가 자막을 한국어로 번역합니다.
- **자막 오버레이**: 영상 위에 반투명 자막 박스를 표시하고 원문/번역을 토글할 수 있습니다.
- **사전 버퍼링**: 재생 위치 앞 청크를 미리 번역해 시청 중 끊김을 줄입니다.
- **SQLite 캐시**: `(video_id, chunk_hash)` 기준으로 번역 결과를 저장합니다.
- **긴 영상 대응**: 자막을 30초-1분 단위로 나누어 긴 강의나 발표 영상도 처리합니다.
- **데스크톱 UX**: Tauri v2 기반으로 Windows 우선 데스크톱 앱 경험을 제공합니다.

## 작동 방식

```text
YouTube URL 입력
  -> 자막 fetch
  -> 30초-1분 청크 분할
  -> SQLite 캐시 확인
  -> 캐시 miss 청크를 Claude Code CLI subprocess로 번역
  -> JSON 결과 검증
  -> 캐시 저장
  -> 영상 위 자막 오버레이 표시
```

재생 위치가 바뀌면 Rust BufferManager가 현재 위치 기준으로 우선 번역할 청크를 다시 계산합니다. 캐시 hit 위치는 즉시 표시하고, 캐시 miss 위치는 번역 준비 상태를 보여준 뒤 완료되는 즉시 자막을 갱신합니다.

## 기술 스택

| 영역 | 기술 |
|------|------|
| Desktop | Tauri v2, Rust 2021, Tokio |
| Frontend | React 19, TypeScript 5.7, Vite 6 |
| Routing / State | TanStack Router, TanStack Query, Zustand |
| Styling | Tailwind CSS v4, Pretendard, Motion |
| Translation | Claude Code CLI subprocess, stream-json parsing |
| Subtitle / Cache | yt-transcript-rs, quick-xml, rusqlite |

## 개발 명령어

```bash
# Tauri 앱 + Vite dev server
pnpm tauri dev

# 프론트엔드만 실행
pnpm dev

# 프로덕션 빌드
pnpm tauri build

# 린트와 포맷 검사
pnpm check

# 프론트엔드 테스트
pnpm test

# Rust 테스트
cd src-tauri && cargo test

# Rust 린트
cd src-tauri && cargo clippy
```

## 프로젝트 구조

```text
cc-youtube-sub/
├── src/                        # React 프론트엔드
│   ├── routes/                 # Home, Player 라우트
│   ├── components/             # URL 입력, YouTube Player, 자막 오버레이
│   ├── hooks/                  # 번역 파이프라인, 버퍼링, 단축키
│   ├── stores/                 # Zustand UI 상태
│   └── lib/                    # Tauri command wrapper, mock-tauri, 유틸리티
├── src-tauri/                  # Rust 백엔드
│   └── src/
│       ├── subtitle/           # 자막 fetch, 파싱, 청크 분할
│       ├── translate/          # 프롬프트, JSONL 파싱, 검증
│       ├── claude/             # Claude Code CLI 프로세스 관리
│       ├── cache.rs            # SQLite 캐시
│       └── buffer_manager.rs   # 재생 위치 기반 사전 번역
├── docs/                       # PRD, 설계 문서, 테스트 전략
└── docs/assets/readme/         # README 이미지와 데모 영상
```

## 키보드 단축키

| 키 | 동작 |
|----|------|
| `T` | 원본 자막 / 번역 토글 |
| `F` | 풀스크린 |
| `+` / `-` | 자막 크기 조절 |
| `Space` | 재생 / 일시정지 |
| `←` | 홈으로 돌아가기 |

## 문서

- [PRD](docs/prd.md)
- [기술 스택](docs/tech-stack.md)
- [테스트 전략](docs/test-strategy.md)
- [디자인 시스템](docs/design-system.md)
- [Phase 계획](docs/phases/overview.md)

## 라이선스

[MIT](LICENSE) &copy; 2026 ChangSik Yoon
```

- [ ] **Step 2: Verify README does not mention Codex**

Run:

```bash
rg -n "Codex|codex" README.md
```

Expected: no output and exit code 1.

- [ ] **Step 3: Verify README asset paths exist**

Run:

```bash
test -f docs/assets/readme/hero.png && test -f docs/assets/readme/home.png && test -f docs/assets/readme/player.png && test -f docs/assets/readme/demo.mp4 && test -f docs/assets/readme/demo.gif
```

Expected: exit code 0.

- [ ] **Step 4: Commit README rewrite**

Run:

```bash
git add README.md
git commit -m "docs: README를 쇼케이스형 소개로 개편" -m "README 상단을 제품 가치와 데모 중심으로 재구성한다.

- Claude Code CLI 기반 제품 설명 명확화
- README 자산과 데모 섹션 추가
- 빠른 시작, 기능, 작동 방식, 개발 문서 링크 정리

검증: rg -n \"Codex|codex\" README.md; test -f docs/assets/readme/hero.png && test -f docs/assets/readme/home.png && test -f docs/assets/readme/player.png && test -f docs/assets/readme/demo.mp4 && test -f docs/assets/readme/demo.gif"
```

Expected: commit succeeds. If the repository hook runs Rust checks, wait for completion and verify exit code 0.

## Task 6: Final Verification

**Files:**
- Read: `README.md`
- Read: `docs/assets/readme/*`
- Read: `docs/remotion/readme-demo/*`

- [ ] **Step 1: Run workspace checks**

Run:

```bash
pnpm check
```

Expected: exit code 0.

- [ ] **Step 2: Run frontend tests**

Run:

```bash
pnpm test
```

Expected: all Vitest tests pass.

- [ ] **Step 3: Run Rust tests**

Run:

```bash
cd src-tauri && cargo test
```

Expected: all non-ignored Rust tests pass.

- [ ] **Step 4: Verify Remotion render scripts still work**

Run:

```bash
pnpm --filter readme-demo render:hero
pnpm --filter readme-demo render:video
```

Expected: both commands exit 0 and update `docs/assets/readme/hero.png` and `docs/assets/readme/demo.mp4`.

- [ ] **Step 5: Check for accidental product Codex mention**

Run:

```bash
rg -n "Codex|codex" README.md docs/remotion/readme-demo
```

Expected: no output and exit code 1.

- [ ] **Step 6: Check staged diff cleanliness**

Run:

```bash
git status --short
git diff --check
```

Expected: `git diff --check` exits 0. `git status --short` shows only intentional files if any render command updated tracked assets after the previous commits.

- [ ] **Step 7: Commit final render updates if verification changed assets**

Run this only if Task 6 Step 6 shows modified README assets:

```bash
git add docs/assets/readme
git commit -m "docs: README 데모 자산 최종 렌더 반영" -m "최종 검증 중 재렌더된 README 데모 자산을 반영한다.

- Remotion hero/video 렌더 결과 갱신
- README 참조 경로 유지

검증: pnpm check; pnpm test; cd src-tauri && cargo test; pnpm --filter readme-demo render:hero; pnpm --filter readme-demo render:video"
```

Expected: commit succeeds. If the repository hook runs Rust checks, wait for completion and verify exit code 0.

- [ ] **Step 8: Report completion**

Summarize:

```text
- README showcase rewrite completed.
- Assets: docs/assets/readme/hero.png, home.png, player.png, demo.mp4, demo.gif.
- Remotion source: docs/remotion/readme-demo/.
- Verification: pnpm check, pnpm test, cd src-tauri && cargo test, Remotion render commands.
- Remaining untracked files were not touched unless they were part of this plan.
```
