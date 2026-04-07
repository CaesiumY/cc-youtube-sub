# Phase 1: 자막 파이프라인

## 목표

YouTube 영상의 자막을 실시간으로 한국어 번역하는 핵심 파이프라인을 검증한다. 영상 URL 입력 후 **5초 이내에 첫 한국어 번역 자막을 표시**하고, 청크 단위 번역(30초~1분)으로 긴 영상(1시간 이상)도 지원하는 것을 확인한다. Paperclip의 `claude_local` 어댑터 패턴(ServerAdapter 인터페이스, graceful shutdown, 환경 검증)을 전체 구현하여 Claude CLI subprocess 관리의 안정성을 확보한다.

## 검증 리스크

| 리스크 | 영향 | 완화 전략 |
|--------|------|---------|
| YouTube timedtext API 구조 변경 | 자막 파싱 실패 | 실제 영상 3개 이상 테스트 (한국어/영어 자막 모두) |
| Claude CLI 응답 포맷 변경 | JSONL 파싱 실패 | `claude --output-format stream-json` 명시적 확인, 실제 스트림 테스트 |
| subprocess 좀비 프로세스 | 메모리 누수, 재실행 불가 | SIGTERM → SIGKILL 타임아웃 검증, 여러 번 실행/종료 반복 테스트 |
| 청크 경계 맥락 단절 | 부자연스러운 번역 | 전후 5줄 맥락 포함 여부 확인 (번역 품질 테스트) |
| 환경 검증 실패 (Claude CLI 미설치) | 사용자 혼란 | testEnvironment() 구현 및 에러 메시지 명시화 |
| 캐시 miss 시 지연 | 사용자 체감 불만 | 캐시 miss 위치 seek 시 '번역 준비 중...' 인디케이터 표시 검증 |

## 구현 범위

### 1. YouTube 자막 fetch — `yt-transcript-rs` crate (Rust)
- [ ] `yt-transcript-rs` crate 의존성 추가 (`Cargo.toml`)
- [ ] `TranscriptApi::get_transcript(video_id, &["en"])` 호출로 자막 fetch
- [ ] 자막 데이터 구조 정의: `{text: String, start: f64, duration: f64}` (crate 반환값 매핑)
- [ ] 자막 없는 영상 감지 및 에러 처리 (`yt-transcript-rs` 에러 → `AppError::CaptionFetch`)
- [ ] 중복 자막 제거 (필요 시 후처리)
- [ ] **참고**: timedtext API 직접 호출 불필요 — `yt-transcript-rs`가 내부적으로 처리

### 2. 자막 데이터 파싱
- [ ] `yt-transcript-rs` 반환값 → `{start, end}` 정규화 (`start + duration = end`)
- [ ] 특수문자 디코딩 (HTML entities: `&quot;`, `&amp;` 등 — crate 미처리 시 `quick-xml` 보조 사용)
- [ ] 빈 자막 라인 필터링
- [ ] **참고**: XML 파싱은 `yt-transcript-rs`가 내부 처리. `quick-xml`은 커스텀 파싱이 필요한 경우에만 사용

### 3. 청크 분할 로직
- [ ] 자막 라인 배열 → 시간 범위 기반 청크 생성
- [ ] 청크 크기: 30초~1분 (최소 1줄, 최대 20줄)
- [ ] 청크 경계: 자막 라인 기준 (시간 기반이 아닌 라인 기반)
- [ ] 청크 구조: `{index: i32, start_time: f64, end_time: f64, lines: [String]}`
- [ ] 첫 청크에 영상 설명 prepend (별도 fetch 또는 사용자 입력)

### 4. Claude ServerAdapter 인터페이스 구현

#### 4a. testEnvironment() — CLI 바이너리 PATH 확인
- [ ] `claude --version` 또는 `claude --help` 실행 가능 확인
- [ ] 실행 결과 stdout에 "claude" 문자열 포함 여부 검증
- [ ] 실패 시 에러 메시지: "Claude CLI not found. Please install Claude Code CLI and add to PATH"
- [ ] Tauri 앱 시작 시 호출 (동기)

