use serde::Deserialize;
use yt_transcript_rs::transcript_parser::TranscriptParser;
use yt_transcript_rs::YouTubeTranscriptApi;

use crate::error::AppError;
use crate::subtitle::SubtitleLine;
use crate::translate::VideoInfo;

use super::parser::{
    merge_into_sentences, normalize_transcript, split_lines_on_sentence_boundaries,
};

// ── ANDROID InnerTube 클라이언트 메타데이터 ──────────────
//
// YOUTUBE_CLIENT_METADATA_LAST_UPDATED: 2026-04-16
// 기준: yt-dlp 2026.01 (yt_dlp/extractor/youtube/_base.py)
//
// YouTube는 InnerTube 클라이언트 버전을 주기적으로 무효화한다. 3차 폴백이
// HTTP 400을 다시 반환하기 시작하면 아래 상수들을 yt-dlp 최신 마스터 기준으로
// 갱신하고 LAST_UPDATED 날짜만 바꾸면 된다.
const ANDROID_CLIENT_NAME: &str = "ANDROID";
const ANDROID_CLIENT_NAME_ID: &str = "3"; // X-YouTube-Client-Name 헤더 값
const ANDROID_CLIENT_VERSION: &str = "21.02.35";
const ANDROID_SDK_VERSION: u32 = 30;
const ANDROID_OS_NAME: &str = "Android";
const ANDROID_OS_VERSION: &str = "11";
const ANDROID_USER_AGENT: &str = "com.google.android.youtube/21.02.35 (Linux; U; Android 11) gzip";

// ── ANDROID InnerTube 직접 요청용 타입 ──────────────

/// YouTube InnerTube player 응답에서 캡션 트랙 추출용
#[derive(Debug, Deserialize)]
struct InnerTubePlayerResponse {
    captions: Option<CaptionsData>,
}

#[derive(Debug, Deserialize)]
struct CaptionsData {
    #[serde(rename = "playerCaptionsTracklistRenderer")]
    renderer: Option<CaptionTracklistRenderer>,
}

#[derive(Debug, Deserialize)]
struct CaptionTracklistRenderer {
    #[serde(rename = "captionTracks", default)]
    caption_tracks: Vec<CaptionTrack>,
}

#[derive(Debug, Deserialize)]
struct CaptionTrack {
    #[serde(rename = "baseUrl")]
    base_url: String,
    #[serde(rename = "languageCode")]
    language_code: String,
    /// `"asr"` = auto-generated (speech recognition). 수동 자막은 이 필드가 없음.
    #[serde(default)]
    kind: Option<String>,
}

