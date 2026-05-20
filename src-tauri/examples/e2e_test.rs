//! Phase 1 수동 e2e 테스트
//! cargo run --manifest-path src-tauri/Cargo.toml --example e2e_test

use youtube_subtitle_lib::subtitle::chunk::split_into_chunks;
use youtube_subtitle_lib::subtitle::fetch;
use youtube_subtitle_lib::translate::jsonl_parser::extract_text_from_jsonl;
use youtube_subtitle_lib::translate::prompt::build_prompt;
use youtube_subtitle_lib::translate::validator::validate_translation;

const VIDEO_ID: &str = "4nVoLX2taFg";

#[tokio::main]
async fn main() {
    println!("=== Phase 1 E2E 테스트 ===\n");

    // Step 1: 자막 fetch (라이브러리 함수 사용 — InnerTube 우회 fallback 포함)
    println!("[1/6] YouTube 자막 fetch (video_id: {VIDEO_ID})...");
    let lines = match fetch::fetch_subtitles(VIDEO_ID).await {
        Ok(l) => {
            println!("  ✅ {} lines", l.len());
            if let Some(first) = l.first() {
                println!(
                    "  첫 줄: [{:.1}s-{:.1}s] {}",
                    first.start, first.end, first.text
                );
            }
            if let Some(last) = l.last() {
                println!(
                    "  끝 줄: [{:.1}s-{:.1}s] {}",
                    last.start, last.end, last.text
                );
            }
            l
        }
        Err(e) => {
            eprintln!("  ❌ 자막 fetch 실패: {e}");
            return;
        }
    };

    // Step 2: 영상 정보 fetch
    println!("\n[2/6] 영상 메타데이터 fetch...");
    let video_info = match fetch::fetch_video_info(VIDEO_ID).await {
        Ok(info) => {
            println!("  ✅ 제목: {}", info.title);
            let desc_preview = if info.description.len() > 100 {
                format!("{}...", &info.description[..100])
            } else {
                info.description.clone()
            };
            println!("  ✅ 설명: {}", desc_preview);
            Some(info)
        }
        Err(e) => {
            println!("  ⚠️ 영상 정보 fetch 실패 (계속 진행): {e}");
            None
        }
    };

    // Step 3: 청크 분할
    println!("\n[3/6] 청크 분할 (max 60s / max 20 lines)...");
    let chunks = split_into_chunks(&lines);
    println!("  ✅ {} chunks 생성", chunks.len());
    for chunk in &chunks {
        println!(
            "  chunk {}: {:.1}s-{:.1}s ({} lines)",
            chunk.index,
            chunk.start_time,
            chunk.end_time,
            chunk.lines.len()
        );
    }

    // Step 4: 프롬프트 생성 (첫 청크)
    println!("\n[4/6] 번역 프롬프트 구성 (첫 청크)...");
    let first_chunk = match chunks.first() {
        Some(c) => c,
        None => {
            eprintln!("  ❌ 청크가 없습니다");
            return;
        }
    };
    let prompt = build_prompt(first_chunk, video_info.as_ref(), None, true);
    println!("  ✅ 프롬프트 길이: {} chars", prompt.len());
    println!("  --- 프롬프트 미리보기 (처음 500자) ---");
    println!("{}", &prompt[..prompt.len().min(500)]);
    println!("  ---");

    // Step 5: Claude CLI 환경 확인
    println!("\n[5/6] Claude CLI 환경 확인...");
    if let Err(e) = youtube_subtitle_lib::claude::adapter::ClaudeAdapter::test_environment().await {
        println!("  ⚠️ Claude CLI 미설치 — 번역 단계 건너뜀: {e}");
        println!("\n  Steps 1-4 성공! Claude CLI가 있으면 번역까지 가능합니다.");
        println!("\n=== E2E 테스트 완료 (부분) ===");
        return;
    }
    println!("  ✅ Claude CLI 확인됨");

    // Step 6: Claude 번역 실행
    println!("\n[6/6] Claude CLI 번역 실행 (최대 120초)...");
    match youtube_subtitle_lib::claude::adapter::ClaudeAdapter::execute(
        youtube_subtitle_lib::claude::adapter::ExecuteParams {
            prompt: &prompt,
            timeout_secs: 120,
            model: None,
            session_id: None,
            is_first_in_session: true,
        },
    )
    .await
    {
        Ok(result) => {
            let raw_output = result.raw_output;
            println!("  ✅ Claude 응답 수신 ({} bytes)", raw_output.len());

            match extract_text_from_jsonl(&raw_output) {
                Ok(json_text) => {
                    println!("  ✅ JSONL 파싱 성공 ({} chars)", json_text.len());

                    match validate_translation(&json_text) {
                        Ok(entries) => {
                            println!("  ✅ 번역 검증 성공! {} entries\n", entries.len());
                            for entry in entries.iter().take(3) {
                                println!("  [{:.1}s-{:.1}s]", entry.start, entry.end);
                                println!("    EN: {}", entry.original);
                                println!("    KO: {}", entry.translated);
                                println!();
                            }
                            if entries.len() > 3 {
                                println!("  ... (외 {} entries)", entries.len() - 3);
                            }
                        }
                        Err(e) => {
                            eprintln!("  ❌ 번역 검증 실패: {e}");
                            println!("  Raw JSON: {}", &json_text[..json_text.len().min(500)]);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("  ❌ JSONL 파싱 실패: {e}");
                    println!(
                        "  Raw output:\n{}",
                        &raw_output[..raw_output.len().min(500)]
                    );
                }
            }
        }
        Err(e) => eprintln!("  ❌ Claude 실행 실패: {e}"),
    }

    println!("\n=== E2E 테스트 완료 ===");
}
