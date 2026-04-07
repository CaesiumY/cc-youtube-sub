# 테스트 전략: YouTube 번역 자막 데스크탑 앱

> Deep Dive 인터뷰 기반 테스트 전략 (7라운드, 모호성 14.5%)
>
> 이 문서는 **전략 + 실행 가이드**입니다. 구현자가 각 Phase에서 어떤 테스트를 어떻게 작성해야 하는지 바로 따라할 수 있는 수준으로 작성되었습니다.

---

## 1. 테스트 가능한 설계 원칙

> POC 전체(Phase 1~3)에 **필수 적용**. 이 원칙을 따르면 단위 테스트 케이스를 10~15개 확보할 수 있다.

### Rust: 순수 함수 분리

모든 Tauri 커맨드(`#[tauri::command]`) 내부 로직을 순수 함수로 분리한다. 커맨드 함수는 얇은 위임자(thin delegator) 역할만 수행한다.

```rust
// ❌ Bad: 커맨드 안에 로직이 직접 포함
#[tauri::command]
async fn fetch_subtitles(video_id: String) -> Result<Vec<Subtitle>, AppError> {
    let raw = yt_transcript_rs::get_transcript(&video_id, &["en"]).await?;
    let normalized = raw.iter().map(|e| /* ... */).collect();
    let chunks = /* 청크 분할 로직 */;
    Ok(chunks)
}

// ✅ Good: 순수 함수 체인으로 분리
#[tauri::command]
async fn fetch_subtitles(
    video_id: String,
    fetcher: State<'_, Box<dyn TranscriptFetcher>>,
) -> Result<Vec<Subtitle>, AppError> {
    let raw = fetcher.fetch(&video_id).await
        .map_err(|e| AppError::CaptionFetch(e.to_string()))?;
    let normalized = transcript::normalize(&raw);      // 순수 함수
    let chunks = chunk::split(&normalized, 30.0, 60.0); // 순수 함수
    Ok(chunks)
}
```

### Rust: trait 추상화

외부 의존성(yt-transcript-rs, Claude subprocess, SQLite)을 trait으로 추상화하여 테스트에서 mock 구현체를 주입한다.

```rust
// 자막 fetch 추상화
#[async_trait]
pub trait TranscriptFetcher: Send + Sync {
    async fn fetch(&self, video_id: &str) -> anyhow::Result<Vec<TranscriptEntry>>;
}

// 프로덕션 구현체
pub struct YtTranscriptFetcher;

#[async_trait]
impl TranscriptFetcher for YtTranscriptFetcher {
    async fn fetch(&self, video_id: &str) -> anyhow::Result<Vec<TranscriptEntry>> {
        let api = TranscriptApi::new();
        Ok(api.get_transcript(video_id, &["en"]).await?)
    }
}

// 테스트 mock 구현체
#[cfg(test)]
pub struct MockTranscriptFetcher {
    pub response: Vec<TranscriptEntry>,
}

#[cfg(test)]
#[async_trait]
impl TranscriptFetcher for MockTranscriptFetcher {
    async fn fetch(&self, _video_id: &str) -> anyhow::Result<Vec<TranscriptEntry>> {
        Ok(self.response.clone())
    }
}
```

동일한 패턴을 `ServerAdapter` (이미 trait으로 정의됨)와 `CacheRepository` (SQLite 접근)에도 적용한다.

### React: lib/ 위임

React 훅은 상태 관리와 사이드 이펙트만 담당하고, 계산 로직은 `src/lib/`에 독립 함수로 분리한다.

```typescript
// ❌ Bad: 훅 안에 계산 로직 인라인
function useSubtitleSync(subtitles: Subtitle[], currentTime: number) {
  return useMemo(() => {
    let left = 0, right = subtitles.length - 1;
    while (left <= right) { /* 이진 탐색 */ }
    return null;
  }, [subtitles, currentTime]);
}

// ✅ Good: 계산 로직을 lib/에 분리
// src/lib/subtitle.ts — 순수 함수, Vitest로 직접 단위 테스트 가능
export function findMatchingSubtitle(currentTime: number, subtitles: Subtitle[]): Subtitle | null {
  let left = 0, right = subtitles.length - 1;
  while (left <= right) { /* 이진 탐색 */ }
  return null;
}

// src/hooks/use-subtitle-sync.ts — 훅은 위임만
function useSubtitleSync(subtitles: Subtitle[], currentTime: number) {
  return useMemo(() => findMatchingSubtitle(currentTime, subtitles), [subtitles, currentTime]);
}
```

