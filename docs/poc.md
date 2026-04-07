# YouTube 번역 자막 서비스 — POC 기획 문서

> **참고**: 이 문서는 초기 POC 기획서입니다. 이후 Deep Interview를 통해 주요 결정이 변경되었습니다:
> - 투명 오버레이 윈도우 → 앱 내 임베드 플레이어 + 영상 위 자막 오버레이
> - 전체 일괄 번역 → 청크 단위(30초~1분) 번역
> - Tauri 윈도우 풀스크린 (YouTube iframe 풀스크린 대체)
> - 최종 설계는 [PRD](prd.md)와 [디자인 시스템](design-system.md)을 참조하세요.

> 작성일: 2026-04-06  
> 상태: POC 설계 확정

---

## 1. 개요

Claude Code 구독 모델을 활용해 YouTube 영상의 자동 자막을 실시간으로 한국어로 번역하여, 화면 하단에 투명 오버레이 자막으로 표시하는 데스크탑 앱이다.

기존 allang.ai 같은 유사 서비스와의 핵심 차이점은 **별도 API 비용 없이 사용자의 Claude Code 구독을 그대로 활용**한다는 점이다. Paperclip이 Claude Code CLI를 subprocess로 spawn하는 방식을 차용한다.

---

## 2. 목표

### POC에서 검증할 것

- YouTube transcript API로 자동 자막 fetch가 안정적으로 동작하는가
- `claude --print - --output-format stream-json` subprocess 방식으로 번역이 정상 동작하는가
- Tauri 투명 오버레이 윈도우가 YouTube 풀스크린 위에 표시되는가
- 타임스탬프 기반 자막 싱크가 시청 경험을 해치지 않는가

### POC에서 검증하지 않을 것

- 브라우저 창 위치 자동 추적
- 자막 없는 영상 처리
- 다국어 지원
- 사용자 계정 / 설정 저장

---

## 3. 기술 스택

| 레이어 | 기술 | 선택 이유 |
|---|---|---|
| 앱 프레임워크 | Tauri 2.x | 포터블 .exe 배포, 번들 크기 ~15MB |
| UI | React + TypeScript | 기존 스택 재사용 |
| 번역 엔진 | Claude Code CLI subprocess | 구독 모델 활용, API 비용 없음 |
| 자막 소스 | YouTube timedtext (undocumented API) | API 키 불필요 |
| 타겟 OS | Windows (우선), macOS (추후) | 개발자 OS 기준 |

---

## 4. 아키텍처

```
사용자 로컬 머신
│
├─ Claude Code CLI (구독 로그인 완료 전제)
│
└─ Tauri 앱 (포터블 .exe)
    │
    ├─ [메인 윈도우]
    │   └─ YouTube URL 입력 UI
    │       ↓
    │   YouTube timedtext API 호출
    │       ↓
    │   자막 데이터 fetch
    │   [{ text: "Hello", start: 7.58, duration: 4.0 }, ...]
    │       ↓
    │   claude subprocess spawn
    │   claude --print - --output-format stream-json
    │       ↓ stdin: 번역 프롬프트
    │       ↓ stdout: JSONL 스트리밍
    │   번역 완료된 자막 데이터
    │   [{ original: "Hello", translated: "안녕하세요", start: 7.58, end: 11.58 }, ...]
    │
    └─ [오버레이 윈도우]
        ├─ transparent: true
        ├─ always_on_top: true
        ├─ decorations: false
        └─ 화면 하단 고정
            ↓
        재생 시간 폴링 (500ms)
            ↓
        현재 시간에 맞는 자막 표시
```

---

## 5. 핵심 플로우

### 5-1. Claude Code subprocess 방식 (Paperclip 참조)

Paperclip의 `claude_local` 어댑터 방식을 그대로 차용한다.

```bash
# 실제 실행 커맨드
claude \
  --print \
  - \
  --output-format stream-json \
  --verbose
```

**중요 포인트**
- `CLAUDECODE` 환경변수를 자식 프로세스에서 반드시 제거해야 한다 (nested session 오류 방지)
- stdin으로 프롬프트 주입, stdout JSONL 파싱으로 결과 수신
- `ANTHROPIC_API_KEY` 없으면 구독 OAuth 세션 자동 사용 → 비용 $0

### 5-2. YouTube 자막 fetch

YouTube 내부 timedtext undocumented API를 사용한다.

