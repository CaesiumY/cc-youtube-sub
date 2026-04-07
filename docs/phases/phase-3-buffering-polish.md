# Phase 3: 버퍼링 + 완성

## 목표

Phase 1(자막 fetch + 표시)과 Phase 2(Claude 번역 + 캐시)를 기반으로, 재생 위치 기준 선행 청크 번역으로 체감 지연을 제거하고, Seek 처리 및 에러 핸들링을 완성하여 **전체 15개 Acceptance Criteria 검증**을 마친다. 성공 KPI는 URL 입력 후 5초 이내 첫 자막 표시이며, 긴 영상(1시간+)에서도 안정적으로 동작해야 한다.

## 검증 리스크

**최고 우선 기술 리스크**: Translation Buffer Manager가 재생 위치 앞의 청크를 정확하게 스케줄하고, Seek 시 큐를 올바르게 리셋하는가?

- 버퍼 스케줄링 알고리즘 결함 → 자막이 뜨는 타이밍이 불안정할 수 있음
- Seek 후 중복 번역 또는 스킵된 청크 발생 가능
- 동시 실행 제한 미흡 → Claude 프로세스 폭증으로 메모리/한도 초과 위험
- 긴 영상에서 버퍼 관리 누수 → 메모리 증가, 앱 성능 저하

**이 Phase 실패 = 사용자 경험 저하 (자막 끊김, 지연 증가, 에러 메시지 노출)** → POC 검증 목표 달성 불가

## 구현 범위

- [ ] **Translation Buffer Manager 구현**
  - [ ] 재생 위치 기준 선행 청크 스케줄링 로직
    - 현재 재생 시간 기준으로 다음 N개 청크 식별
    - 우선순위 큐: 현재 위치에 가장 가까운 청크부터 번역
    - Look-ahead 윈도우: 기본 2~3개 청크 (30초~1분 단위)
  - [ ] 버퍼 큐 관리
    - 동시 실행 제한: 최대 1~2개 번역 프로세스만 병렬 실행 (Claude 한도 고려)
    - 큐 상태 추적: pending, in_progress, completed
    - 타임아웃 처리: 5초 이상 응답 없는 프로세스는 취소 후 재시도
  - [ ] 메모리 누수 방지
    - 완료된 청크 메타데이터 정리
    - 대기 중인 프로세스 정리 (Seek 시)

- [ ] **Seek 처리**
  - [ ] 캐시 hit → 즉시 자막 표시
    - DB에서 해당 청크 조회
    - 지연 0, 로딩 표시 없음
  - [ ] 캐시 miss → '번역 준비 중...' 인디케이터
    - SubtitleOverlay 박스 내부에 로딩 텍스트 표시 (shimmer 효과)
    - 해당 청크를 우선순위 큐 최상단에 추가
    - 번역 완료 시 자막 전환 (fade-in)
  - [ ] 버퍼 큐 리셋 + 재스케줄링
    - Seek 이벤트 수신 → 기존 pending 큐 모두 취소
    - 새 위치 기준으로 버퍼링 재시작
    - 진행 중인 프로세스는 완료 후 폐기 (결과 캐시하지 않음)

- [ ] **풀스크린 처리 (Tauri 윈도우 풀스크린)**
  - [ ] F키 → `appWindow.setFullscreen(true/false)` 토글
  - [ ] YouTube iframe 풀스크린 비활성화 (자막 오버레이가 가려지므로)
  - [ ] Tauri 풀스크린 전환 후 SubtitleOverlay position: absolute 유지 확인
  - [ ] 풀스크린 상태에서 오버레이 박스 크기/위치 올바르게 조정

