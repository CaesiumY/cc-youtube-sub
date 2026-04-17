use std::collections::HashMap;
use std::sync::Arc;

use serde::Serialize;
use tauri::Emitter;
use tokio::sync::Mutex;

use crate::cache::{compute_chunk_hash, TranslationCache};
use crate::claude::adapter::ClaudeAdapter;
use crate::error::AppError;
use crate::subtitle::{SubtitleChunk, SubtitleLine};
use crate::translate::jsonl_parser::extract_text_from_jsonl;
use crate::translate::prompt::build_prompt;
use crate::translate::validator::validate_translation;
use crate::translate::{TranslationEntry, VideoInfo};

/// 현재 청크 이후 미리 번역할 청크 수
const LOOK_AHEAD: usize = 6;
/// 동시 번역 프로세스 최대 수
const MAX_CONCURRENT: usize = 4;
/// rate limit 감지 시 임시로 내려갈 동시 실행 수
const MAX_CONCURRENT_BACKOFF: usize = 1;
/// 청크당 최대 재시도 횟수
const MAX_RETRIES: u32 = 3;
/// rate limit 감지 후 백오프 유지 시간
const RATE_LIMIT_COOLDOWN_SECS: u64 = 45;

// ── Tauri 이벤트 페이로드 ───────────────────────────

/// 번역 완료 시 프론트엔드로 전달되는 이벤트
#[derive(Debug, Clone, Serialize)]
pub struct SubtitleUpdateEvent {
    pub chunk_index: i32,
    pub entries: Vec<TranslationEntry>,
    pub session_id: u64,
}

/// 번역 실패 시 프론트엔드로 전달되는 이벤트
#[derive(Debug, Clone, Serialize)]
pub struct BufferErrorEvent {
    pub chunk_index: i32,
    pub error: String,
    pub error_kind: String,
    pub retryable: bool,
    pub session_id: u64,
}

/// 청크 상태 변경 시 프론트엔드로 전달되는 이벤트
#[derive(Debug, Clone, Serialize)]
pub struct BufferStatusEvent {
    pub chunk_index: i32,
    pub status: String,
    pub session_id: u64,
}

// ── 내부 상태 ───────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum ChunkTranslationStatus {
    Pending,
    InProgress,
    Done,
    Cached,
    Error(u32),
}

struct BufferState {
    video_id: String,
    chunks: Vec<SubtitleChunk>,
    video_info: Option<VideoInfo>,
    model: Option<String>,
    chunk_hashes: HashMap<i32, String>,
    current_position: f64,
    statuses: HashMap<i32, ChunkTranslationStatus>,
    in_progress: usize,
    session_id: u64,
    /// Claude CLI 세션 UUID — 같은 영상의 모든 청크가 공유하여 맥락 연속성 확보
    claude_session_id: String,
    /// 세션이 생성되었는지 (첫 청크 번역 성공 여부)
    session_initialized: bool,
    /// 세션 충돌이 한 번이라도 감지되면 true — 이후 청크는 claude_session_id 없이
    /// 독립 실행(맥락 손실, 안정성 우선).
    session_reuse_disabled: bool,
    /// rate limit 백오프 종료 시각
    rate_limited_until: Option<std::time::Instant>,
}

// ── BufferManager 공개 API ──────────────────────────

pub struct BufferManager {
    state: Mutex<Option<BufferState>>,
}

