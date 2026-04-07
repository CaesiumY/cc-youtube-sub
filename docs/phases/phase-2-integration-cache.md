# Phase 2: 통합 + 캐시

## 목표

Phase 0(YouTube 임베드 플레이어)과 Phase 1(Claude 번역 파이프라인)을 통합하여 실제 시청 환경에서 번역 자막을 실시간으로 표시한다. SQLite 캐시를 도입하여 동일 영상 재방문 시 번역을 재사용함으로써 재원소 낭비를 방지하고, 재생 위치 기반 자막 매칭 및 사전 버퍼링으로 끊김 없는 자막 경험을 제공하는 것을 검증한다.

## 검증 리스크

| 리스크 | 영향 | 완화 전략 |
|--------|------|---------|
| 캐시 키 중복 충돌 | 잘못된 번역 로드 | chunk_hash + video_id 조합 검증, 실제 여러 영상 테스트 |
| 동시 번역 요청 처리 | 자막 표시 순서 뒤바뀜 | 버퍼링 큐의 우선순위 검증, 경쟁 조건 테스트 |
| SQLite 동시성 제약 | 읽기/쓰기 락 대기 | WAL(Write-Ahead Logging) 활성화, 캐시 쓰기는 백그라운드 스레드에서만 |
| Seek 후 버퍼 재스케줄링 | 이전 청크 번역이 여전히 도착 | 버퍼링 세션 ID 또는 타임스탬프로 폐기된 청크 감지 |
| SubtitleOverlay 렌더링 지연 | 자막이 늦게 나타남 | 재생 시간 폴링 500ms → 200ms 단축 테스트 |
| 캐시 miss 시 '번역 준비 중...' 인디케이터 | 사용자 혼란 | seek 위치별 캐시 상태 미리 파악, 로딩 상태 오버레이 박스 내에 명확히 표시 |

## 구현 범위

### 1. Phase 0 + Phase 1 통합
- [ ] Phase 0의 YouTubePlayer 컴포넌트에 Phase 1의 번역 파이프라인 연결
- [ ] URL 입력 → 자막 fetch → 청크 분할 → 번역 시작 플로우 구성
- [ ] 번역 진행률을 UI에 반영 (예: "청크 1/10 번역 중...")
- [ ] 첫 번역 완료 후 플레이어 재생 가능 상태로 전환

### 2. 재생 시간 기반 자막 매칭 로직
- [ ] 현재 재생 시간 → 매칭되는 자막 찾기 알고리즘
- [ ] 자막 배열에서 `start <= currentTime < end` 조건의 자막 검색
- [ ] 이진 검색 또는 포인터 기반 선형 탐색 (성능 고려)
- [ ] 매칭되지 않는 시간대 처리 (자막 표시 안 함)
- [ ] 빠른 seek 후에도 정확한 자막 동기화 확인

### 3. SubtitleOverlay 컴포넌트 (번역 자막 표시)
- [ ] React 컴포넌트: 영상 위 오버레이 (position: absolute, bottom: ~60px — YouTube 컨트롤 바 바로 위)
- [ ] 스타일: 반투명 검정 박스 (rgba(0,0,0,0.75)), 흰 글자, 반응형 폰트 크기
- [ ] 상태 관리:
  - [ ] 현재 표시할 자막 텍스트 (원본/번역)
  - [ ] 번역 진행 상태 (준비 중/완료/오류)
  - [ ] 캐시 상태 표시 (캐시 hit/miss)
  - [ ] 원본 텍스트 토글 상태 (기본: 숨김)
- [ ] UI 요소:
  - [ ] 번역 자막 텍스트 영역 (한국어) — 기본 표시
  - [ ] 원본 자막 텍스트 (T키 토글 시 번역 아래 14px 회색으로 표시)
  - [ ] 로딩 인디케이터 (캐시 miss 시 오버레이 박스 내에 "번역 준비 중..." 표시)
- [ ] 키보드 단축키:
  - [ ] T: 원본 텍스트 토글 (번역 아래 원문 표시/숨김)
  - [ ] +/-: 자막 폰트 크기 조절
  - [ ] Space: 재생/일시정지

### 3-1. ProgressBar (번역 진행률 표시)
- [ ] 영상 컨테이너 바로 아래 2px 얇은 진행률 바
- [ ] 번역 진행 상태에 따라 너비 증가 (0% → 100%)
- [ ] 전체 번역 완료 시 자동 사라짐 (fade-out)