- [ ] **에러 핸들링**
  - [ ] 자막 없는 영상 안내
    - timedtext API 응답 empty → 사용자 메시지 표시 (SubtitleOverlay 영역 내)
    - 메시지: "이 영상에는 자막이 없습니다. 다른 영상을 시도해주세요."
  - [ ] Claude 구독 한도 초과 안내
    - subprocess stderr에서 `rate limit` 또는 `exceeded` 감지
    - 메시지: SubtitleOverlay 영역 내에 "Claude Code 구독 한도를 초과했습니다. 나중에 다시 시도해주세요." 표시
  - [ ] Claude CLI 미설치 안내
    - 앱 시작 시 `claude --version` 실행 실패
    - 메시지: 모달 다이얼로그 (앱 차단) — "Claude Code CLI가 설치되지 않았습니다. `npm install -g @anthropic-ai/claude-code` 실행 후 다시 시도해주세요."
  - [ ] 번역 중 프로세스 비정상 종료 복구
    - SIGTERM/SIGKILL 감지 → 해당 청크 상태를 pending으로 변경
    - 사용자 메시지: SubtitleOverlay 영역 내에 "번역 중 오류가 발생했습니다. 자막을 다시 로드합니다." 표시
    - 자동 재시도: 최대 3회

- [ ] **성능 최적화**
  - [ ] URL 입력 → 첫 자막 5초 이내 목표
    - 첫 청크 번역 시간 측정 (timedtext fetch + Claude subprocess)
    - 목표: 합계 < 5초
    - 최적화: 병렬 fetch + 조기 버퍼링 시작 (자막 일부만 필요)
  - [ ] 긴 영상(1시간+) 안정성 검증
    - 60분 강의 URL 테스트
    - 메모리 누수 확인 (초기 vs 종료 시 메모리 사용량)
    - 버퍼 큐 크기 모니터링 (최대 몇 개인가)
    - 전체 영상 재생 시 자막 끊김 없이 완료

- [ ] **전체 Acceptance Criteria 검증 (16개)**
  - [ ] AC 1: YouTube URL 입력 → 앱 내 임베드 플레이어에서 영상 재생
  - [ ] AC 2: URL 입력 후 5초 이내에 첫 한국어 번역 자막 표시
  - [ ] AC 3: 재생 중 끊김 없이 자막이 연속으로 표시됨 (사전 버퍼링)
  - [ ] AC 4: 동일 영상 재방문 시 SQLite 캐시에서 즉시 자막 로드
  - [ ] AC 5: 번역 시 영상 설명 + 전후 맥락이 프롬프트에 포함됨
  - [ ] AC 6: Claude Code CLI subprocess가 Paperclip ServerAdapter 패턴으로 관리됨
  - [ ] AC 7: `CLAUDECODE` 환경변수가 자식 프로세스에서 제거됨
  - [ ] AC 8: 앱 시작 시 Claude CLI 바이너리 PATH 존재 확인 (testEnvironment)
  - [ ] AC 9: Claude 프로세스 비정상 종료 시 SIGTERM → SIGKILL graceful shutdown
  - [ ] AC 10: Claude 구독 한도 초과 시 사용자에게 에러 메시지 표시
  - [ ] AC 11: 자막 없는 영상 입력 시 안내 메시지 표시
  - [ ] AC 12: Claude Code 구독 외 추가 비용 $0
  - [ ] AC 13: 캐시 miss 위치로 seek 시 '번역 준비 중...' 인디케이터 표시
  - [ ] AC 14: 캐시 hit 위치로 seek 시 즉시 자막 표시
  - [ ] AC 15: 번역 결과가 JSON 배열 `[{original, translated, start, end}]` 포맷으로 구조화됨
  - [ ] AC 16: Tauri 풀스크린에서 SubtitleOverlay가 영상 위에 유지됨 (F키 토글)

## 제외 범위

- 사용자 계정 / 인증 시스템 (v1으로 연기)
- 다국어 지원 (영어 → 한국어 단방향 고정)
- 시청 이력, 즐겨찾기, 번역 품질 피드백 (v1으로 연기)
- 확장 가능한 DB 스키마 (POC에서는 캐시 1 테이블)
- 자막 없는 영상의 STT(Speech-to-Text) 처리 (Non-Goal)
- macOS 네이티브 풀스크린 지원 (Non-Goal — Tauri 윈도우 풀스크린으로 대체, macOS 네이티브 전체화면 API는 미구현)

## 기술 상세

### Translation Buffer Manager 아키텍처

