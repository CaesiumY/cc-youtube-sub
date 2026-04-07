# Design System: YouTube 번역 자막 앱

> shadcn/ui Luma 프리셋 기반 | 다크 모드 우선 | 시네마 UX
> 디자인 인터뷰: 7라운드, 모호성 19%

---

## 0. 디자인 결정 요약

| 결정 | 선택 | 이유 |
|------|------|------|
| 디자인 프리셋 | shadcn/ui Luma (Neutral OKLCH) | 무채색 팔레트가 영상 콘텐츠를 돋보이게 함 |
| 레이아웃 | 시네마 싱글 컬럼 | 영상이 전체 너비 차지, 자막은 영상 아래 전용 영역 |
| 자막 표시 | 번역만 기본, 원문은 토글(T키) | 미니멀한 시청 경험, 필요 시 원문 확인 |
| 빈 상태 | URL 입력만 (로고/브랜딩 없음) | 최대한 깨끗하게, 행동 유도에 집중 |
| 키보드 단축키 | T(원문 토글), +/-(자막 크기), Space(재생) | POC 기본 세트 |
| YouTube 플레이어 | Phase 0에서 lite-youtube vs 직접 iframe 둘 다 테스트 | 초기 로딩 성능 vs Player API 안정성 비교 검증 |
| 환경 검증 UX | 모달 다이얼로그 (앱 차단) | Claude CLI 미설치 시 설치 가이드를 명확히 안내 |
| 한글 폰트 | Pretendard | Inter와 메트릭 호환, 한글 최적화 |

---

## 1. 디자인 철학

### 컨셉: "Cinema Noir"

이 앱은 번역 도구가 아니라 **프리미엄 비디오 플레이어**다. 극장에서 자막 영화를 보는 경험 — 스크린(영상)과 자막만 보이고, 나머지는 어둠 속에 사라진다.

**핵심 원칙:**

| 원칙 | 설명 |
|------|------|
| 영상이 주인공 | 모든 UI 요소는 영상에 시선을 양보한다 |
| 자막은 조연 | 읽기 쉽지만 영상을 가리지 않는 위치와 크기 |
| Chrome은 투명인간 | URL 바, 상태 표시는 필요할 때만 나타난다 |
| 상태는 분위기로 | 로딩/에러를 팝업이 아닌 색상/애니메이션으로 전달 |

### 차별화 포인트

> "자막이 영상 위에 떠다니는 게 아니라, 영상 아래 전용 스테이지에서 공연한다"

대부분의 자막 앱은 영상 위에 텍스트를 오버레이한다. 이 앱은 **자막 전용 영역**을 영상 아래에 배치한다:
- YouTube 자체 UI/자막과 충돌하지 않음
- 배경이 통제되어 가독성 보장
- 원문 + 번역을 동시에 보여줄 수 있음
- 번역 상태 표시를 자연스럽게 통합

---

## 2. 색상 시스템

### shadcn Luma Neutral (OKLCH)

다크 모드를 기본으로 사용한다. 무채색 팔레트가 영상 콘텐츠를 돋보이게 한다.

#### 다크 모드 (기본)

```css
:root {
  /* 배경 계층 */
  --background: oklch(0.145 0 0);          /* 앱 배경 — 거의 검정 */
  --card: oklch(0.205 0 0);                /* 카드/패널 — 약간 밝은 검정 */
  --popover: oklch(0.205 0 0);             /* 팝오버 */

  /* 텍스트 */
  --foreground: oklch(0.985 0 0);          /* 주요 텍스트 — 거의 흰색 */
  --muted-foreground: oklch(0.708 0 0);    /* 보조 텍스트 — 중간 회색 */

  /* 인터랙티브 */
  --primary: oklch(0.922 0 0);             /* 버튼/링크 */
  --secondary: oklch(0.269 0 0);           /* 보조 버튼 배경 */
  --accent: oklch(0.269 0 0);              /* 호버/포커스 */
  --destructive: oklch(0.704 0.191 22.216); /* 에러 */

  /* 구조 */
  --border: oklch(1 0 0 / 10%);            /* 테두리 — 흰색 10% */
  --input: oklch(1 0 0 / 15%);             /* 입력 필드 테두리 */
  --ring: oklch(0.556 0 0);                /* 포커스 링 */
  --radius: 0.625rem;                       /* 기본 반경 */
}
```

