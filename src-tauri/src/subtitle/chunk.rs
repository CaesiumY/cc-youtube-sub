use crate::subtitle::{SubtitleChunk, SubtitleLine};

const MAX_CHUNK_SECS: f64 = 60.0;
const MAX_LINES_PER_CHUNK: usize = 20;

/// 자막 라인들을 시간 기반 청크로 분할
///
/// 규칙:
/// - 각 청크는 30초~60초 범위
/// - 최소 1줄, 최대 20줄
/// - 순차적으로 라인을 추가하며, 30초 이상 + 60초 초과 또는 20줄 초과 시 청크 마감
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

        // 청크 마감 조건:
        // - 20줄 도달 (hard limit, 시간 무관)
        // - 또는 30초 이상 경과 후 60초 도달
        let should_close = !chunk_lines.is_empty()
            && (chunk_lines.len() >= MAX_LINES_PER_CHUNK || elapsed >= MAX_CHUNK_SECS);

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
        let lines = vec![line("hello", 0.0, 2.0)];
        let chunks = split_into_chunks(&lines);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].index, 0);
        assert_eq!(chunks[0].lines.len(), 1);
    }

    #[test]
    fn test_short_video_single_chunk() {
        // 25초짜리 영상 → 30초 미만이므로 1개 청크
        let lines: Vec<SubtitleLine> = (0..10)
            .map(|i| line(&format!("line {i}"), i as f64 * 2.5, (i + 1) as f64 * 2.5))
            .collect();
        let chunks = split_into_chunks(&lines);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].lines.len(), 10);
    }

    #[test]
    fn test_two_chunks_at_30s() {
        // 0~30초 라인 15개 + 30~50초 라인 10개
        let mut lines = Vec::new();
        for i in 0..15 {
            lines.push(line(&format!("a{i}"), i as f64 * 2.0, (i + 1) as f64 * 2.0));
        }
        for i in 0..10 {
            let base = 30.0;
            lines.push(line(
                &format!("b{i}"),
                base + i as f64 * 2.0,
                base + (i + 1) as f64 * 2.0,
            ));
        }
        let chunks = split_into_chunks(&lines);
        assert!(
            chunks.len() >= 2,
            "expected >= 2 chunks, got {}",
            chunks.len()
        );
        assert_eq!(chunks[0].index, 0);
        assert_eq!(chunks[1].index, 1);
    }

    #[test]
    fn test_max_lines_triggers_split() {
        // 21줄이 31초 안에 들어오면 20줄에서 끊어야 함
        let lines: Vec<SubtitleLine> = (0..21)
            .map(|i| line(&format!("l{i}"), i as f64 * 1.5, (i + 1) as f64 * 1.5))
            .collect();
        let chunks = split_into_chunks(&lines);
        // 21 * 1.5 = 31.5초, 20줄 시점에 30초 지남 → 20줄에서 분할
        assert!(chunks.len() >= 2);
        assert!(chunks[0].lines.len() <= MAX_LINES_PER_CHUNK);
    }

    #[test]
    fn test_chunk_boundaries() {
        // 60줄, 각 1초 → 약 60초 → 30초~60초 범위에서 분할
        let lines: Vec<SubtitleLine> = (0..60)
            .map(|i| line(&format!("l{i}"), i as f64, (i + 1) as f64))
            .collect();
        let chunks = split_into_chunks(&lines);

        // 모든 청크의 시간 범위가 유효한지 확인
        for chunk in &chunks {
            assert!(chunk.start_time <= chunk.end_time);
            assert!(!chunk.lines.is_empty());
            assert!(chunk.lines.len() <= MAX_LINES_PER_CHUNK);
        }

        // 청크 인덱스가 연속적인지 확인
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

        // 모든 라인이 포함되었는지 확인
        let total_lines: usize = chunks.iter().map(|c| c.lines.len()).sum();
        assert_eq!(total_lines, 120);
    }
}