/// YouTube 영상의 영어 자막을 fetch하여 정규화된 SubtitleLine 목록으로 반환
///
/// `yt-transcript-rs`의 `Transcript::fetch()`가 InnerTube 재요청에서
/// 실패하는 경우가 있어, `list_transcripts`로 URL을 얻은 뒤
/// 직접 HTTP GET으로 자막 XML을 가져온다.
pub async fn fetch_subtitles(video_id: &str) -> Result<Vec<SubtitleLine>, AppError> {
    // cookie_store 활성화: YouTube가 CONSENT 쿠키를 설정하면 이후 요청에서
    // ip=0.0.0.0 대신 실제 IP가 포함된 timedtext URL을 반환할 수 있음
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .cookie_store(true)
        .default_headers({
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                reqwest::header::ACCEPT_LANGUAGE,
                "en-US".parse().unwrap(),
            );
            headers
        })
        .build()
        .map_err(|e| AppError::CaptionFetch(format!("HTTP 클라이언트 생성 실패: {}", e)))?;

    let api = YouTubeTranscriptApi::new(None, None, Some(client))?;

    // 1차: 직접 fetch 시도 (InnerTube가 정상 동작하는 경우 빠름).
    // `fetch_transcript`는 내부적으로 find_transcript를 사용 — manual 자막을 우선하고
    // 없으면 auto-generated로 폴백한다 (yt-transcript-rs의 TranscriptList 설계).
    match api.fetch_transcript(video_id, &["en"], false).await {
        Ok(transcript) => {
            let lines = normalize_transcript(&transcript.snippets);
            if !lines.is_empty() {
                let is_auto = transcript.is_generated;
                eprintln!(
                    "[fetch] 1차 성공: {} lines (is_auto={})",
                    lines.len(),
                    is_auto
                );
                return Ok(postprocess_lines(lines, is_auto));
            }
            eprintln!("[fetch] 1차: transcript OK but 0 lines after normalize");
        }
        Err(e) => {
            eprintln!("[fetch] 1차 실패: {:?}", e.reason);
        }
    }

    // 2차: list_transcripts → URL 직접 fetch (InnerTube 우회)
    let transcript_list = api
        .list_transcripts(video_id)
        .await
        .map_err(|e| AppError::CaptionFetch(format!("자막 목록 조회 실패: {:?}", e.reason)))?;

    let transcript = transcript_list.find_transcript(&["en"]).map_err(|e| {
        AppError::CaptionFetch(format!("영어 자막을 찾을 수 없습니다: {:?}", e.reason))
    })?;
    let is_auto = transcript.is_generated;

    // 2-a: Transcript::fetch() 시도 (라이브러리 내장 — InnerTube 재요청)
    // cookie_store 활성화: YouTube CONSENT 쿠키를 자동 처리하여 이후 timedtext 요청에서
    // ip=0.0.0.0으로 인한 빈 응답 문제를 줄인다.
    let lib_client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .cookie_store(true)
        .build()
        .unwrap_or_default();

    match transcript.fetch(&lib_client, false).await {
        Ok(fetched) => {
            let lines = normalize_transcript(&fetched.snippets);
            if !lines.is_empty() {
                eprintln!(
                    "[fetch] 2a 성공: {} lines (is_auto={})",
                    lines.len(),
                    is_auto
                );
                return Ok(postprocess_lines(lines, is_auto));
            }
            eprintln!("[fetch] 2a: fetch OK but 0 lines after normalize");
        }
        Err(e) => {
            eprintln!("[fetch] 2a 실패 (Transcript::fetch): {:?}", e.reason);
        }
    }

    // 2-b: Transcript.url로 직접 XML fetch (User-Agent 포함)
    eprintln!("[fetch] 2b: URL = {}", &transcript.url);
    let response = lib_client
        .get(&transcript.url)
        .send()
        .await
        .map_err(|e| AppError::CaptionFetch(format!("자막 URL 요청 실패: {}", e)))?;
    eprintln!("[fetch] 2b: HTTP status = {}", response.status());

    if !response.status().is_success() {
        return Err(AppError::CaptionFetch(format!(
            "자막 URL 응답 오류: HTTP {}",
            response.status()
        )));
    }

    let xml_text = response
        .text()
        .await
        .map_err(|e| AppError::CaptionFetch(format!("자막 XML 읽기 실패: {}", e)))?;

    eprintln!("[fetch] 2b: XML length = {}", xml_text.len());
    if !xml_text.is_empty() {
        let parser = TranscriptParser::new(false);
        if let Ok(snippets) = parser.parse(&xml_text) {
            let lines = normalize_transcript(&snippets);
            if !lines.is_empty() {
                eprintln!(
                    "[fetch] 2b 성공: {} lines (is_auto={})",
                    lines.len(),
                    is_auto
                );
                return Ok(postprocess_lines(lines, is_auto));
            }
        }
    }

    // 3차: ANDROID InnerTube 클라이언트로 직접 요청
    // yt-transcript-rs가 사용하는 WEB 클라이언트(2023.12 버전)에서 캡션을 반환하지
    // 않는 경우, ANDROID 클라이언트는 캡션을 반환하는 것으로 알려져 있음 (yt-dlp 방식)
    eprintln!("[fetch] 3차: ANDROID InnerTube 직접 요청 시도");
    match fetch_via_android_innertube(video_id, &lib_client).await {
        Ok((lines, is_auto)) => {
            eprintln!(
                "[fetch] 3차 성공: {} lines (is_auto={})",
                lines.len(),
                is_auto
            );
            return Ok(postprocess_lines(lines, is_auto));
        }
        Err(e) => {
            eprintln!("[fetch] 3차 실패: {}", e);
        }
    }

    Err(AppError::CaptionFetch(
        "모든 자막 fetch 방법이 실패했습니다 (InnerTube WEB/ANDROID, URL 직접 fetch)".into(),
    ))
}

/// 자동 자막이면 문장 경계에서 먼저 쪼개고 시간 단위로 다시 병합한다.
/// 수동 자막은 이미 문장 단위로 정돈되어 있어 그대로 반환.
///
/// 순서:
/// 1. `split_lines_on_sentence_boundaries`: snippet 내부 구두점으로 세분화
/// 2. `merge_into_sentences`: 종결 단위/시간 한도로 재병합
///
/// 결과적으로 allang.ai 수준의 문장 단위 자막 블록이 만들어진다.
fn postprocess_lines(lines: Vec<SubtitleLine>, is_auto: bool) -> Vec<SubtitleLine> {
    if is_auto {
        let split = split_lines_on_sentence_boundaries(lines);
        merge_into_sentences(split)
    } else {
        lines
    }
}