#### 앱 전용 시맨틱 토큰

```css
:root {
  /* 자막 영역 */
  --subtitle-bg: oklch(0.175 0 0);         /* 자막 패널 배경 */
  --subtitle-text: oklch(0.98 0 0);        /* 번역 자막 텍스트 */
  --subtitle-original: oklch(0.556 0 0);   /* 원문 텍스트 (dimmed) */
  --subtitle-border: oklch(1 0 0 / 6%);    /* 자막 영역 상단 구분선 */

  /* 번역 상태 */
  --progress-track: oklch(0.269 0 0);      /* 진행바 트랙 */
  --progress-fill: oklch(0.708 0 0);       /* 진행바 채움 */
  --progress-active: oklch(0.85 0 0);      /* 번역 진행 중 (밝게) */

  /* 상태 색상 */
  --status-translating: oklch(0.75 0.1 250);  /* 번역 중 — 차가운 파랑 */
  --status-cached: oklch(0.75 0.15 160);      /* 캐시 hit — 초록 */
  --status-error: oklch(0.704 0.191 22.216);  /* 에러 — 빨강 */
}
```

#### 라이트 모드 (선택적)

영상 시청 앱이므로 다크 모드가 기본이지만, 라이트 모드도 지원한다. Luma의 라이트 토큰을 그대로 사용:

```css
.light {
  --background: oklch(1 0 0);
  --foreground: oklch(0.145 0 0);
  --card: oklch(1 0 0);
  --subtitle-bg: oklch(0.97 0 0);
  --subtitle-text: oklch(0.145 0 0);
  --subtitle-original: oklch(0.556 0 0);
}
```

---

## 3. 타이포그래피

### 폰트 스택

| 용도 | 폰트 | 이유 |
|------|------|------|
| UI 텍스트 | **Pretendard** | Inter와 메트릭 호환, 한글 최적화 |
| 번역 자막 | **Pretendard SemiBold** | 영상 아래에서 빠르게 읽혀야 하므로 약간 굵게 |
| 원문 자막 | **Pretendard Regular** | 번역보다 한 단계 가볍게 |
| 모노스페이스 | **JetBrains Mono** | 타임코드, 디버그 정보 |

### 타입 스케일

```css
:root {
  --font-family: 'Pretendard', 'Inter', -apple-system, sans-serif;

  /* UI */
  --text-xs: 0.75rem;      /* 12px — 메타 정보, 타임코드 */
  --text-sm: 0.875rem;     /* 14px — 보조 텍스트, 상태 */
  --text-base: 1rem;       /* 16px — 기본 UI */
  --text-lg: 1.125rem;     /* 18px — 강조 */

  /* 자막 */
  --subtitle-size: 1.25rem;       /* 20px — 번역 자막 (기본) */
  --subtitle-size-lg: 1.5rem;     /* 24px — 큰 자막 모드 */
  --subtitle-original-size: 0.875rem; /* 14px — 원문 */

  /* 행간 */
  --leading-subtitle: 1.6;        /* 자막 행간 (넉넉하게) */
  --leading-ui: 1.5;              /* UI 행간 */
}
```

---

## 4. 레이아웃

### 추천 레이아웃: "시네마 싱글 컬럼"