### 4. SQLite 스키마 설계 + tauri-plugin-sql 연동
- [ ] Cargo.toml에 tauri-plugin-sql 의존성 추가
- [ ] 초기화 마이그레이션: `CREATE TABLE IF NOT EXISTS` 스크립트
- [ ] 스키마 설계:
  ```sql
  CREATE TABLE translation_cache (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    video_id TEXT NOT NULL,
    chunk_hash TEXT NOT NULL,
    translated_json TEXT NOT NULL,  -- [{original, translated, start, end}, ...]
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(video_id, chunk_hash)
  );
  CREATE INDEX idx_video_chunk ON translation_cache(video_id, chunk_hash);
  ```
- [ ] WAL(Write-Ahead Logging) 활성화: `PRAGMA journal_mode=WAL;`
- [ ] 연결 풀 설정 (동시 다중 쿼리 대비)

### 5. 캐시 저장 (번역 완료 시 자동 저장)
- [ ] 번역 결과 도착 → 구조 검증 → SQLite INSERT
- [ ] 중복 키 처리: `INSERT OR REPLACE` (또는 `ON CONFLICT`)
- [ ] chunk_hash 생성 알고리즘:
  - [ ] 청크의 원본 자막 텍스트를 SHA256로 해싱
  - [ ] 동일 청크 내용이면 같은 해시 (language-agnostic)
  - [ ] 예: `SHA256(chunk_lines.join(" "))` → 16진수 문자열
- [ ] 백그라운드 스레드에서 쓰기 (UI 블로킹 방지)
- [ ] 쓰기 실패 시 로깅만 진행 (재시도 없음, Phase 2 범위)

### 6. 캐시 조회 (영상 로드 시 캐시 우선 확인)
- [ ] 영상 URL 입력 → video ID 추출 → 캐시 사전 조회
- [ ] 각 청크별로 캐시 확인:
  - [ ] 캐시 hit: 캐시된 번역 즉시 반환
  - [ ] 캐시 miss: 번역 파이프라인 실행
- [ ] 쿼리: `SELECT translated_json FROM translation_cache WHERE video_id = ? AND chunk_hash = ?`
- [ ] 쿼리 실패 시 에러 로깅 및 번역 진행 (cache fallback)

### 7. 재방문 시 즉시 자막 로드 플로우
- [ ] 사용자가 이전에 본 영상 URL 재입력
- [ ] 전체 청크 캐시 조회 (1회 배치 쿼리로 최적화)
- [ ] 캐시된 모든 청크를 메모리로 로드
- [ ] 플레이어 재생 준비 완료 (UI 진행률 100%)
- [ ] 재생 시작 → SubtitleOverlay가 캐시된 자막 즉시 표시
- [ ] 예상 지연: 0~1초 (네트워크/DB 없음)

### 8. 번역 진행률 UI (ProgressBar)
- [ ] 영상 컨테이너 바로 아래 2px 얇은 ProgressBar
- [ ] 상태 업데이트:
  - [ ] 청크 번역 시작 → 진행률 증가
  - [ ] 캐시 hit → 진행률 즉시 증가
  - [ ] 모든 청크 완료 → 100% 후 fade-out
- [ ] 진행 중 텍스트 표시 (ProgressBar 옆 또는 아래): "청크 2/10 번역 중..."
- [ ] 캐시 히트 수 표시: "캐시에서 로드됨 8개"

### 9. Seek 처리 (사용자가 영상 중간으로 점프)
- [ ] seek 이벤트 감지 (플레이어 onSeek 콜백)
- [ ] 새 위치의 캐시 상태 확인:
  - [ ] 캐시 hit: 즉시 자막 표시
  - [ ] 캐시 miss: "번역 준비 중..." 표시 + 해당 청크 우선 번역
- [ ] 버퍼링 큐 재스케줄링:
  - [ ] 이전 대기 중인 청크 번역 취소
  - [ ] 새 위치 기준 사전 버퍼링 청크 재계산
  - [ ] 새 세션 ID 부여 (이전 청크 도착 시 자동 폐기)

