use crate::error::AppError;
use crate::translate::TranslationEntry;

/// 번역 결과 JSON 문자열을 검증하고 파싱
///
/// 검증 항목:
/// - JSON 배열 파싱 가능 여부
/// - 필수 필드 존재: original, translated, start, end
/// - 배열 길이 >= 1
/// - translated 필드에 한국어 포함 여부 (경고만, 에러는 아님)
pub fn validate_translation(json_str: &str) -> Result<Vec<TranslationEntry>, AppError> {
    let entries: Vec<TranslationEntry> = serde_json::from_str(json_str).map_err(|e| {
        AppError::Translation(format!(
            "번역 결과 JSON 파싱 실패: {}. 원본: {}",
            e,
            truncate(json_str, 200)
        ))
    })?;

    if entries.is_empty() {
        return Err(AppError::Translation("번역 결과가 비어 있습니다".into()));
    }

    // 필드 유효성 검사
    for (i, entry) in entries.iter().enumerate() {
        if entry.original.is_empty() {
            return Err(AppError::Translation(format!(
                "항목 {}: original 필드가 비어 있습니다",
                i
            )));
        }
        if entry.translated.is_empty() {
            return Err(AppError::Translation(format!(
                "항목 {}: translated 필드가 비어 있습니다",
                i
            )));
        }
        if entry.start < 0.0 || entry.end < 0.0 {
            return Err(AppError::Translation(format!(
                "항목 {}: 시간 값이 음수입니다 (start={}, end={})",
                i, entry.start, entry.end
            )));
        }
    }

    Ok(entries)
}

/// translated 텍스트에 한국어 문자가 포함되어 있는지 확인
#[allow(dead_code)]
pub fn contains_korean(text: &str) -> bool {
    text.chars().any(|c| ('\u{AC00}'..='\u{D7AF}').contains(&c))
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_translation() {
        let json = r#"[
            {
                "original": "Hello world",
                "translated": "안녕하세요 세계",
                "start": 0.0,
                "end": 2.5
            },
            {
                "original": "How are you",
                "translated": "어떻게 지내세요",
                "start": 2.5,
                "end": 5.0
            }
        ]"#;

        let entries = validate_translation(json).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].original, "Hello world");
        assert_eq!(entries[0].translated, "안녕하세요 세계");
        assert_eq!(entries[0].start, 0.0);
        assert_eq!(entries[0].end, 2.5);
    }

    #[test]
    fn test_empty_array_is_error() {
        let json = "[]";
        let err = validate_translation(json).unwrap_err();
        match err {
            AppError::Translation(msg) => assert!(msg.contains("비어 있습니다")),
            _ => panic!("wrong error type"),
        }
    }

    #[test]
    fn test_invalid_json_is_error() {
        let json = "not json at all";
        assert!(validate_translation(json).is_err());
    }

    #[test]
    fn test_missing_field_is_error() {
        let json = r#"[{"original": "hi", "start": 0.0, "end": 1.0}]"#;
        assert!(validate_translation(json).is_err());
    }

    #[test]
    fn test_empty_original_is_error() {
        let json = r#"[{
            "original": "",
            "translated": "안녕",
            "start": 0.0,
            "end": 1.0
        }]"#;
        let err = validate_translation(json).unwrap_err();
        match err {
            AppError::Translation(msg) => assert!(msg.contains("original")),
            _ => panic!("wrong error type"),
        }
    }

    #[test]
    fn test_empty_translated_is_error() {
        let json = r#"[{
            "original": "hello",
            "translated": "",
            "start": 0.0,
            "end": 1.0
        }]"#;
        assert!(validate_translation(json).is_err());
    }

    #[test]
    fn test_negative_time_is_error() {
        let json = r#"[{
            "original": "hello",
            "translated": "안녕",
            "start": -1.0,
            "end": 1.0
        }]"#;
        assert!(validate_translation(json).is_err());
    }

    #[test]
    fn test_contains_korean() {
        assert!(contains_korean("안녕하세요"));
        assert!(contains_korean("Hello 세계"));
        assert!(!contains_korean("Hello world"));
        assert!(!contains_korean("12345"));
        assert!(!contains_korean(""));
        // 일본어 히라가나는 한국어가 아님
        assert!(!contains_korean("こんにちは"));
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("a long string here", 5), "a lon...");
    }
}