```
┌──────────────────────────────────────────────────────┐
│                                                      │
│   ┌──────────────────────────────────────────────┐   │ ← URL Bar
│   │  🔗  YouTube URL을 붙여넣으세요...            │   │    (auto-hide on play)
│   └──────────────────────────────────────────────┘   │
│                                                      │
│   ┌──────────────────────────────────────────────┐   │
│   │                                              │   │
│   │                                              │   │
│   │              YouTube Player                  │   │ ← 16:9 aspect ratio
│   │              (iframe embed)                  │   │    fills available width
│   │                                              │   │
│   │                                              │   │
│   └──────────────────────────────────────────────┘   │
│                                                      │
│   ┌──────────────────────────────────────────────┐   │ ← Subtitle Stage
│   │                                              │   │
│   │     여러분 안녕하세요, 오늘 강의에             │   │    translated (large, bright)
│   │     오신 것을 환영합니다                       │   │
│   │                                              │   │
│   │     Hello everyone, welcome to               │   │    original (small, muted)
│   │     today's lecture                           │   │
│   │                                              │   │
│   │  ═══════════════════════░░░░  80%  3/10 청크  │   │ ← progress (inline, subtle)
│   └──────────────────────────────────────────────┘   │
│                                                      │
└──────────────────────────────────────────────────────┘
```

### 왜 이 레이아웃인가

| 대안 | 문제 | 이 레이아웃의 장점 |
|------|------|-------------------|
| 사이드바 + 콘텐츠 | 영상 면적 감소, POC에 불필요한 복잡도 | 영상이 전체 너비를 차지 |
| 자막 오버레이 (플레이어 위) | YouTube 자체 자막/UI와 충돌 | 자막 전용 공간에서 충돌 없음 |
| 하단 패널 분리 | 자막과 영상이 분리된 느낌 | 자막이 영상의 자연스러운 연장 |

### 앱 상태별 레이아웃 변화

#### State 1: 빈 상태 (앱 시작)

```
┌──────────────────────────────────────────────────┐
│                                                  │
│                                                  │
│                                                  │
│                                                  │
│                                                  │
│    ┌──────────────────────────────────────────┐   │
│    │  🔗  YouTube URL을 붙여넣으세요...       │   │  ← 중앙 대형 URL 입력
│    └──────────────────────────────────────────┘   │
│                                                  │
│                                                  │
│                                                  │
│                                                  │
└──────────────────────────────────────────────────┘
```

- 브랜딩/로고 없이 URL 입력만 화면 중앙에 표시
- 최대한 깨끗한 첫 화면, 행동 유도에만 집중
- Cmd/Ctrl+V 붙여넣기 자동 감지

#### State 2: 로딩 (자막 fetch + 번역 시작)

```
┌──────────────────────────────────────────────────┐
│  ┌────────────────────────────────────────────┐  │
│  │ 🔗 https://youtube.com/watch?v=...    ✕   │  │  ← URL 상단으로 이동
│  └────────────────────────────────────────────┘  │
│                                                  │
│  ┌────────────────────────────────────────────┐  │
│  │                                            │  │
│  │         YouTube Player (loading)           │  │
│  │                                            │  │
│  └────────────────────────────────────────────┘  │
│                                                  │
│  ┌────────────────────────────────────────────┐  │
│  │                                            │  │
│  │    ◐  자막을 번역하고 있습니다...           │  │  ← 로딩 상태
│  │       영상 설명을 분석하는 중               │  │
│  │                                            │  │
│  │  ════════░░░░░░░░░░░░░░░░░░  10%          │  │
│  └────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────┘
```

- URL 입력이 상단 바로 축소
- 플레이어는 이미 로드 (바로 재생 가능)
- 자막 영역에 번역 진행 상태 표시

#### State 3: 재생 중 (핵심 상태)

```
┌──────────────────────────────────────────────────┐
│  🔗 YouTube 강의 제목...              ⚙  🌙     │  ← 호버시만 표시 (auto-hide)
│                                                  │
│  ┌────────────────────────────────────────────┐  │
│  │                                            │  │
│  │                                            │  │
│  │         YouTube Player (재생 중)            │  │
│  │                                            │  │
│  │                                            │  │
│  └────────────────────────────────────────────┘  │
│                                                  │
│  ┌────────────────────────────────────────────┐  │
│  │                                            │  │
│  │    여러분 안녕하세요, 오늘 강의에           │  │  ← 번역만 표시 (기본)
│  │    오신 것을 환영합니다                     │  │
│  │                                            │  │
│  │    Hello everyone, welcome to today's      │  │  ← 원문 (T키 토글, 기본 숨김)
│  │    lecture                                 │  │
│  │                                            │  │
│  └────────────────────────────────────────────┘  │
│  ───────────────────────────────── cached ✓      │  ← 상태 라인 (미니멀)
└──────────────────────────────────────────────────┘
```