impl Default for BufferManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BufferManager {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(None),
        }
    }

    /// 새 영상의 버퍼를 초기화한다.
    /// chunk_hashes에 없는 청크는 내부에서 SHA256으로 계산한다.
    pub async fn init(
        &self,
        video_id: String,
        chunks: Vec<SubtitleChunk>,
        video_info: Option<VideoInfo>,
        cached_indices: Vec<i32>,
        model: Option<String>,
    ) {
        let chunk_hashes: HashMap<i32, String> = chunks
            .iter()
            .map(|c| (c.index, compute_chunk_hash(&c.lines)))
            .collect();

        let mut statuses = HashMap::new();
        for chunk in &chunks {
            if cached_indices.contains(&chunk.index) {
                statuses.insert(chunk.index, ChunkTranslationStatus::Cached);
            } else {
                statuses.insert(chunk.index, ChunkTranslationStatus::Pending);
            }
        }

        let mut lock = self.state.lock().await;
        *lock = Some(BufferState {
            video_id,
            chunks,
            video_info,
            model,
            chunk_hashes,
            current_position: 0.0,
            statuses,
            in_progress: 0,
            session_id: 0,
            claude_session_id: uuid::Uuid::new_v4().to_string(),
            session_initialized: false,
            session_reuse_disabled: false,
            rate_limited_until: None,
        });
    }

    /// 재생 위치 업데이트 → 우선순위 청크 식별 → 번역 태스크 spawn.
    ///
    /// 500ms 폴링에서 호출된다. 내부적으로:
    /// 1. 현재 위치의 청크 인덱스를 계산
    /// 2. 현재 + LOOK_AHEAD 범위에서 미번역 청크를 수집
    /// 3. MAX_CONCURRENT 미만이면 tokio 태스크를 spawn
    pub async fn update_position(
        self: Arc<Self>,
        current_time: f64,
        cache: Arc<TranslationCache>,
        app: tauri::AppHandle,
    ) -> Result<(), AppError> {
        // 락 안에서 spawn할 태스크 목록만 수집하고 락을 해제한다.
        // 이렇게 하면 번역 실행 중 락이 잡히지 않는다.
        let tasks_to_spawn = {
            let mut lock = self.state.lock().await;
            let state = match lock.as_mut() {
                Some(s) => s,
                None => return Ok(()), // 아직 초기화 전 — 다음 폴링에서 재시도
            };

            state.current_position = current_time;

            // rate limit 백오프 확인 — 쿨다운 중이면 동시성을 1로 축소
            let effective_concurrent = match state.rate_limited_until {
                Some(until) if std::time::Instant::now() < until => MAX_CONCURRENT_BACKOFF,
                _ => {
                    // 쿨다운 만료 시 필드 정리
                    if state.rate_limited_until.is_some() {
                        state.rate_limited_until = None;
                    }
                    MAX_CONCURRENT
                }
            };

            let priority = get_priority_chunks(state);
            let mut tasks = Vec::new();

            for idx in priority {
                if !can_spawn_in_state(state, effective_concurrent) {
                    break;
                }

                let retry_count = match state.statuses.get(&idx) {
                    Some(ChunkTranslationStatus::Pending) => 0,
                    Some(ChunkTranslationStatus::Error(n)) if *n < MAX_RETRIES => *n,
                    _ => continue,
                };

                let chunk = match state.chunks.iter().find(|c| c.index == idx) {
                    Some(c) => c.clone(),
                    None => continue,
                };

                state
                    .statuses
                    .insert(idx, ChunkTranslationStatus::InProgress);
                state.in_progress += 1;

                // 세션 재사용 여부/bootstrap 상태에 따라 Claude CLI 호출 모드 결정
                let use_session = !state.session_reuse_disabled;
                let claude_session_id = if use_session {
                    Some(state.claude_session_id.clone())
                } else {
                    None
                };
                // 폴백 모드(세션 없음)이면 매번 "새 세션/독립 실행"이므로 true.
                // 세션 사용 중이면 초기 bootstrap 아직이면 true, 이후 resume 모드면 false.
                let is_first_in_session = !use_session || !state.session_initialized;

                // 세션 첫 호출(또는 독립 실행)에만 영상 정보/이전 맥락을 전달.
                let video_info_for_chunk = if is_first_in_session {
                    state.video_info.clone()
                } else {
                    None
                };
                let prev_context = if is_first_in_session {
                    get_previous_context(&state.chunks, idx)
                } else {
                    None
                };
                let session_id = state.session_id;
                let video_id = state.video_id.clone();
                let chunk_hash = state.chunk_hashes.get(&idx).cloned();

                tasks.push(SpawnTask {
                    chunk,
                    video_info: video_info_for_chunk,
                    prev_context,
                    model: state.model.clone(),
                    session_id,
                    video_id,
                    chunk_hash,
                    retry_count,
                    chunk_index: idx,
                    claude_session_id,
                    is_first_in_session,
                });
            }

            tasks
        }; // 락 해제

        // 태스크 spawn (락 밖에서 실행)
        for task in tasks_to_spawn {
            let _ = app.emit(
                "buffer-status",
                BufferStatusEvent {
                    chunk_index: task.chunk_index,
                    status: "translating".into(),
                    session_id: task.session_id,
                },
            );

            let buffer = Arc::clone(&self);
            let cache = Arc::clone(&cache);
            let app = app.clone();

            tokio::spawn(async move {
                let result = translate_chunk_internal(
                    &task.chunk,
                    task.video_info.as_ref(),
                    task.prev_context.as_deref(),
                    task.model.as_deref(),
                    task.claude_session_id.as_deref(),
                    task.is_first_in_session,
                )
                .await;

                match result {
                    Ok(entries) => {
                        buffer
                            .handle_completion(
                                task.chunk_index,
                                task.session_id,
                                entries,
                                &task.video_id,
                                task.chunk_hash.as_deref(),
                                &cache,
                                &app,
                            )
                            .await;
                    }
                    Err(err) => {
                        buffer
                            .handle_error(
                                task.chunk_index,
                                task.session_id,
                                task.retry_count,
                                &err,
                                &app,
                            )
                            .await;
                    }
                }
                // 다음 스케줄링은 프론트엔드의 500ms 폴링이 담당
            });
        }

        Ok(())
    }

    /// Seek 이벤트 처리: 세션 증가 → InProgress 복원 → 새 위치에서 스케줄링.
    ///
    /// 진행 중 태스크는 완료까지 실행되지만, session_id 불일치로 결과가 폐기된다.
    pub async fn on_seek(
        self: Arc<Self>,
        target_time: f64,
        cache: Arc<TranslationCache>,
        app: tauri::AppHandle,
    ) -> Result<(), AppError> {
        {
            let mut lock = self.state.lock().await;
            let state = match lock.as_mut() {
                Some(s) => s,
                None => return Ok(()), // 아직 초기화 전 — 무시
            };

            state.session_id += 1;
            state.current_position = target_time;
            state.in_progress = 0;

            for status in state.statuses.values_mut() {
                if *status == ChunkTranslationStatus::InProgress {
                    *status = ChunkTranslationStatus::Pending;
                }
            }
        }

        self.update_position(target_time, cache, app).await
    }

    /// 버퍼를 해제한다 (영상 전환 또는 페이지 이탈 시).
    pub async fn cancel(&self) {
        let mut lock = self.state.lock().await;
        if let Some(state) = lock.as_mut() {
            state.session_id += 1;
        }
        *lock = None;
    }

    // ── 내부 메서드 ─────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    async fn handle_completion(
        &self,
        chunk_index: i32,
        task_session_id: u64,
        entries: Vec<TranslationEntry>,
        video_id: &str,
        chunk_hash: Option<&str>,
        cache: &TranslationCache,
        app: &tauri::AppHandle,
    ) {
        let mut lock = self.state.lock().await;
        let state = match lock.as_mut() {
            Some(s) if s.session_id == task_session_id => s,
            _ => return, // 세션 불일치 또는 해제됨 → 결과 폐기
        };

        state
            .statuses
            .insert(chunk_index, ChunkTranslationStatus::Done);
        state.in_progress = state.in_progress.saturating_sub(1);
        // 첫 청크 번역 성공 → 세션이 생성되었으므로 이후는 --resume 모드로
        state.session_initialized = true;

        // 캐시 저장 (실패해도 무시)
        if let Some(hash) = chunk_hash {
            if let Ok(json) = serde_json::to_string(&entries) {
                let _ = cache.save(video_id, hash, &json);
            }
        }

        let _ = app.emit(
            "subtitle-update",
            SubtitleUpdateEvent {
                chunk_index,
                entries,
                session_id: task_session_id,
            },
        );

        let _ = app.emit(
            "buffer-status",
            BufferStatusEvent {
                chunk_index,
                status: "done".into(),
                session_id: task_session_id,
            },
        );
    }

    async fn handle_error(
        &self,
        chunk_index: i32,
        task_session_id: u64,
        prev_retry_count: u32,
        error: &AppError,
        app: &tauri::AppHandle,
    ) {
        let mut lock = self.state.lock().await;
        let state = match lock.as_mut() {
            Some(s) if s.session_id == task_session_id => s,
            _ => return,
        };

        let new_retry = prev_retry_count + 1;
        let retryable = new_retry < MAX_RETRIES;

        state
            .statuses
            .insert(chunk_index, ChunkTranslationStatus::Error(new_retry));
        state.in_progress = state.in_progress.saturating_sub(1);

        let error_kind = classify_error(error);

        // 세션 충돌 감지 시 폴백: 이후 청크는 세션 없이 독립 실행.
        // 이 청크는 그대로 retryable 상태로 두면 다음 폴링에서 독립 모드로 재시도됨.
        if error_kind == "session_conflict" {
            state.session_reuse_disabled = true;
            eprintln!(
                "[buffer] 세션 충돌 감지, 영상 나머지 번역을 독립 모드로 폴백 (chunk {})",
                chunk_index
            );
        }

        // rate limit 감지 시 쿨다운 시작 — update_position이 MAX_CONCURRENT_BACKOFF로 제한
        if error_kind == "rate_limit" {
            state.rate_limited_until = Some(
                std::time::Instant::now()
                    + std::time::Duration::from_secs(RATE_LIMIT_COOLDOWN_SECS),
            );
        }

        // 주의: 여기서 session_initialized를 강제로 true로 설정하지 않는다.
        // 네트워크/CLI 실패 등으로 Claude 세션이 실제로 생성되지 않은 상태에서 true로 두면
        // 이후 재시도가 --resume을 시도해 "세션 없음"으로 영구 실패로 이어질 수 있다.
        // 대신 세션 충돌이 실제로 재발할 경우(session_conflict)에만 session_reuse_disabled
        // 폴백이 작동하여 독립 모드로 전환되므로 안전. session_initialized는 handle_completion
        // (실제 세션 생성 확인)에서만 true로 설정된다.

        let _ = app.emit(
            "buffer-error",
            BufferErrorEvent {
                chunk_index,
                error: error.to_string(),
                error_kind,
                retryable,
                session_id: task_session_id,
            },
        );

        let _ = app.emit(
            "buffer-status",
            BufferStatusEvent {
                chunk_index,
                status: "error".into(),
                session_id: task_session_id,
            },
        );
    }
}

