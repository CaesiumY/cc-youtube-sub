# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

YouTube 영상의 자막을 Claude AI(subprocess)로 실시간 번역하는 Tauri v2 데스크톱 앱. Claude Code CLI 구독만으로 추가 API 비용 없이 동작한다.

## Commands

```bash
# 개발 (Tauri 앱 + Vite dev server 동시 실행)
pnpm tauri dev

# 프론트엔드만 개발 (브라우어 모드, mock-tauri 자동 적용)
pnpm dev

# 프로덕션 빌드 (NSIS/DMG/AppImage/deb)
pnpm tauri build

# 린트 & 포매팅 검사
pnpm check

# 린트 & 포매팅 자동 수정
pnpm check:fix

# Rust 테스트
cd src-tauri && cargo test

# Rust 린트
cd src-tauri && cargo clippy
```

## Architecture

**Tauri v2** 앱으로, React 프론트엔드와 Rust 백엔드가 IPC(`invoke()` / `emit()`)로 통신한다.

### Frontend (`src/`)
- **React 19** + **TypeScript 5.7**, **Vite 6** 번들러
- **TanStack Router** (hash history — Tauri 필수) : `/` (Home) → `/watch/$videoId` (Player)
- **TanStack Query** : Rust invoke() 결과 캐싱 (staleTime 5분)
- **Zustand** : UI 상태 (player, translation, settings, updater 4개 스토어)
- **Tailwind CSS v4** (CSS-first config), shadcn Luma 프리셋(OKLCH), 기본 다크 모드
- **Motion (Framer Motion)** : 자막 fade 애니메이션

### Backend (`src-tauri/src/`)
- **lib.rs** : 12개 Tauri command 등록 (진입점)
- **subtitle/** : `fetch.rs` (yt-transcript-rs) → `parser.rs` (XML 파싱) → `chunk.rs` (30초~1분 청크 분할)
- **translate/** : `prompt.rs` (프롬프트 빌드) → `validator.rs` (JSON 검증) → `jsonl_parser.rs` (스트림 파싱)
- **claude/adapter.rs** : Claude CLI subprocess 실행 (`claude --print - --output-format stream-json`), Windows에서 `CREATE_NO_WINDOW` 플래그 사용
- **cache.rs** : SQLite(WAL 모드), `(video_id, chunk_hash)` 유니크 키
- **buffer_manager.rs** : 재생 위치 기반 사전 번역 스케줄링, `subtitle-update` 이벤트 emit
- **error.rs** : `AppError` enum (thiserror), 프론트엔드에 `{ kind, message }` JSON 직렬화

### Key Data Flow
1. URL 입력 → `fetch_subtitles` (자막 fetch + 청크 분할)
2. `batch_query_cache` → 캐시 히트는 즉시 표시, 미스는 번역 큐에 추가
3. `translate_chunk` → 프롬프트 빌드 → Claude subprocess → JSONL 파싱 → 검증 → 캐시 저장
4. Phase 3: `init_buffer` → Rust BufferManager가 재생 위치 앞 청크를 미리 번역

### IPC Boundary (`src/lib/tauri-commands.ts`)
- `__TAURI_INTERNALS__` 글로벌로 Tauri 환경 감지
- 없으면 `mock-tauri.ts` 폴백 → 브라우저에서도 프론트엔드 개발 가능

## Conventions

- **패키지 매니저**: pnpm (npm/yarn 사용 금지)
- **린터/포매터**: Biome (ESLint/Prettier 대신). indent: space 2칸
- **경로 alias**: `@/*` → `./src/*`
- **폰트**: Pretendard (한국어 최적화)
- **Rust 에러**: `AppError` enum에 variant 추가 후 `thiserror` derive
- **새 Tauri command 추가 시**: `lib.rs`의 `generate_handler![]` 매크로에 등록 필수
- **상태 관리 원칙**: 서버 데이터는 TanStack Query, UI 상태는 Zustand
- **캐시 키**: 청크 내 자막 라인들의 SHA256 해시

## gstack + superpowers (프로젝트 특화)

전역 라우팅 규칙은 `~/.claude/CLAUDE.md`의 `gstack + superpowers routing` 섹션을 따른다. 이 프로젝트 특화 추가 제약:

- **`/qa` 범위**: Playwright Chromium은 Tauri 네이티브 윈도우에 접근 불가. `/qa`는 `pnpm dev` (Vite dev server + `mock-tauri` 폴백)로 띄운 프론트엔드 부분에만 적용 가능. Rust 백엔드와 IPC 경유 플로우는 `cargo test` 또는 수동 테스트로 대체.
- **`/ship` 전 `/cso`**: Claude CLI subprocess 실행 + 외부 URL fetch + SQLite 영속화 조합으로 공격 표면이 작지 않음. `translate/`나 `claude/adapter.rs` 수정 시 `/cso daily` 권장 (프롬프트 인젝션 경계 확인).
- **`/plan-eng-review` 중점 대상**: `buffer_manager.rs`(재생 위치 기반 상태 전이) 또는 `subtitle/` 파이프라인(fetch → parse → chunk → translate 데이터 흐름) 변경 시 시퀀스/상태 다이어그램 요구.
