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
    use crate::claude::adapter::ClaudeAdapter;
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
        let prompt = build_prompt(chunk, video_info, previous_context);
        let raw = ClaudeAdapter::execute(&prompt, 120, model)
            .await
            .expect("Claude subprocess 실행 실패");
        let json_text = extract_text_from_jsonl(&raw).expect("JSONL 파싱 실패");
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
}