### 10. 번역 버퍼링 전략 (사전 버퍼링)
- [ ] 현재 재생 시간 기준 선행 청크 계산
- [ ] 선행 거리: 현재 위치 + 30초 (다음 청크 미리 번역)
- [ ] 버퍼 큐: `[{chunk_index, priority, status}, ...]`
- [ ] 우선순위 조정:
  - [ ] 현재 재생 중인 청크: 최고 우선
  - [ ] 다음 청크 (30초 선행): 높음
  - [ ] 그 다음 청크들: 낮음
- [ ] 버퍼 관리:
  - [ ] 최대 동시 번역: 2개 (Claude 한도 고려)
  - [ ] 낮은 우선순위 청크는 필요 시 취소 (리소스 절약)

## 제외 범위

- 사용자 설정 저장 (캐시 경로, 언어 설정 등)
- 시청 이력 및 재개 위치 (v1)
- 번역 품질 피드백 시스템 (v1)
- 캐시 TTL(Time-To-Live) 정책 (POC에서는 무제한 보관)
- 캐시 크기 제한 (POC에서는 무제한)
- 다중 언어 번역 (영어 → 한국어만)
- 사용자 계정/클라우드 동기화 (로컬 DB만)

## 기술 상세

### SQLite 스키마

```sql
CREATE TABLE translation_cache (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  video_id TEXT NOT NULL,
  chunk_hash TEXT NOT NULL,
  translated_json TEXT NOT NULL,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(video_id, chunk_hash)
);

CREATE INDEX idx_video_chunk ON translation_cache(video_id, chunk_hash);

-- WAL 활성화 (동시성 개선)
PRAGMA journal_mode=WAL;
PRAGMA synchronous=NORMAL;
```

### chunk_hash 생성 로직 (Rust)

```rust
use sha2::{Sha256, Digest};

fn compute_chunk_hash(chunk_lines: &[String]) -> String {
    let combined_text = chunk_lines.join(" ");
    let mut hasher = Sha256::new();
    hasher.update(combined_text.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)  // 16진수 문자열
}
```

### 자막 매칭 알고리즘 (TypeScript)

```typescript
interface Subtitle {
  original: string;
  translated: string;
  start: number;
  end: number;
}

function findMatchingSubtitle(
  currentTime: number,
  subtitles: Subtitle[]
): Subtitle | null {
  // 이진 검색: O(log n)
  let left = 0, right = subtitles.length - 1;
  
  while (left <= right) {
    const mid = Math.floor((left + right) / 2);
    const sub = subtitles[mid];
    
    if (sub.start <= currentTime && currentTime < sub.end) {
      return sub;
    } else if (sub.start > currentTime) {
      right = mid - 1;
    } else {
      left = mid + 1;
    }
  }
  
  return null;
}
```

### SubtitleOverlay 컴포넌트 (React + TypeScript)

오버레이는 영상 위에 position: absolute로 배치되며, YouTube 컨트롤 바 바로 위(bottom: ~60px)에 위치한다.

```typescript
interface SubtitleOverlayProps {
  subtitle: Subtitle | null;
  isLoading: boolean;
  cacheStatus: 'hit' | 'miss' | 'idle';
  showOriginal: boolean;  // T키 토글 상태
}

export function SubtitleOverlay({
  subtitle,
  isLoading,
  cacheStatus,
  showOriginal,
}: SubtitleOverlayProps) {
  return (
    // position: absolute, bottom: ~60px, 영상 컨테이너 기준
    <div className="subtitle-overlay">
      {isLoading && (
        <div className="loading">번역 준비 중...</div>
      )}

      {subtitle && !isLoading && (
        <div className="subtitle-text">
          {/* 번역 자막 — 기본 표시 */}
          <div className="translated">{subtitle.translated}</div>
          {/* 원문 — T키 토글 시 번역 아래 14px 회색으로 표시 */}
          {showOriginal && (
            <div className="original">{subtitle.original}</div>
          )}
        </div>
      )}
    </div>
  );
}

// 스타일 (CSS)
// .subtitle-overlay {
//   position: absolute;
//   bottom: 60px;  /* YouTube 컨트롤 바 높이 위 */
//   left: 50%;
//   transform: translateX(-50%);
//   background: rgba(0, 0, 0, 0.75);
//   padding: 8px 16px;
//   border-radius: 4px;
// }
// .translated { color: oklch(0.98 0 0); font-size: 18px; }
// .original   { color: oklch(0.556 0 0); font-size: 14px; margin-top: 4px; }
```

