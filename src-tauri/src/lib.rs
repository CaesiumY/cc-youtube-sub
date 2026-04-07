mod error;

use error::AppError;

#[tauri::command]
fn fetch_subtitles(video_id: String) -> Result<String, AppError> {
    // Phase 1에서 구현: yt-transcript-rs 자막 fetch
    Ok(format!("Subtitles stub for video {}", video_id))
}

#[tauri::command]
fn translate_chunk(text: String) -> Result<String, AppError> {
    // Phase 2에서 구현: Claude subprocess 번역
    Ok(format!("Translation stub: {}", text))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![fetch_subtitles, translate_chunk])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
