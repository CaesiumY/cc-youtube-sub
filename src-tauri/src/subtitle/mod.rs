pub mod chunk;
pub mod fetch;
pub mod parser;

use serde::{Deserialize, Serialize};

/// 정규화된 자막 한 줄 (start + end 시간 포함)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleLine {
    pub text: String,
    pub start: f64,
    pub end: f64,
}

/// 시간 기반으로 분할된 자막 청크 (30s~1m)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleChunk {
    pub index: i32,
    pub start_time: f64,
    pub end_time: f64,
    pub lines: Vec<SubtitleLine>,
}