### tauri-plugin-sql 초기화 (Rust)

```rust
use tauri_plugin_sql::{Migration, MigrationManager};

#[tauri::command]
async fn init_database(app_handle: tauri::AppHandle) -> Result<(), String> {
    let manager = MigrationManager::new(&app_handle);
    
    manager
        .run(vec![
            Migration {
                version: 1,
                description: "Create translation_cache table",
                sql: r#"
                    CREATE TABLE IF NOT EXISTS translation_cache (
                      id INTEGER PRIMARY KEY AUTOINCREMENT,
                      video_id TEXT NOT NULL,
                      chunk_hash TEXT NOT NULL,
                      translated_json TEXT NOT NULL,
                      created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                      UNIQUE(video_id, chunk_hash)
                    );
                    CREATE INDEX IF NOT EXISTS idx_video_chunk 
                      ON translation_cache(video_id, chunk_hash);
                "#
                .to_string(),
                kind: tauri_plugin_sql::MigrationKind::Up,
            },
        ])
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(())
}
```

### 캐시 저장 (백그라운드 스레드)

```rust
async fn save_to_cache(
    video_id: String,
    chunk_hash: String,
    translated_json: String,
    db: &tauri_plugin_sql::Database,
) -> Result<(), String> {
    // 스레드 풀에서 실행 (UI 블로킹 안 함)
    tokio::spawn(async move {
        let query = r#"
            INSERT OR REPLACE INTO translation_cache 
              (video_id, chunk_hash, translated_json)
            VALUES (?, ?, ?)
        "#;
        
        if let Err(e) = db.execute(query, [&video_id, &chunk_hash, &translated_json]).await {
            eprintln!("Cache save error: {}", e);
        }
    });
    
    Ok(())
}
```

### 캐시 조회

```rust
async fn query_cache(
    video_id: String,
    chunk_hash: String,
    db: &tauri_plugin_sql::Database,
) -> Result<Option<String>, String> {
    let query = r#"
        SELECT translated_json FROM translation_cache
        WHERE video_id = ? AND chunk_hash = ?
    "#;
    
    let result = db.select(query, [&video_id, &chunk_hash]).await
        .map_err(|e| e.to_string())?;
    
    if let Some(mut row) = result {
        return Ok(Some(row.take::<String, _>(0).map_err(|e| e.to_string())?));
    }
    
    Ok(None)
}
```

### 재생 시간 폴링 + 자막 동기화 (React)

```typescript
useEffect(() => {
  const pollInterval = setInterval(() => {
    if (playerRef.current) {
      const currentTime = playerRef.current.getCurrentTime();
      const matchingSubtitle = findMatchingSubtitle(currentTime, allSubtitles);
      setCurrentSubtitle(matchingSubtitle);
    }
  }, 200);  // 200ms 폴링 (5fps 자막 갱신)
  
  return () => clearInterval(pollInterval);
}, [allSubtitles]);
```

## 완료 기준

### 기능 완료
- [ ] Phase 0 + Phase 1 통합: URL 입력 → 자막 fetch → 번역 → 표시 전체 플로우 동작
- [ ] SubtitleOverlay 컴포넌트 구현 및 렌더링 확인 (영상 위 오버레이, bottom: ~60px)
- [ ] 키보드 단축키 동작: T(원문 토글), +/-(폰트 크기), Space(재생/일시정지)
- [ ] ProgressBar: 영상 컨테이너 아래 2px 바, 번역 완료 시 fade-out
- [ ] SQLite 테이블 생성 및 쿼리 성공 (tauri-plugin-sql 연동)
- [ ] 번역 결과 자동 캐시 저장 (백그라운드 스레드)
- [ ] 캐시 조회 동작:
  - [ ] 캐시 hit: 0.1초 이내 반환 (디비 로드)
  - [ ] 캐시 miss: 번역 파이프라인 실행
- [ ] 재방문 시 즉시 자막 로드:
  - [ ] 동일 영상 URL 재입력 → 0~1초 내 자막 준비
  - [ ] 재생 시작 → 캐시된 자막 동기화 표시
- [ ] 재생 시간 기반 자막 매칭:
  - [ ] currentTime 폴링으로 정확한 자막 동기화 (±200ms 오차)
  - [ ] 빠른 seek 후에도 올바른 자막 표시
