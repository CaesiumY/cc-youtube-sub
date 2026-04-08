use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::error::AppError;

/// Claude CLI 환경 검증 및 실행을 담당하는 어댑터
///
/// Paperclip의 ServerAdapter 패턴을 참조:
/// - test_environment(): CLI 존재 여부 확인
/// - execute(): 프롬프트를 stdin으로 전달하고 stream-json 출력 수집
pub struct ClaudeAdapter;

impl ClaudeAdapter {
    /// Claude CLI가 설치되어 있고 실행 가능한지 확인
    pub async fn test_environment() -> Result<(), AppError> {
        let output = Command::new("claude")
            .arg("--version")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| {
                AppError::EnvironmentCheck(format!(
                    "claude 명령어를 실행할 수 없습니다. Claude Code CLI가 설치되어 있는지 확인하세요: {}",
                    e
                ))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::EnvironmentCheck(format!(
                "claude --version 실행 실패: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.to_lowercase().contains("claude") {
            return Err(AppError::EnvironmentCheck(format!(
                "예상치 못한 claude --version 출력: {}",
                stdout.trim()
            )));
        }

        Ok(())
    }

    /// Claude CLI를 subprocess로 실행하여 번역 수행
    ///
    /// - `--print -`: stdin에서 프롬프트를 읽어 단일 응답 출력
    /// - `--output-format stream-json`: JSONL 스트림 형식 출력
    /// - `CLAUDECODE` 환경변수 제거: Paperclip 패턴 (재귀 방지)
    pub async fn execute(prompt: &str, timeout_secs: u64) -> Result<String, AppError> {
        let mut child = Command::new("claude")
            .args(["--print", "-", "--output-format", "stream-json"])
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
        .map_err(|_| {
            AppError::Process(format!("Claude 응답 타임아웃 ({}초)", timeout_secs))
        })?
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

        Ok(stdout)
    }

    /// 실행 중인 Claude 프로세스를 종료 (graceful shutdown)
    #[allow(dead_code)]
    pub async fn shutdown(child: &mut tokio::process::Child) -> Result<(), AppError> {
        // SIGTERM으로 먼저 시도
        child.kill().await.map_err(|e| {
            AppError::Process(format!("Claude 프로세스 종료 실패: {}", e))
        })?;
        Ok(())
    }
}
