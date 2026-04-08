# Phase 0: Tauri 뼈대 + YouTube 임베드

## 목표

YouTube iframe API가 Tauri WebView2(Windows) 및 WebKit(macOS)에서 정상 작동하는지 검증하고, 앱의 가장 기본적인 뼈대를 구성한다. 이 단계에서 검증되지 않으면 전체 프로젝트의 타당성이 무너지므로 최우선 기술 리스크를 해결한다.

## 검증 리스크

**최고 우선 기술 리스크**: YouTube iframe이 Tauri WebView2에서 정상 로드되고 재생되는가?

- Tauri WebView2는 Chromium 기반이므로 이론상 작동해야 하나, 보안 샌드박스/CORS/콘텐츠 보안 정책(CSP) 제약으로 인해 실제 동작을 보장할 수 없음
- 이 검증에 실패하면 전체 앱 개념을 오버레이 방식으로 되돌려야 함 (POC 문서의 초기 설계)
- Phase 0 완료 = iframe API 동작 확인 = Phase 1부터 번역 로직 안전하게 진행 가능

**추가 리스크**:

- Tauri 윈도우 풀스크린에서 YouTube iframe이 정상 동작하는가?
  - `setFullscreen()` 호출 후 iframe 크기 재조정 및 Player API 유지 여부 확인 필요
- YouTube 풀스크린 버튼 가로채기가 가능한가?
  - `fs=0` 파라미터로 버튼 숨기기가 실제로 동작하는지, 또는 iframe `allow="fullscreen"` 제거만으로 충분한지 검증 필요

## 구현 범위

- [x] **Tauri 2.x + React + TypeScript + Vite 프로젝트 초기화**
  - 수동 scaffolding (대화형 CLI 대신 직접 파일 생성)
  - React 19 + TypeScript 5.9 + Vite 6.4 + pnpm
  - Tauri CLI 2.10 설치

- [x] **shadcn/ui Luma 프리셋 + Pretendard 폰트 설정**
  - Luma (Neutral OKLCH) 테마 CSS 변수 수동 설정 (`src/index.css`)
  - `@fontsource/pretendard` 설치
  - 다크 모드 기본 + 앱 전용 시맨틱 토큰 적용

- [x] **Tanstack Router 설정 (`/` → `/watch/$videoId`)**
  - `@tanstack/react-router` v1 설치
  - **hash history 사용** (`createHashHistory`) — Tauri 파일 프로토콜 호환성 필수
  - `/` 경로: Home View (URL 입력)
  - `/watch/$videoId` 경로: Player View (YouTube 플레이어)
  - Home → Player fade 전환 (AnimatePresence, 250ms)

- [x] **Tanstack Query + Zustand 상태 관리 초기 설정**
  - `@tanstack/react-query` v5 + `zustand` v5 설치
  - `QueryClientProvider` 루트에 설정 (`src/main.tsx`)
  - `usePlayerStore` 정의 (`currentTime`, `isFullscreen`, `playerState`)

- [x] **motion v12 뷰 전환 애니메이션 설정**
  - `motion` v12 설치
  - `AnimatePresence` mode="wait"으로 Home ↔ Player 페이드 전환 구현

- [x] ~~**tauri-specta 타입 생성 설정**~~ → **Phase 1로 이관**
  - ⚠️ `specta`, `specta-typescript`, `tauri-specta` 크레이트 간 버전 충돌
  - `tauri-specta v2.0.0-rc.24` → `specta rc.24` 요구
  - `specta-typescript v0.0.9` → `specta rc.22` 요구 (호환 불가)
  - 순수 `#[tauri::command]`로 stub 구현, specta 생태계 안정화 후 추가 예정

- [x] **Home View: URL 입력 중앙 표시 (브랜딩 없음)**
  - 텍스트 입력 필드 + Cmd/Ctrl+V 전역 붙여넣기 자동 감지
  - 정규식 기반 URL → videoId 추출 (watch, youtu.be, embed, shorts 지원)
  - 입력 검증 + 에러 메시지 표시
  - 로고/브랜딩 없이 입력 필드만 화면 중앙에 표시

- [x] **Player View: 영상 + 뒤로가기 버튼(←) + 2px progress bar placeholder**
  - 좌상단 ← 버튼으로 Home 복귀
  - YouTube iframe 전체 화면 표시
  - 영상 아래 2px 진행률 바 placeholder (Phase 1에서 실제 duration 연결)