- [ ] Seek 처리:
  - [ ] 캐시 hit 위치: 즉시 자막 표시
  - [ ] 캐시 miss 위치: 오버레이 박스 내 "번역 준비 중..." 표시 + 해당 청크 우선 번역
- [ ] 번역 진행률 UI:
  - [ ] 청크 완료 시마다 진행률 업데이트
  - [ ] 캐시 히트 수 표시
  - [ ] 최종 "준비 완료" 상태 표시

### 성능 기준
- [ ] 재방문 시 자막 로드: < 1초 (캐시 쿼리 + 메모리 로드)
- [ ] 자막 동기화 지연: ±200ms 이내
- [ ] 캐시 저장 오버헤드: UI 블로킹 없음 (백그라운드 스레드)
- [ ] SQLite 쿼리 응답: < 100ms (인덱스 기반)

### 에러 처리
- [ ] 캐시 DB 초기화 실패: 사용자에게 안내 메시지, 번역 진행 (DB fallback)
- [ ] 캐시 저장 실패: 로깅만 진행 (재시도 없음), 번역 흐름 계속
- [ ] 캐시 쿼리 오류: 캐시 miss 처리로 진행 (안전한 fallback)
- [ ] Seek 중복 이벤트: 이전 버퍼링 취소 후 새로 스케줄링

### 테스트 시나리오
- [ ] **첫 방문 플로우**: URL 입력 → 자막 fetch → 첫 청크 5초 내 번역 → 플레이어 재생 가능 → 자막 표시
- [ ] **재방문 플로우**: 이전 영상 URL 재입력 → 캐시에서 즉시 로드 → 0~1초 내 "준비 완료" → 재생 → 자막 동기화
- [ ] **Seek 시나리오**:
  - [ ] 캐시 hit 위치로 seek → 즉시 자막 표시
  - [ ] 캐시 miss 위치로 seek → "번역 준비 중..." → 해당 청크 번역 → 자막 전환
- [ ] **장시간 영상**: 1시간 강의 입력 → 청크 분할 → 사전 버퍼링으로 끊김 없는 자막 표시
- [ ] **캐시 무결성**: 여러 영상 번역 후 DB 확인 → video_id + chunk_hash 조합 정확성

## 다음 Phase 의존성

**Phase 3 (고도화 및 최적화)**는 Phase 2의 다음 결과물에 의존:

1. **캐시 쿼리 인터페이스 확정**
   - 메인 UI 에러 처리 및 재시도 정책
   - 캐시 만료/갱신 정책 (v1 요구사항)

2. **번역 버퍼링 성능 기준**
   - 선행 거리 최적화 (현재 30초 → 조정 가능)
   - 동시 번역 개수 한도 (현재 2개)

3. **UI 상태 관리 구조**
   - 플레이어 상태 + 번역 상태 + 캐시 상태 통합
   - 복잡한 상태 변화 시나리오 처리 (seek + 버퍼링 + 캐시)

## 실패 시 대안

| 시나리오 | 원인 | 대안 |
|---------|------|------|
| 캐시 중복 충돌 | chunk_hash 동일성 부족 | video_id + chunk_index (순번) 복합 키로 전환 |
| SQLite 동시성 병목 | 많은 청크 동시 쓰기 | Redis 또는 인메모리 캐시로 전환 (개발 복잡도 증가) |
| Seek 후 버퍼 혼란 | 세션 ID 미구현 | 각 seek마다 고유 session_id 부여, 폐기 청크 자동 감지 |
| 자막 동기화 지연 | 폴링 간격 500ms → 너무 김 | 폴링 간격 200ms로 단축 (CPU 부하 미미) |
| 첫 번역 지연 > 5초 | Claude 응답 느림 | 더 간단한 프롬프트로 토큰 감소, timeout 재조정 |
| 캐시 hit 확률 낮음 | chunk_hash 불안정 | 자막 텍스트 정규화 (공백/구두점 제거) 후 해싱 |

---

**Phase 2 완료 조건**: 캐시와 버퍼링이 안정적으로 작동하여 재방문 시 0~1초 내 자막 로드, 실시간 자막 표시 지연 ±200ms 이내

**예상 소요 시간**: 4-5일 (개발 + 테스트 + 캐시 검증)
