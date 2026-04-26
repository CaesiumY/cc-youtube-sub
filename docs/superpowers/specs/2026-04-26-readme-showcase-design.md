# README Showcase Design

## Goal

`README.md`를 GitHub 방문자가 제품 가치를 빠르게 이해하고, 사용자는 바로 실행 방법을 찾고, 개발자는 상세 문서로 자연스럽게 이동할 수 있는 소개 문서로 개선한다.

제품 설명은 Claude Code CLI 기반으로 고정한다. Codex는 이 저장소 개발 도구 맥락에만 해당하므로 README 제품 소개에는 노출하지 않는다.

## Current Context

- 현재 README는 기본적인 기능, 기술 스택, 실행 명령, 작동 방식을 포함한다.
- 스크린샷은 주석 처리되어 있고 실제 시각 자료가 없다.
- README 상단은 문서형 설명에 가깝고, 앱이 실제로 어떤 화면과 사용 흐름을 제공하는지 즉시 보이지 않는다.
- 코드와 기존 문서의 실제 제품 구현은 `claude --print - --output-format stream-json` 기반 Claude Code CLI subprocess를 사용한다.
- 브라우저 개발 모드는 `mock-tauri` 폴백을 제공하므로 실제 Tauri/Claude 경로가 막혀도 프론트엔드 화면 캡처를 시도할 수 있다.

## Audience

README는 세 종류의 독자를 모두 지원하되, 상단 우선순위는 쇼케이스형 첫인상이다.

1. GitHub 방문자: 앱이 무엇이고 실제로 어떤 경험을 제공하는지 빠르게 판단한다.
2. 사용자: 사전 요구사항과 실행 명령을 확인하고 로컬에서 앱을 실행한다.
3. 개발자: 구조, 명령어, 상세 설계 문서 위치를 파악한다.

## Product Positioning

상단 문구는 다음 의미를 명확히 전달한다.

- YouTube 영상 자막을 Claude Code CLI로 실시간 한국어 번역한다.
- 영상 위에 번역 자막 오버레이를 표시한다.
- 번역 결과를 SQLite에 캐시해 동일 영상 재방문 시 재번역을 줄인다.
- Claude Code 구독 외 별도 API 키나 API 과금 없이 동작하는 로컬 데스크톱 앱이다.

피해야 할 표현:

- Codex 기반 제품처럼 보이는 설명
- 실제 구현과 다른 모델/API 직접 호출 표현
- “완전 무료”처럼 Claude Code 구독 전제를 흐리는 표현

## README Structure

README는 다음 순서로 재구성한다.

1. 프로젝트명, 라이선스/기술 배지
2. 한 줄 소개와 짧은 보조 설명
3. Hero 영역: 대표 스크린샷 또는 데모 프리뷰
4. 핵심 가치 3개: 실시간 번역, 로컬 캐시, 별도 API 키 불필요
5. 빠른 시작: 사전 요구사항, 설치, 개발 실행
6. 데모: Home 화면, Player 화면, 20-30초 사용 영상
7. 주요 기능
8. 작동 방식: 짧은 데이터 흐름
9. 기술 스택: 간결한 표
10. 개발 명령어
11. 프로젝트 구조
12. 키보드 단축키
13. 문서 링크와 라이선스

기존 README의 세부 기술 설명은 유지하되, 상단에서는 제품 가치와 사용 흐름을 먼저 보여준다. 상세 아키텍처 설명은 `docs/` 문서로 연결한다.

## Visual Assets

README용 자산은 `docs/assets/readme/` 아래에 둔다.

- `hero.png`: README 상단 대표 이미지
- `home.png`: URL 입력 화면
- `player.png`: 번역 자막 오버레이 화면
- `demo.mp4`: README용 20-30초 사용 영상
- `demo.gif`: GitHub README에서 `demo.mp4` 표시가 제한적일 때 사용할 대체 영상

자산 alt 텍스트는 실제성과 범위를 정확히 표현한다.

- 실제 캡처: `App screenshot`
- 재현 화면: `App preview`
- Remotion 편집 영상: `Demo preview`

## Capture And Demo Strategy

기본 전략은 실제 앱 캡처 우선, 실패 시 코드 기반 재현이다.

1. `pnpm dev`로 Vite 브라우저 모드를 실행한다.
2. `mock-tauri` 폴백 화면에서 Home/Player UI 캡처를 시도한다.
3. 캡처가 가능하면 Remotion에서 이미지 위에 줌, 콜아웃, 진행 흐름을 얹어 `demo.mp4`를 만든다.
4. 앱 실행, 브라우저 접근, 캡처 자동화가 막히면 React 컴포넌트와 CSS 토큰을 참고해 Remotion 안에서 동일한 Home/Player 화면을 재현한다.
5. 재현 영상은 실제 캡처와 혼동되지 않도록 README 문구와 파일명에서 preview 성격을 드러낸다.