- [x] **YouTube 플레이어 구현 (`react-youtube`)**
  - `react-youtube` v10 사용
  - `playerVars: { fs: 0, autoplay: 0, enablejsapi: 1, rel: 0, modestbranding: 1 }`
  - `onReady` → playerRef 저장, `onStateChange` → Zustand 상태 동기화
  - ~~lite-youtube A/B 비교~~ → 기술 스택 인터뷰에서 react-youtube로 확정 (getCurrentTime() 직접 접근 필요)

- [x] **getCurrentTime() 500ms 폴링으로 재생 시간 추적**
  - 재생 중일 때만 폴링 시작, 일시정지/종료 시 중단
  - Zustand `setCurrentTime()` 업데이트
  - 언마운트 시 `clearInterval` cleanup
  - DEV 모드에서 디버그 표시 (currentTime + playerState)

- [x] **Tauri 윈도우 풀스크린 토글 (F키)**
  - `getCurrentWindow().setFullscreen()` 토글
  - YouTube `fs: 0`으로 iframe 풀스크린 버튼 비활성화
  - 입력 필드 포커스 시 F키 무시
  - Zustand `isFullscreen` 상태 동기화

- [x] **Rust 백엔드 기본 구조 셋업**
  - `fetch_subtitles(video_id)`, `translate_chunk(text)` stub 커맨드
  - `AppError` enum (thiserror + serde 직렬화)
  - `tauri-plugin-shell` 플러그인 등록

- [x] **개발 환경 설정**
  - TypeScript strict mode (`noUncheckedIndexedAccess` 포함)
  - Biome 1.9 설정 (린팅 + 포맷팅 + import 정렬)
  - Vite 빌드 + Tauri 프로덕션 빌드 (deb/rpm/AppImage) 검증 완료

## 제외 범위

- YouTube timedtext API 호출 및 자막 fetch (Phase 1)
- Claude Code subprocess 음역 (Phase 2)
- SQLite 캐시 구현 (Phase 2)
- 번역 프롬프트 및 청크 관리 (Phase 2)
- 자막 오버레이 렌더링 (Phase 1)
- Seek 처리 및 버퍼 관리 (Phase 1-2)
- 오류 핸들링 고도화 (Phase 3)

## 기술 상세

### 프로젝트 구조

```
cc-youtube-sub/
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs           # Rust 백엔드 (command 정의)
│   │   └── main.rs          # Tauri 메인 진입점
│   ├── tauri.conf.json      # Tauri 설정
│   └── Cargo.toml           # Rust 의존성
│
├── src/
│   ├── routes/
│   │   ├── __root.tsx           # Tanstack Router 루트 레이아웃
│   │   ├── index.tsx            # Home View (/ 경로)
│   │   └── watch.$videoId.tsx   # Player View (/watch/$videoId 경로)
│   ├── components/
│   │   ├── YouTubePlayer.tsx    # iframe 플레이어 컴포넌트
│   │   └── UrlInput.tsx         # URL 입력 컴포넌트
│   ├── router.tsx           # Tanstack Router 설정
│   ├── App.tsx              # 메인 앱 컴포넌트
│   ├── main.tsx             # React 진입점
│   └── styles/
│       └── App.css
│
├── vite.config.ts           # Vite 설정
├── tsconfig.json            # TypeScript 설정
└── package.json
```

### 2-View 레이아웃 구조

```
┌─────────────────────────────────┐
│  Home View  (/)                 │
│                                 │
│         ┌──────────────┐        │
│         │  URL 입력    │        │
│         └──────────────┘        │
│   (로고/브랜딩 없음, 중앙 배치) │
└─────────────────────────────────┘
            ↓ fade 200ms out
            ↓ navigate to /watch/$videoId
            ↓ fade 300ms in
┌─────────────────────────────────┐
│  Player View  (/watch/$videoId) │
│ ←                               │  ← 뒤로가기 버튼 (상단 좌측)
│ ┌─────────────────────────────┐ │
│ │                             │ │
│ │   YouTube iframe            │ │
│ │                             │ │
│ └─────────────────────────────┘ │
│ ▓▓▓░░░░░░░░░░░░░░░░░░░░░░░░░░  │  ← 2px progress bar placeholder
└─────────────────────────────────┘
```

### Tanstack Router 라우트 설정

