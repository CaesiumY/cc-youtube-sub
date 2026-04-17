use crate::subtitle::{SubtitleChunk, SubtitleLine};
use crate::translate::VideoInfo;

/// 번역 프롬프트를 구성
///
/// 두 가지 모드:
/// - `is_session_continuation == false` (세션 생성): 시스템 지시 + 영상 설명 + 현재 청크 + 지시
/// - `is_session_continuation == true` (세션 이어감): Claude CLI 세션이 시스템 지시와
///   영상 설명을 이미 기억하므로 현재 청크와 간결한 지시만 전송
pub fn build_prompt(
    chunk: &SubtitleChunk,
    video_info: Option<&VideoInfo>,
    previous_context: Option<&[SubtitleLine]>,
    is_session_continuation: bool,
) -> String {
    let mut parts = Vec::new();

    if !is_session_continuation {
        parts.push(
            "You are a professional subtitle translator. \
             Translate the following English subtitles into natural, fluent Korean. \
             ALWAYS use Korean formal polite speech (합쇼체) — every sentence must end with \
             endings like ~습니다, ~니다, ~입니다, ~죠, or polite question forms like ~습니까, ~나요. \
             Never use plain form (~다, ~이다, ~해, ~한다) or casual speech (~야, ~어, ~지). \
             Preserve the original meaning and nuance, but always keep the polite register \
             regardless of the speaker's tone in the source. \
             Keep technical terms in English when commonly used in Korean context."
                .to_string(),
        );

        if let Some(info) = video_info {
            parts.push(format!(
                "[VIDEO_DESCRIPTION]\nTitle: {}\nDescription: {}",
                info.title, info.description
            ));
        }

        if let Some(context) = previous_context {
            if !context.is_empty() {
                let context_text: String = context
                    .iter()
                    .map(|l| format!("[{:.1}s] {}", l.start, l.text))
                    .collect::<Vec<_>>()
                    .join("\n");
                parts.push(format!(
                    "[CONTEXT_FROM_PREVIOUS_CHUNK]\n\
                     The following are the last few lines from the previous chunk for context:\n{}",
                    context_text
                ));
            }
        }
    } else {
        parts.push(
            "Continue translating the next chunk. \
             Maintain consistency with previous chunks' terminology. \
             Keep using Korean formal polite speech (합쇼체, ~습니다/~니다)."
                .to_string(),
        );
    }

    let subtitle_text: String = chunk
        .lines
        .iter()
        .map(|l| format!("[{:.1}-{:.1}s] {}", l.start, l.end, l.text))
        .collect::<Vec<_>>()
        .join("\n");
    parts.push(format!(
        "[CURRENT_CHUNK_SUBTITLES]\n\
         Chunk {} ({}s - {}s):\n{}",
        chunk.index,
        format_time(chunk.start_time),
        format_time(chunk.end_time),
        subtitle_text
    ));

    if is_session_continuation {
        parts.push(
            "[TRANSLATION_INSTRUCTION]\n\
             Respond ONLY with a JSON array of objects with fields: \
             \"original\", \"translated\", \"start\" (number), \"end\" (number)."
                .to_string(),
        );
    } else {
        parts.push(
            "[TRANSLATION_INSTRUCTION]\n\
             Translate each subtitle line and respond with a JSON array. \
             Each element must have these exact fields:\n\
             - \"original\": the original English text\n\
             - \"translated\": the Korean translation\n\
             - \"start\": start time in seconds (number)\n\
             - \"end\": end time in seconds (number)\n\n\
             Respond ONLY with the JSON array, no other text."
                .to_string(),
        );
    }

    parts.join("\n\n")
}

