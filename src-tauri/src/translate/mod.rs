pub mod codex_event_parser;
pub mod jsonl_parser;
pub mod prompt;
pub mod validator;

use serde::{Deserialize, Serialize};

/// 영상 메타데이터 (번역 프롬프트에 포함)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoInfo {
    pub title: String,
    pub description: String,
}

/// 번역된 자막 한 줄
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationEntry {
    pub original: String,
    pub translated: String,
    pub start: f64,
    pub end: f64,
}

#[cfg(test)]
mod integration_tests {
    //! 번역 파이프라인 E2E 통합 테스트.
    //!
    //! 실행 요건:
    //! - Claude Code CLI 설치 (`npm install -g @anthropic-ai/claude-code`)
    //! - 네트워크 (자막 fetch 테스트의 경우)
    //!
    //! 실행: `cargo test --lib -- --ignored --nocapture`

    use super::*;
    use crate::claude::adapter::{ClaudeAdapter, ExecuteParams};
    use crate::subtitle::chunk::split_into_chunks;
    use crate::subtitle::fetch::fetch_subtitles;
    use crate::subtitle::{SubtitleChunk, SubtitleLine};
    use crate::translate::jsonl_parser::extract_text_from_jsonl;
    use crate::translate::prompt::build_prompt;
    use crate::translate::validator::{contains_korean, validate_translation};

    /// 번역 파이프라인 한 사이클을 실행하는 헬퍼.
    /// prompt 구성 → Claude subprocess → JSONL 파싱 → 검증 → TranslationEntry 반환.
    async fn translate_pipeline(
        chunk: &SubtitleChunk,
        video_info: Option<&VideoInfo>,
        previous_context: Option<&[SubtitleLine]>,
        model: Option<&str>,
    ) -> Vec<TranslationEntry> {
        // 통합 테스트 파이프라인은 세션 없이 독립 실행 = 첫 호출 모드
        let prompt = build_prompt(chunk, video_info, previous_context, true);
        let result = ClaudeAdapter::execute(ExecuteParams {
            prompt: &prompt,
            timeout_secs: 120,
            model,
            session_id: None,
            is_first_in_session: true,
        })
        .await
        .expect("Claude subprocess 실행 실패");
        let json_text = extract_text_from_jsonl(&result.raw_output).expect("JSONL 파싱 실패");
        validate_translation(&json_text).expect("번역 결과 검증 실패")
    }

    fn line(text: &str, start: f64, end: f64) -> SubtitleLine {
        SubtitleLine {
            text: text.to_string(),
            start,
            end,
        }
    }

    /// Claude CLI 환경이 올바른지(설치 여부, 실행 가능) 확인.
    #[tokio::test]
    #[ignore]
    async fn test_claude_cli_available() {
        ClaudeAdapter::test_environment()
            .await
            .expect("Claude CLI가 설치되어 있지 않거나 실행 불가능");
    }

    /// 인라인 픽스처(한 청크)만으로 빠르게 번역 파이프라인을 돌린다.
    /// 네트워크 자막 fetch 없이 Claude subprocess 경로만 검증.
    #[tokio::test]
    #[ignore]
    async fn test_translate_simple_chunk_with_haiku() {
        let lines = vec![
            line("Hello everyone, welcome back to the channel.", 0.0, 3.0),
            line(
                "Today we're going to build a mobile app with React Native.",
                3.0,
                7.0,
            ),
            line("Let's get started.", 7.0, 9.0),
        ];
        let chunk = SubtitleChunk {
            index: 0,
            start_time: 0.0,
            end_time: 9.0,
            lines: lines.clone(),
        };

        let entries = translate_pipeline(&chunk, None, None, Some("haiku")).await;

        // 기본 검증
        assert_eq!(
            entries.len(),
            lines.len(),
            "원본 라인 수({})와 번역 결과 수({})가 같아야 함",
            lines.len(),
            entries.len()
        );

        // 각 엔트리 검증: original 일치, translated에 한국어 포함
        for (i, entry) in entries.iter().enumerate() {
            assert_eq!(
                entry.original.trim(),
                lines[i].text.trim(),
                "항목 {}: original이 입력 자막과 달라짐",
                i
            );
            assert!(
                contains_korean(&entry.translated),
                "항목 {}: 번역 텍스트에 한국어가 없음 — {:?}",
                i,
                entry.translated
            );
            assert!(entry.start >= 0.0 && entry.end > entry.start);
        }

        eprintln!("=== Haiku 번역 결과 ({} 라인) ===", entries.len());
        for e in &entries {
            eprintln!(
                "  [{:>5.1}s] {}\n          → {}",
                e.start, e.original, e.translated
            );
        }
    }