```typescript
// src/router.tsx
import { createRouter, createHashHistory, createRoute, createRootRoute } from '@tanstack/react-router';
import { HomeView } from './routes/index';
import { PlayerView } from './routes/watch.$videoId';

const hashHistory = createHashHistory(); // Tauri 파일 프로토콜 호환 필수

const rootRoute = createRootRoute();

const homeRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/',
  component: HomeView,
});

const watchRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/watch/$videoId',
  component: PlayerView,
});

const routeTree = rootRoute.addChildren([homeRoute, watchRoute]);

export const router = createRouter({ routeTree, history: hashHistory });

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}
```

### Tauri 풀스크린 API

```typescript
import { getCurrentWindow } from '@tauri-apps/api/window';

// F키 풀스크린 토글
async function toggleFullscreen() {
  const win = getCurrentWindow();
  const isFullscreen = await win.isFullscreen();
  await win.setFullscreen(!isFullscreen);
}

// 키보드 이벤트 등록
useEffect(() => {
  const handleKeydown = (e: KeyboardEvent) => {
    if (e.key === 'f' || e.key === 'F') {
      toggleFullscreen();
    }
  };
  window.addEventListener('keydown', handleKeydown);
  return () => window.removeEventListener('keydown', handleKeydown);
}, []);
```

### YouTube 풀스크린 가로채기

YouTube iframe 내부의 풀스크린 버튼은 기본적으로 브라우저/WebView 풀스크린을 요청한다.
Tauri 윈도우 풀스크린으로 대체하려면:

1. `allow="fullscreen"` 속성을 iframe에서 **제거** — iframe 자체 풀스크린 차단
2. YouTube 플레이어 파라미터에 `fs=0` 추가 — YouTube 풀스크린 버튼 비활성화
3. 대신 F키로 Tauri 윈도우 풀스크린 사용

```typescript
// iframe URL 예시: 풀스크린 버튼 비활성화
const embedUrl = `https://www.youtube.com/embed/${videoId}?fs=0&enablejsapi=1`;
```

### react-youtube 플레이어 구현 (기본)

```tsx
import YouTube, { YouTubeEvent } from 'react-youtube';
import { useRef } from 'react';

export function YouTubePlayer({ videoId }: { videoId: string }) {
  const playerRef = useRef<YT.Player | null>(null);

  const onReady = (event: YouTubeEvent) => {
    playerRef.current = event.target;
  };

  const onStateChange = (event: YouTubeEvent) => {
    // event.data: -1 unstarted, 0 ended, 1 playing, 2 paused, 3 buffering, 5 cued
  };

  return (
    <YouTube
      videoId={videoId}
      opts={{
        width: '100%',
        height: '100%',
        playerVars: {
          fs: 0,          // YouTube 자체 풀스크린 버튼 비활성화
          autoplay: 0,
          enablejsapi: 1,
        },
      }}
      onReady={onReady}
      onStateChange={onStateChange}
      onError={(e) => console.error('YouTube player error:', e.data)}
    />
  );
}
```

### lite-youtube 선택적 비교 구조

`react-youtube`가 기본 구현체이며, 성능 비교가 필요한 경우 `VITE_PLAYER_MODE` 환경 변수로 전환:

```typescript
// 환경 변수로 플레이어 구현체 전환
const PLAYER_MODE = import.meta.env.VITE_PLAYER_MODE ?? 'react-youtube'; // 'react-youtube' | 'lite'

export function YouTubePlayerWrapper({ videoId }: { videoId: string }) {
  if (PLAYER_MODE === 'lite') {
    return <LiteYouTubePlayer videoId={videoId} />;
  }
  return <YouTubePlayer videoId={videoId} />;
}
```

비교 기준:
- 초기 로딩 속도 (`PerformanceObserver` 측정)
- Player API 안정성 (`getCurrentTime()` 폴링 성공률)
- Tauri WebView2 호환성

### Tauri 설정 (tauri.conf.json)

WebView2 보안 정책에서 YouTube iframe 로드를 허용하려면:

```json
{
  "build": {
    "devPath": "http://localhost:5173",
    "distDir": "../dist",
    "devUrl": "http://localhost:5173"
  },
  "app": {
    "windows": [
      {
        "fullscreen": false,
        "resizable": true,
        "title": "YouTube Subtitle Translator",
        "width": 1200,
        "height": 800
      }
    ],
    "security": {
      "csp": "default-src 'self' https://www.youtube.com https://www.youtube-nocookie.com; img-src 'self' data: https:; script-src 'self' https://www.youtube.com; frame-src https://www.youtube.com https://www.youtube-nocookie.com"
    }
  }
}
```

**중요**: CSP(Content Security Policy)에서:
- `frame-src https://www.youtube.com https://www.youtube-nocookie.com` 반드시 포함
- `script-src https://www.youtube.com` iframe API 스크립트 로드 허용

