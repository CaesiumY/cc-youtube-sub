import { describe, expect, it } from "vitest";
import { extractVideoId, isValidYouTubeUrl } from "./youtube-url";

describe("extractVideoId", () => {
  it("standard watch URL", () => {
    expect(extractVideoId("https://www.youtube.com/watch?v=dQw4w9WgXcQ")).toBe(
      "dQw4w9WgXcQ",
    );
  });

  it("watch URL with extra params", () => {
    expect(
      extractVideoId("https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=120"),
    ).toBe("dQw4w9WgXcQ");
  });

  it("short URL (youtu.be)", () => {
    expect(extractVideoId("https://youtu.be/dQw4w9WgXcQ")).toBe("dQw4w9WgXcQ");
  });

  it("embed URL", () => {
    expect(extractVideoId("https://www.youtube.com/embed/dQw4w9WgXcQ")).toBe(
      "dQw4w9WgXcQ",
    );
  });

  it("shorts URL", () => {
    expect(extractVideoId("https://youtube.com/shorts/dQw4w9WgXcQ")).toBe(
      "dQw4w9WgXcQ",
    );
  });

  it("raw 11-char video ID", () => {
    expect(extractVideoId("dQw4w9WgXcQ")).toBe("dQw4w9WgXcQ");
  });

  it("ID with hyphen and underscore", () => {
    expect(extractVideoId("a1B-c2D_e3F")).toBe("a1B-c2D_e3F");
  });

  it("trims whitespace", () => {
    expect(extractVideoId("  dQw4w9WgXcQ  ")).toBe("dQw4w9WgXcQ");
  });

  it("returns null for invalid URL", () => {
    expect(extractVideoId("https://example.com/video")).toBeNull();
  });

  it("returns null for empty string", () => {
    expect(extractVideoId("")).toBeNull();
  });

  it("returns null for too-short string", () => {
    expect(extractVideoId("abc")).toBeNull();
  });
});

describe("isValidYouTubeUrl", () => {
  it("returns true for valid URL", () => {
    expect(isValidYouTubeUrl("https://youtu.be/dQw4w9WgXcQ")).toBe(true);
  });

  it("returns false for invalid input", () => {
    expect(isValidYouTubeUrl("not-a-url")).toBe(false);
  });
});
