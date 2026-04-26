export const appName = "YouTube Subtitle Translator";

export const valueProps = [
  {
    title: "실시간 오버레이",
    body: "YouTube 플레이어 위에 한국어 번역 자막을 자연스럽게 표시",
  },
  {
    title: "Claude Code CLI",
    body: "구독 중인 CLI를 subprocess로 실행해 별도 API 키 없이 번역",
  },
  {
    title: "SQLite 캐시",
    body: "한 번 번역한 청크는 로컬에 저장해 재방문 시 즉시 재사용",
  },
] as const;

export const sampleLines = [
  {
    original:
      "Today we are going to look at how local-first apps can feel fast.",
    translated: "로컬 우선 앱이 빠르게 느껴지는 이유를 살펴봅니다.",
  },
  {
    original: "The key is to translate ahead of the playback position.",
    translated: "핵심은 재생 위치보다 앞선 자막을 미리 번역해두는 것입니다.",
  },
] as const;