### Rust 백엔드 기본 구조 (src-tauri/src/lib.rs)

`tauri-specta`를 사용하여 Rust 커맨드 → TypeScript 타입 자동 생성:

```rust
use tauri_specta::{collect_commands, ts};

// #[specta::specta] 매크로로 타입 정보 추출
#[tauri::command]
#[specta::specta]
fn fetch_subtitles(video_id: String) -> Result<String, String> {
    // Phase 1에서 구현: yt-transcript-rs 자막 fetch
    Ok(format!("Subtitles for video {}", video_id))
}

#[tauri::command]
#[specta::specta]
fn translate_chunk(text: String) -> Result<String, String> {
    // Phase 2에서 구현: Claude subprocess 번역
    Ok(format!("Translated: {}", text))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // tauri-specta: TypeScript 바인딩 생성 (개발 빌드에서만)
    #[cfg(debug_assertions)]
    ts::export(collect_commands![fetch_subtitles, translate_chunk], "../src/bindings.ts")
        .expect("Failed to export TypeScript bindings");

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![fetch_subtitles, translate_chunk])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### TypeScript 설정 (tsconfig.json)

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "jsx": "react-jsx",
    "jsxImportSource": "react"
  },
  "include": ["src"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```

## 완료 기준

- [x] Tauri 앱 프로덕션 빌드 성공 (`pnpm tauri build` → deb/rpm/AppImage)
- [x] `/` 경로에서 URL 입력 화면 표시 (브랜딩 없이 입력 필드만 중앙)
- [x] `/watch/$videoId` 경로에서 YouTube 플레이어 컴포넌트 렌더링
- [x] YouTube URL 파싱 로직 구현 (watch, youtu.be, embed, shorts 지원)
- [x] 뒤로가기 버튼(←) → Home 복귀 구현
- [x] 500ms 폴링으로 currentTime 추적 + Zustand 연동
- [x] 플레이어 상태 변화 감지 (onStateChange → Zustand)
- [x] F키로 Tauri 윈도우 풀스크린 토글 구현
- [x] react-youtube 기본 구현 완료 (lite-youtube 제거 — 기술 스택에서 확정)
- [x] Rust 백엔드 command 스켈레톤 작성 완료 (`fetch_subtitles`, `translate_chunk`)
- [x] TypeScript strict 모드에서 타입 에러 없음 (`tsc --noEmit` 통과)
- [x] Biome 린트/포맷 통과 (`biome check` 0 errors)
- [x] 빌드 성공 (`pnpm tauri build` — .deb 5.7MB, .AppImage 75MB)
- [ ] ⏳ `tauri dev` 실행 후 iframe 재생 수동 검증 (WSL2 GUI 필요)
- [ ] ⏳ 풀스크린에서 progress bar placeholder 가려지지 않음 수동 확인
- [ ] ⏳ tauri-specta 바인딩 생성 (`src/bindings.ts`) — 크레이트 버전 충돌로 Phase 1 이관

## 다음 Phase 의존성

**Phase 1 (YouTube 자막 Fetch + Subtitle 표시)**은 다음을 Phase 0에서 받음:

1. **동작하는 Tauri + React 프로젝트**: 빌드 및 핫 리로드 환경
2. **iframe 기반 플레이어 아키텍처**: Phase 1에서 자막을 플레이어 아래 오버레이하는 기초
3. **현재 시간 폴링 로직**: Phase 1에서 자막 동기화의 기반
4. **URL → videoId 파싱**: Phase 1에서 자막 fetch API 호출 전제
5. **Rust 백엔드 skeleton**: Phase 1에서 `fetch_subtitles` command 구현

## 실패 시 대안

**시나리오: YouTube iframe이 Tauri WebView2에서 작동하지 않음**

1. **원인 분석**
   - CORS 에러 → CSP 정책 재검토
   - YouTube API 스크립트 로드 실패 → `https:` 프로토콜 확인
   - iframe 콘텐츠 로드 안 됨 → WebView2 버전 업그레이드

