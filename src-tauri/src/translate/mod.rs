pub mod jsonl_parser;
pub mod prompt;
pub mod validator;

use serde::{Deserialize, Serialize};

/// 영상 메타데이터 (번역 프롬프트에 포함)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoInfo {
    pub title: String,
    pub description: String,
}

/// 번역된 자막 한 줄
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationEntry {
    pub original: String,
    pub translated: String,
    pub start: f64,
    pub end: f64,
}
