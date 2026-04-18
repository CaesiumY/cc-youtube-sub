import { describe, expect, it } from "vitest";
import { buildChunkHashInput, findSubtitleAt } from "./subtitle-matcher";
import type { TranslationEntry } from "./tauri-commands";

const entries: TranslationEntry[] = [
  { original: "Hello", translated: "안녕", start: 1.0, end: 3.0 },
  { original: "World", translated: "세계", start: 5.0, end: 8.0 },
  { original: "Foo", translated: "푸", start: 10.0, end: 15.0 },
];

describe("findSubtitleAt", () => {
  it("returns null for empty array", () => {
    expect(findSubtitleAt([], 5)).toBeNull();
  });

  it("returns entry at exact start time", () => {
    expect(findSubtitleAt(entries, 1.0)).toEqual(entries[0]);
  });

  it("returns entry at mid-range time", () => {
    expect(findSubtitleAt(entries, 6.5)).toEqual(entries[1]);
  });

  it("returns entry just before end (exclusive boundary)", () => {
    expect(findSubtitleAt(entries, 2.999)).toEqual(entries[0]);
  });

  it("returns null at exact end time (exclusive)", () => {
    expect(findSubtitleAt(entries, 3.0)).toBeNull();
  });

  it("returns null before all entries", () => {
    expect(findSubtitleAt(entries, 0)).toBeNull();
  });

  it("returns null after all entries", () => {
    expect(findSubtitleAt(entries, 999)).toBeNull();
  });

  it("returns null in gap between entries", () => {
    expect(findSubtitleAt(entries, 4.0)).toBeNull();
  });

  describe("with leadSec / lingerSec", () => {
    it("lead 0.5 pulls entry start forward", () => {
      // entry[0].start = 1.0 → lead 0.5로 0.5초부터 매칭
      expect(findSubtitleAt(entries, 0.5, { leadSec: 0.5 })).toEqual(
        entries[0],
      );
      expect(findSubtitleAt(entries, 0.49, { leadSec: 0.5 })).toBeNull();
    });

    it("lead does not affect dismiss timing", () => {
      // entry[0].end = 3.0 → lead 0.5여도 end는 그대로 3.0에서 사라짐
      expect(findSubtitleAt(entries, 2.99, { leadSec: 0.5 })).toEqual(
        entries[0],
      );
      expect(findSubtitleAt(entries, 3.0, { leadSec: 0.5 })).toBeNull();
    });

    it("linger extends dismiss", () => {
      // linger 0.5 → entry[0].end(3.0) + 0.5 = 3.5까지 표시
      expect(findSubtitleAt(entries, 3.4, { lingerSec: 0.5 })).toEqual(
        entries[0],
      );
      expect(findSubtitleAt(entries, 3.5, { lingerSec: 0.5 })).toBeNull();
    });

    it("zero opts equivalent to no opts", () => {
      expect(
        findSubtitleAt(entries, 2.0, { leadSec: 0, lingerSec: 0 }),
      ).toEqual(entries[0]);
      expect(
        findSubtitleAt(entries, 3.0, { leadSec: 0, lingerSec: 0 }),
      ).toBeNull();
    });

    it("lead prioritizes next entry when both could match", () => {
      // entry[1].start = 5.0. time=4.6, lead=0.5 → 4.6 + 0.5 = 5.1 >= 5.0 → entry[1] 매칭
      // (entry[0]은 end=3.0이라 이미 아님)
      expect(findSubtitleAt(entries, 4.6, { leadSec: 0.5 })).toEqual(
        entries[1],
      );
    });
  });
});

describe("buildChunkHashInput", () => {
  it("joins text with spaces", () => {
    const lines = [{ text: "hello" }, { text: "world" }];
    expect(buildChunkHashInput(lines)).toBe("hello world");
  });

  it("returns single text for single line", () => {
    expect(buildChunkHashInput([{ text: "only" }])).toBe("only");
  });
});