#### 4b. execute(prompt: String) → 비동기 subprocess 생성 (tauri-plugin-shell)
- [ ] `app.shell().command("claude").args([...]).spawn()` 패턴 사용
- [ ] `.env_remove("CLAUDECODE")` — nested session 방지 (필수)
- [ ] `child.write(prompt.as_bytes())` 로 stdin 프롬프트 전송
- [ ] `CommandEvent::Stdout(line)` 수신 → `app.emit("translation-chunk", &text)` 로 프론트 스트리밍
- [ ] `CommandEvent::Terminated(status)` 수신 → 완료 이벤트 emit
- [ ] 타임아웃 설정: 30초 (청크당)
- [ ] 에러 처리: `CommandEvent::Stderr` 캡처, `AppError::Process`로 변환

#### 4c. graceful shutdown — tauri-plugin-shell 프로세스 정리
- [ ] `tauri-plugin-shell`의 `Child` 핸들로 프로세스 관리
- [ ] 타임아웃(30초) 초과 시 `child.kill()` 호출
- [ ] 종료 상태 로깅 (`CommandEvent::Terminated` 의 exit code 확인)
- [ ] **참고**: `std::process::Command` 직접 사용 대신 `tauri-plugin-shell` 사용으로 프로세스 자동 정리 보장

### 5. CLAUDECODE 환경변수 제거
- [ ] 자식 프로세스 환경: 부모의 `CLAUDECODE` 제거
- [ ] Command 생성 시 env() 메서드로 커스텀 env 전달
- [ ] 테스트: nested session 에러 미발생 확인

### 6. 번역 프롬프트 구성
- [ ] 프롬프트 구조:
  ```
  [VIDEO_DESCRIPTION]
  영상 제목: {title}
  설명: {description}

  [CONTEXT_FROM_PREVIOUS_CHUNK]
  (이전 청크 마지막 5줄, 첫 청크면 생략)

  [CURRENT_CHUNK_SUBTITLES]
  {자막 텍스트 30초~1분 분량}

  [TRANSLATION_INSTRUCTION]
  Please translate the above subtitles to Korean.
  Format: JSON array [{original, translated, start, end}, ...]
  ```
- [ ] 영상 설명: 첫 청크에만 포함 (token 절감)
- [ ] 이전 청크 맥락: 2~N번 청크부터 포함 (마지막 5줄)
- [ ] 시간 정보: 각 자막 라인에 start, end 포함

### 7. stdout JSONL 스트리밍 파싱
- [ ] 라인 단위로 읽기 (BufRead)
- [ ] 각 라인을 JSON 파싱 (serde_json)
- [ ] 예상 응답 형식: `{original: String, translated: String, start: f64, end: f64}`
- [ ] 파싱 실패 시 에러 처리 (불완전한 라인 제외)
- [ ] 스트림 완료 여부 판단 (EOF 또는 특수 마커)

### 8. 번역 결과 JSON 배열 검증
- [ ] 응답 배열 길이 > 0 확인
- [ ] 각 요소의 필수 필드 검증: `{original, translated, start, end}`
- [ ] start, end 타입: f64 (초 단위)
- [ ] translated 필드가 한국어인지 휴리스틱 검증 (선택: CJK 문자 포함)
- [ ] 응답 예시:
  ```json
  [
    {
      "original": "Hello everyone, welcome to today's lecture",
      "translated": "여러분 안녕하세요, 오늘 강의에 오신 것을 환영합니다",
      "start": 7.58,
      "end": 11.58
    }
  ]
  ```

## 제외 범위

- 자막 없는 영상의 STT(Speech-to-Text) 처리
- Tauri UI 프론트엔드 (입력 폼, 플레이어, 자막 표시)
- YouTube iframe 플레이어 임베드
- SQLite 캐시 테이블 설계 및 쿼리 (Phase 2)
- 사전 버퍼링 스케줄러 (Phase 2)
- 다국어 지원 (영어 → 한국어만)
- 번역 품질 피드백 시스템 (v1)

## 기술 상세

### YouTube 자막 fetch — `yt-transcript-rs`

```rust
use yt_transcript_rs::TranscriptApi;

async fn fetch_transcript(video_id: &str) -> anyhow::Result<Vec<TranscriptEntry>> {
    let api = TranscriptApi::new();
    let transcript = api.get_transcript(video_id, &["en"]).await?;
    Ok(transcript)
}
```

- `yt-transcript-rs`가 timedtext API 호출 및 XML 파싱을 내부적으로 처리
- `TranscriptEntry { text, start, duration }` 구조체 반환
- `quick-xml`은 커스텀 XML 파싱이 추가로 필요한 경우에만 직접 사용