- URL 바는 auto-hide (마우스 호버 시 표시)
- 영상이 최대 면적 차지
- 자막 영역은 항상 표시 (영상의 자연스러운 확장)
- 하단 상태 라인은 한 줄 (cached ✓ / translating... / error)

#### State 4: Seek → 캐시 miss

```
  ┌────────────────────────────────────────────┐
  │                                            │
  │    ◌  번역 준비 중...                       │  ← 자막 영역에 shimmer 효과
  │       ━━━━━━━━░░░░░░                       │
  │                                            │
  └────────────────────────────────────────────┘
```

#### State 5: 에러

```
  ┌────────────────────────────────────────────┐
  │                                            │
  │    ⚠  이 영상에는 자막이 없습니다           │  ← 에러 메시지
  │       자동 자막이 있는 영상을 시도해주세요   │
  │                                            │
  └────────────────────────────────────────────┘
```

---

## 5. 컴포넌트 아키텍처

### 컴포넌트 트리

```
App
├── URLBar                    ← 상단 URL 입력 (auto-hide 가능)
│   ├── URLInput              ← shadcn Input + 붙여넣기 감지
│   └── ActionButtons         ← 설정, 테마 토글
│
├── EmptyState                ← 빈 상태 (URL 미입력)
│   ├── AppLogo
│   ├── WelcomeText
│   └── URLInputLarge         ← 중앙 대형 입력
│
├── VideoPlayer               ← YouTube iframe 래퍼
│   ├── YouTubeEmbed          ← iframe API 관리
│   └── PlayerSkeleton        ← 로딩 placeholder
│
├── SubtitleStage             ← 자막 표시 영역 (핵심 컴포넌트)
│   ├── TranslatedText        ← 번역 자막 (크고 밝게)
│   ├── OriginalText          ← 원문 (작고 흐리게)
│   ├── LoadingState          ← "번역 준비 중..." shimmer
│   └── ErrorState            ← 에러 메시지
│
├── StatusLine                ← 하단 상태 표시줄
│   ├── ProgressBar           ← 번역 진행률
│   ├── ChunkInfo             ← "3/10 청크"
│   └── CacheStatus           ← "cached ✓" / "translating..."
│
└── EnvironmentCheck          ← 앱 시작 시 Claude CLI 검증 모달
```

### shadcn 컴포넌트 매핑

| 앱 컴포넌트 | shadcn 베이스 | 커스터마이징 |
|------------|--------------|-------------|
| URLInput | `Input` | rounded-4xl, 대형 높이, 붙여넣기 아이콘 |
| ActionButtons | `Button` (ghost variant) | 아이콘 전용, 호버 시 툴팁 |
| SubtitleStage | `Card` | 커스텀 배경, 상단 border-t 구분선 |
| ProgressBar | `Progress` | 얇은 라인, subtle 색상 |
| ErrorState | `Alert` | destructive variant, 인라인 |
| EnvironmentCheck | `Dialog` | 앱 시작 시 한 번만 표시 |
| StatusLine | — | 커스텀 (shadcn 없음) |

---

## 6. 자막 표시 상세

### SubtitleStage 디자인

자막 영역은 이 앱의 **핵심 UX 차별화 포인트**다.

```css
.subtitle-stage {
  /* 영상과 자연스럽게 연결되는 느낌 */
  background: var(--subtitle-bg);
  border-top: 1px solid var(--subtitle-border);
  padding: 1.5rem 2rem;
  min-height: 120px;
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  gap: 0.75rem;
}

.subtitle-translated {
  font-family: var(--font-family);
  font-size: var(--subtitle-size);         /* 20px */
  font-weight: 600;
  color: var(--subtitle-text);
  line-height: var(--leading-subtitle);
  text-align: center;
  max-width: 80%;

  /* 부드러운 전환 — 자막이 바뀔 때 */
  animation: subtitle-fade-in 0.3s ease-out;
}

.subtitle-original {
  font-size: var(--subtitle-original-size); /* 14px */
  font-weight: 400;
  color: var(--subtitle-original);          /* dimmed */
  text-align: center;
  max-width: 80%;
}

@keyframes subtitle-fade-in {
  from {
    opacity: 0;
    transform: translateY(4px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
```

