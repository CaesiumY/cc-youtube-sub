/// Claude `--output-format stream-json` JSONL 응답에서 텍스트를 추출
///
/// 각 라인은 JSON 객체이며, `type == "content_block_delta"` 라인에서
/// `delta.text` 필드를 순서대로 연결하여 완전한 텍스트를 구성한다.
pub fn extract_text_from_jsonl(jsonl: &str) -> Result<String, anyhow::Error> {
    let mut assembled = String::new();

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
            Some("content_block_delta") => {
                if let Some(text) = value
                    .get("delta")
                    .and_then(|d| d.get("text"))
                    .and_then(|t| t.as_str())
                {
                    assembled.push_str(text);
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
            _ => {} // message_start, content_block_start 등은 무시
        }
    }

    if assembled.is_empty() {
        return Err(anyhow::anyhow!(
            "JSONL 응답에서 텍스트를 추출할 수 없습니다"
        ));
    }

    Ok(assembled)
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