**레거시 참고** (직접 구현 불필요):
```
엔드포인트: https://www.youtube.com/api/timedtext?v={video_id}&lang=en
응답: XML — <text start="7.58" dur="4">Hello everyone, welcome</text>
```

### Claude CLI 명령어
```bash
echo "{prompt_text}" | claude --print - --output-format stream-json --verbose
```
- `--print -`: stdout으로 전체 응답 출력
- `--output-format stream-json`: 스트리밍 JSON (JSONL 형식)
- `--verbose`: 디버깅용 (선택사항, Phase 1에서는 로그 용도)

### Rust 라이브러리
- **자막 fetch**: `yt-transcript-rs` — YouTube 자막 전용 crate (timedtext API + XML 파싱 내장)
- **subprocess**: `tauri-plugin-shell` — spawn/stream/kill, Tauri 이벤트 통합
- **XML 파싱**: `quick-xml` — `yt-transcript-rs` 미처리 커스텀 파싱 시에만 사용
- **JSON 파싱/직렬화**: `serde_json`
- **에러 처리**: `thiserror` (커맨드 경계 구조화 에러) + `anyhow` (내부 로직)
- **정규식**: `regex` (video ID 추출)
- **타입 안전성**: `tauri-specta` v2 — Rust 커맨드 → TypeScript 타입 자동 생성

### 에러 처리 패턴 (thiserror + anyhow)

```rust
use thiserror::Error;
use serde::Serialize;

/// 프론트엔드에 전달되는 구조화된 에러 (tauri 커맨드 경계)
#[derive(Debug, Error, Serialize)]
#[serde(tag = "kind", content = "message")]
pub enum AppError {
    #[error("자막을 가져올 수 없습니다: {0}")]
    CaptionFetch(String),
    #[error("번역 중 오류가 발생했습니다: {0}")]
    Translation(String),
    #[error("Claude CLI를 찾을 수 없습니다: {0}")]
    EnvironmentCheck(String),
    #[error("프로세스 오류: {0}")]
    Process(String),
}

// 내부 로직: anyhow 사용 → 커맨드 경계에서 AppError로 변환
#[tauri::command]
#[specta::specta]
async fn fetch_subtitles(video_id: String) -> Result<Vec<Subtitle>, AppError> {
    internal_fetch(&video_id)
        .await
        .map_err(|e| AppError::CaptionFetch(e.to_string()))
}
```

### ServerAdapter 인터페이스 (유사 Paperclip 패턴, tauri-plugin-shell 기반)

```rust
use tauri_plugin_shell::ShellExt;

trait ServerAdapter {
    async fn test_environment(&self) -> Result<(), AppError>;
    async fn execute(&self, app: &tauri::AppHandle, prompt: String) -> Result<(), AppError>;
    async fn shutdown(&self, child: Child) -> Result<(), AppError>;
}

struct ClaudeServerAdapter;

impl ServerAdapter for ClaudeServerAdapter {
    async fn execute(&self, app: &tauri::AppHandle, prompt: String) -> Result<(), AppError> {
        let shell = app.shell();
        let (mut rx, child) = shell
            .command("claude")
            .args(["--print", "-", "--output-format", "stream-json"])
            .env_remove("CLAUDECODE")
            .spawn()
            .map_err(|e| AppError::Process(e.to_string()))?;

        child.write(prompt.as_bytes())
            .map_err(|e| AppError::Process(e.to_string()))?;

        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => {
                    let text = String::from_utf8_lossy(&line);
                    let _ = app.emit("translation-chunk", &*text);
                }
                CommandEvent::Terminated(_) => break,
                _ => {}
            }
        }
        Ok(())
    }
}
```

### 프롬프트 예시
```
Video Description:
Title: Introduction to Rust Programming
Description: A beginner-friendly introduction to Rust...

Context from previous chunk:
(if exists, last 5 lines)
...

Current chunk subtitles (30s-1m duration):
00:07.58 - 00:11.58: Hello everyone, welcome to today's lecture.
00:11.58 - 00:15.22: We're going to explore the basics of Rust.

Instructions:
Translate the above subtitles to Korean.
Return JSON array: [{original, translated, start, end}, ...]
```

