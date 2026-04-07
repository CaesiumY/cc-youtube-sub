# Deep Interview Spec: YouTube 번역 자막 데스크탑 앱

## Metadata
- Interview ID: yt-subtitle-2026-04-07
- Rounds: 11
- Final Ambiguity Score: 9%
- Type: greenfield
- Generated: 2026-04-07
- Threshold: 20%
- Status: PASSED

## Clarity Breakdown
| Dimension | Score | Weight | Weighted |
|-----------|-------|--------|----------|
| Goal Clarity | 0.95 | 0.40 | 0.380 |
| Constraint Clarity | 0.90 | 0.30 | 0.270 |
| Success Criteria | 0.87 | 0.30 | 0.261 |
| **Total Clarity** | | | **0.911** |
| **Ambiguity** | | | **0.089** |

## Goal

YouTube 영상 URL을 입력하면 앱 내 임베드 플레이어에서 영상을 재생하면서, Claude Code CLI subprocess를 통해 자막을 청크 단위(30초~1분)로 한국어 번역하여 실시간으로 표시하는 Tauri 데스크탑 앱을 만든다. Paperclip의 `claude_local` 어댑터 패턴(ServerAdapter 인터페이스, graceful shutdown, 환경 검증)을 전체 차용한다. 번역 시 영상 설명과 전후 맥락을 포함하여 고품질 번역을 제공하되, 응답 속도를 최우선으로 한다.

## Constraints

- **앱 프레임워크**: Tauri 2.x (포터블 .exe 배포, ~15MB 번들)
- **UI**: React + TypeScript
- **번역 엔진**: Claude Code CLI subprocess (`claude --print - --output-format stream-json`)
- **자막 소스**: YouTube timedtext undocumented API (API 키 불필요)
- **DB**: SQLite — POC에서는 번역 캐시 테이블 1개만 사용 (v1에서 시청 이력, 설정, 피드백 등으로 확장)
- **타겟 OS**: Windows 우선, macOS 추후
- **번역 단위**: 청크 기반 (30초~1분), 영상 설명은 첫 청크에 포함
- **Paperclip 패턴**: ServerAdapter 인터페이스(execute/testEnvironment), SIGTERM→SIGKILL graceful shutdown, `CLAUDECODE` 환경변수 제거, 환경 검증, 타임아웃 관리 전체 구현
- **비용**: Claude Code 구독 외 추가 비용 $0 (API 키 불필요)
- **사전 버퍼링**: 재생 위치 앞의 자막을 미리 번역하여 체감 지연 최소화
- **캐시 재활용**: 동일 영상 재방문 시 DB에서 즉시 로드 (재번역 방지)
- **라우팅**: Tanstack Router (`/` → `/watch/$videoId`)
- **풀스크린**: Tauri 윈도우 풀스크린 (YouTube iframe 풀스크린 대체)
- **자막 표시**: 영상 위 자막 오버레이 (반투명 박스, YouTube 컨트롤 바 바로 위)

## Non-Goals

- 자막 없는 영상의 STT(Speech-to-Text) 처리
- 브라우저 창 위치 자동 추적 (앱 내 임베드로 불필요해짐)
- macOS 네이티브 풀스크린 지원
- API 키 폴백 (Claude Code 구독 전제)
- 사용자 계정 / 인증 시스템
- 다국어 지원 (영어 → 한국어 단방향 고정)
- 시청 이력, 즐겨찾기, 번역 품질 피드백 (v1으로 연기)
- 확장 가능한 DB 스키마 (POC에서는 캐시 1 테이블)

## Acceptance Criteria

