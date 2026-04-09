use yt_transcript_rs::transcript_parser::TranscriptParser;
use yt_transcript_rs::YouTubeTranscriptApi;

use crate::error::AppError;
use crate::subtitle::SubtitleLine;
use crate::translate::VideoInfo;

use super::parser::normalize_transcript;

/// YouTube 영상의 영어 자막을 fetch하여 정규화된 SubtitleLine 목록으로 반환
///
/// `yt-transcript-rs`의 `Transcript::fetch()`가 InnerTube 재요청에서
/// 실패하는 경우가 있어, `list_transcripts`로 URL을 얻은 뒤
/// 직접 HTTP GET으로 자막 XML을 가져온다.
pub async fn fetch_subtitles(video_id: &str) -> Result<Vec<SubtitleLine>, AppError> {
    let api = YouTubeTranscriptApi::new(None, None, None)?;

    // 1차: 직접 fetch 시도 (InnerTube가 정상 동작하는 경우 빠름)
    if let Ok(transcript) = api.fetch_transcript(video_id, &["en"], false).await {
        let lines = normalize_transcript(&transcript.snippets);
        if !lines.is_empty() {
            return Ok(lines);
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

    // Transcript.url로 직접 XML fetch
    let client = reqwest::Client::new();
    let response = client
        .get(&transcript.url)
        .send()
        .await
        .map_err(|e| AppError::CaptionFetch(format!("자막 URL 요청 실패: {}", e)))?;

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

    if xml_text.is_empty() {
        return Err(AppError::CaptionFetch("자막 XML이 비어 있습니다".into()));
    }

    // XML → FetchedTranscriptSnippet 파싱
    let parser = TranscriptParser::new(false);
    let snippets = parser
        .parse(&xml_text)
        .map_err(|e| AppError::CaptionFetch(format!("자막 XML 파싱 실패: {}", e)))?;

    let lines = normalize_transcript(&snippets);

    if lines.is_empty() {
        return Err(AppError::CaptionFetch("자막이 비어 있습니다".into()));
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
