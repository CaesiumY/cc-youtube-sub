//! Codex CLI `exec --json` 이벤트 스트림 파서.
//!
//! `codex exec --json`은 각 줄이 독립 JSON 객체인 JSONL을 출력한다. `type` 필드가
//! 최상위에 있고 점 표기법을 쓴다. codex-cli 0.132.0 기준 실측한 형식:
//!
//! ```text
//! {"type":"thread.started","thread_id":"019e4549-a3a6-7683-824e-455ee5ccb6a2"}
//! {"type":"turn.started"}
//! {"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"..."}}
//! {"type":"turn.completed","usage":{...}}
//! ```
//!
//! - **최종 텍스트**: `item.completed` 이벤트의 `item` 객체 중 `item.type == "agent_message"`인
//!   것의 `text` 필드. reasoning 등 다른 item type은 제외(Claude 파서가 thinking 블록을
//!   제외하는 것과 같은 패턴).
//! - **세션 ID**: `thread.started` 이벤트의 `thread_id` — adapter가 첫 호출 후 캡처해
//!   후속 청크의 `codex exec resume <id>`에 사용.
//!
//! 에러 처리: codex가 `--json` 스트림에 어떤 에러 이벤트를 내는지 버전에 따라 다를 수
//! 있어, 알려진 에러성 type을 방어적으로 매칭한다. 매칭에 실패하더라도 텍스트를 못 찾으면
//! 최종적으로 `Err`를 반환하므로 silent failure는 없다 (adapter는 non-zero exit code와
//! 빈 출력도 별도로 검사한다).

/// Codex `--json` 출력에서 최종 모델 응답 텍스트를 추출.
pub fn extract_text_from_codex(events: &str) -> Result<String, anyhow::Error> {
    let mut agent_text = String::new();

    for line in events.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let value: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue, // 불완전한 라인은 건너뜀
        };

        match value.get("type").and_then(|t| t.as_str()).unwrap_or("") {
            // 완성된 item — agent_message 타입의 text만 누적
            "item.completed" => {
                if let Some(item) = value.get("item") {
                    if item.get("type").and_then(|t| t.as_str()) == Some("agent_message") {
                        if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                            agent_text.push_str(text);
                        }
                    }
                }
            }
            // 에러성 이벤트 — 즉시 실패. 버전별 명칭 차이를 방어적으로 흡수.
            "error" | "thread.error" | "turn.failed" | "stream_error" => {
                let detail = extract_error_detail(&value);
                return Err(anyhow::anyhow!("Codex CLI 에러: {}", detail));
            }
            _ => {}
        }
    }

    if !agent_text.is_empty() {
        return Ok(agent_text);
    }

    Err(anyhow::anyhow!(
        "Codex 이벤트 스트림에서 agent_message 텍스트를 추출할 수 없습니다"
    ))
}

/// 에러성 이벤트에서 사람이 읽을 메시지를 최대한 추출.
fn extract_error_detail(value: &serde_json::Value) -> String {
    // 흔한 위치들을 순서대로 시도: error.message → message → error(문자열) → error 이벤트 객체.
    if let Some(s) = value
        .get("error")
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
    {
        return s.to_string();
    }
    if let Some(s) = value.get("message").and_then(|m| m.as_str()) {
        return s.to_string();
    }
    if let Some(s) = value.get("error").and_then(|e| e.as_str()) {
        return s.to_string();
    }
    "unknown error".to_string()
}

