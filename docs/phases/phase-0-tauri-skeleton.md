# Phase 0: Tauri 뼈대 + YouTube 임베드

## 목표

YouTube iframe API가 Tauri WebView2(Windows) 및 WebKit(macOS)에서 정상 작동하는지 검증하고, 앱의 가장 기본적인 뼈대를 구성한다. 이 단계에서 검증되지 않으면 전체 프로젝트의 타당성이 무너지므로 최우선 기술 리스크를 해결한다.

## 검증 리스크

**최고 우선 기술 리스크**: YouTube iframe이 Tauri WebView2에서 정상 로드되고 재생되는가?

- Tauri WebView2는 Chromium 기반이므로 이론상 작동해야 하나, 보안 샌드박스/CORS/콘텐츠 보안 정책(CSP) 제약으로 인해 실제 동작을 보장할 수 없음
- 이 검증에 실패하면 전체 앱 개념을 오버레이 방식으로 되돌려야 함 (POC 문서의 초기 설계)
- Phase 0 완료 = iframe API 동작 확인 = Phase 1부터 번역 로직 안전하게 진행 가능

## 구현 범위

- [ ] **Tauri 2.x + React + TypeScript + Vite 프로젝트 초기화**
  - `npm create tauri-app` 으로 scaffolding
  - React + TypeScript + Vite 템플릿 선택
  - Tauri CLI 1.6+ 설치
  
- [ ] **YouTube URL 입력 컴포넌트**
  - 텍스트 입력 필드 (예: `https://www.youtube.com/watch?v=...`)
  - 파싱 로직: URL → video ID 추출 (regex 또는 URL API)
  - 입력 검증 (YouTube URL 형식 확인)
  
- [ ] **YouTube iframe API 임베드 플레이어 구현**
  - `<iframe>` 태그로 `https://www.youtube.com/embed/{videoId}` 렌더링
  - iframe API 스크립트 로드 및 초기화
  - 플레이어 상태 리스너 설정 (`onStateChange`, `onError`)
  
- [ ] **getCurrentTime() 500ms 폴링으로 재생 시간 추적**
  - `setInterval(player.getCurrentTime(), 500)` 폴링 로직
  - 현재 재생 시간을 UI 상태에 반영
  - 폴링 cleanup (언마운트 시 `clearInterval`)
  
- [ ] **Rust 백엔드 기본 구조 셋업**
  - Tauri command 인터페이스 정의 (나중 Phase의 자막 fetch/번역용 예약)
  - `src-tauri/src/lib.rs` 에 `#[tauri::command]` 매크로로 기본 skeleton 작성
  - 프론트엔드-백엔드 통신 구조 확인
  
- [ ] **개발 환경 설정**
  - TypeScript strict mode 활성화
  - ESLint + Prettier 설정
  - Tauri 핫 리로드 (`tauri dev`)
  - Vite 빌드 설정 확인

## 제외 범위

- YouTube timedtext API 호출 및 자막 fetch (Phase 1)
- Claude Code subprocess 음역 (Phase 2)
- SQLite 캐시 구현 (Phase 2)
- 번역 프롬프트 및 청크 관리 (Phase 2)
- 자막 오버레이 렌더링 (Phase 1)
- Seek 처리 및 버퍼 관리 (Phase 1-2)
- 오류 핸들링 고도화 (Phase 3)

## 기술 상세

### 프로젝트 구조

```
youtube-subtitle-for-claude/
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs           # Rust 백엔드 (command 정의)
│   │   └── main.rs          # Tauri 메인 진입점
│   ├── tauri.conf.json      # Tauri 설정
│   └── Cargo.toml           # Rust 의존성
│
├── src/
│   ├── components/
│   │   ├── YouTubePlayer.tsx    # iframe 플레이어 컴포넌트
│   │   └── UrlInput.tsx         # URL 입력 컴포넌트
│   ├── App.tsx              # 메인 앱 컴포넌트
│   ├── main.tsx             # React 진입점
│   └── styles/
│       └── App.css
│
├── vite.config.ts           # Vite 설정
├── tsconfig.json            # TypeScript 설정
└── package.json
```