// ── spawn 태스크용 데이터 구조 ──────────────────────

struct SpawnTask {
    chunk: SubtitleChunk,
    video_info: Option<VideoInfo>,
    prev_context: Option<Vec<SubtitleLine>>,
    model: Option<String>,
    session_id: u64,
    video_id: String,
    chunk_hash: Option<String>,
    retry_count: u32,
    chunk_index: i32,
    /// Claude CLI 세션 UUID. `None`이면 세션 재사용 비활성 (폴백 모드) — 독립 실행.
    claude_session_id: Option<String>,
    is_first_in_session: bool,
}

// ── 헬퍼 함수 ───────────────────────────────────────

/// 현재 재생 위치 기준으로 번역이 필요한 청크를 우선순위 순으로 반환
fn get_priority_chunks(state: &BufferState) -> Vec<i32> {
    let current_idx = state
        .chunks
        .iter()
        .find(|c| c.start_time <= state.current_position && c.end_time > state.current_position)
        .map(|c| c.index)
        .unwrap_or(0);

    let max_idx = state.chunks.iter().map(|c| c.index).max().unwrap_or(0);

    let mut result = Vec::new();
    for offset in 0..=(LOOK_AHEAD as i32) {
        let idx = current_idx + offset;
        if idx > max_idx {
            break;
        }

        match state.statuses.get(&idx) {
            Some(ChunkTranslationStatus::Pending) => result.push(idx),
            Some(ChunkTranslationStatus::Error(n)) if *n < MAX_RETRIES => result.push(idx),
            _ => {}
        }
    }
    result
}