### 자막 전환 애니메이션

자막이 바뀔 때 **부드러운 fade + 미세한 상승** 효과:

| 상태 | 애니메이션 |
|------|-----------|
| 새 자막 등장 | fade-in + translateY(4px → 0) over 300ms |
| 자막 사라짐 | fade-out over 200ms |
| 로딩 (캐시 miss) | shimmer 효과 (왼쪽→오른쪽 gradient sweep) |
| 에러 | 즉시 표시, subtle shake (2px, 300ms) |

---

## 7. 인터랙션 패턴

### URL 입력 플로우

```
1. 앱 시작 → 빈 상태 (중앙 URL 입력)
2. URL 붙여넣기 → 즉시 video ID 파싱
   ├─ 유효한 URL → 플레이어 로드 + 자막 fetch 시작
   │                URL 바가 상단으로 슬라이드 (400ms ease)
   │                빈 상태가 fade-out (300ms)
   └─ 무효한 URL → Input에 빨간 테두리 + shake 애니메이션
3. 새 URL 입력 → 기존 영상 교체 (확인 없이 즉시)
```

### URL Bar Auto-hide

재생 중에는 URL 바가 자동으로 사라져 영상 면적을 극대화:

```
재생 시작 → 3초 후 URL 바 fade-out (opacity: 0, height: 0)
마우스가 상단 48px 영역 진입 → URL 바 slide-down 표시
마우스가 영역 이탈 → 2초 후 fade-out
URL 바 클릭/포커스 중 → 항상 표시
일시정지 → 항상 표시
```

### 번역 진행 표시

```
번역 시작 → StatusLine에 progress bar 등장 (fade-in)
각 청크 완료 → progress bar 증가 (ease-out 전환)
                ChunkInfo 업데이트 ("3/10 청크")
전체 완료 → progress bar fade-out
             "cached ✓" 표시 (2초 후 fade-out)
```

### Seek 시 피드백

```
Seek (캐시 hit) → 자막 즉시 전환 (fade-in 300ms)
Seek (캐시 miss) → SubtitleStage에 shimmer 로딩
                    StatusLine: "번역 준비 중..."
                    해당 청크 번역 완료 → 자막 표시
```

---

## 8. 윈도우 설정

### Tauri 윈도우 기본값

```json
{
  "label": "main",
  "title": "ClaudeSub",
  "width": 960,
  "height": 700,
  "minWidth": 640,
  "minHeight": 480,
  "resizable": true,
  "decorations": true,
  "transparent": false,
  "center": true
}
```

### 반응형 동작

| 윈도우 너비 | 동작 |
|------------|------|
| ≥ 960px | 기본 레이아웃, 자막 max-width: 80% |
| 640-959px | 자막 max-width: 90%, 폰트 약간 축소 |
| < 640px | 지원하지 않음 (minWidth: 640) |

### 16:9 영상 비율 유지

```css
.video-container {
  position: relative;
  width: 100%;
  padding-bottom: 56.25%; /* 16:9 */
  background: oklch(0.1 0 0);
  border-radius: var(--radius);
  overflow: hidden;
}

.video-container iframe {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
}
```

---

## 9. 간격 시스템

Luma의 기본 spacing에 앱 전용 값 추가:

```css
:root {
  /* 앱 레이아웃 */
  --app-padding: 1rem;            /* 앱 가장자리 여백 */
  --section-gap: 0.5rem;          /* 섹션 간 간격 (영상↔자막) */

  /* URL Bar */
  --url-bar-height: 48px;
  --url-bar-padding: 0 1rem;

  /* 자막 영역 */
  --subtitle-padding-x: 2rem;
  --subtitle-padding-y: 1.5rem;
  --subtitle-gap: 0.75rem;        /* 번역↔원문 간격 */

  /* 상태 라인 */
  --status-height: 28px;
  --status-padding: 0 1rem;
}
```

