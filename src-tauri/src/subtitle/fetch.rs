use serde::Deserialize;
use yt_transcript_rs::transcript_parser::TranscriptParser;
use yt_transcript_rs::YouTubeTranscriptApi;

use crate::error::AppError;
use crate::subtitle::SubtitleLine;
use crate::translate::VideoInfo;

use super::parser::normalize_transcript;

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

    // 1차: 직접 fetch 시도 (InnerTube가 정상 동작하는 경우 빠름)
    match api.fetch_transcript(video_id, &["en"], false).await {
        Ok(transcript) => {
            let lines = normalize_transcript(&transcript.snippets);
            if !lines.is_empty() {
                eprintln!("[fetch] 1차 성공: {} lines", lines.len());
                return Ok(lines);
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

    // 2-a: Transcript::fetch() 시도 (라이브러리 내장 — InnerTube 재요청)
    let lib_client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .unwrap_or_default();

    match transcript.fetch(&lib_client, false).await {
        Ok(fetched) => {
            let lines = normalize_transcript(&fetched.snippets);
            if !lines.is_empty() {
                eprintln!("[fetch] 2a 성공: {} lines", lines.len());
                return Ok(lines);
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
                eprintln!("[fetch] 2b 성공: {} lines", lines.len());
                return Ok(lines);
            }
        }
    }

    // 3차: ANDROID InnerTube 클라이언트로 직접 요청
    // yt-transcript-rs가 사용하는 WEB 클라이언트(2023.12 버전)에서 캡션을 반환하지
    // 않는 경우, ANDROID 클라이언트는 캡션을 반환하는 것으로 알려져 있음 (yt-dlp 방식)
    eprintln!("[fetch] 3차: ANDROID InnerTube 직접 요청 시도");
    match fetch_via_android_innertube(video_id, &lib_client).await {
        Ok(lines) => {
            eprintln!("[fetch] 3차 성공: {} lines", lines.len());
            return Ok(lines);
        }
        Err(e) => {
            eprintln!("[fetch] 3차 실패: {}", e);
        }
    }

    Err(AppError::CaptionFetch(
        "모든 자막 fetch 방법이 실패했습니다 (InnerTube WEB/ANDROID, URL 직접 fetch)".into(),
    ))
}

/// ANDROID InnerTube 클라이언트로 캡션 URL을 가져와 자막을 fetch
///
/// WEB 클라이언트가 캡션을 미반환할 때 ANDROID 클라이언트는 반환하는 경우가 있다.
/// yt-dlp가 사용하는 것과 동일한 접근 방식.
async fn fetch_via_android_innertube(
    video_id: &str,
    client: &reqwest::Client,
) -> Result<Vec<SubtitleLine>, AppError> {
    let body = serde_json::json!({
        "context": {
            "client": {
                "clientName": "ANDROID",
                "clientVersion": "19.09.37",
                "androidSdkVersion": 30,
                "hl": "en",
                "gl": "US"
            }
        },
        "videoId": video_id
    });

    let resp = client
        .post("https://www.youtube.com/youtubei/v1/player?prettyPrint=false")
        .header("Content-Type", "application/json")
        .header("X-YouTube-Client-Name", "3") // ANDROID = 3
        .header("X-YouTube-Client-Version", "19.09.37")
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

    // 영어 트랙 찾기
    let en_track = tracks
        .iter()
        .find(|t| t.language_code == "en" || t.language_code.starts_with("en-"))
        .or_else(|| tracks.first())
        .ok_or_else(|| AppError::CaptionFetch("영어 캡션 트랙을 찾을 수 없습니다".into()))?;

    eprintln!("[fetch] 3차: caption URL = {}", &en_track.base_url);

    // 캡션 XML fetch
    let xml_resp = client
        .get(&en_track.base_url)
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

    Ok(lines)
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