### JSONL 스트리밍 파싱 예시
```
응답 (stdout):
{"type":"content_block_start","content_block":{"type":"text"}}
{"type":"content_block_delta","delta":{"type":"text_delta","text":"[{\"original\":"}}
{"type":"content_block_delta","delta":{"type":"text_delta","text":"\"Hello\",..."}}
...
{"type":"message_stop"}

파싱 전략:
1. 각 라인을 JSON 파싱
2. type이 "content_block_delta" 인 라인의 text 필드 추출
3. text 조각들을 연결하여 완전한 JSON 배열 구성
4. 최종 JSON 배열을 파싱 및 검증
```

## 완료 기준

모든 항목이 실제 영상(최소 3개)으로 end-to-end 테스트를 통과해야 Phase 1 완료로 간주:

### 기능 완료
- [ ] YouTube timedtext API에서 실제 자막 fetch 성공 (영어, 한국어 자막 각 1개 영상)
- [ ] XML → 구조화된 배열 파싱 성공
- [ ] 청크 분할 로직이 30초~1분 범위 내에서 동작 확인
- [ ] Claude ServerAdapter 구현 완료:
  - [ ] testEnvironment() 호출 시 Claude CLI 감지 또는 에러 메시지 반환
  - [ ] execute() 호출 시 subprocess 생성 및 stdin/stdout 연결
  - [ ] 3번 이상 연속 execute() 호출 후 모두 정상 완료
  - [ ] graceful shutdown 테스트: 30초 타임아웃 설정, SIGTERM 전송 후 2초 내 프로세스 종료 확인
- [ ] CLAUDECODE 환경변수가 자식 프로세스에서 제거됨을 검증 (env 출력 또는 로그)
- [ ] 번역 프롬프트 구성 검증:
  - [ ] 첫 청크: 영상 설명 포함
  - [ ] 2번 청크: 이전 청크 마지막 5줄 포함
  - [ ] 각 청크: 시간 정보(start, end) 명시
- [ ] stdout JSONL 파싱 성공 및 모든 라인 수집 완료
- [ ] 번역 결과 JSON 배열 검증:
  - [ ] 필드 검증: original, translated, start, end 모두 존재
  - [ ] 길이 검증: 배열 요소 수 >= 1
  - [ ] 타입 검증: start, end는 f64, 나머지는 String
  - [ ] 번역 언어 검증: translated 필드가 한국어 텍스트 포함 (CJK 문자 휴리스틱)

### 성능 기준
- [ ] 첫 번역 청크: 5초 이내 완료 (네트워크 포함)
- [ ] 각 청크 처리: 평균 < 3초
- [ ] subprocess 생성/종료 오버헤드: < 500ms

### 에러 처리
- [ ] Claude CLI 미설치 시: testEnvironment() 에러, 사용자 가이드 메시지
- [ ] 자막 없는 영상: XML 파싱 후 빈 배열 감지, "No subtitles available" 메시지
- [ ] Claude 구독 한도 초과: stderr 캡처, 사용자에게 "Claude subscription limit exceeded" 메시지
- [ ] subprocess 타임아웃: 30초 후 SIGKILL, "Translation timeout" 메시지

## 다음 Phase 의존성

Phase 2 (캐시 및 버퍼링)는 Phase 1의 다음 결과물에 의존:
- Claude ServerAdapter 인터페이스 확정
- 번역 결과 JSON 배열 포맷 확정 (변경 불가)
- 청크 분할 로직 및 경계 명세 (캐시 키 생성에 필요)

Phase 3 (UI 표시)은 다음에 의존:
- 번역 결과 반환 구조 (Phase 1)
- 캐시 쿼리 인터페이스 (Phase 2)

## 실패 시 대안

| 시나리오 | 대안 |
|---------|------|
| YouTube timedtext API 차단/변경 | youtube-dl 또는 yt-dlp 라이브러리 사용 (자막 fetch) |
| Claude CLI subprocess 불안정 | Tauri-http + Claude API 직접 호출 (비용 증가, 제약 확대) |
| JSONL 파싱 복잡도 높음 | Claude에게 완전한 JSON 배열 반환 요청 (스트리밍 포기) |
| 청크 경계 맥락 단절 심각 | 전체 일괄 번역으로 전환 (초기 대기 증가, 긴 영상 제약) |
| subprocess 좀비 프로세스 반복 | 별도 watchdog 스레드 구현 (프로세스 재정기 모니터링) |