---

## 2. 테스트 피라미드

```
        ┌─────────┐
        │  E2E    │  15% — Playwright + CDP
        │  (7개)  │  사용자 시나리오, UI 플로우
        ├─────────┤
        │  통합   │  25% — Vitest + mockIPC / Rust + mock traits
        │ (7~10개)│  IPC 경계, 모듈 협력, DB CRUD
        ├─────────┤
        │  단위   │  60% — Vitest + Rust #[test]
        │(15~20개)│  순수 함수, 알고리즘, 직렬화
        └─────────┘
```

| 레벨 | 비율 | 도구 | 대상 |
|------|------|------|------|
| 단위 | 60% | Rust `#[test]` + Vitest | 순수 함수, 알고리즘, 직렬화/역직렬화 |
| 통합 | 25% | Vitest + `mockIPC` + Rust with mock traits | Tauri IPC 경계, 모듈 간 협력, SQLite CRUD |
| E2E | 15% | Playwright + CDP | 사용자 시나리오 기반 전체 UI 플로우 |

**커버리지 목표**: % 목표 없음. 핵심 경로(AC에 직결되는 로직)만 집중 테스트. v1에서 재검토.

---

## 3. AC별 자동화/수동 분류 매트릭스

> PRD 체크박스 기준 **18개 AC**를 분류 (Phase 3 문서의 "16개" 표기와 차이가 있으나, PRD 실제 항목 기준)

| AC | 내용 | 자동화 레벨 | 난이도 | 모킹 필요 | 수동 필요 |
|----|------|-----------|--------|----------|----------|
| 1 | URL → 임베드 플레이어 재생 | E2E | 보통 | YouTube mock (선택) | iframe 실제 재생은 수동 |
| 2 | 5초 이내 첫 자막 | **수동** | — | — | 실제 Claude CLI 응답 시간 의존 |
| 3 | 끊김 없이 자막 연속 | **수동** | — | — | 시각적 연속성 판단 |
| 4 | 재방문 캐시 즉시 로드 | 통합 | 보통 | SQLite in-memory | — |
| 5 | 영상 설명 + 맥락 프롬프트 포함 | 단위(부분) | 쉬움 | — | 맥락 품질은 수동 |
| 6 | ServerAdapter 패턴 관리 | 단위 | 보통 | subprocess mock | — |
| 7 | CLAUDECODE env 제거 | 단위 | 쉬움 | — | — |
| 8 | CLI PATH 확인 (testEnvironment) | 단위 | 쉬움 | PATH mock | — |
| 9 | SIGTERM→SIGKILL shutdown | **수동** | — | — | 실제 OS 시그널 타이밍 |
| 10 | 구독 한도 에러 표시 | E2E | 보통 | Claude stderr mock | — |
| 11 | 자막 없는 영상 안내 | E2E | 보통 | yt-transcript-rs 에러 mock | — |
| 12 | 추가 비용 $0 | **수동** | — | — | 아키텍처 검토 수준 |
| 13 | 캐시 miss seek → '준비 중...' | E2E | 어려움 | 번역 지연 mock | — |
| 14 | 캐시 hit seek → 즉시 표시 | 통합 | 보통 | SQLite in-memory | — |
| 15 | JSON 배열 포맷 구조화 | 단위 | 쉬움 | — | — |
| 16 | 풀스크린 오버레이 유지 | **수동** | — | — | WSL2 GUI 필요 |
| 17 | Home→Player fade 애니메이션 | E2E | 어려움 | — | DOM 상태만 확인 |
| 18 | 뒤로가기 버튼 Home 복귀 | E2E | 쉬움 | — | — |

**요약**: 자동화 가능 **12개** (67%) / 수동 필수 **5개** (28%) / 부분 자동화 **1개** (6%)

### 수동 검증이 불가피한 유형

