# WSL2 개발 환경 제약

> 의사결정 기록: WSL2에서의 개발/테스트 제약과 해결 방향

---

## 문제 요약

WSL2가 주 개발 환경인 상황에서, **실제 파이프라인(자막 → 번역 → 캐시)을 end-to-end로 확인할 경로가 없다.**

```
경로 1: pnpm tauri dev (Tauri 앱)
  → WSLg/X서버 필요 → GUI 렌더링 불안정 또는 불가

경로 2: pnpm dev (브라우저)
  → isTauri() === false → 전부 mock 데이터
```

---

## 브라우저 dev 서버의 동작 차이

`src/lib/tauri-commands.ts`의 모든 함수가 `isTauri()` 분기를 가진다.  
브라우저에서는 `window.__TAURI_INTERNALS__`가 없으므로 항상 mock으로 빠진다.

| 기능 | Tauri 앱 (정상) | 브라우저 dev 서버 |
|------|----------------|------------------|
| 자막 fetch | Rust → YouTube InnerTube API | 고정 30줄 영어 mock 데이터 |
| 번역 | Rust → Claude CLI subprocess | 하드코딩된 한글 매핑 반환 |
| 캐시 | SQLite (앱 데이터 디렉토리) | 메모리 `Map` (새로고침 시 소멸) |
| 버퍼 매니저 | Rust 사전 버퍼링 + Seek 처리 | `if (!isTauri()) return;` — 완전 무시 |
| 번역 스케줄링 | Rust BufferManager 재생 위치 기반 | 프론트엔드 큐 fallback (MAX_CONCURRENT=2) |

### 관련 코드

| 파일 | 역할 |
|------|------|
| `src/lib/tauri-commands.ts` | IPC 경계 — 모든 함수에 `isTauri()` 분기 |
| `src/lib/mock-tauri.ts` | 브라우저용 fixture 데이터 + 지연 시뮬레이션 |
| `src/hooks/use-translation-pipeline.ts:162-169` | 브라우저 모드 시 프론트엔드 큐 fallback |

---

## 검토한 해결 방향

| # | 방향 | 설명 | 장점 | 단점 |
|---|------|------|------|------|
| 1 | **Rust 백엔드를 HTTP 서버로 분리** | 기존 Tauri 커맨드를 Axum/Actix API로 노출, 브라우저가 mock 대신 HTTP로 호출 | 코드 재활용 높음, 웹 버전 확장 가능 | 서버 관리 이중화, Tauri State 접근 방식 변경 필요 |
| 2 | **mock-tauri.ts를 웹 API로 교체** | Node.js 프록시 등으로 실제 YouTube/Claude 호출 | 프론트엔드만 수정 | Rust 백엔드 로직 JS로 재구현 필요 |
| 3 | **WSLg 환경 정비** | Windows 11 WSLg로 Tauri GUI 직접 실행 | 코드 변경 없음 | 환경 의존적, WebView2 호환성 미보장 |

---

## 현재 상태

- **결정 보류** — POC 단계에서는 mock 기반 UI 개발로 진행
- 실제 기능 검증은 Windows 네이티브 빌드 또는 WSLg 환경에서 수행

---

## 관련 문서

- [기술 스택](tech-stack.md) — 아키텍처 다이어그램, Tauri IPC 패턴
- [Phase 로드맵](phases/overview.md) — 전체 구현 단계