- [ ] YouTube URL 입력 → 앱 내 임베드 플레이어에서 영상 재생
- [ ] URL 입력 후 5초 이내에 첫 한국어 번역 자막 표시
- [ ] 재생 중 끊김 없이 자막이 연속으로 표시됨 (사전 버퍼링)
- [ ] 동일 영상 재방문 시 SQLite 캐시에서 즉시 자막 로드
- [ ] 번역 시 영상 설명 + 전후 맥락이 프롬프트에 포함됨
- [ ] Claude Code CLI subprocess가 Paperclip ServerAdapter 패턴으로 관리됨
- [ ] `CLAUDECODE` 환경변수가 자식 프로세스에서 제거됨
- [ ] 앱 시작 시 Claude CLI 바이너리 PATH 존재 확인 (testEnvironment)
- [ ] Claude 프로세스 비정상 종료 시 SIGTERM → SIGKILL graceful shutdown
- [ ] Claude 구독 한도 초과 시 사용자에게 에러 메시지 표시
- [ ] 자막 없는 영상 입력 시 안내 메시지 표시
- [ ] Claude Code 구독 외 추가 비용 $0
- [ ] 캐시 miss 위치로 seek 시 '번역 준비 중...' 인디케이터 표시
- [ ] 캐시 hit 위치로 seek 시 즉시 자막 표시
- [ ] 번역 결과가 JSON 배열 `[{original, translated, start, end}]` 포맷으로 구조화됨
- [ ] Tauri 윈도우 풀스크린(F키)에서 자막 오버레이 유지
- [ ] Home → Player 뷰 전환이 fade 애니메이션으로 동작
- [ ] Player에서 뒤로가기 버튼(←)으로 Home 복귀

## Assumptions Exposed & Resolved

| Assumption | Challenge | Resolution |
|------------|-----------|------------|
| 투명 오버레이 윈도우가 필요하다 | 앱 자체에 YouTube를 임베드하면 별도 윈도우 오버레이가 불필요 | **영상 위 반투명 박스 오버레이로 전환** — 임베드 플레이어 위에 직접 오버레이, 재생 시간 추적이 자연스럽고 오버레이 버그 리스크 제거 |
| 전체 일괄 번역이 최적이다 | 1시간 강의는 일괄 번역 불가, 15초 대기도 길다 | **청크 단위(30초~1분) 번역** — 긴 영상 지원 + 초기 대기 최소화 |
| DB가 처음부터 확장 가능해야 한다 | POC는 검증 도구이지 제품이 아니다 | **SQLite 캐시 1 테이블로 시작**, v1에서 확장 |
| Paperclip 핵심만 차용하면 된다 | 전체 어댑터 패턴이 에러 처리/안정성에 중요 | **ServerAdapter 전체 패턴 구현** — testEnvironment, graceful shutdown 포함 |
| 번역 품질이 최우선이다 | 사용자는 속도를 더 중시 | **응답 속도 우선** — 5초 내 첫 자막, 다소 부자연스러운 번역 감수 가능 |
| YouTube iframe 풀스크린이 작동할 것이다 | iframe 풀스크린 시 DOM 외부 오버레이가 가려짐 | **Tauri 윈도우 풀스크린으로 대체** — F키로 Tauri 윈도우 자체를 풀스크린, 오버레이가 항상 iframe 위에 유지됨 |

## Technical Decisions

| 결정 | 선택지 | 선택 | 이유 |
|------|--------|------|------|
| DB 엔진 | PostgreSQL vs SQLite | **SQLite** | 단일 사용자 데스크탑 앱, DB 서버 설치 불필요, 파일 1개로 포터블 배포. Paperclip은 PostgreSQL이지만 다중 에이전트/멀티 테넌시 서버 앱이라 상황이 다름 |
| 프론트엔드 렌더링 | 네이티브 UI vs 웹뷰 (Tauri) | **웹뷰 (Tauri 2.x)** | OS 내장 WebView2(Windows)/WebKit(macOS) 사용. YouTube iframe API 호환, React 재사용 가능, 번들 ~15MB. Electron 대비 10배 이상 경량 |
| 백엔드 런타임 | Node.js vs Rust | **Rust (Tauri 내장)** | Tauri 백엔드가 Rust이므로 자연스러운 선택. subprocess 관리, SQLite 접근, 파일 시스템 모두 Rust에서 처리 |
| Paperclip 패턴 범위 | subprocess spawn만 vs ServerAdapter 전체 | **ServerAdapter 전체** | testEnvironment(환경 검증), graceful shutdown(SIGTERM→SIGKILL), 타임아웃 관리가 안정성에 필수 |
| 번역 전략 | 전체 일괄 vs 청크 단위 | **청크 (30초~1분)** | 1시간 강의 등 긴 영상은 일괄 불가. 청크 경계 맥락 단절 대가가 있지만 초기 대기 최소화가 더 중요 |
| 번역 결과 포맷 | 플레인 텍스트 vs JSON | **JSON 배열** | `[{original, translated, start, end}]` 구조로 파싱, 캐시 저장, 프론트엔드 렌더링 모두에 직접 매핑 |
| 최적화 방향 | 번역 품질 vs 응답 속도 | **응답 속도 우선** | URL 입력 후 5초 내 첫 자막이 핵심 KPI. 맥락 기반 번역으로 품질도 유지하되, 속도와 충돌 시 속도 우선 |
| 자막 위치 | 영상 아래 별도 영역 vs 영상 위 오버레이 | **영상 위 오버레이** | 넷플릭스 스타일, 영상과 자막이 하나의 시청 경험. YouTube 컨트롤 바 바로 위 반투명 박스 |
| 풀스크린 | YouTube iframe Fullscreen API vs Tauri 윈도우 | **Tauri 윈도우 풀스크린** | iframe 풀스크린 시 오버레이가 가려지는 문제 회피. Tauri `window.setFullscreen()` API 사용 |
| 라우팅 | React Router vs Tanstack Router vs 조건부 렌더링 | **Tanstack Router** | 타입 안전한 경로 파라미터, TypeScript 친화. `/watch/$videoId` 구조 지원 |
| 뷰 구조 | 싱글 페이지 vs 2-View | **2-View (Home + Player)** | URL 입력은 Home(`/`)에만 집중, Player(`/watch/$videoId`)는 영상 시청에 집중. fade 애니메이션으로 전환 |