| 유형 | AC | 이유 |
|------|-----|------|
| 실환경 타이밍 의존 | AC 2, AC 3 | 실제 Claude CLI + YouTube 네트워크 응답 없이 SLA 검증 불가 |
| OS/Tauri 시각 렌더링 | AC 16 | WSL2 GUI 없이 Tauri 윈도우 풀스크린 상태 확인 불가 |
| OS 시그널 타이밍 | AC 9 | CI 환경에서 SIGTERM→SIGKILL 타이밍 재현성 불안정 |
| 아키텍처 검증 | AC 12 | 비용은 외부 청구 시스템 의존, 코드로 검증 불가 |

---

## 4. Phase별 테스트 범위

### Phase 1: 자막 파이프라인 — 테스트 인프라 셋업 + Rust 중심 테스트

**인프라 셋업**
- `vitest.config.ts` 생성 (jsdom 환경, globals, setup 파일)
- `tests/setup.ts` 생성 (mockIPC 초기화)
- `src-tauri/src/`에 `#[cfg(test)]` 모듈 추가
- `package.json`에 `vitest`, `jsdom`, `@vitest/ui` devDependencies 추가

**단위 테스트 (Rust `#[test]`)**

| 모듈 | 테스트 대상 | 예상 케이스 수 |
|------|-----------|-------------|
| `transcript::normalize()` | start + duration → end 계산, HTML entities 디코딩, 빈 라인 필터링 | 4~5 |
| `chunk::split()` | 30초~1분 범위 분할, 빈 배열, 1줄, 마지막 청크, 20줄 제한 | 5~6 |
| `jsonl::parse_stream()` | content_block_delta 추출, 불완전 라인 처리, 조각 연결 | 3~4 |
| `translation::validate()` | 필수 필드 존재, 타입 검증, CJK 문자 포함 여부, 빈 배열 | 4~5 |
| `AppError` 직렬화 | `{kind, message}` JSON 출력, 5개 variant 각각 | 5 |

**단위 테스트 (Vitest)**

| 모듈 | 테스트 대상 | 예상 케이스 수 |
|------|-----------|-------------|
| `src/lib/youtube-url.ts` | 4개 URL 패턴 × 경계 케이스 + videoId 직접 입력 | 8~10 |

**통합 테스트 (Rust)**

| 모듈 | 테스트 대상 | 모킹 |
|------|-----------|------|
| `ServerAdapter::test_environment()` | CLI 존재/부재 확인, 에러 메시지 | MockShell |
| `ServerAdapter::execute()` | stdin 전송 → stdout 수신 시퀀스 | MockShell |

**픽스처**
- `src-tauri/tests/fixtures/transcript-response.xml` — yt-transcript-rs mock 데이터
- `src-tauri/tests/fixtures/claude-stream.jsonl` — Claude subprocess mock 출력

### Phase 2: 통합 + 캐시 — 프론트엔드 + SQLite 테스트 추가

**단위 테스트 (Vitest)**

| 모듈 | 테스트 대상 | 예상 케이스 수 |
|------|-----------|-------------|
| `src/lib/subtitle.ts` | `findMatchingSubtitle()` 이진 탐색, 경계, 빈 배열, 자막 없는 구간 | 6~8 |
| `src/stores/player-store.ts` | 각 액션 상태 전이 (setCurrentTime, setFullscreen, setPlayerState) | 3 |

**단위 테스트 (Rust)**

| 모듈 | 테스트 대상 | 예상 케이스 수 |
|------|-----------|-------------|
| `cache::compute_chunk_hash()` | 동일 입력 → 동일 출력, 다른 입력 → 다른 출력 | 3 |

**통합 테스트 (Rust)**

| 모듈 | 테스트 대상 | 모킹 |
|------|-----------|------|
| SQLite 캐시 CRUD | `save_to_cache()` → `query_cache()` 라운드트립, 중복 키 처리 | `sqlite::memory:` |
| 캐시 배치 조회 | 전체 청크 캐시 조회 (1회 쿼리 최적화) | `sqlite::memory:` |

**통합 테스트 (Vitest)**