### YouTube iframe API 사용법

**YouTube Player 컴포넌트 예시:**

```typescript
import { useEffect, useRef, useState } from 'react';

interface YouTubePlayer {
  getCurrentTime(): number;
  playVideo(): void;
  pauseVideo(): void;
  getPlayerState(): number; // -1: unstarted, 0: ended, 1: playing, 2: paused, 3: buffering, 5: video cued
}

export function YouTubePlayer({ videoId }: { videoId: string }) {
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const playerRef = useRef<YouTubePlayer | null>(null);
  const [currentTime, setCurrentTime] = useState(0);
  const [playerState, setPlayerState] = useState(-1);

  useEffect(() => {
    // 1. 전역 onYouTubeIframeAPIReady 콜백 설정
    const tag = document.createElement('script');
    tag.src = 'https://www.youtube.com/iframe_api';
    document.body.appendChild(tag);

    // 2. YT 객체가 로드되면 플레이어 초기화
    (window as any).onYouTubeIframeAPIReady = () => {
      if (!iframeRef.current) return;
      
      playerRef.current = new (window as any).YT.Player(iframeRef.current, {
        height: '390',
        width: '640',
        videoId: videoId,
        events: {
          onReady: onPlayerReady,
          onStateChange: onPlayerStateChange,
          onError: onPlayerError,
        },
      });
    };

    return () => {
      // cleanup
    };
  }, [videoId]);

  // 3. 500ms 폴링: 현재 재생 시간 추적
  useEffect(() => {
    const pollInterval = setInterval(() => {
      if (playerRef.current) {
        const time = playerRef.current.getCurrentTime();
        setCurrentTime(time);
      }
    }, 500);

    return () => clearInterval(pollInterval);
  }, []);

  const onPlayerReady = (event: any) => {
    console.log('YouTube player ready');
    event.target.playVideo();
  };

  const onPlayerStateChange = (event: any) => {
    setPlayerState(event.data);
    console.log('Player state:', event.data);
  };

  const onPlayerError = (event: any) => {
    console.error('YouTube player error:', event.data);
  };

  return (
    <div>
      <div ref={iframeRef}></div>
      <div>
        Current Time: {currentTime.toFixed(2)}s | State: {playerState}
      </div>
    </div>
  );
}
```

### Tauri 설정 (tauri.conf.json)

WebView2 보안 정책에서 YouTube iframe 로드를 허용하려면:

```json
{
  "build": {
    "devPath": "http://localhost:5173",
    "distDir": "../dist",
    "devUrl": "http://localhost:5173"
  },
  "app": {
    "windows": [
      {
        "fullscreen": false,
        "resizable": true,
        "title": "YouTube Subtitle Translator",
        "width": 1200,
        "height": 800
      }
    ],
    "security": {
      "csp": "default-src 'self' https://www.youtube.com https://www.youtube-nocookie.com; img-src 'self' data: https:; script-src 'self' https://www.youtube.com; frame-src https://www.youtube.com https://www.youtube-nocookie.com"
    }
  }
}
```

**중요**: CSP(Content Security Policy)에서:
- `frame-src https://www.youtube.com https://www.youtube-nocookie.com` 반드시 포함
- `script-src https://www.youtube.com` iframe API 스크립트 로드 허용

### Rust 백엔드 기본 구조 (src-tauri/src/lib.rs)

```rust
#[tauri::command]
fn fetch_subtitles(video_id: String) -> Result<String, String> {
    // Phase 1에서 구현: YouTube timedtext API 호출
    Ok(format!("Subtitles for video {}", video_id))
}

#[tauri::command]
fn translate_chunk(text: String) -> Result<String, String> {
    // Phase 2에서 구현: Claude subprocess 번역
    Ok(format!("Translated: {}", text))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![fetch_subtitles, translate_chunk])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### TypeScript 설정 (tsconfig.json)

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "jsx": "react-jsx",
    "jsxImportSource": "react"
  },
  "include": ["src"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```