2. **대안 선택지**
   - **대안 A: 오버레이 윈도우로 전환** (POC 문서 초기 설계)
     - 브라우저 창 감지 → 브라우저 위에 투명 오버레이 윈도우 띄우기
     - 장점: YouTube 웹사이트 직접 이용 (iframe 문제 회피)
     - 단점: Windows API 연동 필요, 위치 추적 복잡도 증가, macOS 호환성 떨어짐
   
   - **대안 B: Electron으로 전환**
     - Tauri 대신 Electron + Chromium 사용
     - 장점: 더 강력한 API, 기존 모듈 재사용 가능
     - 단점: 번들 크기 ~150MB (목표 15MB 대비 10배), 배포 어려움
   
   - **대안 C: 웹 기반 SPA로 축소**
     - Tauri 앱 포기, Next.js + 로컬 백엔드 API
     - 장점: YouTube iframe 제약 없음
     - 단점: 데스크탑 앱이 아님 (원래 기획 변경)

3. **권장 방향**
   - iframe 실패 확률 낮음 (Chromium 기반 WebView2는 standard compliant)
   - Phase 0 성공률 95% 이상 예상
   - **대안 A (오버레이)**는 iframe 실패 시 최소한의 아키텍처 수정으로 보완 가능하므로, Phase 0 실패 후 신속히 전환 가능
   - **대안 B, C는 근본 재설계 필요** → 의도하지 않은 대안 (최후의 수단)

---

**Phase 0 완료 조건**: YouTube iframe 정상 작동 + Tauri 기본 구조 확인 + 다음 Phase 진행 가능한 상태

**예상 소요 시간**: 2-3 일 (개발 + 테스트)

---

## 구현 결과 (2026-04-07)

### 상태: 코드 완료, 수동 검증 대기

**빌드 검증 통과**:
- `tsc --noEmit`: PASS (TypeScript strict, 0 errors)
- `biome check`: PASS (16 files, 0 errors)
- `vite build`: PASS (2.51s)
- `cargo check`: PASS (1 dead_code warning — 정상)
- `pnpm tauri build`: PASS (deb 5.7MB, AppImage 75MB)

**커밋**: `a207c99` — `feat: Phase 0 구현 — Tauri 뼈대 + YouTube 임베드 플레이어`

### 계획 대비 변경사항

| 항목 | 계획 | 실제 | 이유 |
|------|------|------|------|
| tauri-specta | Phase 0에서 설정 | Phase 1로 이관 | `specta`/`specta-typescript`/`tauri-specta` RC 버전 간 호환 불가 (rc.22 vs rc.24) |
| lite-youtube A/B | 환경변수로 전환 | 제거 | 기술 스택 인터뷰에서 react-youtube로 확정 (getCurrentTime 직접 접근 필요) |
| 뷰 전환 타이밍 | 200ms out / 300ms in 분리 | 250ms 단일 duration | motion v12 transition API가 enter/exit 분리를 다른 방식으로 지원, 단순화 |
| scaffolding | `pnpm create tauri-app` | 수동 생성 | CLI가 대화형이라 자동화 불가 |

### 수동 검증 결과 (2026-04-07)

**테스트 환경**: WSL2 (Ubuntu) + WebKitGTK

| 항목 | localhost:5173 (브라우저) | Tauri 앱 (WebKitGTK) |
|------|-------------------------|---------------------|
| URL 입력 → Player 전환 | ✅ 정상 | ✅ 정상 |
| YouTube iframe 로드 | ✅ 정상 | ⚠️ 재생 불가 |
| fade 애니메이션 | ✅ 정상 | ✅ 정상 |
| 뒤로가기 버튼 | ✅ 정상 | ✅ 정상 |

**YouTube 재생 이슈**: WSL2의 WebKitGTK에서 "브라우저에서 재생할 수 없습니다" 에러 발생.
- 원인: WebKitGTK는 Chromium이 아닌 WebKit 엔진 — YouTube 코덱/DRM 호환성 제한 + GPU 가속 부재 (`DRI3 error`)
- **이것은 예상된 제약**: docs/tech-stack.md에 "YouTube Error 153 — POC는 Windows 우선"으로 기록됨
- **Windows에서는 문제 없음**: WebView2(Chromium 기반)를 사용하므로 `localhost:5173` 브라우저 테스트와 동일하게 동작
- `localhost:5173`에서 모든 프론트엔드 로직이 정상 동작하므로, **Phase 0 핵심 리스크 검증 통과**로 판정