| 모듈 | 테스트 대상 | 모킹 |
|------|-----------|------|
| Tanstack Query + invoke() | `useQuery('subtitles', videoId)` 결과 캐싱 | `mockIPC` |
| 커스텀 event mock | `listen("translation-chunk-complete")` 콜백 트리거 | `vi.mock('@tauri-apps/api/event')` |

**픽스처**
- `tests/fixtures/cache/seed-data.sql` — SQLite 캐시 시드 데이터

### Phase 3: 버퍼링 + 완성 — E2E + CI 구축

**단위 테스트 (Rust)**

| 모듈 | 테스트 대상 | 예상 케이스 수 |
|------|-----------|-------------|
| `buffer::schedule()` | 우선순위 큐 정렬, look-ahead 계산, 동시 실행 제한 | 4~5 |

**통합 테스트 (Rust)**

| 모듈 | 테스트 대상 | 모킹 |
|------|-----------|------|
| Buffer Manager + subprocess | `on_seek()` 상태 전이, pending 취소, 재스케줄링 | MockServerAdapter + `sqlite::memory:` |

**E2E (Playwright + CDP)**

| 시나리오 | AC | 난이도 |
|---------|-----|-------|
| URL 입력 → Player 뷰 전환 | AC 1 | 보통 |
| 뒤로가기 버튼 Home 복귀 | AC 18 | 쉬움 |
| 자막 없는 영상 → 안내 메시지 | AC 11 | 보통 |
| 구독 한도 초과 → 에러 표시 | AC 10 | 보통 |
| 캐시 miss seek → '준비 중...' | AC 13 | 어려움 |
| Home→Player fade 존재 확인 | AC 17 | 어려움 |

**CI**: GitHub Actions 3-잡 구성 (이 Phase에서 `.github/workflows/ci.yml` 생성)

---

## 5. 모킹 전략

### 의존성별 × 테스트 레벨별 매트릭스

| 의존성 | Vitest (단위/통합) | Rust `#[test]` | Playwright E2E |
|--------|------------------|----------------|----------------|
| Tauri IPC invoke | `mockIPC()` | N/A | 실제 Tauri 앱 |
| Tauri emit/listen | 커스텀 event mock (아래 참조) | N/A | 실제 이벤트 |
| Tauri Window API | `vi.mock('@tauri-apps/api/window')` | N/A | 실제 윈도우 API |
| react-youtube | `vi.mock('react-youtube')` + mockPlayer | N/A | 실제 YouTube iframe |
| yt-transcript-rs | N/A | `TranscriptFetcher` trait mock | mock binary |
| Claude subprocess | N/A | `ServerAdapter` trait mock | mock binary |
| SQLite | N/A | `sqlite::memory:` 인메모리 DB | 실제 파일 DB |

### emit/listen 커스텀 event mock

`translate_chunk`는 `Ok(())` 즉시 반환 + 비동기 `app.emit("translation-chunk-complete")` 패턴을 사용한다. `mockIPC`는 invoke만 커버하므로, emit/listen 스트리밍에는 별도 mock이 필요하다.

```typescript
// tests/mocks/tauri-event.ts
import { vi } from 'vitest';

type UnlistenFn = () => void;
type EventCallback = (event: { payload: unknown }) => void;

const listeners = new Map<string, EventCallback[]>();

// listen mock — 콜백을 등록
export function mockListen(event: string, callback: EventCallback): UnlistenFn {
  if (!listeners.has(event)) listeners.set(event, []);
  listeners.get(event)!.push(callback);
  return () => {
    const cbs = listeners.get(event);
    if (cbs) listeners.set(event, cbs.filter(cb => cb !== callback));
  };
}

// emit mock — 테스트에서 이벤트를 수동 트리거
export function triggerEvent(event: string, payload: unknown): void {
  const cbs = listeners.get(event) || [];
  cbs.forEach(cb => cb({ payload }));
}

// 테스트 간 초기화
export function clearListeners(): void {
  listeners.clear();
}
```

```typescript
// tests/setup.ts
import { vi } from 'vitest';
import { mockListen, clearListeners } from './mocks/tauri-event';

// Tauri event 모듈 mock
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn((event: string, callback: any) => mockListen(event, callback)),
  emit: vi.fn(),
}));

beforeEach(() => clearListeners());
```