```
[재생 위치 폴링 (500ms)]
    ↓
[현재 시간 + Look-ahead 윈도우 계산]
    ↓ currentTime = 35.5s, 청크 크기 30초~1분
    ↓ 청크 인덱스: 0: 0-30s, 1: 30-60s, 2: 60-90s, ...
    ↓ 현재 청크: idx=1 (30-60s)
    ↓ 선행 청크: idx=2, 3 (look-ahead=2)
    ↓
[우선순위 큐 관리]
    ├─ idx=1 (현재) — 이미 캐시됨? → skip / 번역 중? → skip / 미완료? → 우선도 높음
    ├─ idx=2 (다음) — 우선도 중
    └─ idx=3 (다음다음) — 우선도 낮음
    ↓
[동시 실행 제한 (max 2 프로세스)]
    ├─ if in_progress < 2:
    │   ├─ pending 중 가장 우선도 높은 것 → spawn subprocess
    │   └─ status = in_progress
    └─ else: 대기 (기존 프로세스 완료 대기)
    ↓
[완료 시 처리]
    ├─ 번역 결과 → DB 캐시
    ├─ 상태 = completed
    └─ 자막 렌더링 (현재 위치가 이 청크 범위면 즉시 표시)
```

### Seek State Machine

```
[재생 위치 @ 35s]
    ↓ (사용자 Seek to 180s)
[Seek 이벤트 감지]
    ↓
[캐시 조회: idx=6 (180s 청크)]
    ├─ Hit (DB에 이미 번역됨):
    │   ├─ 기존 대기 큐 전부 취소
    │   ├─ 자막 즉시 표시 (지연 0)
    │   └─ 새 위치 기준 버퍼링 재시작
    │
    └─ Miss (DB에 없음):
        ├─ '번역 준비 중...' 표시
        ├─ idx=6을 우선도 최고로 큐 추가
        ├─ 기존 pending 큐 모두 취소
        ├─ 진행 중 프로세스: 완료까지 진행하되, 결과 폐기 (idx != 6)
        └─ idx=6 번역 완료 → 자막 전환 (fade-in)
```

### 에러 처리 매트릭스

| 시나리오 | 감지 방법 | 사용자 메시지 | 복구 방식 |
|---------|---------|-------------|---------|
| 자막 없는 영상 | timedtext API 응답 empty | "이 영상에는 자막이 없습니다." | 메시지 표시, 앱 대기 |
| Claude 한도 초과 | subprocess stderr `rate limit` | "Claude Code 구독 한도를 초과했습니다." | 재시도 버튼 제공 |
| Claude CLI 미설치 | 앱 시작 시 `claude --version` 실패 | "Claude Code CLI가 설치되지 않았습니다. `npm install -g @anthropic-ai/claude-code`" | 설치 가이드 표시 |
| 번역 프로세스 비정상 종료 | SIGTERM/SIGKILL 감지 | "번역 중 오류가 발생했습니다. 자막을 다시 로드합니다." | 최대 3회 자동 재시도 |
| Seek 중 네트워크 지연 | Seek 이벤트 후 5초 이상 응답 없음 | "번역 준비 중..." (계속 표시) | 타임아웃 후 취소, 재시도 가능 |

### 성능 벤치마크 목표

| 지표 | 목표 | 측정 방법 |
|-----|-----|---------|
| URL → 첫 자막 시간 | < 5초 | 시작 시간부터 자막 표시까지 소요 시간 |
| 자막 표시 지연 (캐시 hit) | 0초 | Seek → 자막 표시 시간 차이 |
| 자막 표시 지연 (캐시 miss) | < 3초 | Seek → '번역 준비 중...' → 자막 표시 시간 |
| 메모리 누수 (1시간 영상) | < 50MB 증가 | 시작 vs 종료 시 메모리 사용량 |
| 버퍼 큐 최대 크기 | < 10개 청크 | 모니터링 로그 |

### Rust 백엔드 구조 (Phase 3 추가)

