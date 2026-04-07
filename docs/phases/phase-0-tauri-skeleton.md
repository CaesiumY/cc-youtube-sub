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

- [ ] **Tauri 2.x + React + TypeScript + Vite 프로젝트 초기화**
  - `pnpm create tauri-app` 으로 scaffolding (패키지 매니저: pnpm)
  - React + TypeScript + Vite 템플릿 선택
  - Tauri CLI 2.x 설치

- [ ] **shadcn/ui Luma 프리셋 + Pretendard 폰트 설정**
  - shadcn/ui 설치 및 Luma (Neutral OKLCH) 테마 적용
  - Pretendard 폰트 설치 (`@fontsource/pretendard` 또는 CDN)
  - 다크 모드 기본 CSS 변수 설정

- [ ] **Tanstack Router 설정 (`/` → `/watch/$videoId`)**
  - `@tanstack/react-router` 설치
  - **hash history 사용** (`createHashHistory`) — Tauri 파일 프로토콜 호환성 필수
  - `/` 경로: Home View (URL 입력)
  - `/watch/$videoId` 경로: Player View (YouTube 플레이어)
  - Home → Player fade 전환 (200ms out → 300ms in)

- [ ] **Tanstack Query + Zustand 상태 관리 초기 설정**
  - `@tanstack/react-query` + `zustand` 설치
  - `QueryClientProvider` 루트에 설정
  - Zustand 스토어 기본 구조 정의 (`currentTime`, `isFullscreen` 등)

- [ ] **motion v12 뷰 전환 애니메이션 설정**
  - `motion` 패키지 설치 (`motion/react`)
  - `AnimatePresence`로 Home ↔ Player 페이드 전환 구현 (200ms out → 300ms in)

- [ ] **tauri-specta 타입 생성 설정**
  - `tauri-specta` v2 설치 (Rust + TypeScript)
  - Rust 커맨드에 `#[specta::specta]` 매크로 적용
  - `export_bindings!()` 매크로로 TypeScript 타입 자동 생성

- [ ] **Home View: URL 입력 중앙 표시 (브랜딩 없음)**
  - 텍스트 입력 필드 (예: `https://www.youtube.com/watch?v=...`)
  - 파싱 로직: URL → video ID 추출 (regex 또는 URL API)
  - 입력 검증 (YouTube URL 형식 확인)
  - 로고/브랜딩 없이 입력 필드만 화면 중앙에 표시

- [ ] **Player View: 영상 + 뒤로가기 버튼(←) + 2px progress bar placeholder**
  - Player 상단 구석에 작은 뒤로가기 버튼(←) — Home으로 복귀
  - YouTube iframe 전체 화면으로 표시
  - 영상 아래 2px 얇은 진행률 바 placeholder (Phase 1에서 연결)

- [ ] **YouTube 플레이어 구현 (`react-youtube` 기본)**
  - `react-youtube` 설치 및 사용 (기본 구현체)
  - `<YouTube videoId={videoId} opts={{ playerVars: { fs: 0 } }} onReady={onReady} />` 패턴
  - `onReady` 콜백으로 `YT.Player` 인스턴스 획득 → `playerRef`에 저장
  - 플레이어 상태 리스너 설정 (`onStateChange`, `onError`)
  - **lite-youtube 선택적 비교**: `VITE_PLAYER_MODE` 환경 변수로 전환 가능하게 분기 (`react-youtube` vs `lite-youtube`)

- [ ] **getCurrentTime() 500ms 폴링으로 재생 시간 추적**
  - `setInterval(player.getCurrentTime(), 500)` 폴링 로직
  - 현재 재생 시간을 UI 상태에 반영
  - 폴링 cleanup (언마운트 시 `clearInterval`)

- [ ] **Tauri 윈도우 풀스크린 토글 (F키) — YouTube iframe 풀스크린 가로채기**
  - `F` 키 입력 시 `window.__TAURI__.window.getCurrent().setFullscreen(true/false)` 호출
  - YouTube iframe 자체 풀스크린 버튼 클릭 이벤트 가로채기 (iframe 내 `postMessage` 활용)
  - 풀스크린 전환 시 오버레이(자막 영역)가 가려지지 않도록 z-index 보장

- [ ] **Rust 백엔드 기본 구조 셋업**
  - Tauri command 인터페이스 정의 (나중 Phase의 자막 fetch/번역용 예약)
  - `src-tauri/src/lib.rs` 에 `#[tauri::command]` 매크로로 기본 skeleton 작성
  - 프론트엔드-백엔드 통신 구조 확인

- [ ] **개발 환경 설정**
  - TypeScript strict mode 활성화
  - Biome 설정 (ESLint + Prettier 대체, `biome.json` 구성)
  - Tauri 핫 리로드 (`tauri dev`)
  - Vite 빌드 설정 확인

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
youtube-subtitle-for-claude/
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

- [ ] Tauri 앱 `tauri dev` 실행 → 개발 서버 정상 시작
- [ ] `/` 경로에서 URL 입력 화면 표시 (브랜딩 없이 입력 필드만 중앙)
- [ ] `/watch/:id` 경로에서 YouTube 플레이어 표시
- [ ] YouTube URL 입력 필드에 유효한 URL 입력 가능 (검증 통과)
- [ ] iframe으로 YouTube 비디오 로드 → 플레이어 정상 표시
- [ ] 재생 버튼 클릭 → 영상 재생 시작 확인
- [ ] 뒤로가기 버튼(←)으로 Home 복귀 확인
- [ ] 500ms 폴링으로 currentTime 값 변화 감지 (UI에 시간 표시)
- [ ] 플레이어 상태 변화 감지 (재생/일시정지/버퍼링 이벤트 콘솔 출력)
- [ ] F키로 Tauri 풀스크린 토글 동작 확인
- [ ] 풀스크린에서 오버레이 영역(progress bar placeholder)이 가려지지 않음 확인
- [ ] react-youtube 기본 모드 동작 확인 (`VITE_PLAYER_MODE` 미설정 시 react-youtube 사용)
- [ ] lite-youtube 비교 모드 전환 동작 확인 (`VITE_PLAYER_MODE=lite`)
- [ ] Rust 백엔드 command 스켈레톤 작성 완료 + tauri-specta 바인딩 생성 확인 (`src/bindings.ts`)
- [ ] TypeScript strict 모드에서 타입 에러 없음 (`pnpm biome check .` + `tsc --noEmit` 통과)
- [ ] 개발 환경에서 핫 리로드 동작 확인 (파일 수정 → 즉시 새로고침)
- [ ] 빌드 성공 (`pnpm build` / `pnpm tauri build`)

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