/// ANDROID InnerTube 클라이언트로 캡션 URL을 가져와 자막을 fetch
///
/// WEB 클라이언트가 캡션을 미반환할 때 ANDROID 클라이언트는 반환하는 경우가 있다.
/// yt-dlp가 사용하는 것과 동일한 접근 방식.
///
/// 반환: `(lines, is_auto_generated)` — 호출측에서 자동 자막일 때만 문장 병합을 적용.
async fn fetch_via_android_innertube(
    video_id: &str,
    client: &reqwest::Client,
) -> Result<(Vec<SubtitleLine>, bool), AppError> {
    // 클라이언트 메타데이터는 파일 상단 const 블록에서 관리 (업데이트 시 그쪽만 수정).
    let body = serde_json::json!({
        "context": {
            "client": {
                "clientName": ANDROID_CLIENT_NAME,
                "clientVersion": ANDROID_CLIENT_VERSION,
                "androidSdkVersion": ANDROID_SDK_VERSION,
                "osName": ANDROID_OS_NAME,
                "osVersion": ANDROID_OS_VERSION,
                "hl": "en",
                "gl": "US"
            }
        },
        "videoId": video_id
    });

    let resp = client
        .post("https://www.youtube.com/youtubei/v1/player?prettyPrint=false")
        .header("Content-Type", "application/json")
        .header("User-Agent", ANDROID_USER_AGENT)
        .header("X-YouTube-Client-Name", ANDROID_CLIENT_NAME_ID)
        .header("X-YouTube-Client-Version", ANDROID_CLIENT_VERSION)
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::CaptionFetch(format!("ANDROID InnerTube 요청 실패: {}", e)))?;

    if !resp.status().is_success() {
        return Err(AppError::CaptionFetch(format!(
            "ANDROID InnerTube 응답 오류: HTTP {}",
            resp.status()
        )));
    }

    let player: InnerTubePlayerResponse = resp
        .json()
        .await
        .map_err(|e| AppError::CaptionFetch(format!("InnerTube 응답 파싱 실패: {}", e)))?;

    let tracks = player
        .captions
        .and_then(|c| c.renderer)
        .map(|r| r.caption_tracks)
        .unwrap_or_default();

    if tracks.is_empty() {
        return Err(AppError::CaptionFetch(
            "ANDROID InnerTube: 캡션 트랙 없음".into(),
        ));
    }

    // 영어 트랙 선택 — manual 우선, 없으면 auto, 그래도 없으면 첫 트랙.
    // `kind == "asr"`는 YouTube가 자동 생성한 자막을 표시 (speech recognition).
    let is_english =
        |t: &&CaptionTrack| t.language_code == "en" || t.language_code.starts_with("en-");
    let is_manual = |t: &&CaptionTrack| t.kind.as_deref() != Some("asr");

    let en_track = tracks
        .iter()
        .find(|t| is_english(t) && is_manual(t))
        .or_else(|| tracks.iter().find(is_english))
        .or_else(|| tracks.first())
        .ok_or_else(|| AppError::CaptionFetch("영어 캡션 트랙을 찾을 수 없습니다".into()))?;
    let is_auto = en_track.kind.as_deref() == Some("asr");

    // ANDROID 클라이언트가 반환한 baseUrl은 `fmt=srv3`로 끝나는 경우가 많은데,
    // 이 포맷은 빈 응답을 내는 경우가 있다. `fmt` 파라미터를 제거하면
    // YouTube가 기본 XML(srv1 계열)을 반환하여 안정적으로 파싱 가능.
    let caption_url = strip_fmt_param(&en_track.base_url);
    eprintln!("[fetch] 3차: caption URL = {}", &caption_url);

    // 캡션 XML fetch
    let xml_resp = client
        .get(&caption_url)
        .send()
        .await
        .map_err(|e| AppError::CaptionFetch(format!("캡션 XML 요청 실패: {}", e)))?;

    let xml_text = xml_resp
        .text()
        .await
        .map_err(|e| AppError::CaptionFetch(format!("캡션 XML 읽기 실패: {}", e)))?;

    if xml_text.is_empty() {
        return Err(AppError::CaptionFetch(
            "ANDROID InnerTube: 캡션 XML이 비어 있습니다".into(),
        ));
    }

    let parser = TranscriptParser::new(false);
    let snippets = parser
        .parse(&xml_text)
        .map_err(|e| AppError::CaptionFetch(format!("캡션 XML 파싱 실패: {}", e)))?;

    let lines = normalize_transcript(&snippets);
    if lines.is_empty() {
        return Err(AppError::CaptionFetch(
            "ANDROID InnerTube: 자막이 비어 있습니다".into(),
        ));
    }

    Ok((lines, is_auto))
}