## Technical Context

### Paperclip claude_local 어댑터 참조

Paperclip의 `claude_local` 어댑터 핵심 구현을 차용한다:

- **`runChildProcess`**: Node.js subprocess 스폰, stdout/stderr 스트리밍, 생명주기 관리
- **`ServerAdapter` 인터페이스**: `execute()` (에이전트 실행), `testEnvironment()` (CLI 바이너리 PATH 확인, 인증 검증)
- **환경변수**: `CLAUDECODE` 제거 (nested session 오류 방지), 커스텀 env 주입
- **Graceful Shutdown**: 타임아웃 초과 시 SIGTERM 전송, 유예 기간 내 미종료 시 SIGKILL
- **스트리밍 파싱**: stdout JSONL을 파싱하여 번역 결과 추출

### 실행 커맨드
```bash
claude --print - --output-format stream-json --verbose
# stdin으로 번역 프롬프트 주입, stdout JSONL 스트리밍으로 결과 수신
```

### 번역 프롬프트 구조 (청크 기반)
```
[영상 설명] (첫 청크에만 포함)
[이전 청크 마지막 5줄 — 맥락 제공]
[현재 청크 자막 30초~1분 분량]
→ 한국어 번역 요청
→ 응답 포맷: JSON 배열 [{original, translated, start, end}, ...]
```

### 번역 결과 포맷
```json
[
  {
    "original": "Hello everyone, welcome to today's lecture",
    "translated": "여러분 안녕하세요, 오늘 강의에 오신 것을 환영합니다",
    "start": 7.58,
    "end": 11.58
  },
  ...
]
```

### Seek 동작
- **캐시 hit**: 즉시 자막 표시 (지연 0)
- **캐시 miss**: '번역 준비 중...' 인디케이터 표시 → 해당 청크 우선 번역 → 완료 후 자막 전환
- **버퍼 리셋**: seek 시 현재 버퍼링 큐를 새 위치 기준으로 재스케줄링

### 아키텍처
```
Tauri 앱 (포터블 .exe)
├─ [Home View] (/)
│   └─ URL 입력 (중앙)
│
├─ [Player View] (/watch/$videoId)
│   ├─ 뒤로가기 버튼 (←)
│   ├─ YouTube 임베드 플레이어 (iframe API)
│   │   ├─ onStateChange / getCurrentTime() 이벤트
│   │   └─ SubtitleOverlay (반투명 박스, 컨트롤 바로 위)
│   └─ ProgressBar (2px, 영상 아래)
│
├─ [Rust 백엔드]
│   ├─ YouTube timedtext API → 자막 fetch
│   ├─ Claude ServerAdapter (Paperclip 패턴)
│   │   ├─ testEnvironment() — 앱 시작 시 CLI 검증
│   │   ├─ execute() — 청크 번역 subprocess 실행
│   │   └─ graceful shutdown (SIGTERM → SIGKILL)
│   ├─ Translation Buffer Manager
│   │   └─ 재생 위치 기준 선행 청크 번역 스케줄링
│   └─ SQLite Cache
│       └─ video_id + chunk_hash → translated_text
│
└─ [데이터 흐름]
    URL 입력 (Home) → video ID 파싱 → Player View 전환
    → timedtext API 호출 (자막 fetch)
    → 캐시 확인 (hit → 즉시 표시)
    → 캐시 miss → 청크 분할 → Claude subprocess 번역
    → 결과 캐시 저장 + 화면 표시
    → 재생 진행에 따라 다음 청크 선행 번역
```

