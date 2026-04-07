# Tech Stack: YouTube 번역 자막 앱

> 기술 스택 인터뷰: 12라운드, 모호성 ~15%

---

## 기술 결정 요약

| 결정 | 선택 | 이유 |
|------|------|------|
| 상태 관리 | Tanstack Query + Zustand | Query: Tauri invoke() 캐싱. Zustand: UI 상태. 역할 분리 |
| CSS | Tailwind CSS v4 | shadcn/ui Luma v4 필수. OKLCH 네이티브, CSS-first 설정 |
| 애니메이션 | motion v12 (`motion/react`) | AnimatePresence로 자막 fade-in/out 선언적 처리 |
| YouTube 임베드 | react-youtube | getCurrentTime() 직접 접근, React 라이프사이클 관리 |
| 자막 fetch | yt-transcript-rs (Rust) | YouTube 자막 전용 crate. 직접 구현 불필요 |
| Claude subprocess | tauri-plugin-shell (spawn) | live stdout 스트리밍, stdin 쓰기, 프로세스 자동 정리 |
| SQLite | tauri-plugin-sql (sqlite) | 마이그레이션 내장, 비동기, 전 플랫폼 지원 |
| 에러 처리 (Rust) | thiserror (커맨드) + anyhow (내부) | Tauri 커맨드는 Serialize 필수 → thiserror. 내부 로직은 anyhow |
| 타입 안전성 | tauri-specta v2 | Rust 커맨드 → TypeScript 타입 자동 생성 |
| 라우팅 | Tanstack Router (hash history) | 타입 안전, hash history로 Tauri 호환성 보장 |
| 패키지 매니저 | pnpm | 빠르고 디스크 효율적 |
| 린팅/포맷팅 | Biome | ESLint+Prettier 대체. 설정 최소, 매우 빠름 |
| CI | GitHub Actions | PR에서 테스트/빌드 자동화 |
| 테스트 전략 | Rust #[test] 우선 + Vitest + Playwright | 백엔드 우선, 프론트 핵심 훅, E2E 핵심 플로우 |

---

## 아키텍처 다이어그램

```
┌─ Tauri 앱 ──────────────────────────────────────────────┐
│                                                          │
│  ┌─ WebView2 (React 앱) ─────────────────────────────┐  │
│  │                                                    │  │
│  │  react-youtube       → YouTube iframe 임베드       │  │
│  │  Tanstack Router     → / → /watch/$videoId        │  │
│  │  Tanstack Query      → invoke() 결과 캐싱         │  │
│  │  Zustand             → UI 상태 (재생 시간, 토글)   │  │
│  │  motion/react        → 자막 fade, 뷰 전환         │  │
│  │  shadcn/ui Luma      → UI 컴포넌트                 │  │
│  │  Tailwind CSS v4     → 스타일링 (OKLCH)           │  │
│  │                                                    │  │
│  │           invoke() ↕ emit()/listen()               │  │
│  └────────────────────┬───────────────────────────────┘  │
│                       │ IPC                               │
│  ┌────────────────────▼───────────────────────────────┐  │
│  │                                                    │  │
│  │  Rust 백엔드                                        │  │
│  │                                                    │  │
│  │  yt-transcript-rs    → YouTube 자막 fetch          │  │
│  │  tauri-plugin-shell  → Claude CLI spawn/stream     │  │
│  │  tauri-plugin-sql    → SQLite 캐시                  │  │
│  │  quick-xml + serde   → XML/JSON 파싱              │  │
│  │  thiserror + anyhow  → 에러 처리                   │  │
│  │  tauri-specta        → TS 타입 자동 생성           │  │
│  │  reqwest + tokio     → HTTP + 비동기 (Tauri 내장)  │  │
│  │                                                    │  │
│  └────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────┘
```

---

## Frontend 상세

### 패키지 목록

```json
{
  "dependencies": {
    "react": "^19",
    "react-dom": "^19",
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-shell": "^2",
    "@tauri-apps/plugin-sql": "^2",
    "@tanstack/react-router": "^1",
    "@tanstack/react-query": "^5",
    "zustand": "^5",
    "motion": "^12",
    "react-youtube": "^10",
    "lucide-react": "^0.460"
  },
  "devDependencies": {
    "typescript": "^5.7",
    "vite": "^6",
    "@biomejs/biome": "^1.9",
    "vitest": "^3",
    "@vitest/ui": "^3",
    "jsdom": "^25",
    "@playwright/test": "^1.49",
    "tailwindcss": "^4",
    "@tailwindcss/vite": "^4"
  }
}
```

### 상태 관리 패턴

