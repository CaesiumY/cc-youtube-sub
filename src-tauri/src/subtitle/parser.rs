use yt_transcript_rs::FetchedTranscriptSnippet;

use crate::subtitle::SubtitleLine;

/// HTML entity를 디코딩하고 빈 라인을 필터링하여 SubtitleLine으로 변환
pub fn normalize_transcript(snippets: &[FetchedTranscriptSnippet]) -> Vec<SubtitleLine> {
    snippets
        .iter()
        .filter_map(|s| {
            let text = decode_html_entities(&s.text);
            let trimmed = text.trim();
            if trimmed.is_empty() || trimmed == "[Music]" || trimmed == "[Applause]" {
                return None;
            }
            Some(SubtitleLine {
                text: trimmed.to_string(),
                start: s.start,
                end: s.start + s.duration,
            })
        })
        .collect()
}

/// 기본 HTML entity 디코딩
/// yt-transcript-rs가 처리하지 않는 HTML entity를 수동으로 변환
fn decode_html_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&#x27;", "'")
        .replace("&#x2F;", "/")
        .replace("&nbsp;", " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snippet(text: &str, start: f64, duration: f64) -> FetchedTranscriptSnippet {
        FetchedTranscriptSnippet {
            text: text.to_string(),
            start,
            duration,
        }
    }

    #[test]
    fn test_html_entity_decoding() {
        assert_eq!(decode_html_entities("Tom &amp; Jerry"), "Tom & Jerry");
        assert_eq!(
            decode_html_entities("&lt;b&gt;bold&lt;/b&gt;"),
            "<b>bold</b>"
        );
        assert_eq!(
            decode_html_entities("he said &quot;hello&quot;"),
            "he said \"hello\""
        );
        assert_eq!(decode_html_entities("it&#39;s fine"), "it's fine");
        assert_eq!(decode_html_entities("a &amp; b &lt; c"), "a & b < c");
    }

    #[test]
    fn test_normalize_basic() {
        let snippets = vec![
            snippet("Hello world", 0.0, 2.5),
            snippet("How are you", 2.5, 3.0),
        ];
        let lines = normalize_transcript(&snippets);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "Hello world");
        assert_eq!(lines[0].start, 0.0);
        assert_eq!(lines[0].end, 2.5);
        assert_eq!(lines[1].text, "How are you");
        assert_eq!(lines[1].end, 5.5);
    }

    #[test]
    fn test_normalize_filters_empty() {
        let snippets = vec![
            snippet("Hello", 0.0, 1.0),
            snippet("", 1.0, 1.0),
            snippet("  ", 2.0, 1.0),
            snippet("World", 3.0, 1.0),
        ];
        let lines = normalize_transcript(&snippets);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "Hello");
        assert_eq!(lines[1].text, "World");
    }

    #[test]
    fn test_normalize_filters_music_tags() {
        let snippets = vec![
            snippet("[Music]", 0.0, 5.0),
            snippet("Actual text", 5.0, 2.0),
            snippet("[Applause]", 7.0, 3.0),
        ];
        let lines = normalize_transcript(&snippets);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "Actual text");
    }

    #[test]
    fn test_normalize_decodes_entities() {
        let snippets = vec![
            snippet("Tom &amp; Jerry", 0.0, 2.0),
            snippet("it&#39;s great", 2.0, 1.5),
        ];
        let lines = normalize_transcript(&snippets);
        assert_eq!(lines[0].text, "Tom & Jerry");
        assert_eq!(lines[1].text, "it's great");
    }

    #[test]
    fn test_normalize_trims_whitespace() {
        let snippets = vec![snippet("  hello world  ", 0.0, 1.0)];
        let lines = normalize_transcript(&snippets);
        assert_eq!(lines[0].text, "hello world");
    }

    #[test]
    fn test_empty_input() {
        let lines = normalize_transcript(&[]);
        assert!(lines.is_empty());
    }
}
