use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::backend::ExecuteResult;
use crate::error::AppError;

// ExecuteParams의 단일 정의는 `crate::backend`에 있다. `claude::adapter::ExecuteParams` 경로로
// 임포트하던 호출자가 그대로 동작하도록 재공개 (모듈 내부에서도 이름 ExecuteParams로 접근 가능).
pub use crate::backend::ExecuteParams;

/// 에러 메시지 prefix — 프론트엔드에서 에러 종류를 구분하는 데 사용
const ERR_NOT_INSTALLED: &str = "NOT_INSTALLED";
const ERR_EXECUTION_FAILED: &str = "EXECUTION_FAILED";

/// Windows에서 cmd /c가 명령을 찾지 못할 때 반환하는 exit code
#[cfg(target_os = "windows")]
const CMD_NOT_FOUND_EXIT_CODE: i32 = 9009;

/// 플랫폼별 claude Command를 생성하는 헬퍼
///
/// Windows: `cmd /c claude <args>` — shell이 PATHEXT를 해석하여 .cmd 래퍼를 자동 탐색
/// Others: `claude <args>` — 직접 실행
///
/// `kill_on_drop(true)`: 응답 타임아웃 시 `wait_with_output` future가 drop될 때
/// 자식 프로세스가 orphan으로 남지 않도록 정리한다.
fn build_claude_command(args: &[&str]) -> Command {
    #[cfg(target_os = "windows")]
    {
        #[allow(unused_imports)]
        use std::os::windows::process::CommandExt;
        // GUI 앱에서 cmd.exe 자식이 새 콘솔 창을 할당하는 것을 방지
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let mut cmd = Command::new("cmd");
        let mut full_args = vec!["/c", "claude"];
        full_args.extend_from_slice(args);
        cmd.args(&full_args);
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd.kill_on_drop(true);
        cmd
    }
    #[cfg(not(target_os = "windows"))]
    {
        let mut cmd = Command::new("claude");
        cmd.args(args);
        cmd.kill_on_drop(true);
        cmd
    }
}

/// 허용된 모델 alias 목록 — IPC 경계에서의 입력 검증용
const ALLOWED_MODELS: &[&str] = &["haiku", "sonnet"];

/// Claude CLI 환경 검증 및 실행을 담당하는 어댑터
///
/// Paperclip의 ServerAdapter 패턴을 참조:
/// - test_environment(): CLI 존재 여부 확인
/// - execute(): 프롬프트를 stdin으로 전달하고 stream-json 출력 수집
pub struct ClaudeAdapter;