```
┌─ Tanstack Query (서버 상태) ─────────────────────┐
│ useQuery('subtitles', videoId)  → 자막 fetch      │
│ useQuery('cache', videoId)     → 캐시 확인        │
│ useMutation('translate')       → 번역 트리거      │
│                                                   │
│ invoke()를 queryFn으로 사용                        │
│ staleTime으로 재요청 방지                          │
└───────────────────────────────────────────────────┘

┌─ Zustand (클라이언트 상태) ───────────────────────┐
│ currentTime        → 재생 시간 (500ms 폴링)       │
│ showOriginal       → 원문 토글 (T키)              │
│ subtitleSize       → 자막 크기 (+/- 키)           │
│ isFullscreen       → 풀스크린 상태                 │
│ translationStatus  → 번역 진행률                   │
└───────────────────────────────────────────────────┘
```

### Tanstack Router 경로 구성

```typescript
import { createRouter, createHashHistory, createRootRoute, createRoute } from '@tanstack/react-router'

const hashHistory = createHashHistory()  // Tauri 호환 필수

const rootRoute = createRootRoute({ component: RootLayout })

const homeRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/',
  component: HomePage,
})

const watchRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/watch/$videoId',       // videoId가 string으로 타입 추론됨
  component: PlayerPage,
})

const router = createRouter({
  routeTree: rootRoute.addChildren([homeRoute, watchRoute]),
  history: hashHistory,
})
```

### react-youtube 사용 패턴

```typescript
import YouTube, { YouTubeEvent } from 'react-youtube'

function VideoPlayer({ videoId }: { videoId: string }) {
  const playerRef = useRef<YT.Player>(null)
  const setCurrentTime = usePlayerStore(s => s.setCurrentTime)

  const onReady = (event: YouTubeEvent) => {
    playerRef.current = event.target

    // 500ms 폴링으로 재생 시간 추적
    setInterval(() => {
      const time = playerRef.current?.getCurrentTime()
      if (time !== undefined) setCurrentTime(time)
    }, 500)
  }

  return (
    <YouTube
      videoId={videoId}
      opts={{
        width: '100%',
        height: '100%',
        playerVars: { autoplay: 0, fs: 0 },  // fs: 0 → YouTube 풀스크린 비활성화
      }}
      onReady={onReady}
    />
  )
}
```

---

## Backend 상세 (Rust)

### Cargo.toml 의존성

```toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-shell = "2"
tauri-plugin-sql = { version = "2", features = ["sqlite"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
thiserror = "2"
anyhow = "1"
quick-xml = { version = "0.37", features = ["serialize"] }
yt-transcript-rs = "0.1"
specta = "2"
tauri-specta = { version = "2", features = ["derive", "typescript"] }
```

### 에러 처리 패턴

```rust
use thiserror::Error;
use serde::Serialize;

/// 프론트엔드에 전달되는 구조화된 에러
/// React에서 { kind: 'CaptionFetch', message: '...' } 형태로 수신
#[derive(Debug, Error, Serialize)]
#[serde(tag = "kind", content = "message")]
pub enum AppError {
    #[error("자막을 가져올 수 없습니다: {0}")]
    CaptionFetch(String),

    #[error("번역 중 오류가 발생했습니다: {0}")]
    Translation(String),

    #[error("데이터베이스 오류: {0}")]
    Database(String),

    #[error("Claude CLI를 찾을 수 없습니다: {0}")]
    EnvironmentCheck(String),

    #[error("프로세스 오류: {0}")]
    Process(String),
}

// 내부 로직에서는 anyhow 사용, 커맨드 경계에서 AppError로 변환
#[tauri::command]
async fn fetch_subtitles(video_id: String) -> Result<Vec<Subtitle>, AppError> {
    internal_fetch(&video_id)
        .await
        .map_err(|e| AppError::CaptionFetch(e.to_string()))
}
```

### Claude subprocess 스트리밍 패턴

```rust
use tauri_plugin_shell::ShellExt;

#[tauri::command]
async fn translate_chunk(
    app: tauri::AppHandle,
    chunk: TranslationChunk,
) -> Result<(), AppError> {
    let shell = app.shell();

    // Claude CLI spawn (stdin으로 프롬프트 전송)
    let (mut rx, child) = shell
        .command("claude")
        .args(["--print", "-", "--output-format", "stream-json"])
        .env_remove("CLAUDECODE")  // nested session 방지
        .spawn()
        .map_err(|e| AppError::Process(e.to_string()))?;

    // stdin으로 프롬프트 쓰기
    child.write(chunk.prompt.as_bytes())
        .map_err(|e| AppError::Process(e.to_string()))?;

    // stdout 스트리밍 → Tauri Event로 프론트엔드 전달
    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => {
                    let text = String::from_utf8_lossy(&line);
                    let _ = app.emit("translation-chunk-complete", &text);
                }
                CommandEvent::Terminated(status) => {
                    let _ = app.emit("translation-process-exit", status.code);
                }
                _ => {}
            }
        }
    });

    Ok(())
}
```