/// 첫 호출 응답에서 thread(session) ID를 추출.
///
/// codex는 첫 호출 시 `thread.started` 이벤트를 emit하며 `thread_id` 필드에 UUID가
/// 들어 있다. BufferManager가 이를 받아 후속 청크의 `codex exec resume <id>`에 사용.
pub fn extract_session_id_from_codex_events(events: &str) -> Option<String> {
    for line in events.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let value: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if value.get("type").and_then(|t| t.as_str()) == Some("thread.started") {
            if let Some(id) = value.get("thread_id").and_then(|v| v.as_str()) {
                return Some(id.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// codex-cli 0.132.0에서 실제로 캡처한 출력 (번역 프롬프트 응답).
    const REAL_FIXTURE: &str = r#"{"type":"thread.started","thread_id":"019e4549-a3a6-7683-824e-455ee5ccb6a2"}
{"type":"turn.started"}
{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"[{\"original\":\"Hello everyone\",\"translated\":\"여러분 안녕하세요\",\"start\":0.0,\"end\":2.0}]"}}
{"type":"turn.completed","usage":{"input_tokens":18311,"cached_input_tokens":4480,"output_tokens":72,"reasoning_output_tokens":39}}"#;

    #[test]
    fn real_fixture_extracts_translation_json() {
        let got = extract_text_from_codex(REAL_FIXTURE).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&got).unwrap();
        assert_eq!(parsed[0]["translated"], "여러분 안녕하세요");
        assert_eq!(parsed[0]["original"], "Hello everyone");
    }

    #[test]
    fn real_fixture_extracts_thread_id() {
        assert_eq!(
            extract_session_id_from_codex_events(REAL_FIXTURE),
            Some("019e4549-a3a6-7683-824e-455ee5ccb6a2".to_string())
        );
    }

    #[test]
    fn multiple_agent_messages_accumulate() {
        let events = r#"{"type":"item.completed","item":{"id":"i0","type":"agent_message","text":"hello "}}
{"type":"item.completed","item":{"id":"i1","type":"agent_message","text":"world"}}"#;
        assert_eq!(extract_text_from_codex(events).unwrap(), "hello world");
    }

    #[test]
    fn non_agent_message_items_are_ignored() {
        // reasoning, command_execution 등 다른 item type은 최종 텍스트에 포함되면 안 됨
        let events = r#"{"type":"item.completed","item":{"id":"i0","type":"reasoning","text":"SECRET REASONING"}}
{"type":"item.completed","item":{"id":"i1","type":"agent_message","text":"visible output"}}"#;
        let got = extract_text_from_codex(events).unwrap();
        assert!(!got.contains("SECRET REASONING"));
        assert_eq!(got, "visible output");
    }

    #[test]
    fn turn_failed_event_propagates() {
        let events = r#"{"type":"thread.started","thread_id":"t1"}
{"type":"turn.failed","error":{"message":"rate limit exceeded"}}"#;
        let err = extract_text_from_codex(events).unwrap_err();
        assert!(err.to_string().contains("rate limit exceeded"));
    }

    #[test]
    fn error_event_with_flat_message_propagates() {
        let events = r#"{"type":"error","message":"network reset"}"#;
        let err = extract_text_from_codex(events).unwrap_err();
        assert!(err.to_string().contains("network reset"));
    }

    #[test]
    fn empty_response_is_error() {
        // thread.started + turn 이벤트만 있고 agent_message가 없으면 에러
        let events = r#"{"type":"thread.started","thread_id":"t1"}
{"type":"turn.started"}
{"type":"turn.completed","usage":{}}"#;
        assert!(extract_text_from_codex(events).is_err());
    }

    #[test]
    fn skips_invalid_json_lines() {
        let events = r#"not json at all
{"type":"item.completed","item":{"type":"agent_message","text":"valid"}}
also not json"#;
        assert_eq!(extract_text_from_codex(events).unwrap(), "valid");
    }

    #[test]
    fn skips_unknown_event_types() {
        let events = r#"{"type":"turn.started"}
{"type":"some.future.event","data":"ignored"}
{"type":"item.completed","item":{"type":"agent_message","text":"ok"}}
{"type":"turn.completed","usage":{}}"#;
        assert_eq!(extract_text_from_codex(events).unwrap(), "ok");
    }

    #[test]
    fn extract_session_id_none_when_no_thread_started() {
        let events = r#"{"type":"item.completed","item":{"type":"agent_message","text":"hi"}}"#;
        assert_eq!(extract_session_id_from_codex_events(events), None);
    }
}
