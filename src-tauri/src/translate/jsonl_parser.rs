/// Claude CLI `--output-format stream-json --verbose` 응답에서 최종 텍스트를 추출
///
/// 지원하는 이벤트 형식:
/// - **새 포맷 (Claude CLI v2+)**: `{"type":"result","result":"..."}` 에서 완성된
///   텍스트를 직접 가져온다. 최우선 사용.
/// - **새 포맷 fallback**: `{"type":"assistant","message":{"content":[{"type":"text","text":"..."}]}}`
///   형태 이벤트들의 `text` 블록을 누적 (thinking 블록은 제외).
/// - **구 포맷 (호환)**: `{"type":"content_block_delta","delta":{"text":"..."}}`
///   델타 이벤트의 `text`를 순차 연결.
///
/// 에러:
/// - `{"type":"error",...}` — 즉시 실패
/// - `{"type":"result","is_error":true,...}` — 즉시 실패
pub fn extract_text_from_jsonl(jsonl: &str) -> Result<String, anyhow::Error> {
    let mut result_text: Option<String> = None;
    let mut assistant_text = String::new();
    let mut delta_text = String::new();

    for line in jsonl.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let value: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue, // 불완전한 라인은 건너뜀
        };

        match value.get("type").and_then(|t| t.as_str()) {
            // 새 포맷: 최종 결과 이벤트 (권위 있는 값)
            Some("result") => {
                let is_error = value
                    .get("is_error")
                    .and_then(|b| b.as_bool())
                    .unwrap_or(false);
                let text = value.get("result").and_then(|r| r.as_str()).unwrap_or("");

                if is_error {
                    return Err(anyhow::anyhow!(
                        "Claude CLI 에러: {}",
                        if text.is_empty() {
                            "unknown error"
                        } else {
                            text
                        }
                    ));
                }
                if !text.is_empty() {
                    result_text = Some(text.to_string());
                }
            }
            // 새 포맷 fallback: assistant 메시지의 text 컨텐츠 블록 누적
            // (thinking 블록은 건너뛰어 최종 텍스트만 추출)
            Some("assistant") => {
                if let Some(content) = value
                    .get("message")
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_array())
                {
                    for item in content {
                        if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                            if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                                assistant_text.push_str(text);
                            }
                        }
                    }
                }
            }
            // 구 포맷 호환: content_block_delta 이벤트의 델타 텍스트 누적
            Some("content_block_delta") => {
                if let Some(text) = value
                    .get("delta")
                    .and_then(|d| d.get("text"))
                    .and_then(|t| t.as_str())
                {
                    delta_text.push_str(text);
                }
            }
            Some("message_stop") => break,
            Some("error") => {
                let msg = value
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("unknown error");
                return Err(anyhow::anyhow!("Claude API 에러: {}", msg));
            }
            _ => {} // system, rate_limit_event, message_start 등은 무시
        }
    }

    // 우선순위: result 이벤트 → assistant 텍스트 누적 → delta 누적
    if let Some(t) = result_text {
        return Ok(t);
    }
    if !assistant_text.is_empty() {
        return Ok(assistant_text);
    }
    if !delta_text.is_empty() {
        return Ok(delta_text);
    }

    Err(anyhow::anyhow!(
        "JSONL 응답에서 텍스트를 추출할 수 없습니다"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_content_block_delta() {
        let jsonl = r#"{"type":"message_start","message":{"id":"msg_01"}}
{"type":"content_block_start","index":0}
{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"[{\"original\""}}
{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":": \"Hello\","}}
{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"\"translated\": \"안녕\"}]"}}
{"type":"content_block_stop","index":0}
{"type":"message_stop"}"#;

        let result = extract_text_from_jsonl(jsonl).unwrap();
        assert_eq!(result, r#"[{"original": "Hello","translated": "안녕"}]"#);
    }

    #[test]
    fn test_stops_at_message_stop() {
        let jsonl = r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"hello"}}
{"type":"message_stop"}
{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":" ignored"}}"#;

        let result = extract_text_from_jsonl(jsonl).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_skips_invalid_json_lines() {
        let jsonl = r#"not json at all
{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"valid"}}
also not json
{"type":"message_stop"}"#;

        let result = extract_text_from_jsonl(jsonl).unwrap();
        assert_eq!(result, "valid");
    }

    #[test]
    fn test_empty_response_is_error() {
        let jsonl = r#"{"type":"message_start","message":{"id":"msg_01"}}
{"type":"message_stop"}"#;

        assert!(extract_text_from_jsonl(jsonl).is_err());
    }

    #[test]
    fn test_error_event() {
        let jsonl =
            r#"{"type":"error","error":{"type":"api_error","message":"rate limit exceeded"}}"#;

        let err = extract_text_from_jsonl(jsonl).unwrap_err();
        assert!(err.to_string().contains("rate limit exceeded"));
    }

    #[test]
    fn test_skips_empty_lines() {
        let jsonl = r#"
{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"a"}}

