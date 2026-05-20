//! OpenAI Codex CLI(`codex exec`) subprocess 어댑터.
//!
//! Claude 어댑터와 같은 정적 메서드 패턴(`test_environment`, `execute`)을 따르며,
//! 두 어댑터를 `crate::backend::TranslationBackend` enum이 dispatch한다.
//!
//! 두 어댑터의 비대칭:
//! - **세션 모델**: Claude는 클라이언트가 UUID를 미리 생성해 `--session-id`/`--resume`에
//!   넘기지만, Codex는 서버가 첫 호출의 `session_configured` 이벤트로 thread_id를 반환한다.
//!   본 어댑터는 첫 호출(`is_first_in_session=true`) 시 `extract_session_id_from_codex_events`로
//!   thread_id를 추출하여 `ExecuteResult.returned_session_id`에 채워 반환한다.
//!   후속 호출(`is_first_in_session=false`)에는 BufferManager가 넘긴 `session_id`를
//!   `codex exec resume <id>`로 사용.
//! - **출력 형식**: Claude의 `stream-json`은 `type: result|assistant|...` JSONL,
//!   Codex의 `--json`은 `msg.type: task_complete|agent_message|...` NDJSON.
//!   파서는 각각 `translate::jsonl_parser`와 `translate::codex_event_parser`에 분리.
//!
//! ⚠️ 정확한 CLI 플래그/이벤트 구조는 Codex CLI 버전에 따라 다를 수 있다. 본 어댑터는
//! 공개 SDK 문서/Github 관찰에 근거한 추정으로 구현되었으며, 실제 환경에서 검증·조정 필요.

use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::backend::{ExecuteParams, ExecuteResult};
use crate::error::AppError;
use crate::translate::codex_event_parser::extract_session_id_from_codex_events;

const ERR_NOT_INSTALLED: &str = "NOT_INSTALLED";
const ERR_EXECUTION_FAILED: &str = "EXECUTION_FAILED";

#[cfg(target_os = "windows")]
const CMD_NOT_FOUND_EXIT_CODE: i32 = 9009;

/// 플랫폼별 codex Command 생성.
///
/// Windows: `cmd /c codex <args>` + CREATE_NO_WINDOW (Claude 어댑터와 동일 패턴).
fn build_codex_command(args: &[&str]) -> Command {
    #[cfg(target_os = "windows")]
    {
        #[allow(unused_imports)]
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let mut cmd = Command::new("cmd");
        let mut full_args = vec!["/c", "codex"];
        full_args.extend_from_slice(args);
        cmd.args(&full_args);
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd
    }
    #[cfg(not(target_os = "windows"))]
    {
        let mut cmd = Command::new("codex");
        cmd.args(args);
        cmd
    }
}

/// `codex exec`에 항상 적용하는 reasoning effort.
///
/// 자막 번역은 복잡한 추론이 거의 필요 없으므로 `low`로 고정해 응답 속도를 우선한다.
/// 사용자의 `~/.codex/config.toml` 설정(예: `xhigh`)을 `-c`로 오버라이드한다.
const CODEX_REASONING_EFFORT_OVERRIDE: &str = "model_reasoning_effort=\"low\"";