## 완료 기준

- [ ] Tauri 앱 `tauri dev` 실행 → 개발 서버 정상 시작
- [ ] YouTube URL 입력 필드에 유효한 URL 입력 가능 (검증 통과)
- [ ] iframe으로 YouTube 비디오 로드 → 플레이어 정상 표시
- [ ] 재생 버튼 클릭 → 영상 재생 시작 확인
- [ ] 500ms 폴링으로 currentTime 값 변화 감지 (UI에 시간 표시)
- [ ] 플레이어 상태 변화 감지 (재생/일시정지/버퍼링 이벤트 콘솔 출력)
- [ ] Rust 백엔드 command 스켈레톤 작성 완료 (나중 호출용 준비)
- [ ] TypeScript strict 모드에서 타입 에러 없음 (`npm run check` 또는 `tsc --noEmit` 통과)
- [ ] 개발 환경에서 핫 리로드 동작 확인 (파일 수정 → 즉시 새로고침)
- [ ] 빌드 성공 (`npm run build` / `tauri build`)

## 다음 Phase 의존성

**Phase 1 (YouTube 자막 Fetch + Subtitle 표시)**은 다음을 Phase 0에서 받음:

1. **동작하는 Tauri + React 프로젝트**: 빌드 및 핫 리로드 환경
2. **iframe 기반 플레이어 아키텍처**: Phase 1에서 자막을 플레이어 아래 오버레이하는 기초
3. **현재 시간 폴링 로직**: Phase 1에서 자막 동기화의 기반
4. **URL → videoId 파싱**: Phase 1에서 자막 fetch API 호출 전제
5. **Rust 백엔드 skeleton**: Phase 1에서 `fetch_subtitles` command 구현

## 실패 시 대안

**시나리오: YouTube iframe이 Tauri WebView2에서 작동하지 않음**

1. **원인 분석**
   - CORS 에러 → CSP 정책 재검토
   - YouTube API 스크립트 로드 실패 → `https:` 프로토콜 확인
   - iframe 콘텐츠 로드 안 됨 → WebView2 버전 업그레이드

2. **대안 선택지**
   - **대안 A: 오버레이 윈도우로 전환** (POC 문서 초기 설계)
     - 브라우저 창 감지 → 브라우저 위에 투명 오버레이 윈도우 띄우기
     - 장점: YouTube 웹사이트 직접 이용 (iframe 문제 회피)
     - 단점: Windows API 연동 필요, 위치 추적 복잡도 증가, macOS 호환성 떨어짐
   
   - **대안 B: Electron으로 전환**
     - Tauri 대신 Electron + Chromium 사용
     - 장점: 더 강력한 API, 기존 모듈 재사용 가능
     - 단점: 번들 크기 ~150MB (목표 15MB 대비 10배), 배포 어려움
   
   - **대안 C: 웹 기반 SPA로 축소**
     - Tauri 앱 포기, Next.js + 로컬 백엔드 API
     - 장점: YouTube iframe 제약 없음
     - 단점: 데스크탑 앱이 아님 (원래 기획 변경)

3. **권장 방향**
   - iframe 실패 확률 낮음 (Chromium 기반 WebView2는 standard compliant)
   - Phase 0 성공률 95% 이상 예상
   - **대안 A (오버레이)**는 iframe 실패 시 최소한의 아키텍처 수정으로 보완 가능하므로, Phase 0 실패 후 신속히 전환 가능
   - **대안 B, C는 근본 재설계 필요** → 의도하지 않은 대안 (최후의 수단)

---

**Phase 0 완료 조건**: YouTube iframe 정상 작동 + Tauri 기본 구조 확인 + 다음 Phase 진행 가능한 상태

**예상 소요 시간**: 2-3 일 (개발 + 테스트)