```typescript
// 테스트에서 사용 예시
import { triggerEvent } from '../mocks/tauri-event';

test('번역 스트리밍 이벤트 수신', async () => {
  // 훅 또는 컴포넌트가 listen 등록한 후
  renderHook(() => useTranslation(videoId));

  // 이벤트 수동 트리거 (Claude subprocess가 emit하는 것을 시뮬레이션)
  triggerEvent('translation-chunk-complete', {
    original: 'Hello',
    translated: '안녕하세요',
    start: 0.0,
    end: 3.0,
  });

  // 결과 검증
  expect(/* ... */).toBe(/* ... */);
});
```

### Vitest + Tauri IPC mock 설정

```typescript
// tests/setup.ts (위의 event mock에 추가)
import { mockIPC } from '@tauri-apps/api/mocks';

beforeEach(() => {
  mockIPC((cmd, args) => {
    switch (cmd) {
      case 'fetch_subtitles':
        return [{ text: 'Hello', start: 0, duration: 3 }];
      case 'translate_chunk':
        return null; // 즉시 반환, 실제 결과는 emit으로 수신
      case 'check_cache':
        return { hit: false };
      default:
        return null;
    }
  });
});
```

---

## 6. CI 워크플로우

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  frontend:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'pnpm'
      - run: pnpm install --frozen-lockfile
      - run: pnpm biome check .
      - run: pnpm tsc --noEmit
      - run: pnpm vitest run

  backend:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: src-tauri
      - run: cargo test --manifest-path src-tauri/Cargo.toml

  e2e:
    runs-on: windows-latest  # WebView2는 Windows에서만
    needs: [frontend, backend]
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'pnpm'
      - uses: dtolnay/rust-toolchain@stable
      - run: pnpm install --frozen-lockfile
      - run: pnpm tauri build
      - run: pnpm playwright install --with-deps chromium
      - run: pnpm playwright test
```

### CI 잡 역할

| 잡 | 실행 환경 | 검증 내용 |
|----|---------|---------|
| `frontend` | ubuntu-latest | Biome lint + TypeScript 타입 체크 + Vitest 단위/통합 테스트 |
| `backend` | ubuntu-latest | Rust `#[test]` 단위/통합 테스트 (in-memory SQLite 포함) |
| `e2e` | windows-latest | Playwright + CDP로 Tauri WebView2 E2E (mock 기반) |

**E2E에서 mock 사용**: CI에서는 YouTube iframe과 Claude CLI subprocess를 mock으로 대체한다. YouTube는 `page.route()` 인터셉트, Claude는 mock binary를 PATH에 등록하여 실제 외부 서비스 의존을 제거한다.

---

## 7. 수동 검증 체크리스트

> 자동화 불가능한 AC 5개 + Phase별 수동 검증 항목

### AC 기반 수동 검증

- [ ] **AC 2**: URL 입력 후 5초 이내 첫 한국어 번역 자막 표시
  - 조건: 실제 Claude CLI 사용, 영어 자막 있는 영상
  - 방법: 3회 측정 평균
  - 판정: 평균 ≤ 5초이면 PASS

- [ ] **AC 3**: 재생 중 끊김 없이 자막이 연속으로 표시됨
  - 조건: 10분 이상 영상 전체 시청
  - 방법: 재생 중 자막이 비어있는 구간(500ms 이상) 카운트
  - 판정: 0회이면 PASS

- [ ] **AC 9**: Claude 프로세스 비정상 종료 시 SIGTERM → SIGKILL graceful shutdown
  - 조건: 번역 중 앱 강제 종료
  - 방법: 프로세스 모니터링 도구(Task Manager / ps)로 잔여 프로세스 확인
  - 판정: claude 프로세스가 10초 이내 완전 종료되면 PASS

- [ ] **AC 12**: Claude Code 구독 외 추가 비용 $0
  - 조건: 아키텍처 리뷰
  - 방법: 코드에서 외부 유료 API 호출 여부 확인 (API 키 사용, 클라우드 서비스 등)
  - 판정: Claude CLI 이외 유료 서비스 미사용이면 PASS