{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"b"}}

{"type":"message_stop"}"#;

        let result = extract_text_from_jsonl(jsonl).unwrap();
        assert_eq!(result, "ab");
    }

    // ── 새 포맷 (Claude CLI v2+) ─────────────

    #[test]
    fn test_extract_from_result_event_v2() {
        let jsonl = r#"{"type":"system","subtype":"init","session_id":"abc"}
{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"Let me translate..."}]}}
{"type":"assistant","message":{"content":[{"type":"text","text":"[{\"original\":\"Hi\",\"translated\":\"안녕\"}]"}]}}
{"type":"result","subtype":"success","is_error":false,"result":"[{\"original\":\"Hi\",\"translated\":\"안녕\"}]"}"#;

        let result = extract_text_from_jsonl(jsonl).unwrap();
        assert_eq!(result, r#"[{"original":"Hi","translated":"안녕"}]"#);
    }

    #[test]
    fn test_extract_from_assistant_text_blocks_fallback() {
        // result 이벤트 없이 assistant text 블록만 있는 경우 (fallback)
        let jsonl = r#"{"type":"system","subtype":"init"}
{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"ignore me"}]}}
{"type":"assistant","message":{"content":[{"type":"text","text":"hello"}]}}
{"type":"assistant","message":{"content":[{"type":"text","text":" world"}]}}"#;

        let result = extract_text_from_jsonl(jsonl).unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_thinking_block_is_excluded() {
        // thinking 블록은 최종 결과에 포함되면 안 됨
        let jsonl = r#"{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"SECRET REASONING"},{"type":"text","text":"visible output"}]}}"#;

        let result = extract_text_from_jsonl(jsonl).unwrap();
        assert!(!result.contains("SECRET REASONING"));
        assert_eq!(result, "visible output");
    }

    #[test]
    fn test_result_event_with_is_error_true_fails() {
        let jsonl =
            r#"{"type":"result","subtype":"error","is_error":true,"result":"rate limit exceeded"}"#;

        let err = extract_text_from_jsonl(jsonl).unwrap_err();
        assert!(err.to_string().contains("rate limit exceeded"));
    }

    #[test]
    fn test_result_event_preferred_over_assistant_text() {
        // result 이벤트가 있으면 그 값이 권위 있음 (assistant text 누적보다 우선)
        let jsonl = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"partial"}]}}
{"type":"result","subtype":"success","is_error":false,"result":"final"}"#;

        let result = extract_text_from_jsonl(jsonl).unwrap();
        assert_eq!(result, "final");
    }

    #[test]
    fn test_multiline_json_assembly() {
        // 여러 delta가 합쳐져 완전한 JSON 배열이 되는 케이스
        let jsonl = r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"[\n  {"}}
{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"\"original\": \"Hi\","}}
{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"\"translated\": \"안녕\","}}
{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"\"start\": 0.0,"}}
{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"\"end\": 1.5"}}
{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"}\n]"}}
{"type":"message_stop"}"#;

        let result = extract_text_from_jsonl(jsonl).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(parsed.is_array());
        assert_eq!(parsed[0]["translated"], "안녕");
    }
}
