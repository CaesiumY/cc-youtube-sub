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
    Database(String),

    #[error("Claude CLI를 찾을 수 없습니다: {0}")]
    EnvironmentCheck(String),

    #[error("프로세스 오류: {0}")]
    Process(String),
}