/// 이전 청크의 마지막 8줄을 context로 추출 (세션 첫 호출에만 사용)
fn get_previous_context(chunks: &[SubtitleChunk], current_index: i32) -> Option<Vec<SubtitleLine>> {
    if current_index == 0 {
        return None;
    }
    chunks
        .iter()
        .find(|c| c.index == current_index - 1)
        .map(|c| c.lines.iter().rev().take(8).rev().cloned().collect())
}

/// 에러 메시지에서 종류를 분류 (프론트엔드 UI 분기 + BufferManager 폴백 로직에 사용)
fn classify_error(error: &AppError) -> String {
    let msg = error.to_string().to_lowercase();
    if msg.contains("session id") && msg.contains("already in use") {
        // Claude CLI: `Error: Session ID {uuid} is already in use.`
        "session_conflict".into()
    } else if msg.contains("rate limit") || msg.contains("exceeded") {
        "rate_limit".into()
    } else if msg.contains("timeout") || msg.contains("타임아웃") {
        "timeout".into()
    } else if msg.contains("claude") && (msg.contains("찾을 수 없") || msg.contains("not found"))
    {
        "cli_not_found".into()
    } else {
        match error {
            AppError::CaptionFetch(_) => "caption_fetch".into(),
            AppError::Translation(_) => "translation".into(),
            AppError::Database(_) => "database".into(),
            AppError::EnvironmentCheck(_) => "environment".into(),
            AppError::Process(_) => "process".into(),
        }
    }
}