impl ClaudeAdapter {
    /// Claude CLI가 설치되어 있고 실행 가능한지 확인
    pub async fn test_environment() -> Result<(), AppError> {
        let output = build_claude_command(&["--version"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| {
                AppError::EnvironmentCheck(format!(
                    "{}: claude 명령어를 실행할 수 없습니다. Claude Code CLI가 설치되어 있는지 확인하세요: {}",
                    ERR_NOT_INSTALLED, e
                ))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);

            // Windows: cmd /c가 명령을 찾지 못하면 exit code 9009 반환 (로케일 무관)
            #[cfg(target_os = "windows")]
            {
                if output.status.code() == Some(CMD_NOT_FOUND_EXIT_CODE) {
                    return Err(AppError::EnvironmentCheck(format!(
                        "{}: Claude Code CLI가 PATH에 없습니다. npm install -g @anthropic-ai/claude-code로 설치하세요.",
                        ERR_NOT_INSTALLED
                    )));
                }
            }

            return Err(AppError::EnvironmentCheck(format!(
                "{}: claude --version 실행 실패: {}",
                ERR_EXECUTION_FAILED,
                stderr.trim()
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.to_lowercase().contains("claude") {
            return Err(AppError::EnvironmentCheck(format!(
                "{}: 예상치 못한 claude --version 출력: {}",
                ERR_EXECUTION_FAILED,
                stdout.trim()
            )));
        }

        Ok(())
    }

    /// Claude CLI를 subprocess로 실행하여 번역 수행
    ///
    /// - `--print -`: stdin에서 프롬프트를 읽어 단일 응답 출력
    /// - `--output-format stream-json`: JSONL 스트림 형식 출력
    /// - `--model`: 사용할 모델 alias (haiku, sonnet)
    /// - `session_id` + `is_first_in_session`: 같은 영상의 청크들이 동일 Claude 세션을
    ///   재사용하여 맥락 연속성 확보.
    ///   - `is_first_in_session = true` → `--session-id <uuid>`로 세션 생성
    ///   - `is_first_in_session = false` → `--resume <uuid> --fork-session`으로
    ///     부모 세션의 맥락은 계승하면서 fork된 세션에 기록 (동시 호출 충돌 회피)
    /// - `CLAUDECODE` 환경변수 제거: Paperclip 패턴 (재귀 방지)
    pub async fn execute(params: ExecuteParams<'_>) -> Result<ExecuteResult, AppError> {
        let ExecuteParams {
            prompt,
            timeout_secs,
            model,
            session_id,
            is_first_in_session,
        } = params;

        if let Some(m) = model {
            if !ALLOWED_MODELS.contains(&m) {
                return Err(AppError::Process(format!("지원하지 않는 모델: {}", m)));
            }
        }

        // 최신 Claude CLI(v2.1+)는 `--print` + `--output-format stream-json` 조합에
        // `--verbose` 플래그를 필수로 요구한다. 없으면 "requires --verbose" 에러 반환.
        let mut args = vec![
            "--print",
            "-",
            "--output-format",
            "stream-json",
            "--verbose",
        ];
        if let Some(uuid) = session_id {
            if is_first_in_session {
                args.push("--session-id");
                args.push(uuid);
            } else {
                args.push("--resume");
                args.push(uuid);
                // fork-session: 부모 세션 맥락은 계승, 새 분기에 기록 → 동시 실행 안전
                args.push("--fork-session");
            }
        }
        if let Some(m) = model {
            args.push("--model");
            args.push(m);
        }

        let mut child = build_claude_command(&args)
            .env_remove("CLAUDECODE")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| AppError::Process(format!("Claude 프로세스 시작 실패: {}", e)))?;

        // stdin에 프롬프트 전달 후 닫기
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(prompt.as_bytes())
                .await
                .map_err(|e| AppError::Process(format!("stdin 쓰기 실패: {}", e)))?;
            // drop으로 stdin 닫기 — Claude가 EOF를 감지하고 처리 시작
        }

        // 타임아웃 적용하여 출력 대기
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            child.wait_with_output(),
        )
        .await
        .map_err(|_| AppError::Process(format!("Claude 응답 타임아웃 ({}초)", timeout_secs)))?
        .map_err(|e| AppError::Process(format!("Claude 프로세스 대기 실패: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::Process(format!(
                "Claude 프로세스 비정상 종료 (코드: {:?}): {}",
                output.status.code(),
                stderr.trim()
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();

        if stdout.trim().is_empty() {
            return Err(AppError::Process(
                "Claude 프로세스가 빈 응답을 반환했습니다".into(),
            ));
        }

        // Claude는 클라이언트가 UUID를 미리 생성해 세션 ID로 넘기는 모델이므로
        // adapter는 새로 발급할 세션 ID가 없다 — 항상 None.
        Ok(ExecuteResult {
            raw_output: stdout,
            returned_session_id: None,
        })
    }

    /// 실행 중인 Claude 프로세스를 종료 (graceful shutdown)
    #[allow(dead_code)]
    pub async fn shutdown(child: &mut tokio::process::Child) -> Result<(), AppError> {
        // SIGTERM으로 먼저 시도
        child
            .kill()
            .await
            .map_err(|e| AppError::Process(format!("Claude 프로세스 종료 실패: {}", e)))?;
        Ok(())
    }
}

/// Claude 에러 메시지에서 종류를 분류 (BufferManager의 폴백 로직 + 프론트엔드 UI 분기에 사용).
///
/// 텍스트 휴리스틱이므로 Claude CLI 출력 변경에 취약함을 인지. 가능한 한 구체적 패턴을
/// 매칭해 오분류를 줄인다. 특히 `"exceeded"` 단독 매칭은 `context length exceeded`,
/// `token limit exceeded` 등 rate limit이 아닌 케이스까지 잡으므로 사용 금지.
pub fn classify_claude_error(error: &AppError) -> String {
    let msg = error.to_string().to_lowercase();
    if msg.contains("session id") && msg.contains("already in use") {
        // Claude CLI: `Error: Session ID {uuid} is already in use.`
        return "session_conflict".into();
    }
    // rate limit: Claude/Anthropic 주요 표현만 구체 매칭.
    if msg.contains("rate limit")
        || msg.contains("quota exceeded")
        || msg.contains("usage limit")
        || msg.contains("5-hour")
    {
        return "rate_limit".into();
    }
    if msg.contains("timeout") || msg.contains("타임아웃") {
        return "timeout".into();
    }
    if msg.contains("claude") && (msg.contains("찾을 수 없") || msg.contains("not found")) {
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
    fn classify_rate_limit() {
        let err = AppError::Process("Claude rate limit exceeded".into());
        assert_eq!(classify_claude_error(&err), "rate_limit");
    }

    #[test]
    fn classify_rate_limit_variants() {
        let err1 = AppError::Process("quota exceeded for this model".into());
        assert_eq!(classify_claude_error(&err1), "rate_limit");
        let err2 = AppError::Process("monthly usage limit reached".into());
        assert_eq!(classify_claude_error(&err2), "rate_limit");
        let err3 = AppError::Process("5-hour limit hit".into());
        assert_eq!(classify_claude_error(&err3), "rate_limit");
    }

    #[test]
    fn classify_context_length_not_rate_limit() {
        // 회귀 방지: "exceeded" 단독 매칭은 오분류 위험
        let err = AppError::Process("context length exceeded".into());
        assert_ne!(classify_claude_error(&err), "rate_limit");
        let err2 = AppError::Process("maximum token limit exceeded".into());
        assert_ne!(classify_claude_error(&err2), "rate_limit");
    }

    #[test]
    fn classify_timeout() {
        let err = AppError::Process("Claude 응답 타임아웃 (120초)".into());
        assert_eq!(classify_claude_error(&err), "timeout");
    }

    #[test]
    fn classify_generic_translation() {
        let err = AppError::Translation("파싱 실패".into());
        assert_eq!(classify_claude_error(&err), "translation");
    }

    #[test]
    fn classify_session_conflict() {
        let err = AppError::Process(
            "Claude 프로세스 비정상 종료 (코드: Some(1)): \
             Error: Session ID c78bf130-fd78-45d2-bdb2-ee002fcece2e is already in use."
                .into(),
        );
        assert_eq!(classify_claude_error(&err), "session_conflict");
    }

    #[test]
    fn classify_session_conflict_case_insensitive() {
        let err = AppError::Process("SESSION ID abc IS ALREADY IN USE".into());
        assert_eq!(classify_claude_error(&err), "session_conflict");
    }

    #[test]
    fn classify_session_conflict_not_overmatched() {
        let err = AppError::Process("some session-related issue, not a conflict".into());
        assert_ne!(classify_claude_error(&err), "session_conflict");
        let err2 = AppError::Process("port is already in use".into());
        assert_ne!(classify_claude_error(&err2), "session_conflict");
    }
}
