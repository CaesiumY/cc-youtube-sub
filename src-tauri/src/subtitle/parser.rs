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

/// 자동 자막 라인을 먼저 문장 종결 부호 위치에서 쪼개어 준다.
///
/// YouTube 자동 자막(ASR)은 `<text>` snippet 내부에 구두점이 생기기도 하는데,
/// 한 snippet에 여러 문장이 붙어있는 경우가 많다. 예를 들어 snippet이
/// `"Yesterday, the most ironic thing ever happened. Anthropic, a $380 billion startup..."`
/// 처럼 되어 있으면, `merge_into_sentences`가 이 snippet을 볼 때 마지막이 종결로
/// 끝나지 않으므로 "새 문장이 진행 중"으로 오판한다. 이 함수는 각 라인을 종결
/// 부호 바로 뒤에서 잘라 `["Yesterday, ... happened.", "Anthropic, ..."]` 2개로
/// 분리해 준다. 시간은 문자 수 비례로 분배한다 (근사).
///
/// 수동 자막은 이미 문장 단위이므로 호출하지 않는다.
pub fn split_lines_on_sentence_boundaries(lines: Vec<SubtitleLine>) -> Vec<SubtitleLine> {
    let mut result = Vec::with_capacity(lines.len());
    for line in lines {
        result.extend(split_single_line_on_sentence_boundaries(line));
    }
    result
}

fn split_single_line_on_sentence_boundaries(line: SubtitleLine) -> Vec<SubtitleLine> {
    let text = &line.text;

    // 문장 종결 부호 직후(char 경계)의 바이트 오프셋 수집.
    //
    // `.`은 약어/버전번호에도 쓰이므로 뒤 문맥을 본다:
    //   "Anthropic. Within" → 공백 + 대문자 → 종결 ✓
    //   "4:00 a.m. officially" → 내부 `.`는 `m` 붙어있음, 마지막 `.`은 공백+소문자 → 종결 ✗
    //   "version 2.1.88 of" → 각 `.` 뒤에 숫자 바로 붙음 → 종결 ✗
    //   "Mr. Smith" → 공백 + 대문자 → 안타깝게도 종결로 판정되지만 영어 일반적 문장 시작과 구분 불가
    // 한중일어 전용 부호(`。`,`！`,`？`)와 `!`,`?`,`…`은 항상 종결.
    let mut boundaries: Vec<usize> = Vec::new();
    for (i, c) in text.char_indices() {
        let after = i + c.len_utf8();
        let is_terminator = match c {
            '!' | '?' | '…' | '。' | '！' | '？' => true,
            '.' => is_period_sentence_terminator(text, after),
            _ => false,
        };
        if is_terminator {
            boundaries.push(after);
        }
    }

    // 종결이 없거나, 있어도 문자열 끝에만 있어 쪼갤 것이 없으면 그대로.
    if boundaries.is_empty() {
        return vec![line];
    }

    // 종결점마다 슬라이스를 잘라 trimmed part 수집.
    let mut parts: Vec<String> = Vec::new();
    let mut prev = 0usize;
    for &b in &boundaries {
        if b <= prev {
            continue; // 연속 종결 부호 방어 (".." 같은 경우)
        }
        let piece = text[prev..b].trim();
        if !piece.is_empty() {
            parts.push(piece.to_string());
        }
        prev = b;
    }
    // 마지막 종결 이후 꼬리
    if prev < text.len() {
        let tail = text[prev..].trim();
        if !tail.is_empty() {
            parts.push(tail.to_string());
        }
    }

    if parts.len() <= 1 {
        return vec![line];
    }

    let total_chars: usize = parts.iter().map(|p| p.chars().count()).sum();
    if total_chars == 0 {
        return vec![line];
    }

    // 문자 수 비례 분배 시 매우 짧은 part(`"A."` 2자)가 중간에 끼면 duration이
    // 거의 0으로 수축되어 자막 매칭 시 깜빡 뜨고 사라지는 현상 발생. 이 최소값
    // 아래로는 수축하지 않도록 가드. 너무 크면 긴 꼬리 part가 시간을 잠식하므로
    // 시각적 최소 표시 시간(0.3초) 수준으로 설정.
    const MIN_PART_DURATION: f64 = 0.3;

    let total_duration = line.end - line.start;

    // 원본 duration이 너무 짧아 MIN * parts.len()도 못 채우면 split을 포기하고 원본 유지
    if total_duration < MIN_PART_DURATION * (parts.len() as f64) {
        return vec![line];
    }

    let mut result = Vec::with_capacity(parts.len());
    let mut cursor = line.start;
    let last_idx = parts.len() - 1;

    for (idx, piece) in parts.into_iter().enumerate() {
        let piece_chars = piece.chars().count();
        let end = if idx == last_idx {
            line.end
        } else {
            let proportional = cursor + total_duration * (piece_chars as f64 / total_chars as f64);
            // MIN 가드: 이 part가 너무 짧아지지 않도록, 그리고 이후 part에게
            // 최소 duration을 남겨주도록 조정.
            let min_end_for_this = cursor + MIN_PART_DURATION;
            let max_end_for_this = line.end - MIN_PART_DURATION * ((last_idx - idx) as f64);
            proportional.max(min_end_for_this).min(max_end_for_this)
        };
        result.push(SubtitleLine {
            text: piece,
            start: cursor,
            end,
        });
        cursor = end;
    }
    result
}