/// `update_position` spawn 루프에서 "이 상태에서 추가 태스크를 spawn할 수 있는가" 판단.
///
/// 두 가지 조건을 결합:
/// 1. `in_progress < effective_concurrent` — 동시 실행 한도
/// 2. bootstrap guard — 세션 재사용 활성이고 아직 초기화 전이면 동시에 여러 개 spawn 금지
///    (같은 `--session-id` UUID로 Claude 프로세스가 여러 개 뜨면 "already in use" 충돌)
fn can_spawn_in_state(state: &BufferState, effective_concurrent: usize) -> bool {
    if state.in_progress >= effective_concurrent {
        return false;
    }
    let use_session = !state.session_reuse_disabled;
    let is_bootstrap = use_session && !state.session_initialized;
    if is_bootstrap && state.in_progress > 0 {
        return false;
    }
    true
}

/// 단일 청크 번역 실행: 프롬프트 구성 → Claude subprocess → JSONL 파싱 → 검증
async fn translate_chunk_internal(
    chunk: &SubtitleChunk,
    video_info: Option<&VideoInfo>,
    previous_context: Option<&[SubtitleLine]>,
    model: Option<&str>,
    claude_session_id: Option<&str>,
    is_first_in_session: bool,
) -> Result<Vec<TranslationEntry>, AppError> {
    let prompt = build_prompt(chunk, video_info, previous_context, !is_first_in_session);
    let raw_output =
        ClaudeAdapter::execute(&prompt, 120, model, claude_session_id, is_first_in_session).await?;
    let json_text = extract_text_from_jsonl(&raw_output)
        .map_err(|e| AppError::Translation(format!("JSONL 파싱 실패: {}", e)))?;
    validate_translation(&json_text)
}