/// `ExecuteParams`로부터 `codex exec` 인자 배열을 구성.
///
/// 분리한 이유: subprocess 없이 인자 생성 로직을 단위 테스트할 수 있도록.
///
/// 형식 (codex-cli 0.132.0 실측 기준):
/// - 첫 호출:   `exec --json --skip-git-repo-check --sandbox read-only -c model_reasoning_effort="low" -`
/// - 이어가기:  `exec resume <thread_id> --json --skip-git-repo-check --sandbox read-only -c model_reasoning_effort="low" -`
///
/// - `--json`: 이벤트를 JSONL로 stdout에 출력
/// - `--skip-git-repo-check`: codex는 기본적으로 git 저장소 안에서 실행됨을 가정한다.
///   이 데스크톱 앱은 임의 위치에서 실행되므로 이 플래그가 없으면 거부될 수 있다.
/// - `--sandbox read-only`: 자막 번역은 파일 쓰기·셸 실행이 전혀 불필요하다. 외부 데이터
///   (YouTube 자막/영상 설명)가 프롬프트에 보간되므로, 악의적 자막이 프롬프트 인젝션으로
///   도구 호출을 유도하더라도 read-only sandbox가 셸/파일 쓰기를 차단한다 (defense-in-depth).
/// - `-c model_reasoning_effort="low"`: 빠른 응답 우선 (위 상수 참조).
/// - 마지막 `-`: stdin에서 prompt를 읽으라는 codex의 관례 (PROMPT 인자 자리에 `-`).
///
/// 모델 이름은 지정하지 않는다 — codex CLI는 모델 카탈로그 명령이 없어 alias를 신뢰할 수
/// 없고, codex 기본 모델(사용자 config의 `model` 값)을 그대로 사용한다.
fn build_codex_args(session_id: Option<&str>, is_first_in_session: bool) -> Vec<&str> {
    let mut args = vec!["exec"];
    if let Some(uuid) = session_id {
        if !is_first_in_session {
            args.push("resume");
            args.push(uuid);
        }
    }
    args.push("--json");
    args.push("--skip-git-repo-check");
    args.push("--sandbox");
    args.push("read-only");
    args.push("-c");
    args.push(CODEX_REASONING_EFFORT_OVERRIDE);
    args.push("-");
    args
}

pub struct CodexAdapter;

