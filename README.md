# YouTube Subtitle Translator

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Tauri v2](https://img.shields.io/badge/Tauri-v2-blue?logo=tauri)](https://v2.tauri.app)
[![Rust](https://img.shields.io/badge/Rust-2021-orange?logo=rust)](https://www.rust-lang.org)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.7-blue?logo=typescript)](https://www.typescriptlang.org)

YouTube 영상의 자막을 **Claude AI**로 실시간 번역하는 Tauri 데스크톱 앱입니다.
Claude Code 구독만으로 추가 API 비용 없이 자연스러운 한국어 번역을 제공합니다.

<!-- 스크린샷은 추후 추가
![screenshot](docs/assets/screenshot.png)
-->

## 주요 기능

- **실시간 자막 번역** — YouTube 영상 재생 중 Claude AI가 자막을 한국어로 번역
- **청크 기반 처리** — 자막을 30초~1분 단위로 분할하여 긴 영상도 효율적으로 번역
- **사전 버퍼링** — 재생 위치 앞의 자막을 미리 번역하여 끊김 없는 시청 경험
- **SQLite 캐시** — 한 번 번역된 자막은 로컬 DB에 저장, 재방문 시 즉시 로드
- **자막 오버레이** — 영상 위에 반투명 자막 표시, 원본/번역 토글 가능
- **추가 비용 없음** — Claude Code 구독 외 별도 API 키나 비용 불필요

## 기술 스택

### Frontend

| 기술 | 버전 | 용도 |
|------|------|------|
| React | 19 | UI 렌더링 |
| TypeScript | 5.7 | 타입 안전성 |
| TanStack Router | 1 | SPA 라우팅 (hash history) |
| TanStack Query | 5 | 서버 상태 관리 및 캐싱 |
| Zustand | 5 | UI 상태 관리 |
| Tailwind CSS | 4 | 스타일링 |
| Motion (Framer) | 12 | 애니메이션 |
| Vite | 6 | 번들링 |

### Backend

| 기술 | 버전 | 용도 |
|------|------|------|
| Tauri | 2 | 데스크톱 앱 프레임워크 |
| Rust | 2021 ed. | 백엔드 로직 |
| Tokio | 1 | 비동기 런타임 |
| rusqlite | 0.39 | SQLite 캐시 |
| yt-transcript-rs | 0.1 | YouTube 자막 API |
| reqwest | 0.12 | HTTP 클라이언트 |

### AI

| 기술 | 용도 |
|------|------|
| Claude Code CLI | subprocess로 번역 실행 |
| stream-json | 스트리밍 응답 파싱 |

## 시작하기

### 사전 요구사항

- [Node.js](https://nodejs.org) 18+
- [pnpm](https://pnpm.io)
- [Rust](https://rustup.rs)
- [Tauri 사전 요구사항](https://v2.tauri.app/start/prerequisites/)
- [Claude Code CLI](https://docs.anthropic.com/en/docs/claude-code) (로그인 완료 상태)

### 설치

```bash
# 저장소 클론
git clone https://github.com/CaesiumY/cc-youtube-sub.git
cd cc-youtube-sub

# 의존성 설치
pnpm install

# 개발 서버 실행 (Tauri 앱)
pnpm tauri dev
```

### 빌드

```bash
# 프로덕션 빌드
pnpm tauri build
```

## 프로젝트 구조

```
cc-youtube-sub/
├── src/                        # React 프론트엔드
│   ├── routes/                 #   페이지 (Home, Player)
│   ├── components/             #   UI 컴포넌트
│   ├── hooks/                  #   커스텀 훅 (번역 파이프라인, 버퍼링)
│   ├── stores/                 #   Zustand 상태 관리
│   └── lib/                    #   유틸리티
├── src-tauri/                  # Rust 백엔드
│   └── src/
│       ├── subtitle/           #   자막 fetch · 파싱 · 청크 분할
│       ├── translate/          #   번역 프롬프트 · 검증
│       ├── claude/             #   Claude CLI 프로세스 관리
│       ├── cache.rs            #   SQLite 캐시
│       └── buffer_manager.rs   #   사전 버퍼링
└── docs/                       # 설계 문서 (PRD, 기술 스택, Phase별 계획)
```

## 작동 방식

```
YouTube URL 입력
      │
      ▼
자막 Fetch (yt-transcript-rs)
      │
      ▼
청크 분할 (30초~1분 단위)
      │
      ▼
캐시 확인 ──── Hit ──→ 즉시 표시
      │
    Miss
      │
      ▼
Claude CLI로 번역 (subprocess)
      │
      ▼
번역 결과 검증 + 캐시 저장
      │
      ▼
영상 위 자막 오버레이 표시
```

1. YouTube URL을 입력하면 자막을 가져와 시간 기반으로 청크를 분할합니다
2. 각 청크는 Claude Code CLI를 subprocess로 실행하여 한국어로 번역됩니다
3. 번역 결과는 SQLite에 캐싱되어 동일 영상 재방문 시 즉시 로드됩니다
4. 재생 위치 앞의 청크를 미리 번역(사전 버퍼링)하여 끊김을 최소화합니다

## 키보드 단축키

| 키 | 동작 |
|----|------|
| `T` | 원본 자막 / 번역 토글 |
| `F` | 풀스크린 |
| `+` / `-` | 자막 크기 조절 |
| `Space` | 재생 / 일시정지 |
| `←` | 홈으로 돌아가기 |

## 라이선스

[MIT](LICENSE) &copy; 2026 ChangSik Yoon
