//! 번역 백엔드 추상화 — Claude CLI와 Codex CLI 어댑터를 enum dispatch로 통합.
//!
//! `ExecuteParams`와 `ExecuteResult`는 두 어댑터가 공유하는 호출 규약이다.
//! 두 어댑터의 비대칭은 `ExecuteResult.returned_session_id`로 흡수한다:
//! - Claude는 클라이언트가 UUID를 미리 생성해 `session_id`로 넘기는 모델이라
//!   adapter는 항상 `None`을 반환한다.
//! - Codex는 서버가 첫 호출 응답의 `session_configured` 이벤트로 `thread_id`를
//!   알려주는 모델이라, 첫 호출에서만 `Some(thread_id)`를 반환한다. BufferManager가
//!   이를 캡처해 후속 청크의 `--resume <thread_id>`에 사용한다.

/// `*Adapter::execute` 호출 파라미터.
pub struct ExecuteParams<'a> {
    pub prompt: &'a str,
    pub timeout_secs: u64,
    pub model: Option<&'a str>,
    /// 백엔드 세션 토큰.
    /// - Claude: 미리 생성된 UUID. `is_first_in_session=true`면 `--session-id <uuid>`,
    ///   false면 `--resume <uuid> --fork-session`.
    /// - Codex: 첫 호출 시 `None`(서버가 thread_id 생성). `is_first_in_session=false`이고
    ///   `Some(thread_id)`면 `codex exec resume <thread_id>`.
    pub session_id: Option<&'a str>,
    pub is_first_in_session: bool,
}

/// 어댑터 실행 결과.
///
/// `raw_output`은 백엔드별 원시 스트림(Claude의 stream-json JSONL, Codex의 `--json`
/// NDJSON 이벤트). 호출자는 `TranslationBackend::extract_text`로 파싱해야 한다.
pub struct ExecuteResult {
    pub raw_output: String,
    /// Codex의 첫 호출에서만 `Some(thread_id)`. Claude나 Codex의 이어가기 호출은 `None`.
    pub returned_session_id: Option<String>,
}

/// 번역 백엔드 종류 — Claude CLI 또는 OpenAI Codex CLI.
///
/// IPC 경계에서는 문자열(`"claude"`/`"codex"`)로 전달되며, `from_str`로 파싱한다.
/// 모든 백엔드 분기는 이 enum의 match로 단일화되어 새 백엔드 추가 시 컴파일러가
/// 누락된 arm을 잡아준다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranslationBackend {
    Claude,
    Codex,
}

impl TranslationBackend {
    /// IPC 문자열에서 enum으로 변환. 알 수 없는 값은 기본값 `Claude`로 안전 매핑.
    ///
    /// `FromStr` 트레이트가 아닌 inherent 메서드로 두는 이유: 알 수 없는 값을 에러로
    /// 반환하지 않고 안전한 기본값으로 매핑하는 IPC 경계 정책을 명시적으로 표현하기 위함.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "codex" => Self::Codex,
            _ => Self::Claude,
        }
    }

    /// 디버깅/로깅용 이름.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }

    /// 백엔드 CLI가 설치되어 있고 실행 가능한지 확인.
    pub async fn test_environment(&self) -> Result<(), crate::error::AppError> {
        match self {
            Self::Claude => crate::claude::adapter::ClaudeAdapter::test_environment().await,
            Self::Codex => crate::codex::adapter::CodexAdapter::test_environment().await,
        }
    }

    /// 번역 한 사이클 실행.
    pub async fn execute(
        &self,
        params: ExecuteParams<'_>,
    ) -> Result<ExecuteResult, crate::error::AppError> {
        match self {
            Self::Claude => crate::claude::adapter::ClaudeAdapter::execute(params).await,
            Self::Codex => crate::codex::adapter::CodexAdapter::execute(params).await,
        }
    }

    /// 백엔드별 원시 출력에서 모델 응답 텍스트 추출.
    pub fn extract_text(&self, raw: &str) -> Result<String, anyhow::Error> {
        match self {
            Self::Claude => crate::translate::jsonl_parser::extract_text_from_jsonl(raw),
            Self::Codex => crate::translate::codex_event_parser::extract_text_from_codex(raw),
        }
    }

    /// 에러 메시지를 백엔드 인식형으로 분류.
    pub fn classify_error(&self, error: &crate::error::AppError) -> String {
        match self {
            Self::Claude => crate::claude::adapter::classify_claude_error(error),
            Self::Codex => crate::codex::adapter::classify_codex_error(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str_known_values() {
        assert_eq!(
            TranslationBackend::from_str("claude"),
            TranslationBackend::Claude
        );
        assert_eq!(
            TranslationBackend::from_str("codex"),
            TranslationBackend::Codex
        );
    }

    #[test]
    fn from_str_unknown_defaults_to_claude() {
        // 알 수 없는 값은 안전한 기본값(Claude)으로 — 기존 사용자 보호
        assert_eq!(TranslationBackend::from_str(""), TranslationBackend::Claude);
        assert_eq!(
            TranslationBackend::from_str("gpt-4"),
            TranslationBackend::Claude
        );
    }

    #[test]
    fn as_str_roundtrip() {
        assert_eq!(
            TranslationBackend::from_str(TranslationBackend::Claude.as_str()),
            TranslationBackend::Claude
        );
        assert_eq!(
            TranslationBackend::from_str(TranslationBackend::Codex.as_str()),
            TranslationBackend::Codex
        );
    }

    #[test]
    fn classify_error_dispatches_to_correct_backend() {
        use crate::error::AppError;
        // Claude-specific 세션 충돌 문구
        let claude_err = AppError::Process("Error: Session ID abc is already in use.".into());
        assert_eq!(
            TranslationBackend::Claude.classify_error(&claude_err),
            "session_conflict"
        );
        // Codex-specific 세션 충돌 문구
        let codex_err = AppError::Process("thread thr_xyz not found".into());
        assert_eq!(
            TranslationBackend::Codex.classify_error(&codex_err),
            "session_conflict"
        );
    }
}