    /// 실제 YouTube 자막을 fetch → 청크 분할 → 첫 청크만 번역하는 전체 E2E.
    /// fetch_subtitles 수정과 번역 파이프라인이 함께 정상 동작함을 검증.
    ///
    /// 교육 콘텐츠를 사용한다 — 음악/영화 가사 등 저작권 콘텐츠는 Claude가
    /// 번역을 거부할 수 있어 테스트가 비결정적이 된다.
    #[tokio::test]
    #[ignore]
    async fn test_fetch_and_translate_first_chunk_e2e() {
        // 사용자 리포트 영상 (React Native 강의, 영어 자막 5339 라인)
        let lines = fetch_subtitles("4nVoLX2taFg")
            .await
            .expect("자막 fetch 실패");
        assert!(!lines.is_empty(), "자막이 비어 있으면 안 됨");

        let chunks = split_into_chunks(&lines);
        assert!(!chunks.is_empty(), "청크가 비어 있으면 안 됨");

        let first_chunk = &chunks[0];
        eprintln!(
            "=== 첫 청크: {} 라인, {:.1}s~{:.1}s ===",
            first_chunk.lines.len(),
            first_chunk.start_time,
            first_chunk.end_time
        );

        let entries = translate_pipeline(first_chunk, None, None, Some("haiku")).await;

        assert_eq!(
            entries.len(),
            first_chunk.lines.len(),
            "청크 라인 수와 번역 결과 수가 일치해야 함"
        );
        for (i, entry) in entries.iter().enumerate() {
            assert!(
                !entry.translated.is_empty(),
                "항목 {}: 번역 결과가 비어 있음",
                i
            );
            assert!(
                contains_korean(&entry.translated),
                "항목 {}: 한국어 번역이 아님 — {:?}",
                i,
                entry.translated
            );
        }

        eprintln!("--- 처음 3개 번역 샘플 ---");
        for e in entries.iter().take(3) {
            eprintln!(
                "  [{:>5.1}s] {}\n          → {}",
                e.start, e.original, e.translated
            );
        }
    }

    /// **핵심 품질 검증 테스트**.
    ///
    /// 자동 자막 영상(`mBHRPeg8zPU` — Fireship "Anthropic leaks Claude's source code")을
    /// 대상으로 fetch → 문장 재구성 → 청크 분할 → 번역까지 한 사이클을 돌린다.
    /// stdout 로그를 사람(Claude)이 읽고 품질을 판단한다:
    /// 1) 병합 후 라인들이 문장 단위로 완결된가?
    /// 2) 청크 텍스트가 자연스러운 문단으로 읽히는가?
    /// 3) 번역이 라인 단편이 아니라 문맥 있는 문장 흐름인가?
    /// 4) 고유명사/전문용어가 일관되게 번역되었는가?
    ///
    /// 실행: `cargo test --lib test_auto_caption_sentence_merge_and_translate -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn test_auto_caption_sentence_merge_and_translate() {
        const VIDEO_ID: &str = "mBHRPeg8zPU";

        eprintln!("\n======== [V3] 자동 자막 문장 재구성 + 번역 품질 검증 ========");
        eprintln!("영상: https://youtu.be/{}\n", VIDEO_ID);