---

## 10. 아이콘

Lucide 아이콘 (Luma 기본) 사용:

| 용도 | 아이콘 | Lucide 이름 |
|------|--------|------------|
| URL 입력 | 🔗 | `Link` |
| 붙여넣기 | 📋 | `ClipboardPaste` |
| 설정 | ⚙ | `Settings` |
| 다크/라이트 토글 | 🌙/☀ | `Moon` / `Sun` |
| 닫기 | ✕ | `X` |
| 번역 중 | ◐ | `Loader2` (spinning) |
| 캐시 hit | ✓ | `Check` |
| 에러 | ⚠ | `AlertTriangle` |
| 자막 없음 | 💬 | `MessageSquareOff` |

---

## 11. 모션 가이드라인

### 기본 easing

```css
:root {
  --ease-out: cubic-bezier(0.16, 1, 0.3, 1);      /* 대부분의 전환 */
  --ease-in-out: cubic-bezier(0.65, 0, 0.35, 1);   /* 레이아웃 변경 */
  --ease-spring: cubic-bezier(0.34, 1.56, 0.64, 1); /* 바운스 효과 */
}
```

### 지속시간

| 유형 | 시간 | 용도 |
|------|------|------|
| Micro | 150ms | 호버, 포커스 |
| Fast | 300ms | 자막 전환, fade-in/out |
| Normal | 400ms | 레이아웃 변경 (URL 바 이동) |
| Slow | 600ms | 페이지 전환 (빈 상태 → 플레이어) |

### 핵심 애니메이션

```css
/* 자막 fade-in */
@keyframes subtitle-enter {
  from { opacity: 0; transform: translateY(4px); }
  to   { opacity: 1; transform: translateY(0); }
}

/* URL 바 auto-hide */
@keyframes url-bar-hide {
  from { opacity: 1; max-height: 48px; }
  to   { opacity: 0; max-height: 0; }
}

/* shimmer (캐시 miss 로딩) */
@keyframes shimmer {
  0%   { background-position: -200% 0; }
  100% { background-position: 200% 0; }
}

.shimmer {
  background: linear-gradient(
    90deg,
    var(--subtitle-bg) 25%,
    oklch(0.25 0 0) 50%,
    var(--subtitle-bg) 75%
  );
  background-size: 200% 100%;
  animation: shimmer 1.5s infinite;
}

/* 에러 shake */
@keyframes shake {
  0%, 100% { transform: translateX(0); }
  25%      { transform: translateX(-2px); }
  75%      { transform: translateX(2px); }
}
```

---

## 12. 키보드 단축키

| 키 | 기능 | 상태 |
|---|------|------|
| `T` | 원문(영어) 자막 토글 | 기본: 번역만 표시 → T 누르면 원문 추가 표시 |
| `+` / `=` | 자막 크기 증가 | 20px → 24px (--subtitle-size-lg) |
| `-` | 자막 크기 감소 | 24px → 20px (--subtitle-size) |
| `Space` | 재생 / 일시정지 | YouTube iframe API `playVideo()` / `pauseVideo()` |

- 키보드 단축키는 URL 입력 포커스 시 비활성화 (입력과 충돌 방지)
- YouTube 플레이어 내부 포커스 시에도 동작하도록 window 레벨 이벤트 리스너

---

## 13. YouTube 플레이어 전략

Phase 0에서 두 가지 접근을 모두 테스트:

### Option A: lite-youtube (lazy-load)
```html
<lite-youtube videoid="..." params="enablejsapi=1"></lite-youtube>
```
- 초기 렌더링: 썸네일만 (2.2KB)
- 클릭 시 iframe 삽입 → shadowRoot에서 iframe 추출 → `new YT.Player()` 래핑
- **리스크**: `onYouTubeIframeAPIReady` 불안정성, Shadow DOM 접근 복잡도