fn format_time(seconds: f64) -> String {
    let mins = (seconds / 60.0) as u32;
    let secs = seconds % 60.0;
    if mins > 0 {
        format!("{mins}:{secs:04.1}")
    } else {
        format!("{secs:.1}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subtitle::{SubtitleChunk, SubtitleLine};

    fn make_line(text: &str, start: f64, end: f64) -> SubtitleLine {
        SubtitleLine {
            text: text.to_string(),
            start,
            end,
        }
    }

    fn make_chunk(index: i32, lines: Vec<SubtitleLine>) -> SubtitleChunk {
        let start_time = lines.first().map(|l| l.start).unwrap_or(0.0);
        let end_time = lines.last().map(|l| l.end).unwrap_or(0.0);
        SubtitleChunk {
            index,
            start_time,
            end_time,
            lines,
        }
    }

    #[test]
    fn test_first_chunk_with_video_info() {
        let chunk = make_chunk(0, vec![make_line("Hello world", 0.0, 2.5)]);
        let info = VideoInfo {
            title: "Test Video".into(),
            description: "A test".into(),
        };
        let prompt = build_prompt(&chunk, Some(&info), None, false);

        assert!(prompt.contains("[VIDEO_DESCRIPTION]"));
        assert!(prompt.contains("Test Video"));
        assert!(prompt.contains("A test"));
        assert!(!prompt.contains("[CONTEXT_FROM_PREVIOUS_CHUNK]"));
        assert!(prompt.contains("[CURRENT_CHUNK_SUBTITLES]"));
        assert!(prompt.contains("[TRANSLATION_INSTRUCTION]"));
        assert!(prompt.contains("Hello world"));
    }

    #[test]
    fn test_subsequent_chunk_with_context() {
        let chunk = make_chunk(1, vec![make_line("New content", 35.0, 37.0)]);
        let context = vec![
            make_line("Previous line 1", 28.0, 30.0),
            make_line("Previous line 2", 30.0, 33.0),
        ];
        let prompt = build_prompt(&chunk, None, Some(&context), false);

        assert!(!prompt.contains("[VIDEO_DESCRIPTION]"));
        assert!(prompt.contains("[CONTEXT_FROM_PREVIOUS_CHUNK]"));
        assert!(prompt.contains("Previous line 1"));
        assert!(prompt.contains("Previous line 2"));
        assert!(prompt.contains("Chunk 1"));
    }

    #[test]
    fn test_session_continuation_omits_system_instruction() {
        let chunk = make_chunk(3, vec![make_line("Another line", 60.0, 62.0)]);
        let info = VideoInfo {
            title: "T".into(),
            description: "D".into(),
        };
        let prompt = build_prompt(&chunk, Some(&info), None, true);

        assert!(!prompt.contains("professional subtitle translator"));
        assert!(!prompt.contains("[VIDEO_DESCRIPTION]"));
        assert!(!prompt.contains("[CONTEXT_FROM_PREVIOUS_CHUNK]"));
        assert!(prompt.contains("Continue translating"));
        assert!(prompt.contains("[CURRENT_CHUNK_SUBTITLES]"));
        assert!(prompt.contains("JSON array"));
    }

    #[test]
    fn test_prompt_always_has_instructions() {
        let chunk = make_chunk(0, vec![make_line("test", 0.0, 1.0)]);
        let prompt = build_prompt(&chunk, None, None, false);

        assert!(prompt.contains("professional subtitle translator"));
        assert!(prompt.contains("JSON array"));
        assert!(prompt.contains("\"original\""));
        assert!(prompt.contains("\"translated\""));
    }

    #[test]
    fn test_time_format_in_subtitles() {
        let chunk = make_chunk(0, vec![make_line("test", 65.0, 67.5)]);
        let prompt = build_prompt(&chunk, None, None, false);
        assert!(prompt.contains("[65.0-67.5s]"));
    }

    #[test]
    fn test_format_time() {
        assert_eq!(format_time(0.0), "0.0");
        assert_eq!(format_time(30.5), "30.5");
        assert_eq!(format_time(65.0), "1:05.0");
        assert_eq!(format_time(125.3), "2:05.3");
    }
}