## Ontology (Key Entities)

| Entity | Type | Fields | Relationships |
|--------|------|--------|---------------|
| Tauri App | core domain | URL input, embedded player, subtitle display, settings | contains YouTube Player, manages ServerAdapter |
| YouTube Video | core domain | video_id, URL, title, description, auto_captions | has many Subtitles |
| Subtitle | core domain | text, start, duration, language | belongs to Video, grouped into Chunks |
| Translation | core domain | original_text, translated_text, context_used, timestamp_range | translates Subtitle, stored in Cache |
| Translation Chunk | core domain | chunk_index, start_time, end_time, subtitle_lines[], status | groups Subtitles, input to Claude subprocess |
| Translation Buffer | supporting | current_position, look_ahead_chunks, pending_chunks[] | schedules Chunk translation ahead of playback |
| Translation Cache | supporting | video_id, chunk_hash, translated_text, created_at | persists Translations in SQLite |
| Claude Subprocess | external system | stdin_prompt, stdout_jsonl, pid, timeout, status | generates Translations via ServerAdapter |
| ServerAdapter | core domain | execute(), testEnvironment(), graceful_shutdown() | manages Claude Subprocess lifecycle |
| Database (SQLite) | infrastructure | cache table, connection pool | stores Translation Cache |
| User Settings | supporting (v1) | language_pair, buffer_size, theme | deferred to v1 |
| Watch History | supporting (v1) | video_id, last_position, watched_at | deferred to v1 |

## Ontology Convergence

| Round | Entity Count | New | Changed | Stable | Stability Ratio |
|-------|-------------|-----|---------|--------|----------------|
| 1 | 7 | 7 | - | - | N/A |
| 2 | 10 | 3 | 1 | 6 | 70% |
| 3 | 10 | 0 | 0 | 10 | 100% |
| 4 | 11 | 1 | 0 | 10 | 91% |
| 5-7 | 11 | 0 | 0 | 11 | 100% |
| 8 | 12 | 1 | 0 | 11 | 92% |
| 9 | 12 | 0 | 0 | 12 | 100% |
| 10-11 | 12 | 0 | 0 | 12 | 100% |

도메인 모델은 Round 3부터 안정화되었으며, Round 4(Translation Chunk)과 Round 8(ServerAdapter)에서 신규 엔티티가 추가된 후 즉시 재안정화됨. Round 9 이후 6 라운드 연속 완전 수렴 상태.

## Interview Transcript
<details>
<summary>Full Q&A (11 rounds)</summary>

### Round 1
**Q:** 사용자가 앱을 실행한 후 YouTube URL을 붙여넣고 → 번역이 완료될 때까지 기다린 뒤 → 브라우저에서 직접 재생 버튼을 누르는 흐름인가요? 아니면 앱 자체가 재생 타이밍을 제어하는 방식을 구상하고 있나요?
**A:** 유튜브 url을 붙여 넣으면 유튜브 재생 미디어가 그대로 나와. 그리고 자막을 실시간 번역해서 보여주는 거야. 자막은 이전과 이후 맥락, 유튜브 영상 설명을 포함해서 생성을 진행해야 해. 유저가 자연스러운 번역 타이밍을 기대할 수 있도록 앞 뒤로 미리 번역해둬야 하고, 데이터베이스에 이러한 내용을 저장해서 이미 번역한 내용을 다시 번역해서 리소스 낭비가 생기는 일은 막아야 해.
**Ambiguity:** 67% (Goal: 0.60, Constraints: 0.20, Criteria: 0.10)