### Option B: 직접 YouTube iframe API
```html
<div id="player"></div>
<script>new YT.Player('player', { videoId: '...', events: {...} })</script>
```
- 표준 초기화 패턴, Player API 안정성 보장
- 초기 로딩이 느림 (iframe 즉시 로드)

### Phase 0 판정 기준
| 기준 | 최소 통과 |
|------|----------|
| `getCurrentTime()` 폴링 안정성 | 500ms 간격으로 100회 연속 정상 응답 |
| `onStateChange` 이벤트 수신 | play/pause/seek 모두 감지 |
| 초기 로딩 시간 | URL 입력 → 플레이어 조작 가능까지 |

---

## 14. 환경 검증 (EnvironmentCheck)

앱 시작 시 `testEnvironment()` (Paperclip 패턴)로 Claude CLI 설치를 확인한다.

**미설치 시: 모달 다이얼로그 (앱 차단)**

```
┌─────────────────────────────────────────────┐
│                                             │
│        Claude Code CLI가 필요합니다          │
│                                             │
│   이 앱은 Claude Code CLI를 사용하여         │
│   자막을 번역합니다.                         │
│                                             │
│   1. 터미널에서 아래 명령을 실행하세요:       │
│      npm install -g @anthropic-ai/claude-code│
│                                             │
│   2. 로그인하세요:                           │
│      claude login                           │
│                                             │
│   [ 📋 명령 복사 ]      [ 🔄 다시 확인 ]    │
│                                             │
└─────────────────────────────────────────────┘
```

- shadcn `Dialog` 컴포넌트 사용
- "다시 확인" 버튼으로 설치 완료 후 재검증
- 모달이 닫히기 전까지 메인 UI 비활성화
- 설치 완료 확인 시 모달 자동 닫힘 + 빈 상태 화면으로 전환

---

## 15. 접근성

| 항목 | 기준 |
|------|------|
| 색상 대비 | 자막 텍스트: WCAG AA (4.5:1 이상) |
| 키보드 내비게이션 | Tab으로 URL 입력 → 영상 → 자막 영역 |
| 포커스 링 | var(--ring) 사용, 2px solid |
| 화면 읽기 | 자막 영역에 aria-live="polite" |
| 텍스트 크기 | rem 단위, 브라우저 설정 존중 |

---

## 16. 파일 구조 (프론트엔드)

```
src/
├── app/
│   ├── App.tsx
│   ├── globals.css              ← CSS 변수, 폰트, 기본 스타일
│   └── providers.tsx            ← 테마, 상태 프로바이더
│
├── components/
│   ├── ui/                      ← shadcn 컴포넌트 (자동 생성)
│   │   ├── button.tsx
│   │   ├── input.tsx
│   │   ├── card.tsx
│   │   ├── progress.tsx
│   │   ├── alert.tsx
│   │   └── dialog.tsx
│   │
│   ├── url-bar.tsx              ← URL 입력 바 (auto-hide)
│   ├── empty-state.tsx          ← 빈 상태 화면
│   ├── video-player.tsx         ← YouTube iframe 래퍼
│   ├── subtitle-stage.tsx       ← 자막 표시 영역
│   ├── status-line.tsx          ← 하단 상태 표시줄
│   └── environment-check.tsx    ← Claude CLI 검증 다이얼로그
│
├── hooks/
│   ├── use-player-sync.ts       ← 재생 시간 ↔ 자막 동기화
│   ├── use-translation.ts       ← 번역 상태 관리
│   ├── use-auto-hide.ts         ← URL 바 auto-hide 로직
│   └── use-subtitle-cache.ts    ← 캐시 hit/miss 관리
│
├── lib/
│   ├── tauri-commands.ts        ← Rust 커맨드 invoke 래퍼
│   ├── youtube.ts               ← video ID 파싱, iframe API 유틸
│   └── utils.ts                 ← cn() 등 유틸리티
│
└── types/
    ├── subtitle.ts              ← Subtitle, Translation 타입
    └── player.ts                ← PlayerState 타입
```