/// 자동 생성 자막의 파편화된 라인을 문장 단위로 병합한다.
///
/// YouTube 자동 자막(ASR)은 1-2초 단위 구절로 쪼개져 있고, 구두점은 간헐적으로만
/// 포함된다. 이 함수는 "문장 종결이 나올 때까지 기다리되, hard limit에 도달하면
/// 자연 경계에서 끊는다"는 전략으로 allang.ai 수준의 문장 단위 블록을 생성한다.
///
/// ## 알고리즘
///
/// - 시작 인덱스 `i`부터 앞으로 확장하며 그룹을 만든다.
/// - 확장 중단 조건 (우선순위 순):
///   1. 현재 그룹이 이미 문장 종결로 끝남 → 여기서 flush
///   2. 다음 라인과의 gap이 `GAP_THRESHOLD` 초과 → 장면 전환으로 간주
///   3. 병합 후 문자 수가 `MAX_CHARS` 초과 → 시각적 과부하
///   4. 병합 후 duration이 `HARD_DURATION` 초과 → 무조건 끊기 (매우 긴 문장 안전망)
///   5. `SOFT_DURATION` 초과 + 다음 라인이 문장 종결이 **아님** → 기대 없음, flush
///      (다음 라인이 종결이면 참고 합쳐서 문장을 완성)
///
/// 수동 업로드 자막은 이미 문장 단위로 정돈되어 있으므로 호출하지 않는다.
pub fn merge_into_sentences(lines: Vec<SubtitleLine>) -> Vec<SubtitleLine> {
    const GAP_THRESHOLD: f64 = 1.5;
    const SOFT_DURATION: f64 = 10.0;
    const HARD_DURATION: f64 = 18.0;
    const MAX_CHARS: usize = 280;

    if lines.is_empty() {
        return lines;
    }

    let mut result: Vec<SubtitleLine> = Vec::with_capacity(lines.len());
    let mut i = 0usize;

    while i < lines.len() {
        let start_idx = i;
        let start_time = lines[i].start;
        let mut end_idx = i;
        let mut acc_chars = lines[i].text.chars().count();
        let mut ended_on_terminator = ends_with_sentence_terminator(&lines[i].text);

        while end_idx + 1 < lines.len() {
            let next = &lines[end_idx + 1];
            let cur = &lines[end_idx];

            // 이미 문장 종결로 끝났으면 보통 중단. 단, 현재 라인이 `.`으로 끝나고
            // 다음 라인이 소문자로 시작하면 약어 뒤의 이어지는 문장일 가능성
            // (`"Google Inc."` + `"in Mountain View."` 등)이 있어 한 번 더 확장 허용.
            // `!`/`?`/`…`/CJK 부호(`。`/`！`/`？`)는 명확한 종결이므로 lookahead 하지 않음.
            if ended_on_terminator {
                let ends_with_dot = cur.text.trim_end().ends_with('.');
                if !(ends_with_dot && next_line_starts_lowercase(next)) {
                    break;
                }
            }
            let gap = next.start - cur.end;
            let merged_duration = next.end - start_time;
            let merged_chars = acc_chars + 1 + next.text.chars().count();

            if gap > GAP_THRESHOLD {
                break;
            }
            if merged_chars > MAX_CHARS {
                break;
            }
            if merged_duration > HARD_DURATION {
                break;
            }
            if merged_duration > SOFT_DURATION && !ends_with_sentence_terminator(&next.text) {
                break;
            }

            end_idx += 1;
            acc_chars = merged_chars;
            ended_on_terminator = ends_with_sentence_terminator(&next.text);
        }

        let merged_text = if start_idx == end_idx {
            lines[start_idx].text.clone()
        } else {
            lines[start_idx..=end_idx]
                .iter()
                .map(|l| l.text.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        };

        result.push(SubtitleLine {
            text: merged_text,
            start: lines[start_idx].start,
            end: lines[end_idx].end,
        });

        i = end_idx + 1;
    }

    result
}

/// 다음 라인이 ASCII 소문자로 시작하는지. 약어 continuation 감지용.
/// 예: `"Google Inc."` 다음에 `"in Mountain View."`가 오면 `in`의 `i`가 소문자 →
/// 실제로는 한 문장이 이어지는 것이므로 merge 계속.
fn next_line_starts_lowercase(next: &SubtitleLine) -> bool {
    next.text
        .trim_start()
        .chars()
        .next()
        .map(|c| c.is_lowercase())
        .unwrap_or(false)
}

fn ends_with_sentence_terminator(text: &str) -> bool {
    let trimmed = text.trim_end();
    match trimmed.chars().last() {
        Some('!') | Some('?') | Some('…') | Some('。') | Some('！') | Some('？') => true,
        Some('.') => {
            // 문자열의 끝(trim 후)인 `.`는 종결로 간주 — 단, 직전이 약어/버전번호
            // 형태인 경우는 제외. 가장 실용적인 휴리스틱: `.` 직전이 한 글자 영문(약어)
            // 이거나 숫자(버전)인 경우에도 문장의 끝일 수 있으므로 종결로 봄.
            // 따라서 단순하게 `.`이면 종결 처리 — merge 단계에서 이런 경우에도 다음
            // 라인이 이어지므로 문제가 크지 않다.
            true
        }
        _ => false,
    }
}

/// 자주 쓰이는 영문 약어 목록. 소문자 기준.
///
/// `.`으로 끝나지만 **다음에 대문자가 와도** 문장 종결이 아닌 케이스를 잡기 위함.
/// 예: `"Mr. Smith"`, `"Google Inc. launched"` 등.
///
/// 내부에 `.`이 있는 `a.m.`/`p.m.`/`e.g.`/`i.e.`는 `is_period_sentence_terminator`의
/// "공백 없음" 규칙과 다음 문자 소문자 규칙이 이미 커버하므로 여기에 추가하지 않는다
/// (추가하면 `"p.m. 100"`처럼 숫자로 이어지는 정상적인 문장 분리를 막게 됨).
const ABBREVIATIONS: &[&str] = &[
    // 호칭
    "mr", "mrs", "ms", "dr", "prof", "sr", "jr", "st", // 회사/법인
    "inc", "ltd", "co", "corp", "llc", // 일반
    "etc", "vs", "approx", "ca",
];

/// `text`에서 `period_after` 위치 직전의 `.`이 문장 종결인지 판정.
///
/// 규칙:
/// - `.` 바로 뒤에 공백이 없으면 종결 아님 (약어/버전번호/URL 등)
/// - `.` 직전 단어가 `ABBREVIATIONS`에 있으면 종결 아님 (Mr., Inc., e.g. 등)
/// - `.` 뒤 공백 이후 첫 문자가 대문자/숫자/인용부호류/EOS이면 종결
/// - 공백 이후 첫 문자가 소문자면 종결 아님 (Mr. smith, a.m. officially 등)
fn is_period_sentence_terminator(text: &str, period_after: usize) -> bool {
    // 먼저 `.` 직전 단어가 알려진 약어인지 확인 (대문자 시작이 와도 종결 아님).
    // "Google Inc. launched it." 같은 케이스를 잡는다.
    if preceding_word_is_abbreviation(text, period_after - 1) {
        return false;
    }

    if period_after >= text.len() {
        return true;
    }
    let rest = &text[period_after..];
    let mut chars = rest.chars();
    let first = match chars.next() {
        Some(c) => c,
        None => return true,
    };
    if !first.is_whitespace() {
        return false;
    }
    // 첫 공백 이후 실제 non-ws 문자 탐색
    let mut next_non_ws: Option<char> = None;
    for ch in chars {
        if !ch.is_whitespace() {
            next_non_ws = Some(ch);
            break;
        }
    }
    match next_non_ws {
        None => true,
        // `.` 뒤 첫 문자가 소문자가 아니면 종결로 간주.
        // 이 조건은 대문자, 숫자, CJK 문자(한/중/일), 기호를 모두 포함한다.
        // "Mr. smith"의 s 같은 Latin 소문자일 때만 종결이 아님.
        Some(ch) => !ch.is_lowercase(),
    }
}

/// `text`에서 `period_byte` 위치의 `.` 직전 단어가 `ABBREVIATIONS`에 있는지.
///
/// 단어는 공백(또는 추가 `.`)으로 구분된 마지막 영문 토큰을 본다. 대소문자 무관.
fn preceding_word_is_abbreviation(text: &str, period_byte: usize) -> bool {
    if period_byte == 0 {
        return false;
    }
    let before = &text[..period_byte];
    // 역방향으로 영문 단어 추출 (ASCII alphabetic + 내부 period 허용 아님)
    let last_word: String = before
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    if last_word.is_empty() {
        return false;
    }
    let lowered = last_word.to_lowercase();
    ABBREVIATIONS.iter().any(|&a| a == lowered)
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

    fn line(text: &str, start: f64, end: f64) -> SubtitleLine {
        SubtitleLine {
            text: text.to_string(),
            start,
            end,
        }
    }

    #[test]
    fn test_merge_empty_input() {
        let result = merge_into_sentences(vec![]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_merge_single_line() {
        let lines = vec![line("Hello world.", 0.0, 2.0)];
        let result = merge_into_sentences(lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].text, "Hello world.");
    }

    #[test]
    fn test_merge_consecutive_until_terminator() {
        let lines = vec![
            line("Hello", 0.0, 1.5),
            line("world", 1.5, 3.0),
            line("how are you.", 3.0, 5.0),
            line("I am fine.", 5.5, 7.0),
        ];
        let result = merge_into_sentences(lines);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].text, "Hello world how are you.");
        assert_eq!(result[0].start, 0.0);
        assert_eq!(result[0].end, 5.0);
        assert_eq!(result[1].text, "I am fine.");
    }

    #[test]
    fn test_merge_splits_on_large_gap() {
        let lines = vec![
            line("Hello", 0.0, 1.0),
            line("world", 3.0, 4.0), // gap = 2.0 > 1.5
        ];
        let result = merge_into_sentences(lines);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_merge_allows_gap_just_under_threshold() {
        // gap 1.4 → 병합 허용 (GAP_THRESHOLD = 1.5)
        let lines = vec![line("Hello", 0.0, 1.0), line("world", 2.4, 3.4)];
        let result = merge_into_sentences(lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].text, "Hello world");
    }

    #[test]
    fn test_merge_respects_hard_duration_limit() {
        // 3초씩 7개 = 21초 > HARD_DURATION(18) → 어딘가에서 강제 flush
        let lines: Vec<SubtitleLine> = (0..7)
            .map(|i| line(&format!("w{i}"), i as f64 * 3.0, (i + 1) as f64 * 3.0))
            .collect();
        let result = merge_into_sentences(lines);
        // 모든 그룹의 duration은 18초 이하
        for g in &result {
            assert!(
                g.end - g.start <= 18.0,
                "그룹이 HARD 한도를 넘음: {:.1}",
                g.end - g.start
            );
        }
        assert!(result.len() >= 2);
    }

    #[test]
    fn test_merge_soft_limit_waits_for_terminator() {
        // soft 10초 도달 후에도 다음 라인이 종결로 끝나면 병합 (최대 hard 18초까지)
        let lines = vec![
            line("one", 0.0, 3.0),
            line("two", 3.0, 6.0),
            line("three", 6.0, 9.0),       // 9초 — soft 임박
            line("four five.", 9.0, 12.0), // 12초 — soft 넘었지만 종결 → 병합 OK
        ];
        let result = merge_into_sentences(lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].text, "one two three four five.");
    }

    #[test]
    fn test_merge_soft_limit_flushes_without_terminator() {
        // soft 10초 넘고 다음 라인도 종결이 아니면 거기서 flush
        let lines = vec![
            line("one", 0.0, 3.0),
            line("two", 3.0, 6.0),
            line("three", 6.0, 9.0),
            line("four", 9.0, 12.0), // 이 지점 병합 시 12초, next("five")가 종결 아니면 스톱
            line("five", 12.0, 14.0),
            line("six.", 14.0, 16.0),
        ];
        let result = merge_into_sentences(lines);
        // 첫 그룹은 "one two three" 이나 "one two three four"까지 — 다음 라인(four→five)이 종결 아니면 스톱
        assert!(result.len() >= 2);
    }

    #[test]
    fn test_merge_respects_char_limit() {
        let long_segment = "a".repeat(200);
        let another_long = "b".repeat(100);
        let lines = vec![
            line(&long_segment, 0.0, 2.0),
            line(&another_long, 2.0, 4.0), // 200 + 1 + 100 = 301 > MAX_CHARS(280)
        ];
        let result = merge_into_sentences(lines);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_merge_handles_multilang_terminators() {
        let lines = vec![
            line("안녕하세요.", 0.0, 2.0),
            line("こんにちは。", 2.0, 4.0),
            line("continue", 4.0, 5.0),
            line("this sentence", 5.0, 6.0),
            line("最高！", 6.0, 7.0),
        ];
        let result = merge_into_sentences(lines);
        // 3개 그룹: [안녕.] / [こんにちは。] / [continue this sentence 最高！]
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].text, "안녕하세요.");
        assert_eq!(result[1].text, "こんにちは。");
        assert_eq!(result[2].text, "continue this sentence 最高！");
    }

    #[test]
    fn test_split_no_terminator_returns_as_is() {
        let input = vec![line("Hello world", 0.0, 3.0)];
        let result = split_lines_on_sentence_boundaries(input);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].text, "Hello world");
    }

    #[test]
    fn test_split_terminator_only_at_end_no_split() {
        let input = vec![line("Hello world.", 0.0, 3.0)];
        let result = split_lines_on_sentence_boundaries(input);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].text, "Hello world.");
    }

    #[test]
    fn test_split_two_sentences_in_one_snippet() {
        // "A. B" 형태: 중간에 종결 있음
        let input = vec![line("Hello world. How are you", 0.0, 6.0)];
        let result = split_lines_on_sentence_boundaries(input);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].text, "Hello world.");
        assert_eq!(result[1].text, "How are you");
        // 시간이 문자 수 비례로 분배됨
        assert!(result[0].start == 0.0);
        assert!(result[1].end == 6.0);
        assert!(result[0].end == result[1].start);
    }

    #[test]
    fn test_split_three_sentences() {
        let input = vec![line("A. B! C?", 0.0, 9.0)];
        let result = split_lines_on_sentence_boundaries(input);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].text, "A.");
        assert_eq!(result[1].text, "B!");
        assert_eq!(result[2].text, "C?");
    }

    #[test]
    fn test_split_multilang_terminators() {
        let input = vec![line("안녕하세요. こんにちは。 Hello!", 0.0, 9.0)];
        let result = split_lines_on_sentence_boundaries(input);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].text, "안녕하세요.");
        assert_eq!(result[1].text, "こんにちは。");
        assert_eq!(result[2].text, "Hello!");
    }

    #[test]
    fn test_split_does_not_shrink_tiny_fragments() {
        // "A." 같은 2자 part + 긴 뒤 문장 — A. duration이 MIN 이상
        let input = vec![line("A. This is a much longer sentence here.", 0.0, 10.0)];
        let result = split_lines_on_sentence_boundaries(input);
        assert_eq!(result.len(), 2);
        let first_dur = result[0].end - result[0].start;
        assert!(
            first_dur >= 0.3,
            "짧은 part의 duration이 MIN(0.3) 이상이어야 함: actual={:.3}",
            first_dur
        );
    }

    #[test]
    fn test_split_aborts_when_original_too_short_for_min() {
        // 0.5초 원본에 2개 part → MIN 0.3 * 2 = 0.6 > 0.5 → split 포기
        let input = vec![line("A. B.", 0.0, 0.5)];
        let result = split_lines_on_sentence_boundaries(input);
        assert_eq!(result.len(), 1, "원본 너무 짧으면 split 포기");
        assert_eq!(result[0].text, "A. B.");
    }

    #[test]
    fn test_split_preserves_line_boundaries() {
        // 마지막 파트의 end가 원본 end와 정확히 일치해야 함
        let input = vec![line("One. Two. Three", 0.0, 10.0)];
        let result = split_lines_on_sentence_boundaries(input);
        assert_eq!(result[result.len() - 1].end, 10.0);
        assert_eq!(result[0].start, 0.0);
    }

    #[test]
    fn test_split_preserves_abbreviations() {
        // "4:00 a.m. officially" — a.m. 약어가 깨지지 않고, 뒤에 소문자 'o'라 종결도 아님
        let input = vec![line(
            "leaked at 4:00 a.m. officially making it public.",
            0.0,
            10.0,
        )];
        let result = split_lines_on_sentence_boundaries(input);
        // 전체가 한 덩어리 (또는 최대 2덩어리 — 마지막 `.`에서)
        // 마지막 `.` 뒤는 EOS라 종결. 그 이전 `.`들은 약어 / 공백 뒤 소문자라 종결 아님.
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].text,
            "leaked at 4:00 a.m. officially making it public."
        );
    }

    #[test]
    fn test_split_preserves_version_numbers() {
        // "version 2.1.88 of" — 모든 `.`이 숫자 앞뒤라 종결 아님
        let input = vec![line(
            "version 2.1.88 of the package was released.",
            0.0,
            8.0,
        )];
        let result = split_lines_on_sentence_boundaries(input);
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].text,
            "version 2.1.88 of the package was released."
        );
    }

    #[test]
    fn test_split_mr_smith_not_split() {
        // "Mr. smith" — 공백 뒤 소문자 → 종결 아님 (약어 Mr.)
        let input = vec![line("Mr. smith went home.", 0.0, 5.0)];
        let result = split_lines_on_sentence_boundaries(input);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_split_capital_after_period_splits() {
        // 일반 문장: "One. Two." 대문자 시작 → 종결 ✓
        let input = vec![line("First. Second.", 0.0, 4.0)];
        let result = split_lines_on_sentence_boundaries(input);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].text, "First.");
        assert_eq!(result[1].text, "Second.");
    }

    #[test]
    fn test_split_number_after_period_splits() {
        // "At 3 p.m. 100 people" — 마지막 `.` 뒤 공백+숫자 1 → 종결 ✓
        let input = vec![line("At 3 p.m. 100 people came.", 0.0, 6.0)];
        let result = split_lines_on_sentence_boundaries(input);
        // "At 3 p.m." + "100 people came."
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].text, "At 3 p.m.");
        assert_eq!(result[1].text, "100 people came.");
    }

    #[test]
    fn test_split_preserves_inc_at_end_of_sentence() {
        // "Google Inc. launched it." — Inc. 약어라 문장 경계 아님 → 한 덩어리
        let input = vec![line("Google Inc. launched it.", 0.0, 5.0)];
        let result = split_lines_on_sentence_boundaries(input);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].text, "Google Inc. launched it.");
    }

    #[test]
    fn test_split_preserves_mr_smith_even_with_uppercase_next() {
        // "Mr. Smith" — Mr. 약어라 대문자 뒤에 와도 종결 아님
        let input = vec![line("Mr. Smith went home.", 0.0, 5.0)];
        let result = split_lines_on_sentence_boundaries(input);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_split_preserves_vs_abbreviation() {
        let input = vec![line("React vs. Vue is a hot topic.", 0.0, 5.0)];
        let result = split_lines_on_sentence_boundaries(input);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_merge_continues_across_abbreviation_boundary() {
        // 자동 자막이 약어 뒤에서 라인을 끊은 경우: 다음 라인이 소문자로 시작하면 합침
        let lines = vec![
            line("She works at Google Inc.", 0.0, 3.0),
            line("in Mountain View.", 3.0, 5.0),
        ];
        let result = merge_into_sentences(lines);
        assert_eq!(result.len(), 1, "약어 뒤 소문자 시작 라인은 merge되어야 함");
        assert_eq!(result[0].text, "She works at Google Inc. in Mountain View.");
    }

    #[test]
    fn test_merge_still_splits_at_true_sentence_boundary() {
        // 종결 후 다음 라인이 대문자로 시작 = 진짜 문장 전환 → 분리 유지
        let lines = vec![
            line("I love pizza.", 0.0, 2.0),
            line("She prefers pasta.", 2.0, 4.0),
        ];
        let result = merge_into_sentences(lines);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_preceding_word_is_abbreviation_basic() {
        assert!(preceding_word_is_abbreviation("Google Inc.", 10));
        assert!(preceding_word_is_abbreviation("Mr.", 2));
        assert!(preceding_word_is_abbreviation("React vs.", 8));
        assert!(!preceding_word_is_abbreviation("hello.", 5));
        assert!(!preceding_word_is_abbreviation("test world.", 10));
    }

    #[test]
    fn test_split_then_merge_end_to_end() {
        // Fireship-style: 한 snippet에 여러 문장이 섞여 있고, 인접 snippet으로
        // 이어지는 흐름. split → merge 후에는 문장 단위로 블록이 나와야 한다.
        let input = vec![
            line(
                "Yesterday, something happened. Anthropic is a startup built on safety",
                0.0,
                9.0,
            ),
            line("first, that advocates for closed source.", 9.0, 15.0),
        ];
        let split = split_lines_on_sentence_boundaries(input);
        // split 후 3조각: ["Yesterday... happened.", "Anthropic... safety", "first... closed source."]
        assert_eq!(split.len(), 3);
        assert_eq!(split[0].text, "Yesterday, something happened.");

        let merged = merge_into_sentences(split);
        // merge 후 2블록: [첫 문장], [둘째 문장 = Anthropic... closed source.]
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].text, "Yesterday, something happened.");
        assert!(merged[1].text.ends_with("closed source."));
    }

    #[test]
    fn test_ends_with_sentence_terminator_variants() {
        assert!(ends_with_sentence_terminator("hello."));
        assert!(ends_with_sentence_terminator("hello!"));
        assert!(ends_with_sentence_terminator("hello? "));
        assert!(ends_with_sentence_terminator("end…"));
        assert!(ends_with_sentence_terminator("안녕。"));
        assert!(ends_with_sentence_terminator("안녕！"));
        assert!(ends_with_sentence_terminator("안녕？"));
        assert!(!ends_with_sentence_terminator("hello"));
        assert!(!ends_with_sentence_terminator("hello,"));
    }
}
