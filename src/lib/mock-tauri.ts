/**
 * Mock Tauri 커맨드 구현 — WSL2/브라우저 개발용
 *
 * 실제 YouTube 자막 데이터와 유사한 fixture를 반환하며,
 * 네트워크 지연을 시뮬레이션한다.
 */

import type {
  SubtitleChunk,
  SubtitleLine,
  TranslationEntry,
  VideoInfo,
} from "./tauri-commands";

// ── 헬퍼 ─────────────────────────────────────────────

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

// ── Mock fixture 데이터 ──────────────────────────────

function generateMockSubtitles(): SubtitleChunk[] {
  const lines: SubtitleLine[] = [
    { text: "Hello everyone, welcome to this video.", start: 0.5, end: 3.2 },
    {
      text: "Today we're going to talk about something really interesting.",
      start: 3.5,
      end: 6.8,
    },
    {
      text: "Let me start by explaining the basics.",
      start: 7.0,
      end: 9.5,
    },
    {
      text: "The first thing you need to understand is the concept.",
      start: 10.0,
      end: 13.2,
    },
    {
      text: "It might seem complicated at first.",
      start: 13.5,
      end: 15.8,
    },
    { text: "But trust me, it gets easier.", start: 16.0, end: 18.2 },
    {
      text: "Let's look at some examples.",
      start: 18.5,
      end: 20.5,
    },
    {
      text: "Here's the first example on the screen.",
      start: 21.0,
      end: 23.5,
    },
    { text: "As you can see, it's quite simple.", start: 24.0, end: 26.5 },
    {
      text: "Now let's move on to the next part.",
      start: 27.0,
      end: 29.5,
    },
    {
      text: "This is where things get really interesting.",
      start: 30.0,
      end: 32.8,
    },
    {
      text: "Pay attention to the details here.",
      start: 33.0,
      end: 35.2,
    },
    {
      text: "You'll notice a pattern forming.",
      start: 35.5,
      end: 37.5,
    },
    { text: "This pattern is very important.", start: 38.0, end: 40.2 },
    {
      text: "Let me explain why it matters.",
      start: 40.5,
      end: 42.8,
    },
    {
      text: "In real world applications, this comes up a lot.",
      start: 43.0,
      end: 46.2,
    },
    {
      text: "Companies use this technique every day.",
      start: 46.5,
      end: 49.0,
    },
    {
      text: "Let's take a closer look at the implementation.",
      start: 49.5,
      end: 52.5,
    },
    {
      text: "Here's the code that makes it all work.",
      start: 53.0,
      end: 55.5,
    },
    {
      text: "I'll walk you through it step by step.",
      start: 56.0,
      end: 58.5,
    },
    {
      text: "The first function handles the input.",
      start: 59.0,
      end: 61.5,
    },
    {
      text: "Then we process it through our pipeline.",
      start: 62.0,
      end: 64.8,
    },
    {
      text: "And finally, we get the output.",
      start: 65.0,
      end: 67.2,
    },
    { text: "Pretty cool, right?", start: 67.5, end: 69.0 },
    {
      text: "Now let's talk about optimization.",
      start: 69.5,
      end: 72.0,
    },
    {
      text: "There are several ways to make this faster.",
      start: 72.5,
      end: 75.0,
    },
    {
      text: "The most important one is caching.",
      start: 75.5,
      end: 77.8,
    },
    {
      text: "With caching, we can avoid redundant work.",
      start: 78.0,
      end: 80.5,
    },
    {
      text: "That's all for today's video.",
      start: 81.0,
      end: 83.0,
    },
    {
      text: "Thanks for watching and see you next time!",
      start: 83.5,
      end: 86.0,
    },
  ];

  // 30줄을 ~30초 단위로 청크 분할
  const chunks: SubtitleChunk[] = [];
  let chunkLines: SubtitleLine[] = [];
  let chunkStart = lines[0]!.start;
  let chunkIndex = 0;

  for (const line of lines) {
    const elapsed = line.end - chunkStart;
    if (chunkLines.length >= 10 || elapsed >= 30) {
      const lastLine = chunkLines[chunkLines.length - 1]!;
      chunks.push({
        index: chunkIndex,
        start_time: chunkStart,
        end_time: lastLine.end,
        lines: [...chunkLines],
      });
      chunkIndex++;
      chunkLines = [];
      chunkStart = line.start;
    }
    chunkLines.push(line);
  }
  if (chunkLines.length > 0) {
    const lastLine = chunkLines[chunkLines.length - 1]!;
    chunks.push({
      index: chunkIndex,
      start_time: chunkStart,
      end_time: lastLine.end,
      lines: chunkLines,
    });
  }

  return chunks;
}