        // [1] fetch_subtitles — 내부적으로 is_auto 감지 + merge_into_sentences 적용
        eprintln!("[1] 자막 fetch 시작...");
        let lines = fetch_subtitles(VIDEO_ID).await.expect("자막 fetch 실패");
        assert!(!lines.is_empty(), "자막이 비어 있으면 안 됨");
        eprintln!("[1] 완료: {} lines (postprocess 적용 후)", lines.len());

        let total_duration: f64 = lines
            .last()
            .map(|l| l.end - lines.first().unwrap().start)
            .unwrap_or(0.0);
        let avg_duration = if lines.is_empty() {
            0.0
        } else {
            total_duration / (lines.len() as f64)
        };
        eprintln!(
            "    평균 라인 duration: {:.2}s, 전체 길이: {:.1}s",
            avg_duration, total_duration
        );

        eprintln!("--- 처음 15개 라인 (병합 결과) ---");
        for (i, line) in lines.iter().take(15).enumerate() {
            eprintln!(
                "  #{:02} [{:>6.2}s → {:>6.2}s] ({:>2}자) {}",
                i,
                line.start,
                line.end,
                line.text.chars().count(),
                line.text
            );
        }

        // [2] split_into_chunks
        let chunks = split_into_chunks(&lines);
        assert!(!chunks.is_empty(), "청크가 비어 있으면 안 됨");
        eprintln!("\n[2] {} chunks 생성", chunks.len());
        for chunk in chunks.iter().take(2) {
            let concat: String = chunk
                .lines
                .iter()
                .map(|l| l.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            eprintln!(
                "    chunk {}: [{:.1}s-{:.1}s] {} lines",
                chunk.index,
                chunk.start_time,
                chunk.end_time,
                chunk.lines.len()
            );
            let preview = if concat.chars().count() > 300 {
                let truncated: String = concat.chars().take(300).collect();
                format!("{}...", truncated)
            } else {
                concat
            };
            eprintln!("      full text: {}", preview);
        }

        // [3] Claude 번역 (첫 청크)
        eprintln!("\n[3] Claude CLI 번역 실행 (haiku, 첫 청크)...");
        let first_chunk = &chunks[0];
        let entries = translate_pipeline(first_chunk, None, None, Some("haiku")).await;

        assert_eq!(
            entries.len(),
            first_chunk.lines.len(),
            "청크 라인 수와 번역 결과 수가 일치해야 함"
        );
        for entry in &entries {
            assert!(
                contains_korean(&entry.translated),
                "번역에 한국어가 포함되어야 함: {:?}",
                entry.translated
            );
            assert!(entry.start >= 0.0 && entry.end > entry.start);
        }

        eprintln!("\n--- 첫 청크 번역 결과 ({} entries) ---", entries.len());
        for entry in &entries {
            eprintln!("  [{:>6.2}s → {:>6.2}s]", entry.start, entry.end);
            eprintln!("    EN: {}", entry.original);
            eprintln!("    KO: {}", entry.translated);
        }

        // [4] 두 번째 청크 번역 (맥락 연속성 확인)
        if chunks.len() > 1 {
            eprintln!("\n[4] Claude CLI 번역 (두 번째 청크, 이전 맥락 포함)...");
            let second_chunk = &chunks[1];
            let prev_ctx: Vec<SubtitleLine> = first_chunk
                .lines
                .iter()
                .rev()
                .take(8)
                .rev()
                .cloned()
                .collect();
            let entries2 =
                translate_pipeline(second_chunk, None, Some(&prev_ctx), Some("haiku")).await;
            eprintln!(
                "\n--- 두 번째 청크 번역 결과 ({} entries) ---",
                entries2.len()
            );
            for entry in &entries2 {
                eprintln!("  [{:>6.2}s → {:>6.2}s]", entry.start, entry.end);
                eprintln!("    EN: {}", entry.original);
                eprintln!("    KO: {}", entry.translated);
            }
        }

        eprintln!("\n======== 테스트 완료 — 출력 로그로 품질 판단 ========\n");
    }
}
