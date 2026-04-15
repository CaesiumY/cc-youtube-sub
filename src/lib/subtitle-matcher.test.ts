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