const MOCK_TRANSLATIONS: Record<string, string> = {
  "Hello everyone, welcome to this video.":
    "안녕하세요 여러분, 이 영상에 오신 걸 환영합니다.",
  "Today we're going to talk about something really interesting.":
    "오늘은 정말 흥미로운 것에 대해 이야기해 보겠습니다.",
  "Let me start by explaining the basics.":
    "기본부터 설명하는 것으로 시작하겠습니다.",
  "The first thing you need to understand is the concept.":
    "먼저 이해해야 할 것은 개념입니다.",
  "It might seem complicated at first.": "처음에는 복잡해 보일 수 있습니다.",
  "But trust me, it gets easier.": "하지만 믿으세요, 점점 쉬워집니다.",
  "Let's look at some examples.": "몇 가지 예시를 살펴보겠습니다.",
  "Here's the first example on the screen.":
    "화면에 보이는 첫 번째 예시입니다.",
  "As you can see, it's quite simple.": "보시다시피 꽤 간단합니다.",
  "Now let's move on to the next part.": "이제 다음 부분으로 넘어가겠습니다.",
  "This is where things get really interesting.":
    "여기서부터 정말 흥미로워집니다.",
  "Pay attention to the details here.": "여기서 세부 사항에 주목해 주세요.",
  "You'll notice a pattern forming.": "패턴이 형성되는 것을 알 수 있습니다.",
  "This pattern is very important.": "이 패턴은 매우 중요합니다.",
  "Let me explain why it matters.": "왜 중요한지 설명해 드리겠습니다.",
  "In real world applications, this comes up a lot.":
    "실제 애플리케이션에서 이것은 자주 등장합니다.",
  "Companies use this technique every day.":
    "기업들은 매일 이 기술을 사용합니다.",
  "Let's take a closer look at the implementation.":
    "구현을 좀 더 자세히 살펴보겠습니다.",
  "Here's the code that makes it all work.":
    "이 모든 것을 작동시키는 코드입니다.",
  "I'll walk you through it step by step.": "단계별로 안내해 드리겠습니다.",
  "The first function handles the input.": "첫 번째 함수는 입력을 처리합니다.",
  "Then we process it through our pipeline.":
    "그런 다음 파이프라인을 통해 처리합니다.",
  "And finally, we get the output.": "그리고 마지막으로 출력을 얻습니다.",
  "Pretty cool, right?": "꽤 멋지죠?",
  "Now let's talk about optimization.": "이제 최적화에 대해 이야기해 봅시다.",
  "There are several ways to make this faster.":
    "이것을 더 빠르게 만드는 여러 가지 방법이 있습니다.",
  "The most important one is caching.": "가장 중요한 것은 캐싱입니다.",
  "With caching, we can avoid redundant work.":
    "캐싱을 사용하면 중복 작업을 피할 수 있습니다.",
  "That's all for today's video.": "오늘 영상은 여기까지입니다.",
  "Thanks for watching and see you next time!":
    "시청해 주셔서 감사합니다. 다음에 만나요!",
};

// ── Mock 캐시 저장소 ─────────────────────────────────

const mockCache = new Map<string, string>();

// ── Mock 커맨드 구현 ─────────────────────────────────

export async function checkEnvironment(): Promise<string> {
  await delay(100);
  return "Claude CLI (mock) 정상 동작 중";
}

export async function fetchSubtitles(
  _videoId: string,
): Promise<SubtitleChunk[]> {
  await delay(500);
  return generateMockSubtitles();
}

export async function fetchVideoInfo(_videoId: string): Promise<VideoInfo> {
  await delay(300);
  return {
    title: "Understanding Modern Software Architecture",
    description:
      "In this video, we explore the fundamentals of modern software architecture patterns and their practical applications.",
  };
}

export async function translateChunk(
  chunk: SubtitleChunk,
  _videoInfo?: VideoInfo,
  _previousContext?: SubtitleLine[],
): Promise<TranslationEntry[]> {
  // 실제 Claude 번역 시간 시뮬레이션 (2~4초)
  await delay(2000 + Math.random() * 2000);

  return chunk.lines.map((line) => ({
    original: line.text,
    translated: MOCK_TRANSLATIONS[line.text] ?? `[번역] ${line.text}`,
    start: line.start,
    end: line.end,
  }));
}

export async function getChunkHash(lines: SubtitleLine[]): Promise<string> {
  // 간단한 해시 (mock용 — 실제는 Rust SHA256)
  const input = lines.map((l) => l.text).join(" ");
  let hash = 0;
  for (let i = 0; i < input.length; i++) {
    const char = input.charCodeAt(i);
    hash = ((hash << 5) - hash + char) | 0;
  }
  return Math.abs(hash).toString(16).padStart(8, "0");
}

export async function queryCache(
  videoId: string,
  chunkHash: string,
): Promise<string | null> {
  await delay(10);
  return mockCache.get(`${videoId}:${chunkHash}`) ?? null;
}

export async function saveToCache(
  videoId: string,
  chunkHash: string,
  translatedJson: string,
): Promise<void> {
  await delay(10);
  mockCache.set(`${videoId}:${chunkHash}`, translatedJson);
}

export async function batchQueryCache(
  videoId: string,
  chunkHashes: string[],
): Promise<Record<string, string>> {
  await delay(20);
  const result: Record<string, string> = {};
  for (const hash of chunkHashes) {
    const cached = mockCache.get(`${videoId}:${hash}`);
    if (cached) {
      result[hash] = cached;
    }
  }
  return result;
}
