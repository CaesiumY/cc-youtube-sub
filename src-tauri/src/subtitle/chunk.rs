use crate::subtitle::{SubtitleChunk, SubtitleLine};

const MAX_CHUNK_SECS: f64 = 75.0;
const MAX_LINES_PER_CHUNK: usize = 30;
const MIN_CHUNK_SECS: f64 = 25.0;
const MIN_LINES_PER_CHUNK: usize = 8;

/// 자막 라인들을 시간 기반 청크로 분할
///
/// 규칙:
/// - 최소 25초/8줄 이상이며 문장 종결 부호(. ! ? 등)로 끝나면 분할
/// - 최소 조건 미충족이면 분할 연기 (파편화 방지)
/// - 최대 75초/30줄 도달 시 강제 분할
pub fn split_into_chunks(lines: &[SubtitleLine]) -> Vec<SubtitleChunk> {
    if lines.is_empty() {
        return Vec::new();
    }

    let mut chunks = Vec::new();
    let mut chunk_lines: Vec<SubtitleLine> = Vec::new();
    let mut chunk_start = lines[0].start;
    let mut chunk_index = 0i32;

    for line in lines {
        let elapsed = line.end - chunk_start;

        let reached_max = chunk_lines.len() >= MAX_LINES_PER_CHUNK || elapsed >= MAX_CHUNK_SECS;
        let reached_min = chunk_lines.len() >= MIN_LINES_PER_CHUNK && elapsed >= MIN_CHUNK_SECS;
        let ends_sentence = chunk_lines
            .last()
            .map(|l| ends_with_sentence(&l.text))
            .unwrap_or(false);

        let should_close =
            !chunk_lines.is_empty() && (reached_max || (reached_min && ends_sentence));

        if should_close {
            let end_time = chunk_lines.last().unwrap().end;
            chunks.push(SubtitleChunk {
                index: chunk_index,
                start_time: chunk_start,
                end_time,
                lines: std::mem::take(&mut chunk_lines),
            });
            chunk_index += 1;
            chunk_start = line.start;
        }

        chunk_lines.push(line.clone());
    }

    // 남은 라인으로 마지막 청크 생성
    if !chunk_lines.is_empty() {
        let end_time = chunk_lines.last().unwrap().end;
        chunks.push(SubtitleChunk {
            index: chunk_index,
            start_time: chunk_start,
            end_time,
            lines: chunk_lines,
        });
    }

    chunks
}

fn ends_with_sentence(text: &str) -> bool {
    let trimmed = text.trim_end();
    matches!(
        trimmed.chars().last(),
        Some('.') | Some('!') | Some('?') | Some('。') | Some('！') | Some('？')
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(text: &str, start: f64, end: f64) -> SubtitleLine {
        SubtitleLine {
            text: text.to_string(),
            start,
            end,
        }
    }

    #[test]
    fn test_empty_input() {
        assert!(split_into_chunks(&[]).is_empty());
    }

    #[test]
    fn test_single_line() {
        let lines = vec![line("hello.", 0.0, 2.0)];
        let chunks = split_into_chunks(&lines);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].index, 0);
        assert_eq!(chunks[0].lines.len(), 1);
    }

    #[test]
    fn test_short_video_single_chunk() {
        // 20초짜리 영상 — MIN_CHUNK_SECS(25) 미달이므로 단일 청크
        let lines: Vec<SubtitleLine> = (0..10)
            .map(|i| line(&format!("line {i}."), i as f64 * 2.0, (i + 1) as f64 * 2.0))
            .collect();
        let chunks = split_into_chunks(&lines);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].lines.len(), 10);
    }

    #[test]
    fn test_sentence_boundary_splits_after_min() {
        // 8줄 * 4초 = 32초 시점에 문장 종결 → 분할, 이후 라인은 새 청크
        let mut lines = Vec::new();
        for i in 0..8 {
            // 라인 7(마지막)만 종결 부호 포함
            let text = if i == 7 {
                format!("end{i}.")
            } else {
                format!("mid{i}")
            };
            lines.push(line(&text, i as f64 * 4.0, (i + 1) as f64 * 4.0));
        }
        for i in 0..5 {
            let base = 32.0;
            lines.push(line(
                &format!("b{i}"),
                base + i as f64 * 4.0,
                base + (i + 1) as f64 * 4.0,
            ));
        }
        let chunks = split_into_chunks(&lines);
        assert!(
            chunks.len() >= 2,
            "expected split at sentence boundary, got {} chunks",
            chunks.len()
        );
        assert_eq!(chunks[0].lines.len(), 8);
    }

    #[test]
    fn test_no_sentence_continues_until_max() {
        // 문장 종결 부호 없고 30줄 미만 → 계속 누적 (MIN 충족해도 분할 안 함)
        let lines: Vec<SubtitleLine> = (0..20)
            .map(|i| line(&format!("l{i}"), i as f64 * 2.0, (i + 1) as f64 * 2.0))
            .collect();
        let chunks = split_into_chunks(&lines);
        // 20줄 * 2초 = 40초. 문장 종결 없음, 30줄 미만 → 단일 청크
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].lines.len(), 20);
    }

    #[test]
    fn test_max_lines_triggers_split() {
        // 31줄 — MAX_LINES_PER_CHUNK(30) 도달 시 강제 분할
        let lines: Vec<SubtitleLine> = (0..31)
            .map(|i| line(&format!("l{i}"), i as f64 * 1.5, (i + 1) as f64 * 1.5))
            .collect();
        let chunks = split_into_chunks(&lines);
        assert!(chunks.len() >= 2);
        assert!(chunks[0].lines.len() <= MAX_LINES_PER_CHUNK);
    }

    #[test]
    fn test_max_secs_triggers_split() {
        // 문장 종결 없이 75초 초과 → 강제 분할
        let lines: Vec<SubtitleLine> = (0..25)
            .map(|i| line(&format!("l{i}"), i as f64 * 4.0, (i + 1) as f64 * 4.0))
            .collect();
        let chunks = split_into_chunks(&lines);
        // 25줄 * 4초 = 100초, 75초 도달 시 분할
        assert!(chunks.len() >= 2);
    }

    #[test]
    fn test_chunk_boundaries() {
        let lines: Vec<SubtitleLine> = (0..60)
            .map(|i| line(&format!("l{i}"), i as f64, (i + 1) as f64))
            .collect();
        let chunks = split_into_chunks(&lines);

        for chunk in &chunks {
            assert!(chunk.start_time <= chunk.end_time);
            assert!(!chunk.lines.is_empty());
            assert!(chunk.lines.len() <= MAX_LINES_PER_CHUNK);
        }

        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.index, i as i32);
        }
    }

    #[test]
    fn test_no_gap_between_chunks() {
        let lines: Vec<SubtitleLine> = (0..120)
            .map(|i| line(&format!("l{i}"), i as f64 * 0.5, (i + 1) as f64 * 0.5))
            .collect();
        let chunks = split_into_chunks(&lines);

        let total_lines: usize = chunks.iter().map(|c| c.lines.len()).sum();
        assert_eq!(total_lines, 120);
    }

    #[test]
    fn test_ends_with_sentence_variants() {
        assert!(ends_with_sentence("hello."));
        assert!(ends_with_sentence("hello!"));
        assert!(ends_with_sentence("hello? "));
        assert!(ends_with_sentence("안녕。"));
        assert!(ends_with_sentence("안녕！"));
        assert!(ends_with_sentence("안녕？"));
        assert!(!ends_with_sentence("hello"));
        assert!(!ends_with_sentence("hello,"));
    }
}
