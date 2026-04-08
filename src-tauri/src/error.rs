use serde::Serialize;
use thiserror::Error;

/// 프론트엔드에 전달되는 구조화된 에러
/// React에서 { kind: 'CaptionFetch', message: '...' } 형태로 수신
#[derive(Debug, Error, Serialize)]
#[serde(tag = "kind", content = "message")]
pub enum AppError {
    #[error("자막을 가져올 수 없습니다: {0}")]
    CaptionFetch(String),

    #[error("번역 중 오류가 발생했습니다: {0}")]
    Translation(String),

    #[error("데이터베이스 오류: {0}")]
    #[allow(dead_code)]
    Database(String),

    #[error("Claude CLI를 찾을 수 없습니다: {0}")]
    EnvironmentCheck(String),

    #[error("프로세스 오류: {0}")]
    Process(String),
}

impl From<yt_transcript_rs::CouldNotRetrieveTranscript> for AppError {
    fn from(e: yt_transcript_rs::CouldNotRetrieveTranscript) -> Self {
        AppError::CaptionFetch(format!("video_id={}: {:?}", e.video_id, e.reason))
    }
}

impl From<yt_transcript_rs::CookieError> for AppError {
    fn from(e: yt_transcript_rs::CookieError) -> Self {
        AppError::CaptionFetch(format!("HTTP 클라이언트 초기화 실패: {:?}", e))
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Translation(format!("JSON 파싱 실패: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_caption_fetch() {
        let err = AppError::CaptionFetch("no captions".into());
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"kind\":\"CaptionFetch\""));
        assert!(json.contains("\"message\":\"no captions\""));
    }

    #[test]
    fn test_serialize_environment_check() {
        let err = AppError::EnvironmentCheck("claude not found".into());
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"kind\":\"EnvironmentCheck\""));
    }

    #[test]
    fn test_display_impl() {
        let err = AppError::Process("timeout".into());
        assert_eq!(err.to_string(), "프로세스 오류: timeout");
    }
}
