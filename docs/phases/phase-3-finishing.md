# Phase 3 마무리: 테스트 + CI + Graceful Shutdown

> Phase 3 기능 구현(버퍼링 + Seek + 에러 핸들링)은 완료. 이 문서는 남은 테스트 인프라, CI, Graceful Shutdown 구현을 다룬다.

## 현재 상태

- Rust 테스트: 59개 통과 (buffer_manager 12, cache 8, error 3, subtitle 10, translate 26)
- 프론트엔드 테스트: **없음** (Vitest 미설치, 테스트 파일 0개)
- CI 파이프라인: **없음** (.github/workflows/ 미존재)
- Graceful Shutdown: **미완성** (`adapter.rs`의 `shutdown()`이 SIGKILL만 사용, AC #9 미충족)
- Playwright E2E: **없음**

## 구현 범위

### 1. Vitest 설치 + 설정

**설치:**
```bash
pnpm add -D vitest @testing-library/react @testing-library/jest-dom jsdom
```

**신규 `vitest.config.ts`** (프로젝트 루트):
```ts
import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    setupFiles: ["src/__tests__/setup.ts"],
    include: ["src/**/*.test.ts", "src/**/*.test.tsx"],
    exclude: ["node_modules", "dist", "src-tauri", "e2e"],
  },
});
```

**신규 `src/__tests__/setup.ts`:**
- jsdom에 `__TAURI_INTERNALS__` 없음 → `isTauri()` false → mock-tauri 자동 활성화
- `afterEach`로 Zustand store 리셋 (테스트 간 상태 격리)

**`package.json` scripts 추가:**
```json
"test": "vitest run",
"test:watch": "vitest"
```

### 2. 프론트엔드 단위 테스트 (4개 파일)

#### 2a. `src/lib/__tests__/subtitle-matcher.test.ts`

대상: `findSubtitleAt()` (이진 검색) + `buildChunkHashInput()`
- 소스: `src/lib/subtitle-matcher.ts`

| 테스트 | 입력 | 기대 결과 |
|--------|------|-----------|
| 빈 배열 | `[], 5.0` | `null` |
| 범위 내 시간 | `[{start:1, end:3}], 2.0` | 해당 엔트리 |
| start 경계 | `[{start:1, end:3}], 1.0` | 매칭 (inclusive) |
| end 경계 | `[{start:1, end:3}], 3.0` | `null` (exclusive) |
| 여러 엔트리 중간 | 5개 엔트리, 중간 시간 | 정확한 엔트리 |
| 모든 엔트리 전 | time < 첫 start | `null` |
| 모든 엔트리 후 | time > 마지막 end | `null` |
| 갭 구간 | 엔트리 사이 빈 시간 | `null` |
| buildChunkHashInput | `[{text:"A"},{text:"B"}]` | `"A B"` |

#### 2b. `src/lib/__tests__/youtube-url.test.ts`

대상: `extractVideoId()` + `isValidYouTubeUrl()`
- 소스: `src/lib/youtube-url.ts`

| 테스트 | 입력 | 기대 결과 |
|--------|------|-----------|
| 표준 watch URL | `youtube.com/watch?v=dQw4w9WgXcQ` | `dQw4w9WgXcQ` |
| www + https 포함 | `https://www.youtube.com/watch?v=...` | ID 추출 |
| 추가 쿼리 파라미터 | `?v=ID&list=PL...&t=42` | ID만 추출 |
| youtu.be 단축 | `youtu.be/dQw4w9WgXcQ` | ID |
| embed URL | `youtube.com/embed/ID` | ID |
| shorts URL | `youtube.com/shorts/ID` | ID |
| bare 11자 ID | `dQw4w9WgXcQ` | 그대로 반환 |
| 빈 문자열 | `""` | `null` |
| 무관한 텍스트 | `"hello world"` | `null` |
| isValidYouTubeUrl | 유효/무효 URL | boolean |

#### 2c. `src/stores/__tests__/translation-store.test.ts`

대상: Zustand store — `getState()` 직접 호출
- 소스: `src/stores/translation-store.ts`
- `beforeEach`에서 `reset()` 호출

| 테스트 | 동작 | 기대 결과 |
|--------|------|-----------|
| 초기 상태 | — | totalChunks=0, isLoading=false, sessionId=0 |
| startLoading | `startLoading("vid1")` | videoId="vid1", isLoading=true, 이전 데이터 초기화 |
| setChunks | 3개 청크 설정 | totalChunks=3, 전부 "pending" |
| markChunkStatus("done") | 1개 완료 | completedChunks=1 |
| markChunkStatus("cached") | 1개 캐시 | completedChunks+1, cachedChunks+1 |
| 전부 완료 시 | 모든 청크 done/cached | isLoading=false |
| addTranslations | 비순서 엔트리 | 시간순 정렬 + 중복 방지 |
| incrementSession | 호출 | sessionId += 1 |
| reset | 호출 | 초기 상태로 복원 |

#### 2d. `src/stores/__tests__/player-store.test.ts`

대상: Zustand store
- 소스: `src/stores/player-store.ts`
- 상수: MIN=0.875, MAX=2.0, STEP=0.125, 기본값=1.25

| 테스트 | 동작 | 기대 결과 |
|--------|------|-----------|
| 초기값 | — | subtitleSize=1.25, showOriginal=false |
| 크기 증가 | `increaseSubtitleSize()` | 1.375 |
| 크기 감소 | `decreaseSubtitleSize()` | 1.125 |
| 최대 클램핑 | 7번 increase | 2.0 (초과 안 됨) |
| 최소 클램핑 | 5번 decrease | 0.875 (미만 안 됨) |
| 원본 토글 | `toggleOriginal()` 2회 | true → false |
| setCurrentTime | `setCurrentTime(42.5)` | currentTime=42.5 |

### 3. Graceful Shutdown (Rust)

**`src-tauri/Cargo.toml` 추가:**
```toml
[target.'cfg(unix)'.dependencies]
nix = { version = "0.29", features = ["signal"] }
```

**`src-tauri/src/claude/adapter.rs` — `shutdown()` 재작성:**

현재 (`child.kill()` = SIGKILL 즉시 종료):
```rust
pub async fn shutdown(child: &mut tokio::process::Child) -> Result<(), AppError> {
    child.kill().await.map_err(|e| {
        AppError::Process(format!("Claude 프로세스 종료 실패: {}", e))
    })?;
    Ok(())
}
```

목표 (SIGTERM → 5초 대기 → SIGKILL):
```rust
pub async fn shutdown(child: &mut tokio::process::Child) -> Result<(), AppError> {
    #[cfg(unix)]
    {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;

        if let Some(pid) = child.id() {
            let _ = kill(Pid::from_raw(pid as i32), Signal::SIGTERM);
            match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                child.wait(),
            ).await {
                Ok(_) => return Ok(()),
                Err(_) => {
                    child.kill().await.map_err(|e| {
                        AppError::Process(format!("SIGKILL 전송 실패: {}", e))
                    })?;
                }
            }
        }
    }
    #[cfg(not(unix))]
    {
        child.kill().await.map_err(|e| {
            AppError::Process(format!("프로세스 종료 실패: {}", e))
        })?;
    }
    Ok(())
}
```

**테스트 (`adapter.rs` 내 `#[cfg(test)]`):**
- `sleep 60` spawn → `shutdown()` 호출 → 프로세스 종료 확인

### 4. CI 파이프라인

**신규 `.github/workflows/ci.yml`** — 3개 잡:

| 잡 | 러너 | 단계 |
|----|------|------|
| **frontend** | ubuntu-latest | pnpm install → biome check → tsc --noEmit → vitest run |
| **backend** | ubuntu-latest | apt-get 시스템 의존성* → cargo test |
| **e2e** | ubuntu-latest | placeholder (echo만) |

*시스템 의존성: `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libssl-dev`

트리거: push to main, PR to main

### 5. Playwright E2E (최소한)

**설치:**
```bash
pnpm add -D @playwright/test
npx playwright install chromium
```

**신규 `playwright.config.ts`:**
- testDir: `./e2e`
- webServer: `pnpm dev` (localhost:5173)
- Vite dev 서버 기반 → mock-tauri 자동 활성화 → Tauri 바이너리 불필요

**신규 `e2e/smoke.spec.ts`** (2개 테스트):
1. URL 입력 → `/watch/$videoId` 네비게이션 확인
2. Player 뷰 → mock 번역 데이터로 자막 오버레이 표시 확인 (timeout 10초)

**`package.json` 추가:** `"test:e2e": "playwright test"`

## 파일 변경 목록

| 파일 | 작업 |
|------|------|
| `package.json` | devDeps + scripts 추가 |
| `vitest.config.ts` | **신규** |
| `src/__tests__/setup.ts` | **신규** |
| `src/lib/__tests__/subtitle-matcher.test.ts` | **신규** |
| `src/lib/__tests__/youtube-url.test.ts` | **신규** |
| `src/stores/__tests__/translation-store.test.ts` | **신규** |
| `src/stores/__tests__/player-store.test.ts` | **신규** |
| `src-tauri/Cargo.toml` | nix 의존성 추가 |
| `src-tauri/src/claude/adapter.rs` | shutdown() 재작성 + 테스트 |
| `.github/workflows/ci.yml` | **신규** |
| `playwright.config.ts` | **신규** |
| `e2e/smoke.spec.ts` | **신규** |

## 검증 체크리스트

- [ ] `pnpm test` — Vitest 전체 통과 (~30개 테스트)
- [ ] `cargo test` (src-tauri/) — 기존 59 + shutdown 테스트 통과
- [ ] `npx tsc --noEmit` — TypeScript 에러 없음
- [ ] `pnpm test:e2e` — Playwright smoke 통과 (Vite dev 서버 기반)
- [ ] GitHub Actions CI — push 후 3개 잡 통과 확인

## 구현 순서 권장

1. Vitest 설치 + 설정 (Step 1)
2. 단위 테스트 4개 파일 (Step 2) — `pnpm test` 검증
3. Graceful Shutdown (Step 3) — `cargo test` 검증
4. CI 파이프라인 (Step 4) — push 후 검증
5. Playwright E2E (Step 5) — `pnpm test:e2e` 검증

## 주의사항

- **Zustand 테스트**: 컴포넌트 렌더링 없이 `getState()`/`setState()` 직접 사용. `beforeEach`에서 `reset()` 필수.
- **mock-tauri 자동 활성화**: jsdom에 `__TAURI_INTERNALS__` 없음 → `isTauri()` false → 모든 IPC가 mock으로 전환.
- **CI 시스템 의존성**: Tauri 2.x Linux 빌드에 WebKit2GTK 등 필요. 없으면 `cargo test`도 컴파일 실패.
- **Playwright + hash history**: TanStack Router가 `/#/watch/videoId` 형식 사용. `page.goto("/#/watch/ID")` 형태로 접근.
- **mock-tauri 지연**: `translateChunk`이 2-4초 지연. Playwright에서 `toBeVisible({ timeout: 10_000 })` 사용.