```rust
// src-tauri/src/buffer_manager.rs
pub struct TranslationBufferManager {
    current_position: f64,
    look_ahead_chunks: usize,  // 기본 2~3
    max_concurrent: usize,      // 기본 1~2
    queue: PriorityQueue<ChunkTask>,
    in_progress: HashMap<ChunkId, ProcessHandle>,
    cache: SqliteCache,
}

impl TranslationBufferManager {
    pub fn update_playback_position(&mut self, current_time: f64) {
        // 1. 현재 위치 기반 청크 식별
        // 2. 우선순위 큐 재정렬
        // 3. 스케줄링 (max_concurrent 내)
    }

    pub fn on_seek(&mut self, target_time: f64) {
        // 1. 캐시 조회
        // 2. pending 큐 전부 취소
        // 3. in_progress 상태 추적 (완료 후 폐기)
        // 4. 새 위치 기준 버퍼링 시작
    }

    pub fn handle_process_completion(&mut self, chunk_id: ChunkId, result: TranslationResult) {
        // 1. 번역 결과 → DB 저장
        // 2. UI 업데이트 신호
        // 3. 다음 pending 태스크 spawn
    }

    pub fn handle_process_error(&mut self, chunk_id: ChunkId, error: ProcessError) {
        // 1. 에러 타입별 처리 (rate limit, crash, timeout)
        // 2. 재시도 로직 (최대 3회)
        // 3. 사용자 메시지 반환
    }
}

// src-tauri/src/error_handler.rs
pub enum TranslationError {
    NoSubtitles,           // 자막 없음
    RateLimitExceeded,     // 한도 초과
    CliNotFound,           // CLI 미설치
    ProcessCrashed,        // 프로세스 비정상 종료
    NetworkTimeout,        // 네트워크 타임아웃
}

pub fn handle_error(error: TranslationError) -> String {
    match error {
        TranslationError::NoSubtitles => "이 영상에는 자막이 없습니다. 다른 영상을 시도해주세요.".to_string(),
        TranslationError::RateLimitExceeded => "Claude Code 구독 한도를 초과했습니다. 나중에 다시 시도해주세요.".to_string(),
        TranslationError::CliNotFound => "Claude Code CLI가 설치되지 않았습니다. `npm install -g @anthropic-ai/claude-code` 실행 후 다시 시도해주세요.".to_string(),
        TranslationError::ProcessCrashed => "번역 중 오류가 발생했습니다. 자막을 다시 로드합니다.".to_string(),
        TranslationError::NetworkTimeout => "네트워크 연결이 끊겼습니다. 다시 시도해주세요.".to_string(),
    }
}
```

### React 컴포넌트 (UI 업데이트)

`SubtitleDisplay` 대신 Phase 2에서 정의한 `SubtitleOverlay`를 사용한다. Phase 3에서는 풀스크린 처리와 키보드 단축키(F키 포함)가 추가된다.

```typescript
// src/components/SubtitleOverlay.tsx (Phase 2 기반 확장)
export function SubtitleOverlay({ 
  subtitles, 
  currentTime, 
  loadingState,
  showOriginal,       // T키 토글 (Phase 2)
  onToggleOriginal,
}: Props) {
  const current = subtitles.find(
    s => s.start <= currentTime && currentTime < s.end
  );

  // shimmer 효과: 캐시 miss 후 번역 대기 중
  const isShimmering = loadingState.seeking && !current;

  return (
    // position: absolute, bottom: ~60px, 영상 컨테이너 기준
    <div className={`subtitle-overlay ${isShimmering ? 'shimmer' : ''}`}>
      {isShimmering && (
        <div className="loading">번역 준비 중...</div>
      )}
      {current && !isShimmering && (
        <div className="subtitle-text">
          <div className="translated">{current.translated}</div>
          {showOriginal && (
            <div className="original">{current.original}</div>
          )}
        </div>
      )}
      {/* 에러 상태도 오버레이 내부에 표시 */}
      {loadingState.error && (
        <div className="error-text">{loadingState.error}</div>
      )}
    </div>
  );
}

// src/hooks/useKeyboardShortcuts.ts (Phase 3 추가: F키)
useEffect(() => {
  const handler = async (e: KeyboardEvent) => {
    switch (e.key) {
      case 't': case 'T': onToggleOriginal(); break;
      case '+': case '=': increaseFontSize(); break;
      case '-': decreaseFontSize(); break;
      case ' ': togglePlayPause(); break;
      case 'f': case 'F':
        // Tauri 윈도우 풀스크린 토글 (YouTube iframe 풀스크린 대체)
        const { appWindow } = await import('@tauri-apps/api/window');
        const isFs = await appWindow.isFullscreen();
        await appWindow.setFullscreen(!isFs);
        break;
    }
  };
  window.addEventListener('keydown', handler);
  return () => window.removeEventListener('keydown', handler);
}, []);
```

