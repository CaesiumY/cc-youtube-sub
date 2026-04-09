use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use rusqlite::Connection;
use sha2::{Digest, Sha256};

use crate::error::AppError;
use crate::subtitle::SubtitleLine;

/// SQLite 캐시 — 번역 결과를 영상별/청크별로 저장
///
/// 스키마:
/// - video_id + chunk_hash 조합으로 유니크 캐시 키
/// - chunk_hash = SHA256(라인 텍스트 join)
/// - WAL 모드로 동시 읽기/쓰기 지원
pub struct TranslationCache {
    conn: Mutex<Connection>,
}

impl TranslationCache {
    /// 캐시 DB를 열고 테이블을 초기화한다.
    pub fn new(db_path: PathBuf) -> Result<Self, AppError> {
        // 부모 디렉토리가 없으면 생성
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| AppError::Database(format!("DB 디렉토리 생성 실패: {}", e)))?;
        }

        let conn = Connection::open(&db_path)
            .map_err(|e| AppError::Database(format!("SQLite 열기 실패: {}", e)))?;

        // WAL 모드 활성화
        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .map_err(|e| AppError::Database(format!("WAL 모드 설정 실패: {}", e)))?;

        // 테이블 생성
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS translation_cache (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                video_id TEXT NOT NULL,
                chunk_hash TEXT NOT NULL,
                translated_json TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(video_id, chunk_hash)
            );",
        )
        .map_err(|e| AppError::Database(format!("테이블 생성 실패: {}", e)))?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// 단일 청크 캐시 조회
    pub fn query(&self, video_id: &str, chunk_hash: &str) -> Result<Option<String>, AppError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("DB 락 획득 실패: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT translated_json FROM translation_cache WHERE video_id = ?1 AND chunk_hash = ?2",
            )
            .map_err(|e| AppError::Database(format!("쿼리 준비 실패: {}", e)))?;

        let result = stmt
            .query_row(rusqlite::params![video_id, chunk_hash], |row| {
                row.get::<_, String>(0)
            })
            .ok();

        Ok(result)
    }

    /// 번역 결과 저장 (중복 시 덮어쓰기)
    pub fn save(
        &self,
        video_id: &str,
        chunk_hash: &str,
        translated_json: &str,
    ) -> Result<(), AppError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("DB 락 획득 실패: {}", e)))?;

        conn.execute(
            "INSERT OR REPLACE INTO translation_cache (video_id, chunk_hash, translated_json) VALUES (?1, ?2, ?3)",
            rusqlite::params![video_id, chunk_hash, translated_json],
        )
        .map_err(|e| AppError::Database(format!("캐시 저장 실패: {}", e)))?;

        Ok(())
    }

    /// 여러 청크 캐시 일괄 조회 (재방문 시)
    pub fn batch_query(
        &self,
        video_id: &str,
        chunk_hashes: &[String],
    ) -> Result<HashMap<String, String>, AppError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("DB 락 획득 실패: {}", e)))?;

        let mut result = HashMap::new();

        // 각 해시별로 개별 쿼리 (chunk 수가 적으므로 성능 문제 없음)
        let mut stmt = conn
            .prepare(
                "SELECT chunk_hash, translated_json FROM translation_cache WHERE video_id = ?1 AND chunk_hash = ?2",
            )
            .map_err(|e| AppError::Database(format!("쿼리 준비 실패: {}", e)))?;

        for hash in chunk_hashes {
            if let Ok(json) = stmt.query_row(rusqlite::params![video_id, hash], |row| {
                row.get::<_, String>(1)
            }) {
                result.insert(hash.clone(), json);
            }
        }

        Ok(result)
    }
}

/// 청크 라인들로부터 캐시 해시를 생성한다.
/// SHA256(라인 텍스트를 공백으로 join)
pub fn compute_chunk_hash(lines: &[SubtitleLine]) -> String {
    let input: String = lines
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_cache() -> TranslationCache {
        // in-memory SQLite for testing
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA journal_mode=WAL;").unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS translation_cache (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                video_id TEXT NOT NULL,
                chunk_hash TEXT NOT NULL,
                translated_json TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(video_id, chunk_hash)
            );",
        )
        .unwrap();
        TranslationCache {
            conn: Mutex::new(conn),
        }
    }

    #[test]
    fn test_save_and_query() {
        let cache = temp_cache();
        cache
            .save("vid1", "hash1", r#"[{"translated":"안녕"}]"#)
            .unwrap();

        let result = cache.query("vid1", "hash1").unwrap();
        assert_eq!(result, Some(r#"[{"translated":"안녕"}]"#.to_string()));
    }

    #[test]
    fn test_query_miss() {
        let cache = temp_cache();
        let result = cache.query("vid1", "nonexistent").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_upsert() {
        let cache = temp_cache();
        cache.save("vid1", "hash1", "old").unwrap();
        cache.save("vid1", "hash1", "new").unwrap();

        let result = cache.query("vid1", "hash1").unwrap();
        assert_eq!(result, Some("new".to_string()));
    }

    #[test]
    fn test_batch_query() {
        let cache = temp_cache();
        cache.save("vid1", "h1", "json1").unwrap();
        cache.save("vid1", "h2", "json2").unwrap();
        cache.save("vid1", "h3", "json3").unwrap();

        let result = cache
            .batch_query("vid1", &["h1".into(), "h2".into(), "h4".into()])
            .unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result["h1"], "json1");
        assert_eq!(result["h2"], "json2");
        assert!(!result.contains_key("h4"));
    }

    #[test]
    fn test_different_videos_isolated() {
        let cache = temp_cache();
        cache.save("vid1", "hash1", "data1").unwrap();
        cache.save("vid2", "hash1", "data2").unwrap();

        assert_eq!(
            cache.query("vid1", "hash1").unwrap(),
            Some("data1".to_string())
        );
        assert_eq!(
            cache.query("vid2", "hash1").unwrap(),
            Some("data2".to_string())
        );
    }

    #[test]
    fn test_compute_chunk_hash() {
        let lines = vec![
            SubtitleLine {
                text: "Hello".into(),
                start: 0.0,
                end: 1.0,
            },
            SubtitleLine {
                text: "World".into(),
                start: 1.0,
                end: 2.0,
            },
        ];
        let hash = compute_chunk_hash(&lines);
        assert_eq!(hash.len(), 64); // SHA256 hex = 64 chars

        // 같은 입력 → 같은 해시
        let hash2 = compute_chunk_hash(&lines);
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_different_lines_different_hash() {
        let lines1 = vec![SubtitleLine {
            text: "Hello".into(),
            start: 0.0,
            end: 1.0,
        }];
        let lines2 = vec![SubtitleLine {
            text: "World".into(),
            start: 0.0,
            end: 1.0,
        }];
        assert_ne!(compute_chunk_hash(&lines1), compute_chunk_hash(&lines2));
    }
}