### ServerAdapter Trait

```rust
/// Paperclip의 ServerAdapter를 Rust trait으로 구현
#[async_trait]
pub trait ServerAdapter {
    /// Claude CLI 바이너리 존재 확인 + 인증 상태 검증
    async fn test_environment(&self) -> Result<EnvironmentStatus, AppError>;

    /// 번역 실행 (subprocess spawn + 결과 스트리밍)
    async fn execute(&self, request: TranslationRequest) -> Result<(), AppError>;

    /// 프로세스 정상 종료 (SIGTERM → 유예 → SIGKILL)
    async fn shutdown(&self, child_id: u32) -> Result<(), AppError>;
}
```

---

## DevTools 상세

### pnpm

```bash
# 프로젝트 초기화
pnpm create tauri-app youtube-subtitle -- --template react-ts
cd youtube-subtitle
pnpm install
```

### Biome 설정

```json
// biome.json
{
  "$schema": "https://biomejs.dev/schemas/1.9.0/schema.json",
  "organizeImports": { "enabled": true },
  "linter": {
    "enabled": true,
    "rules": { "recommended": true }
  },
  "formatter": {
    "enabled": true,
    "indentStyle": "space",
    "indentWidth": 2
  }
}
```

### GitHub Actions CI

```yaml
# .github/workflows/ci.yml
name: CI
on: [push, pull_request]

jobs:
  frontend:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
      - run: pnpm install --frozen-lockfile
      - run: pnpm biome check .
      - run: pnpm vitest run

  backend:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --manifest-path src-tauri/Cargo.toml

  e2e:
    runs-on: windows-latest  # WebView2는 Windows에서만
    needs: [frontend, backend]
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
      - run: pnpm install --frozen-lockfile
      - run: pnpm tauri build
      - run: pnpm playwright test
```

---

## Testing 상세

### 테스트 우선순위 (POC)

| 우선순위 | 영역 | 도구 | 테스트 대상 |
|---------|------|------|------------|
| 1 (필수) | Rust 백엔드 | cargo test | 청크 분할, XML 파싱, 캐시 CRUD, AppError 직렬화 |
| 2 (핵심) | React 훅 | Vitest | usePlayerSync, useTranslation, useSubtitleCache |
| 3 (검증) | E2E | Playwright | URL 입력 → 자막 표시 전체 플로우 1개 |

### Vitest + Tauri 모킹

```typescript
// src/test-setup.ts
import { mockIPC } from '@tauri-apps/api/mocks'

beforeEach(() => {
  mockIPC((cmd, args) => {
    if (cmd === 'fetch_subtitles') {
      return [{ text: 'Hello', start: 0, duration: 3 }]
    }
    if (cmd === 'check_cache') {
      return { hit: false }
    }
  })
})
```

### Playwright + Tauri WebView2

```typescript
// e2e/subtitle-flow.spec.ts
import { test, expect, chromium } from '@playwright/test'

test('URL 입력 → 자막 표시', async () => {
  // CDP로 Tauri WebView2 연결
  const browser = await chromium.connectOverCDP('http://localhost:9222')
  const page = browser.contexts()[0].pages()[0]

  // Home View
  await page.fill('input[placeholder*="URL"]', 'https://youtube.com/watch?v=test123')
  await page.keyboard.press('Enter')

  // Player View
  await expect(page.locator('[data-testid="youtube-player"]')).toBeVisible()

  // 자막 표시 대기 (5초 이내)
  await expect(page.locator('[data-testid="subtitle-overlay"]')).toBeVisible({ timeout: 5000 })
})
```

---

## 주요 Gotchas

| 이슈 | 영향 | 대응 |
|------|------|------|
| YouTube Error 153 | macOS/Linux에서 YouTube iframe 로드 실패 | POC는 Windows 우선. macOS는 tauri-plugin-localhost로 우회 |
| tauri-plugin-sql 타입 | 쿼리 결과가 `Record<string, unknown>[]` | tauri-specta로 타입 안전성 보완 |
| yt-transcript-rs 불안정성 | YouTube 내부 API 변경 시 동작 중단 | 에러 복구 + fallback 로직 구현 |
| TanStack Router + Tauri | browser history가 Tauri와 충돌 | hash history 사용 필수 |
| CLAUDECODE 환경변수 | 제거하지 않으면 nested session 오류 | subprocess spawn 시 env_remove 필수 |