- [ ] **AC 16**: Tauri 풀스크린(F키)에서 SubtitleOverlay가 YouTube iframe 위에 유지됨
  - 조건: Windows 데스크탑에서 실행
  - 방법: F키 → 풀스크린 전환 → 자막 오버레이 위치/가시성 확인
  - 판정: 오버레이가 iframe 위에 보이면 PASS

### Phase별 추가 수동 검증

- [ ] **Phase 0 (완료됨)**: YouTube iframe이 Tauri WebView2에서 정상 로드/재생
- [ ] **Phase 2**: 자막 동기화 지연 ±200ms 이내 (타이머 측정)
- [ ] **Phase 3**: 60분 영상 전체 재생 → 자막 끊김 없음 (수동 시청)
- [ ] **Phase 3**: 메모리 누수 검증 (시작 vs 종료 메모리 < 50MB 차이, Task Manager 확인)

---

## 8. 픽스처/목 데이터 관리

### 디렉토리 구조

```
tests/
├── fixtures/
│   ├── subtitles/
│   │   ├── en-lecture-10min.xml        # 10분 강의 영어 자막 (실제 영상에서 추출)
│   │   └── en-tutorial-short.xml       # 3분 튜토리얼 자막
│   ├── translations/
│   │   ├── chunk-1-response.jsonl      # Claude 번역 응답 (첫 청크, 영상 설명 포함)
│   │   └── chunk-2-response.jsonl      # 이전 맥락 포함 청크
│   └── cache/
│       └── seed-data.sql               # SQLite 캐시 시드 (video_id + chunk_hash + translated_json)
├── setup.ts                            # Vitest 글로벌 셋업 (mockIPC, event mock 초기화)
└── mocks/
    ├── tauri-event.ts                  # emit/listen 커스텀 mock (섹션 5 참조)
    └── youtube-player.ts              # react-youtube mock (mockPlayer 객체)

src-tauri/tests/
├── fixtures/
│   ├── transcript-response.xml         # yt-transcript-rs mock 데이터
│   └── claude-stream.jsonl             # Claude subprocess mock stdout 출력
└── common/
    └── mod.rs                          # 공통 mock trait 구현체 (MockTranscriptFetcher 등)
```

### 픽스처 관리 규칙

1. **실제 데이터 기반**: YouTube 자막 XML은 실제 영상에서 추출하여 현실적인 테스트 데이터로 사용
2. **git 버전 관리**: 모든 픽스처는 git에 커밋. `.gitignore`에 포함하지 않음
3. **최소한의 크기**: 10분 강의 1개, 3분 튜토리얼 1개 — 필요 시 추가
4. **JSONL 포맷 일치**: Claude 응답 픽스처는 실제 `--output-format stream-json` 출력 형식과 동일
5. **SQL 시드**: `INSERT` 문으로 구성, `sqlite3 :memory: < seed-data.sql`로 로드 가능

---

## 관련 문서

- [PRD](prd.md) — Deep Interview 기반 요구사항 정의 (모호성 9%)
- [기술 스택](tech-stack.md) — 라이브러리 선택 근거, 아키텍처 다이어그램, 테스트 도구 선택
- [Phase 로드맵](phases/overview.md) — Phase 0~3 구성 및 의존성
- [Phase 0](phases/phase-0-tauri-skeleton.md) — Tauri 뼈대 (완료)
- [Phase 1](phases/phase-1-subtitle-pipeline.md) — 자막 파이프라인 (테스트 인프라 셋업 시작점)
- [Phase 2](phases/phase-2-integration-cache.md) — 통합 + 캐시
- [Phase 3](phases/phase-3-buffering-polish.md) — 버퍼링 + 완성 (E2E + CI 구축)

### 역할 분리

| 문서 | 역할 | 내용 |
|------|------|------|
| `tech-stack.md` | **WHY** — 도구 선택 근거 | 왜 Vitest인지, 왜 Playwright인지, 코드 예시 |
| `test-strategy.md` (이 문서) | **WHAT/HOW** — 실행 전략 | 무엇을 테스트하는지, 어떻게 모킹하는지, CI 구성 |
| Phase 문서 | **WHEN** — 각 Phase의 완료 기준 | 언제 어떤 기능이 완성되어야 하는지 |
