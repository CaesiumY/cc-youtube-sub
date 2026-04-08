use yt_transcript_rs::YouTubeTranscriptApi;

use crate::error::AppError;
use crate::subtitle::SubtitleLine;
use crate::translate::VideoInfo;

use super::parser::normalize_transcript;

/// YouTube 영상의 영어 자막을 fetch하여 정규화된 SubtitleLine 목록으로 반환
pub async fn fetch_subtitles(video_id: &str) -> Result<Vec<SubtitleLine>, AppError> {
    let api = YouTubeTranscriptApi::new(None, None, None)?;

    let transcript = api
        .fetch_transcript(video_id, &["en"], false)
        .await?;

    let lines = normalize_transcript(&transcript.snippets);

    if lines.is_empty() {
        return Err(AppError::CaptionFetch(
            "자막이 비어 있습니다".into(),
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
