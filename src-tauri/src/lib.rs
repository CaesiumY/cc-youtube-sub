mod claude;
mod error;
mod subtitle;
mod translate;

use error::AppError;
use subtitle::chunk::split_into_chunks;
use subtitle::fetch;
use subtitle::{SubtitleChunk, SubtitleLine};
use translate::jsonl_parser::extract_text_from_jsonl;
use translate::prompt::build_prompt;
use translate::validator::validate_translation;
use translate::{TranslationEntry, VideoInfo};

/// Claude CLI 환경 검증
#[tauri::command]
async fn check_environment() -> Result<String, AppError> {
    claude::adapter::ClaudeAdapter::test_environment().await?;
    Ok("Claude CLI가 정상적으로 설치되어 있습니다".into())
}

/// YouTube 영상 자막 fetch + 파싱 + 청크 분할
#[tauri::command]
async fn fetch_subtitles(video_id: String) -> Result<Vec<SubtitleChunk>, AppError> {
    let lines = fetch::fetch_subtitles(&video_id).await?;
    let chunks = split_into_chunks(&lines);
    Ok(chunks)
}

/// YouTube 영상 메타데이터 fetch
#[tauri::command]
async fn fetch_video_info(video_id: String) -> Result<VideoInfo, AppError> {
    fetch::fetch_video_info(&video_id).await
}

/// 단일 청크 번역: 프롬프트 구성 → Claude 실행 → JSONL 파싱 → 검증
#[tauri::command]
async fn translate_chunk(
    chunk: SubtitleChunk,
    video_info: Option<VideoInfo>,
    previous_context: Option<Vec<SubtitleLine>>,
) -> Result<Vec<TranslationEntry>, AppError> {
    let prompt = build_prompt(
        &chunk,
        video_info.as_ref(),
        previous_context.as_deref(),
    );

    let raw_output = claude::adapter::ClaudeAdapter::execute(&prompt, 120).await?;

    let json_text = extract_text_from_jsonl(&raw_output)
        .map_err(|e| AppError::Translation(format!("JSONL 파싱 실패: {}", e)))?;

    let entries = validate_translation(&json_text)?;

    Ok(entries)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            check_environment,
            fetch_subtitles,
            fetch_video_info,
            translate_chunk,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