이 방식은 실제 동작 신뢰를 우선하면서도, 로컬 환경 제약 때문에 README 개선이 막히지 않게 한다.

## Remotion Design

Remotion은 문서용 자산 제작 도구로만 사용한다. 앱 런타임, Tauri 번들, 사용자 실행 경로에는 포함하지 않는다.

Remotion 소스는 `docs/remotion/readme-demo/`에 분리한다. 이 디렉터리는 README 데모 제작물의 소스이며 앱 소스와 독립적이다.

영상 구성은 20-30초 안에 끝낸다.

1. Home 화면: YouTube URL 입력
2. Player 화면 진입: 영상 영역과 자막 오버레이 등장
3. 번역 상태: “번역 준비 중” 또는 진행률 표시
4. 자막 표시: 한국어 번역과 원문 토글 예시
5. 캐시 가치: 재방문 시 즉시 로드되는 흐름을 짧은 콜아웃으로 표현

콜아웃 문구는 짧고 제품 설명에만 집중한다.

- `Claude Code CLI subprocess`
- `Chunked translation`
- `SQLite cache`
- `Subtitle overlay`

## README Copy Direction

상단 예시 톤:

> YouTube 영상의 자막을 Claude Code CLI로 실시간 번역해 영상 위에 한국어 오버레이로 보여주는 Tauri 데스크톱 앱입니다.

핵심 가치 예시:

- 실시간 오버레이: YouTube 플레이어 위에 번역 자막을 자연스럽게 표시
- 청크 번역: 긴 영상도 30초-1분 단위로 나누어 처리
- 로컬 캐시: 한 번 번역한 자막은 SQLite에 저장해 재방문 시 재사용

사전 요구사항은 현재 구현과 일치하게 유지한다.

- Node.js 18+
- pnpm
- Rust
- Tauri prerequisites
- Claude Code CLI 설치 및 로그인 완료

## Non-Goals

이번 README 개선에서 다음은 다루지 않는다.

- 제품 자체를 Codex CLI 기반으로 리브랜딩
- Claude adapter 코드명 변경
- Tauri 백엔드 동작 변경
- 번역 파이프라인, 캐시, 버퍼링 로직 변경
- 실제 배포 릴리스 자동화
- 외부 호스팅 서비스에 데모 영상을 업로드하는 자동화

## Implementation Boundaries

변경 범위는 문서와 README용 제작물에 한정한다.

- 수정: `README.md`
- 생성: `docs/assets/readme/`
- 생성: `docs/remotion/readme-demo/`
- 생성: README 데모 제작용 스크립트 또는 설정 파일

앱 런타임 의존성에 Remotion을 추가하지 않는다. Remotion 패키지는 문서용 하위 디렉터리에서 독립적으로 관리하는 방식을 우선한다. 루트 개발 의존성 추가는 하위 디렉터리 구성이 불가능한 경우에만 구현 계획에서 명시적으로 다룬다.

## Verification

완료 전 확인할 항목:

- README에서 Codex가 제품 기술로 언급되지 않는다.
- Claude Code CLI 요구사항과 현재 코드 구현이 충돌하지 않는다.
- README의 이미지/영상 경로가 실제 파일과 일치한다.
- GitHub Markdown에서 표, 이미지, 코드 블록이 깨지지 않는다.
- `pnpm check`가 문서/설정 변경으로 실패하지 않는다.
- Remotion 소스가 추가되면 최소 한 프레임 렌더 또는 빌드 가능한 상태를 확인한다.

## Risks

- 실제 앱 캡처가 환경 문제로 실패할 수 있다.
  - 대응: 코드 기반 재현 영상을 fallback으로 사용한다.
- GitHub README에서 MP4 자동 재생 또는 표시 방식이 제한될 수 있다.
  - 대응: 대표 스크린샷을 항상 제공하고, MP4 표시가 제한되면 GIF를 추가한다.
- README가 너무 길어질 수 있다.
  - 대응: 상단은 쇼케이스 중심으로 압축하고 상세 설명은 `docs/`로 연결한다.
- “추가 비용 없음” 문구가 오해될 수 있다.
  - 대응: “Claude Code 구독 외 별도 API 키 불필요”로 표현한다.

## Approval Summary

확정된 방향:

- 혼합형 README: 상단은 쇼케이스, 아래쪽은 설치와 개발 문서
- 데모 영상: 실제 앱 흐름 기반 + Remotion 콜아웃을 얹은 20-30초 영상
- 자산 제작: 실제 캡처 우선, 실패 시 코드 기반 재현
- 제품 설명: Claude Code CLI 기반, Codex 언급 제외