## 완료 기준

- [ ] Translation Buffer Manager 구현 완료 (우선순위 큐, 동시 실행 제한)
- [ ] Seek 이벤트 처리: 캐시 hit/miss 분기 동작 확인
- [ ] 에러 처리: 모든 에러 타입에 대해 사용자 메시지 표시 (에러 상태는 SubtitleOverlay 내부에 표시)
- [ ] 풀스크린: Tauri 윈도우 풀스크린 전환 후 SubtitleOverlay 유지 확인
- [ ] 키보드 단축키 F키: Tauri 풀스크린 토글 동작 확인
- [ ] 성능 검증:
  - [ ] URL 입력 → 첫 자막 < 5초 측정 (3회 이상 평균)
  - [ ] 캐시 hit seek → 자막 즉시 표시 (지연 < 500ms)
  - [ ] 캐시 miss seek → '번역 준비 중...' → 자막 전환 (< 3초)
  - [ ] 60분 영상 전체 재생 → 자막 끊김 없음
  - [ ] 메모리 누수 검증 (시작 vs 종료 메모리 < 50MB 차이)
- [ ] 전체 15개 Acceptance Criteria 검증: 모두 PASS
- [ ] TypeScript strict 모드 + ESLint: 타입 에러 없음
- [ ] Rust 백엔드: 컴파일 성공, 런타임 panic 없음
- [ ] UI: 로딩 표시, 에러 메시지 정상 렌더링
- [ ] 빌드 성공 (`npm run build` / `tauri build`)

## 다음 단계 의존성

**Phase 3 완료 = POC 검증 완료**

1. **프로덕션 배포**: 빌드된 .exe를 배포 가능한 형태로 패키징
2. **문서화**: README, 설치 가이드, 사용자 매뉴얼 작성
3. **v1 로드맵**: 사용자 계정, 설정 저장, 시청 이력, 다국어 지원 등 기획

## 실패 시 대안

**시나리오 1: 버퍼 스케줄링이 불안정하여 자막이 끊김**

1. **원인 분석**
   - 우선순위 큐 정렬 로직 결함
   - Look-ahead 윈도우 크기 부적절
   - 동시 실행 제한으로 인한 병목

2. **대안 선택지**
   - **대안 A: Look-ahead 크기 조정**
     - 현재 2~3개 → 1개로 축소 (안정성 우선)
     - 대신 자막 표시 지연 증가 (1~2초)
     - 트레이드오프: 안정성 ↑, 체감 품질 ↓
   
   - **대안 B: 동시 실행 제한 완화**
     - 1~2개 → 3~4개로 증가
     - Claude 한도 모니터링 강화
     - 트레이드오프: 응답 속도 ↑, 한도 초과 위험 ↑
   
   - **대안 C: 전체 일괄 번역으로 되돌리기**
     - 영상 시작 시 전체 번역 후 재생 (Phase 2 방식)
     - 장점: 버퍼 관리 불필요, 간단함
     - 단점: 초기 대기 시간 증가, 긴 영상 불가 (원래 거부했던 설계)
     - **권장하지 않음**: PRD에서 청크 + 버퍼링 명시

3. **권장 방향**
   - 대안 A가 가장 현실적 (안정성 확보 후 최적화)
   - Phase 3 실패 확률 낮음 (버퍼링은 복잡하지만 구현 가능)

---

**Phase 3 완료 조건**: 전체 16개 Acceptance Criteria 모두 PASS + 성능 벤치마크 달성 + 긴 영상 안정성 검증

**예상 소요 시간**: 3-4주 (버퍼 알고리즘 설계 + 구현 + 테스트 + 최적화)

**POC 최종 검증 일정**:
1. 기본 버퍼링 동작 (1주)
2. Seek 처리 + 에러 핸들링 (1주)
3. 성능 최적화 + 벤치마킹 (1주)
4. 전체 15개 AC 검증 + 긴 영상 테스트 (1주)