impl CodexAdapter {
    /// Codex CLI가 설치되어 있고 실행 가능한지 확인.
    pub async fn test_environment() -> Result<(), AppError> {
        let output = build_codex_command(&["--version"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| {
                AppError::EnvironmentCheck(format!(
                    "{}: codex 명령어를 실행할 수 없습니다. Codex CLI가 설치되어 있는지 확인하세요: {}",
                    ERR_NOT_INSTALLED, e
                ))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);

            #[cfg(target_os = "windows")]
            {
                if output.status.code() == Some(CMD_NOT_FOUND_EXIT_CODE) {
                    return Err(AppError::EnvironmentCheck(format!(
                        "{}: Codex CLI가 PATH에 없습니다. npm install -g @openai/codex로 설치하세요.",
                        ERR_NOT_INSTALLED
                    )));
                }
            }

            return Err(AppError::EnvironmentCheck(format!(
                "{}: codex --version 실행 실패: {}",
                ERR_EXECUTION_FAILED,
                stderr.trim()
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.to_lowercase().contains("codex") {
            return Err(AppError::EnvironmentCheck(format!(
                "{}: 예상치 못한 codex --version 출력: {}",
                ERR_EXECUTION_FAILED,
                stdout.trim()
            )));
        }

        Ok(())
    }

    /// Codex CLI subprocess 실행하여 번역 수행.
    pub async fn execute(params: ExecuteParams<'_>) -> Result<ExecuteResult, AppError> {
        // `model`은 사용하지 않는다 — codex는 모델 카탈로그가 없어 alias를 신뢰할 수 없고,
        // 본 앱은 항상 codex 기본 모델 + 빠른 reasoning effort로 동작한다 (build_codex_args 참조).
        let ExecuteParams {
            prompt,
            timeout_secs,
            model: _,
            session_id,
            is_first_in_session,
        } = params;

        let args = build_codex_args(session_id, is_first_in_session);

        let mut child = build_codex_command(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| AppError::Process(format!("Codex 프로세스 시작 실패: {}", e)))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(prompt.as_bytes())
                .await
                .map_err(|e| AppError::Process(format!("stdin 쓰기 실패: {}", e)))?;
            // drop으로 stdin 닫기 → EOF
        }

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            child.wait_with_output(),
        )
        .await
        .map_err(|_| AppError::Process(format!("Codex 응답 타임아웃 ({}초)", timeout_secs)))?
        .map_err(|e| AppError::Process(format!("Codex 프로세스 대기 실패: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::Process(format!(
                "Codex 프로세스 비정상 종료 (코드: {:?}): {}",
                output.status.code(),
                stderr.trim()
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();

        if stdout.trim().is_empty() {
            return Err(AppError::Process(
                "Codex 프로세스가 빈 응답을 반환했습니다".into(),
            ));
        }

        // 첫 호출이면 응답에서 thread_id 추출 — BufferManager가 후속 청크의 resume 인자로 사용
        let returned_session_id = if is_first_in_session {
            extract_session_id_from_codex_events(&stdout)
        } else {
            None
        };

        Ok(ExecuteResult {
            raw_output: stdout,
            returned_session_id,
        })
    }
}

/// Codex 에러 메시지에서 종류를 분류.
///
/// Claude의 `buffer_manager::classify_error`와 같은 역할이지만 Codex 특유의 에러 문구를
/// 매칭한다. 실제 환경에서 관찰되는 문구로 확장 필요.
pub fn classify_codex_error(error: &AppError) -> String {
    let msg = error.to_string().to_lowercase();
    // 세션/스레드 충돌(미발견 thread, 동시 사용 등) — Claude의 session_conflict와 같은 폴백 트리거
    if (msg.contains("thread") || msg.contains("session"))
        && (msg.contains("not found") || msg.contains("expired") || msg.contains("already in use"))
    {
        return "session_conflict".into();
    }
    // rate limit / quota — Codex/OpenAI 표준 문구
    if msg.contains("rate limit")
        || msg.contains("quota")
        || msg.contains("usage limit")
        || msg.contains("too many requests")
        || msg.contains("429")
    {
        return "rate_limit".into();
    }
    if msg.contains("timeout") || msg.contains("타임아웃") {
        return "timeout".into();
    }
    if msg.contains("codex") && (msg.contains("찾을 수 없") || msg.contains("not found")) {
        return "cli_not_found".into();
    }
    match error {
        AppError::CaptionFetch(_) => "caption_fetch".into(),
        AppError::Translation(_) => "translation".into(),
        AppError::Database(_) => "database".into(),
        AppError::EnvironmentCheck(_) => "environment".into(),
        AppError::Process(_) => "process".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_call_args_no_session() {
        let args = build_codex_args(None, true);
        assert_eq!(
            args,
            vec![
                "exec",
                "--json",
                "--skip-git-repo-check",
                "--sandbox",
                "read-only",
                "-c",
                CODEX_REASONING_EFFORT_OVERRIDE,
                "-",
            ]
        );
    }

    #[test]
    fn first_call_args_with_session_id_present_but_first_flag_true() {
        // 첫 호출인데 session_id가 있는 경우(보통 발생하지 않지만 방어적):
        // resume를 추가하지 않고 첫 호출 형식으로 호출. 서버가 새 thread를 부여.
        let args = build_codex_args(Some("thr_abc"), true);
        assert_eq!(
            args,
            vec![
                "exec",
                "--json",
                "--skip-git-repo-check",
                "--sandbox",
                "read-only",
                "-c",
                CODEX_REASONING_EFFORT_OVERRIDE,
                "-",
            ]
        );
    }

    #[test]
    fn resume_args_subsequent_call() {
        let args = build_codex_args(Some("thr_abc"), false);
        assert_eq!(
            args,
            vec![
                "exec",
                "resume",
                "thr_abc",
                "--json",
                "--skip-git-repo-check",
                "--sandbox",
                "read-only",
                "-c",
                CODEX_REASONING_EFFORT_OVERRIDE,
                "-",
            ]
        );
    }

    #[test]
    fn reasoning_effort_override_is_low() {
        // 빠른 응답 우선 정책 — 회귀 방지
        assert!(CODEX_REASONING_EFFORT_OVERRIDE.contains("low"));
    }

    #[test]
    fn sandbox_is_always_read_only() {
        // 보안: 자막 번역은 파일 쓰기·셸 실행이 불필요. 프롬프트 인젝션이 도구 호출을
        // 유도해도 read-only sandbox가 차단한다 (defense-in-depth) — 회귀 방지.
        for first in [true, false] {
            let args = build_codex_args(Some("thr"), first);
            let idx = args
                .iter()
                .position(|&a| a == "--sandbox")
                .expect("--sandbox 플래그 누락");
            assert_eq!(args[idx + 1], "read-only");
        }
    }

    #[test]
    fn classify_codex_error_session_conflict() {
        let err = AppError::Process(
            "Codex 프로세스 비정상 종료 (코드: Some(1)): thread thr_abc not found".into(),
        );
        assert_eq!(classify_codex_error(&err), "session_conflict");
    }

    #[test]
    fn classify_codex_error_rate_limit() {
        let err = AppError::Process("OpenAI rate limit exceeded".into());
        assert_eq!(classify_codex_error(&err), "rate_limit");
        let err2 = AppError::Process("HTTP 429 too many requests".into());
        assert_eq!(classify_codex_error(&err2), "rate_limit");
        let err3 = AppError::Process("monthly quota reached".into());
        assert_eq!(classify_codex_error(&err3), "rate_limit");
    }

    #[test]
    fn classify_codex_error_timeout() {
        let err = AppError::Process("Codex 응답 타임아웃 (120초)".into());
        assert_eq!(classify_codex_error(&err), "timeout");
    }

    #[test]
    fn classify_codex_error_generic_translation() {
        let err = AppError::Translation("출력 파싱 실패".into());
        assert_eq!(classify_codex_error(&err), "translation");
    }

    #[test]
    fn classify_codex_error_context_length_not_rate_limit() {
        // 회귀 방지: "exceeded" 단독 매칭은 rate_limit 오분류 위험
        let err = AppError::Process("context length exceeded".into());
        assert_ne!(classify_codex_error(&err), "rate_limit");
    }

    // ── 통합 테스트 (실제 codex CLI 필요) ──────────────────
    //
    // 실행: cargo test --lib codex::adapter -- --ignored --nocapture
    // 요건: codex CLI 설치 + `codex login` 완료.

    #[tokio::test]
    #[ignore]
    async fn integration_codex_cli_available() {
        CodexAdapter::test_environment()
            .await
            .expect("codex CLI가 설치되어 있지 않거나 실행 불가능");
    }

    #[tokio::test]
    #[ignore]
    async fn integration_codex_translate_and_capture_thread_id() {
        use crate::translate::codex_event_parser::{
            extract_session_id_from_codex_events, extract_text_from_codex,
        };

        let prompt = "Translate the English text to Korean. Reply ONLY with a JSON array, \
             no markdown fences, no commentary. \
             Input: [{\"original\":\"Hello everyone\",\"start\":0.0,\"end\":2.0}]. \
             Output schema: [{\"original\":string,\"translated\":string,\"start\":number,\"end\":number}]";

        let result = CodexAdapter::execute(ExecuteParams {
            prompt,
            timeout_secs: 240,
            model: None,
            session_id: None,
            is_first_in_session: true,
        })
        .await
        .expect("codex execute 실패");

        // 첫 호출은 thread.started 이벤트로 thread_id를 반환해야 한다.
        assert!(
            result.returned_session_id.is_some(),
            "첫 호출은 thread_id를 캡처해야 함 — raw:\n{}",
            result.raw_output
        );
        // raw_output에서 직접 추출한 것과 일치하는지 교차 검증
        assert_eq!(
            result.returned_session_id,
            extract_session_id_from_codex_events(&result.raw_output)
        );

        // 텍스트 추출 — JSON 배열 형태여야 함
        let text = extract_text_from_codex(&result.raw_output).expect("이벤트 파싱 실패");
        eprintln!("=== Codex 번역 결과 ===\n{}", text);
        assert!(
            text.trim_start().starts_with('['),
            "번역 결과가 JSON 배열이 아님: {}",
            text
        );
        let parsed: serde_json::Value =
            serde_json::from_str(text.trim()).expect("결과가 유효한 JSON이 아님");
        assert!(parsed.is_array(), "결과가 배열이 아님");
    }
}