/// timedtext URL에서 `fmt=xxx` 쿼리 파라미터를 제거한다.
/// ANDROID 클라이언트가 반환하는 URL에 붙는 `fmt=srv3`가 빈 응답을 유발하는
/// 케이스가 있어 기본 포맷으로 요청하기 위함.
fn strip_fmt_param(url: &str) -> String {
    // Fast path: fmt= 파라미터가 없으면 Vec 할당 없이 원본 그대로 반환.
    // YouTube timedtext URL은 항상 소문자 `fmt=` 를 쓰므로 대소문자 구분 불필요.
    if !url.contains("fmt=") {
        return url.to_string();
    }
    let Some(q_start) = url.find('?') else {
        return url.to_string();
    };
    let (base, query) = url.split_at(q_start + 1);
    let filtered: Vec<&str> = query
        .split('&')
        .filter(|p| !p.starts_with("fmt="))
        .collect();
    if filtered.is_empty() {
        // 쿼리가 fmt= 하나뿐이라 전부 제거된 경우, 물음표도 떼어낸다.
        base.trim_end_matches('?').to_string()
    } else {
        format!("{}{}", base, filtered.join("&"))
    }
}

/// YouTube 영상의 메타데이터(제목, 설명)를 fetch
pub async fn fetch_video_info(video_id: &str) -> Result<VideoInfo, AppError> {
    let api = YouTubeTranscriptApi::new(None, None, None)?;

    let details = api
        .fetch_video_details(video_id)
        .await
        .map_err(AppError::from)?;

    Ok(VideoInfo {
        title: details.title,
        description: details.short_description,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_fmt_removes_srv3_at_end() {
        let url = "https://example.com/api?v=abc&lang=en&fmt=srv3";
        assert_eq!(
            strip_fmt_param(url),
            "https://example.com/api?v=abc&lang=en"
        );
    }

    #[test]
    fn test_strip_fmt_removes_in_middle() {
        let url = "https://example.com/api?v=abc&fmt=srv3&lang=en";
        assert_eq!(
            strip_fmt_param(url),
            "https://example.com/api?v=abc&lang=en"
        );
    }

    #[test]
    fn test_strip_fmt_no_query() {
        let url = "https://example.com/api";
        assert_eq!(strip_fmt_param(url), url);
    }

    #[test]
    fn test_strip_fmt_only_fmt_param() {
        let url = "https://example.com/api?fmt=srv3";
        assert_eq!(strip_fmt_param(url), "https://example.com/api");
    }

    #[test]
    fn test_strip_fmt_no_fmt_param() {
        let url = "https://example.com/api?v=abc&lang=en";
        assert_eq!(strip_fmt_param(url), url);
    }

    /// 사용자 리포트 영상(`4nVoLX2taFg`) 자막 fetch 통합 테스트.
    ///
    /// WEB InnerTube 클라이언트에서 캡션을 반환하지 않는 영상으로, ANDROID 폴백이
    /// 동작해야 성공한다. 네트워크 필요: `cargo test --lib -- --ignored`로 실행.
    #[tokio::test]
    #[ignore]
    async fn test_fetch_subtitles_reported_video() {
        let result = fetch_subtitles("4nVoLX2taFg").await;
        assert!(
            result.is_ok(),
            "fetch_subtitles 실패 — 영상 4nVoLX2taFg: {:?}",
            result.err()
        );
        let lines = result.unwrap();
        assert!(!lines.is_empty(), "자막 라인이 비어 있으면 안 됨");
        for line in &lines {
            assert!(!line.text.is_empty(), "자막 텍스트가 비어 있으면 안 됨");
            assert!(line.start >= 0.0, "start 시간은 0 이상이어야 함");
            assert!(line.end > line.start, "end 시간은 start 시간보다 커야 함");
        }
        eprintln!("=== 4nVoLX2taFg: {} 라인 fetch 성공 ===", lines.len());
        eprintln!("--- 처음 5개 자막 ---");
        for line in lines.iter().take(5) {
            eprintln!(
                "  [{:>7.2}s → {:>7.2}s] {}",
                line.start, line.end, line.text
            );
        }
        eprintln!("--- 마지막 3개 자막 ---");
        for line in lines.iter().rev().take(3).collect::<Vec<_>>().iter().rev() {
            eprintln!(
                "  [{:>7.2}s → {:>7.2}s] {}",
                line.start, line.end, line.text
            );
        }
    }

    /// 잘 알려진 안정적인 영상(Rick Astley) 자막 fetch 테스트.
    #[tokio::test]
    #[ignore]
    async fn test_fetch_subtitles_known_good_video() {
        let result = fetch_subtitles("dQw4w9WgXcQ").await;
        assert!(
            result.is_ok(),
            "fetch_subtitles 실패 — dQw4w9WgXcQ: {:?}",
            result.err()
        );
        let lines = result.unwrap();
        assert!(!lines.is_empty(), "자막 라인이 비어 있으면 안 됨");
    }
}
