# Phase 로드맵 개요

> YouTube 번역 자막 데스크탑 앱 POC 구현 단계

## Phase 구성

```
Phase 0          Phase 1              Phase 2            Phase 3
Tauri 뼈대       자막 파이프라인       통합 + 캐시         버퍼링 + 완성
───────────── → ─────────────────── → ──────────────── → ────────────────
YouTube iframe   timedtext fetch      Phase 0 + 1 연결   사전 버퍼링
+ React 셋업     Claude subprocess    SQLite 캐시         Seek 처리
                 ServerAdapter        자막 싱크 표시       에러 핸들링
                                                          전체 AC 검증
```

## Phase 요약

| Phase | 이름 | 검증 리스크 | 핵심 산출물 |
|-------|------|------------|------------|
| [Phase 0](phase-0-tauri-skeleton.md) | Tauri 뼈대 + YouTube 임베드 | YouTube iframe이 Tauri WebView2에서 동작하는가 | Tauri 앱 + react-youtube 임베드 플레이어 + 재생 시간 추적, Tanstack Router (hash history), 2-View 구조, Tauri 풀스크린, lite-youtube A/B 테스트 |
| [Phase 1](phase-1-subtitle-pipeline.md) | 자막 파이프라인 | timedtext API + Claude subprocess가 안정적인가 | 자막 fetch (yt-transcript-rs) → 청크 분할 → Claude 번역 → JSON 결과 |
| [Phase 2](phase-2-integration-cache.md) | 통합 + 캐시 | 플레이어 + 번역 + 캐시가 매끄럽게 연결되는가 | 재생 중 자막 오버레이 + 키보드 단축키 + SQLite 캐시 + 재방문 즉시 로드 |
| [Phase 3](phase-3-buffering-polish.md) | 버퍼링 + 완성 | 사전 버퍼링이 체감 지연 없이 동작하는가 | Buffer Manager + Seek 처리 + 에러 핸들링 + Tauri 풀스크린 자막 유지 + Vitest + Playwright 테스트 + 전체 AC 통과 |

## 의존성 체인

```
Phase 0 ─────→ Phase 2
               ↗
Phase 1 ─────→ Phase 2 ─────→ Phase 3
```

- Phase 0과 Phase 1은 **독립적으로 병렬 진행 가능**
- Phase 2는 Phase 0 + Phase 1 모두 완료 후 시작
- Phase 3는 Phase 2 완료 후 시작

## 핵심 KPI 달성 경로

| KPI (PRD 기준) | Phase |
|----------------|-------|
| YouTube URL → 앱 내 임베드 재생 | Phase 0 |
| Claude subprocess 안정 동작 | Phase 1 |
| 재방문 시 캐시 즉시 로드 | Phase 2 |
| URL 입력 후 5초 내 첫 자막 | Phase 3 |
| 재생 중 끊김 없는 자막 | Phase 3 |
| Seek 시 '번역 준비 중...' 표시 | Phase 3 |
| Tauri 풀스크린에서 자막 유지 | Phase 0 (풀스크린 검증) + Phase 3 (완성) |
| Home → Player fade 전환 | Phase 0 |

## 관련 문서

- [POC 기획서](../poc.md) — 초기 POC 설계
- [PRD](../prd.md) — Deep Interview 기반 요구사항 정의 (모호성 9%)
- [기술 스택](../tech-stack.md) — 라이브러리 선택 근거, 아키텍처 다이어그램, 테스트 전략
