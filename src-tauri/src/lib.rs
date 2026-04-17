pub mod buffer_manager;
pub mod cache;
pub mod claude;
pub mod error;
pub mod subtitle;
pub mod translate;

use std::collections::HashMap;
use std::sync::Arc;
use tauri::Manager;

use buffer_manager::BufferManager;
use cache::{compute_chunk_hash, TranslationCache};
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
///
/// 이 커맨드는 BufferManager를 거치지 않는 독립 호출용 (브라우저 mock 폴백 등).
/// 세션 재사용은 BufferManager 경로에서만 적용된다.
#[tauri::command]
async fn translate_chunk(
    chunk: SubtitleChunk,
    video_info: Option<VideoInfo>,
    previous_context: Option<Vec<SubtitleLine>>,
    model: Option<String>,
) -> Result<Vec<TranslationEntry>, AppError> {
    let prompt = build_prompt(
        &chunk,
        video_info.as_ref(),
        previous_context.as_deref(),
        false,
    );

    let raw_output =
        claude::adapter::ClaudeAdapter::execute(&prompt, 120, model.as_deref(), None, false)
            .await?;

    let json_text = extract_text_from_jsonl(&raw_output)
        .map_err(|e| AppError::Translation(format!("JSONL 파싱 실패: {}", e)))?;

    let entries = validate_translation(&json_text)?;

    Ok(entries)
}

// ── 캐시 커맨드 ──────────────────────────────────────

/// 단일 청크 캐시 조회
#[tauri::command]
async fn query_cache(
    video_id: String,
    chunk_hash: String,
    cache: tauri::State<'_, Arc<TranslationCache>>,
) -> Result<Option<String>, AppError> {
    cache.query(&video_id, &chunk_hash)
}

/// 번역 결과를 캐시에 저장
#[tauri::command]
async fn save_to_cache(
    video_id: String,
    chunk_hash: String,
    translated_json: String,
    cache: tauri::State<'_, Arc<TranslationCache>>,
) -> Result<(), AppError> {
    cache.save(&video_id, &chunk_hash, &translated_json)
}

/// 여러 청크 캐시 일괄 조회 (재방문 시)
#[tauri::command]
async fn batch_query_cache(
    video_id: String,
    chunk_hashes: Vec<String>,
    cache: tauri::State<'_, Arc<TranslationCache>>,
) -> Result<HashMap<String, String>, AppError> {
    cache.batch_query(&video_id, &chunk_hashes)
}

/// 청크의 캐시 해시를 계산
#[tauri::command]
fn get_chunk_hash(lines: Vec<SubtitleLine>) -> String {
    compute_chunk_hash(&lines)
}

// ── 버퍼 매니저 커맨드 ──────────────────────────────

/// 새 영상의 버퍼 매니저를 초기화
#[tauri::command]
async fn init_buffer(
    video_id: String,
    chunks: Vec<SubtitleChunk>,
    video_info: Option<VideoInfo>,
    cached_indices: Vec<i32>,
    model: Option<String>,
    buffer: tauri::State<'_, Arc<BufferManager>>,
) -> Result<(), AppError> {
    buffer
        .init(video_id, chunks, video_info, cached_indices, model)
        .await;
    Ok(())
}

/// 재생 위치 업데이트 → 사전 버퍼링 스케줄링
#[tauri::command]
async fn update_playback_position(
    current_time: f64,
    buffer: tauri::State<'_, Arc<BufferManager>>,
    cache: tauri::State<'_, Arc<TranslationCache>>,
    app: tauri::AppHandle,
) -> Result<(), AppError> {
    let buffer = Arc::clone(buffer.inner());
    let cache = Arc::clone(cache.inner());
    buffer.update_position(current_time, cache, app).await
}

/// Seek 이벤트 처리
#[tauri::command]
async fn on_seek(
    target_time: f64,
    buffer: tauri::State<'_, Arc<BufferManager>>,
    cache: tauri::State<'_, Arc<TranslationCache>>,
    app: tauri::AppHandle,
) -> Result<(), AppError> {
    let buffer = Arc::clone(buffer.inner());
    let cache = Arc::clone(cache.inner());
    buffer.on_seek(target_time, cache, app).await
}

/// 버퍼링 취소 (영상 전환 시)
#[tauri::command]
async fn cancel_buffering(buffer: tauri::State<'_, Arc<BufferManager>>) -> Result<(), AppError> {
    buffer.cancel().await;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 앱 데이터 디렉토리에 SQLite DB 생성
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let app_dir = app
                .path()
                .app_data_dir()
                .expect("앱 데이터 디렉토리를 찾을 수 없습니다");
            let db_path = app_dir.join("translation_cache.db");

            let cache = TranslationCache::new(db_path).expect("SQLite 캐시 초기화 실패");

            app.manage(Arc::new(cache));
            app.manage(Arc::new(BufferManager::new()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            check_environment,
            fetch_subtitles,
            fetch_video_info,
            translate_chunk,
            query_cache,
            save_to_cache,
            batch_query_cache,
            get_chunk_hash,
            init_buffer,
            update_playback_position,
            on_seek,
            cancel_buffering,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