### Round 2
**Q:** 번역 캐시 DB가 순수하게 '동일 영상 재방문 시 재번역 방지'만 담당하나요? 아니면 사용자의 시청 기록, 즐겨찾기, 번역 품질 피드백 등 더 넓은 데이터도 저장하는 것을 구상하고 계신가요?
**A:** 확장 가능한 범용 DB
**Ambiguity:** 63% (Goal: 0.60, Constraints: 0.35, Criteria: 0.10)

### Round 3
**Q:** 완성된 앱을 보고 '이거다!' 하고 느끼려면 어떤 조건이 충족되어야 할까요? 특히 자막이 화면에 나타나는 타이밍에 대해—영어 원본 자막이 뜨는 순간과 한국어 번역 자막이 뜨는 순간 사이의 허용 가능한 지연은 어느 정도인가요?
**A:** 사전 버퍼링으로 지연 최소화
**Ambiguity:** 55% (Goal: 0.60, Constraints: 0.35, Criteria: 0.35)

### Round 4
**Q:** '맥락 기반 번역'이 핵심 차별화 포인트인데, Claude에게 번역을 요청할 때 한 번의 프롬프트에 어느 범위의 자막을 보내는 것을 상상하고 계신가요?
**A:** 청크 단위 (30초~1분)
**Ambiguity:** 47% (Goal: 0.70, Constraints: 0.50, Criteria: 0.35)

### Round 5 [Contrarian Mode]
**Q:** 청크 단위 번역 대신 전체 일괄 번역이 더 낫지 않은가? 10분 영상은 ~15초면 전체 번역이 끝나고, 청크 경계 맥락 단절과 복잡한 버퍼 관리를 피할 수 있다.
**A:** 아니다, 청크 유지 — 1시간 강의 등 긴 영상은 전체 일괄 불가, 초기 대기 최소화가 핵심
**Ambiguity:** 42% (Goal: 0.75, Constraints: 0.55, Criteria: 0.40)

### Round 6
**Q:** POC의 성공 기준을 구체적으로 정의하고 싶습니다. 핵심 성공 지표는?
**A:** 응답 속도 우선 — URL 입력 후 5초 이내 첫 자막 표시가 핵심. 다소 부자연스러운 번역은 감수
**Ambiguity:** 34% (Goal: 0.80, Constraints: 0.55, Criteria: 0.60)

### Round 7 [Simplifier Mode]
**Q:** POC 단계에서 Tauri 임베드 + 청크 번역 + 확장 DB + 버퍼링이 모두 필요한가?
**A:** POC는 최소한으로 — 임베드 + 번역 + 캐시(SQLite 1 테이블)만. 이력/피드백/확장 DB는 v1으로 미룸
**Ambiguity:** 27% (Goal: 0.85, Constraints: 0.65, Criteria: 0.65)

### Round 8
**Q:** Paperclip의 claude_local 어댑터를 어느 수준까지 따라가려는 건가요?
**A:** 어댑터 패턴 전체 — ServerAdapter 인터페이스, graceful shutdown, 환경 검증, 타임아웃 관리 모두 구현
**Ambiguity:** 21% (Goal: 0.90, Constraints: 0.80, Criteria: 0.65)

### Round 9
**Q:** POC 데모 시나리오를 구체적으로 알려주세요.
**A:** 위 시나리오로 충분 — (1) 영어 강의 URL → 5초 내 첫 자막 (2) 재생 중 끊김 없는 자막 (3) 재방문 시 즉시 로드
**Ambiguity:** 15% (Goal: 0.92, Constraints: 0.82, Criteria: 0.80)

### Round 10
**Q:** 사용자가 영상 중간으로 탐색(seek)하면 어떻게 되어야 하나요? 캐시가 없는 위치로 점프하면 자막 없이 재생되는 게 괜찮은가요?
**A:** 번역 중 표시 — 캐시 miss 시 '번역 준비 중...' 인디케이터 표시. 사용자가 상황을 인지할 수 있도록
**Ambiguity:** 12% (Goal: 0.93, Constraints: 0.85, Criteria: 0.85)

### Round 11
**Q:** 번역 결과 포맷은 어떤 형태를 기대하나요? JSON 배열, 플레인 텍스트, 또는 Claude에게 위임?
**A:** JSON 배열 (구조화) — [{original, translated, start, end}, ...] 형태로 타임스탬프 포함. 파싱과 캐시 저장에 적합
**Ambiguity:** 9% (Goal: 0.95, Constraints: 0.90, Criteria: 0.87)

</details>