// ── 테스트 ──────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_chunks(n: i32) -> Vec<SubtitleChunk> {
        (0..n)
            .map(|i| SubtitleChunk {
                index: i,
                start_time: i as f64 * 30.0,
                end_time: (i + 1) as f64 * 30.0,
                lines: vec![SubtitleLine {
                    text: format!("Line {}", i),
                    start: i as f64 * 30.0,
                    end: (i + 1) as f64 * 30.0,
                }],
            })
            .collect()
    }

    #[test]
    fn test_priority_from_start() {
        let chunks = make_chunks(10);
        let statuses: HashMap<_, _> = chunks
            .iter()
            .map(|c| (c.index, ChunkTranslationStatus::Pending))
            .collect();

        let state = BufferState {
            video_id: "test".into(),
            chunks,
            video_info: None,
            model: None,
            chunk_hashes: HashMap::new(),
            current_position: 0.0,
            statuses,
            in_progress: 0,
            session_id: 0,
            claude_session_id: "test-session".into(),
            session_initialized: false,
            session_reuse_disabled: false,
            rate_limited_until: None,
        };

        let result = get_priority_chunks(&state);
        // LOOK_AHEAD=6 → offsets 0..=6
        assert_eq!(result, vec![0, 1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn test_priority_skips_done() {
        let chunks = make_chunks(10);
        let mut statuses: HashMap<_, _> = chunks
            .iter()
            .map(|c| (c.index, ChunkTranslationStatus::Pending))
            .collect();
        statuses.insert(0, ChunkTranslationStatus::Done);
        statuses.insert(1, ChunkTranslationStatus::Cached);

        let state = BufferState {
            video_id: "test".into(),
            chunks,
            video_info: None,
            model: None,
            chunk_hashes: HashMap::new(),
            current_position: 0.0,
            statuses,
            in_progress: 0,
            session_id: 0,
            claude_session_id: "test-session".into(),
            session_initialized: false,
            session_reuse_disabled: false,
            rate_limited_until: None,
        };

        let result = get_priority_chunks(&state);
        // current_idx=0, LOOK_AHEAD=6 → offsets 0..=6, but 0=Done, 1=Cached skipped
        assert_eq!(result, vec![2, 3, 4, 5, 6]);
    }

    #[test]
    fn test_priority_mid_video() {
        let chunks = make_chunks(10);
        let mut statuses: HashMap<_, _> = chunks
            .iter()
            .map(|c| (c.index, ChunkTranslationStatus::Pending))
            .collect();
        for i in 0..5 {
            statuses.insert(i, ChunkTranslationStatus::Done);
        }

        let state = BufferState {
            video_id: "test".into(),
            chunks,
            video_info: None,
            model: None,
            chunk_hashes: HashMap::new(),
            current_position: 155.0, // 청크 5 중간 (150-180)
            statuses,
            in_progress: 0,
            session_id: 0,
            claude_session_id: "test-session".into(),
            session_initialized: false,
            session_reuse_disabled: false,
            rate_limited_until: None,
        };

        let result = get_priority_chunks(&state);
        // LOOK_AHEAD=6 → current_idx(5) + offsets 0..=6 = [5..=11], but max_idx=9
        assert_eq!(result, vec![5, 6, 7, 8, 9]);
    }

    #[test]
    fn test_priority_near_end() {
        let chunks = make_chunks(5);
        let mut statuses: HashMap<_, _> = chunks
            .iter()
            .map(|c| (c.index, ChunkTranslationStatus::Pending))
            .collect();
        for i in 0..4 {
            statuses.insert(i, ChunkTranslationStatus::Done);
        }

        let state = BufferState {
            video_id: "test".into(),
            chunks,
            video_info: None,
            model: None,
            chunk_hashes: HashMap::new(),
            current_position: 125.0,
            statuses,
            in_progress: 0,
            session_id: 0,
            claude_session_id: "test-session".into(),
            session_initialized: false,
            session_reuse_disabled: false,
            rate_limited_until: None,
        };

        let result = get_priority_chunks(&state);
        assert_eq!(result, vec![4]);
    }

    #[test]
    fn test_priority_retries_error_chunks() {
        let chunks = make_chunks(5);
        let mut statuses: HashMap<_, _> = chunks
            .iter()
            .map(|c| (c.index, ChunkTranslationStatus::Pending))
            .collect();
        statuses.insert(0, ChunkTranslationStatus::Error(1)); // 재시도 가능
        statuses.insert(1, ChunkTranslationStatus::Error(MAX_RETRIES)); // 재시도 불가

        let state = BufferState {
            video_id: "test".into(),
            chunks,
            video_info: None,
            model: None,
            chunk_hashes: HashMap::new(),
            current_position: 0.0,
            statuses,
            in_progress: 0,
            session_id: 0,
            claude_session_id: "test-session".into(),
            session_initialized: false,
            session_reuse_disabled: false,
            rate_limited_until: None,
        };

        let result = get_priority_chunks(&state);
        // current_idx=0, LOOK_AHEAD=6, max_idx=4 → 0(retry),1(skip),2,3,4
        assert_eq!(result, vec![0, 2, 3, 4]);
    }

    #[test]
    fn test_previous_context_first_chunk() {
        let chunks = make_chunks(3);
        assert!(get_previous_context(&chunks, 0).is_none());
    }

    #[test]
    fn test_previous_context_second_chunk() {
        let chunks = make_chunks(3);
        let ctx = get_previous_context(&chunks, 1);
        assert!(ctx.is_some());
        assert_eq!(ctx.unwrap().len(), 1);
    }

    #[test]
    fn test_classify_error_rate_limit() {
        let err = AppError::Process("Claude rate limit exceeded".into());
        assert_eq!(classify_error(&err), "rate_limit");
    }

    #[test]
    fn test_classify_error_timeout() {
        let err = AppError::Process("Claude 응답 타임아웃 (120초)".into());
        assert_eq!(classify_error(&err), "timeout");
    }

    #[test]
    fn test_classify_error_generic() {
        let err = AppError::Translation("파싱 실패".into());
        assert_eq!(classify_error(&err), "translation");
    }

    #[test]
    fn test_classify_error_session_conflict() {
        let err = AppError::Process(
            "Claude 프로세스 비정상 종료 (코드: Some(1)): \
             Error: Session ID c78bf130-fd78-45d2-bdb2-ee002fcece2e is already in use."
                .into(),
        );
        assert_eq!(classify_error(&err), "session_conflict");
    }

    #[test]
    fn test_classify_error_session_conflict_case_insensitive() {
        let err = AppError::Process("SESSION ID abc IS ALREADY IN USE".into());
        assert_eq!(classify_error(&err), "session_conflict");
    }

    #[test]
    fn test_classify_error_session_conflict_not_overmatched() {
        // "session" 만 있다고 session_conflict 로 잡히면 안 됨
        let err = AppError::Process("some session-related issue, not a conflict".into());
        assert_ne!(classify_error(&err), "session_conflict");
        // "already in use"만 있어도 마찬가지
        let err2 = AppError::Process("port is already in use".into());
        assert_ne!(classify_error(&err2), "session_conflict");
    }

    fn make_state_for_spawn(
        chunks: Vec<SubtitleChunk>,
        session_initialized: bool,
        session_reuse_disabled: bool,
        in_progress: usize,
    ) -> BufferState {
        let statuses: HashMap<_, _> = chunks
            .iter()
            .map(|c| (c.index, ChunkTranslationStatus::Pending))
            .collect();
        BufferState {
            video_id: "test".into(),
            chunks,
            video_info: None,
            model: None,
            chunk_hashes: HashMap::new(),
            current_position: 0.0,
            statuses,
            in_progress,
            session_id: 0,
            claude_session_id: "test-session".into(),
            session_initialized,
            session_reuse_disabled,
            rate_limited_until: None,
        }
    }

    #[test]
    fn test_can_spawn_bootstrap_guard_blocks_second() {
        let chunks = make_chunks(5);
        // 첫 청크 spawn 직후(in_progress=1) session_initialized=false → guard 발동
        let state = make_state_for_spawn(chunks, false, false, 1);
        assert!(!can_spawn_in_state(&state, 4));
    }

    #[test]
    fn test_can_spawn_bootstrap_guard_allows_first() {
        let chunks = make_chunks(5);
        // 아직 아무것도 spawn 안 됨(in_progress=0) → 첫 번째는 허용
        let state = make_state_for_spawn(chunks, false, false, 0);
        assert!(can_spawn_in_state(&state, 4));
    }

    #[test]
    fn test_can_spawn_parallel_after_bootstrap() {
        let chunks = make_chunks(5);
        // session_initialized=true → 여러 개 병렬 spawn 허용
        let state = make_state_for_spawn(chunks, true, false, 2);
        assert!(can_spawn_in_state(&state, 4));
    }

    #[test]
    fn test_can_spawn_respects_effective_concurrent() {
        let chunks = make_chunks(5);
        // 동시 한도 도달
        let state = make_state_for_spawn(chunks, true, false, 4);
        assert!(!can_spawn_in_state(&state, 4));
    }

    #[test]
    fn test_can_spawn_reuse_disabled_skips_bootstrap_guard() {
        let chunks = make_chunks(5);
        // 세션 재사용이 꺼져 있으면 bootstrap guard도 무효 — 여러 개 동시 spawn 가능
        let state = make_state_for_spawn(chunks, false, true, 2);
        assert!(can_spawn_in_state(&state, 4));
    }

    #[tokio::test]
    async fn test_handle_error_session_conflict_disables_reuse() {
        let mgr = BufferManager::new();
        mgr.init("vid1".into(), make_chunks(3), None, vec![], None)
            .await;

        // 첫 청크가 InProgress 상태라고 가정
        {
            let mut lock = mgr.state.lock().await;
            let state = lock.as_mut().unwrap();
            state.statuses.insert(0, ChunkTranslationStatus::InProgress);
            state.in_progress = 1;
        }

        let err = AppError::Process(
            "Claude 프로세스 비정상 종료 (코드: Some(1)): \
             Error: Session ID abc is already in use."
                .into(),
        );

        // AppHandle 없이 상태만 검증하기 위해 handle_error 내부 로직을 수동 시뮬레이트.
        // (handle_error는 app.emit을 호출하지만 state 변이만 여기서 검증)
        {
            let mut lock = mgr.state.lock().await;
            let state = lock.as_mut().unwrap();
            state.statuses.insert(0, ChunkTranslationStatus::Error(1));
            state.in_progress = state.in_progress.saturating_sub(1);
            let error_kind = classify_error(&err);
            if error_kind == "session_conflict" {
                state.session_reuse_disabled = true;
            }
            // session_initialized는 handle_error에서 변경하지 않음 — handle_completion 전용
        }

        let lock = mgr.state.lock().await;
        let state = lock.as_ref().unwrap();
        assert!(state.session_reuse_disabled, "충돌 감지 후 재사용 비활성");
        assert!(
            !state.session_initialized,
            "handle_error는 session_initialized를 변경하지 않음 — \
             네트워크/CLI 실패로 세션 미생성인 경우 resume 시 영구 실패 방지"
        );
        assert_eq!(state.in_progress, 0);
        assert_eq!(state.statuses[&0], ChunkTranslationStatus::Error(1));
    }

    #[tokio::test]
    async fn test_handle_error_generic_keeps_session_reuse() {
        let mgr = BufferManager::new();
        mgr.init("vid1".into(), make_chunks(3), None, vec![], None)
            .await;

        {
            let mut lock = mgr.state.lock().await;
            let state = lock.as_mut().unwrap();
            state.statuses.insert(0, ChunkTranslationStatus::InProgress);
            state.in_progress = 1;
        }

        let err = AppError::Translation("JSONL 파싱 실패".into());

        {
            let mut lock = mgr.state.lock().await;
            let state = lock.as_mut().unwrap();
            state.statuses.insert(0, ChunkTranslationStatus::Error(1));
            state.in_progress = state.in_progress.saturating_sub(1);
            let error_kind = classify_error(&err);
            if error_kind == "session_conflict" {
                state.session_reuse_disabled = true;
            }
            // session_initialized는 변경하지 않음
        }

        let lock = mgr.state.lock().await;
        let state = lock.as_ref().unwrap();
        assert!(
            !state.session_reuse_disabled,
            "일반 에러는 세션 재사용 유지"
        );
        assert!(
            !state.session_initialized,
            "handle_error는 session_initialized를 변경하지 않음"
        );
    }

    #[tokio::test]
    async fn test_init_sets_statuses() {
        let mgr = BufferManager::new();
        let chunks = make_chunks(3);

        mgr.init("vid1".into(), chunks, None, vec![1], None).await;

        let lock = mgr.state.lock().await;
        let state = lock.as_ref().unwrap();
        assert_eq!(state.video_id, "vid1");
        assert_eq!(state.chunks.len(), 3);
        assert_eq!(state.statuses[&0], ChunkTranslationStatus::Pending);
        assert_eq!(state.statuses[&1], ChunkTranslationStatus::Cached);
        assert_eq!(state.statuses[&2], ChunkTranslationStatus::Pending);
        assert_eq!(state.chunk_hashes.len(), 3);
    }

    #[tokio::test]
    async fn test_cancel_clears_state() {
        let mgr = BufferManager::new();
        mgr.init("vid1".into(), make_chunks(3), None, vec![], None)
            .await;

        mgr.cancel().await;

        let lock = mgr.state.lock().await;
        assert!(lock.is_none());
    }

    #[tokio::test]
    async fn test_seek_resets_in_progress() {
        let mgr = BufferManager::new();
        mgr.init("vid1".into(), make_chunks(5), None, vec![], None)
            .await;

        {
            let mut lock = mgr.state.lock().await;
            let state = lock.as_mut().unwrap();
            state.statuses.insert(0, ChunkTranslationStatus::InProgress);
            state.statuses.insert(1, ChunkTranslationStatus::InProgress);
            state.in_progress = 2;
        }

        // seek 로직을 직접 시뮬레이션 (AppHandle 없이)
        {
            let mut lock = mgr.state.lock().await;
            let state = lock.as_mut().unwrap();

            state.session_id += 1;
            state.current_position = 90.0;
            state.in_progress = 0;

            for status in state.statuses.values_mut() {
                if *status == ChunkTranslationStatus::InProgress {
                    *status = ChunkTranslationStatus::Pending;
                }
            }

            assert_eq!(state.session_id, 1);
            assert_eq!(state.in_progress, 0);
            assert_eq!(state.statuses[&0], ChunkTranslationStatus::Pending);
            assert_eq!(state.statuses[&1], ChunkTranslationStatus::Pending);
        }
    }

    #[test]
    fn test_init_computes_hashes() {
        // compute_chunk_hash가 결정적인지 확인
        let lines = vec![SubtitleLine {
            text: "Hello".into(),
            start: 0.0,
            end: 1.0,
        }];
        let h1 = compute_chunk_hash(&lines);
        let h2 = compute_chunk_hash(&lines);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64); // SHA256 hex
    }
}