```
입력: YouTube video ID
출력: [{ text, start, duration }, ...]
```

- API 키, headless 브라우저 불필요
- 자동 생성 자막(auto-generated) 포함
- 자막 없는 영상은 에러 처리 후 사용자 안내

### 5-3. 번역 전략

영상 시작 전 **전체 일괄 번역** 방식을 사용한다.

```
자막 fetch 완료
    ↓
전체 텍스트를 하나의 프롬프트로 묶어 claude에 전달
    ↓
번역 완료 후 재생 시작 가능
    ↓
재생 중 타임스탬프 기준으로 자막 순차 표시
```

실시간 번역은 LLM 응답 지연으로 싱크 불안정 → POC에서 제외

### 5-4. 오버레이 윈도우

```
위치: 화면 하단 고정 (브라우저 창 추적 없음, POC 기준)
속성:
  - transparent: true
  - always_on_top: true
  - decorations: false (타이틀바 없음)
  - skip_taskbar: true
  - 너비: 화면 너비의 80%
  - 높이: 자막 2줄 분량

지원 화면 모드:
  - 일반 창 모드 ✅
  - 브라우저 풀스크린 (YouTube 전체화면 버튼 / F키) ✅
  - macOS 네이티브 풀스크린 (초록 버튼 / 새 Space) ❌ (추후)
```

---

## 6. 구현 범위 (In Scope)

| 기능 | 설명 |
|---|---|
| URL 입력 | YouTube URL로부터 video ID 파싱 |
| 자막 fetch | YouTube timedtext API 호출 |
| 번역 | Claude Code subprocess 일괄 번역 |
| 오버레이 표시 | 투명 윈도우, 화면 하단 고정 |
| 자막 싱크 | 재생 시간 폴링 기반 타임스탬프 매칭 |
| 로딩 상태 | 번역 중 진행률 표시 |
| 에러 처리 | 자막 없는 영상, Claude 한도 초과 안내 |

---

## 7. 제외 범위 (Out of Scope)

| 항목 | 이유 |
|---|---|
| 자막 없는 영상 STT | Whisper 등 별도 파이프라인 필요, 복잡도 증가 |
| 브라우저 창 위치 자동 추적 | Windows API 연동 필요, POC 후 추가 |
| macOS 네이티브 풀스크린 | Tauri 버그 이슈 존재, 추후 대응 |
| API 키 폴백 | Claude Code 구독 전제, POC 단순화 |
| 설정 저장 / 사용자 계정 | POC 범위 외 |
| 다국어 지원 | 영어 → 한국어 단방향 고정 |

---

## 8. 리스크 및 제약사항

### 기술적 리스크

| 리스크 | 가능성 | 대응 |
|---|---|---|
| YouTube timedtext API 변경 | 중간 | undocumented API 특성상 언제든 변경 가능. 오픈소스 라이브러리 모니터링 |
| Claude Code 구독 한도 초과 | 낮음 | 번역은 가벼운 작업. 초과 시 에러 메시지 표시 |
| Windows 풀스크린 오버레이 버그 | 낮음 | 브라우저 풀스크린은 일반 윈도우 레이어라 영향 없음 |
| claude 바이너리 PATH 미등록 | 중간 | 앱 시작 시 환경 검증 + 설치 가이드 안내 |

### ToS 관련

Paperclip GitHub 논의 결과, Claude Code CLI 바이너리를 subprocess로 직접 실행하는 방식은 OAuth 토큰을 가로채지 않아 기존 제재 대상과 다르다. 다만 Anthropic 구독 약관상 "일반적인 개인 사용" 전제가 있어, 상업 서비스화 시 재검토 필요.

---

## 9. 배포 형태

```
youtube-translator-win-x64.zip
└─ youtube-translator.exe   ← 압축 풀고 더블클릭으로 실행

전제 조건 (사용자가 별도 설치 필요)
├─ Claude Code CLI (npm install -g @anthropic-ai/claude-code)
└─ claude login (구독 로그인)
```

---

## 10. POC 성공 기준

1. 영어 자막이 있는 YouTube 영상 URL 입력 → 한국어 번역 자막 정상 표시
2. YouTube 전체화면 모드에서 자막 오버레이 유지
3. 자막 싱크 오차 ±2초 이내
4. Claude Code 구독 외 추가 비용 발생 없음
