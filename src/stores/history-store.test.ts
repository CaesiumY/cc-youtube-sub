import { beforeEach, describe, expect, it } from "vitest";
import { useHistoryStore } from "./history-store";

beforeEach(() => {
  useHistoryStore.getState().clearAll();
});

describe("history-store addEntry", () => {
  it("새 항목을 entries에 추가한다", () => {
    useHistoryStore.getState().addEntry("video123aaa", "First Video");
    const { entries } = useHistoryStore.getState();
    expect(entries).toHaveLength(1);
    expect(entries[0]).toMatchObject({
      videoId: "video123aaa",
      title: "First Video",
    });
  });

  it("addedAt 타임스탬프를 함께 기록한다", () => {
    const before = Date.now();
    useHistoryStore.getState().addEntry("video123aaa", "First Video");
    const entry = useHistoryStore.getState().entries[0];
    expect(entry?.addedAt).toBeGreaterThanOrEqual(before);
    expect(entry?.addedAt).toBeLessThanOrEqual(Date.now());
  });

  it("새 항목을 맨 앞에 추가한다 (최신이 먼저)", () => {
    const s = useHistoryStore.getState();
    s.addEntry("oldvideoaaa", "Old");
    s.addEntry("newvideobbb", "New");
    const { entries } = useHistoryStore.getState();
    expect(entries[0]?.videoId).toBe("newvideobbb");
    expect(entries[1]?.videoId).toBe("oldvideoaaa");
  });

  it("같은 videoId를 다시 추가해도 중복되지 않는다", () => {
    const s = useHistoryStore.getState();
    s.addEntry("samevideoxx", "Title");
    s.addEntry("samevideoxx", "Title");
    expect(useHistoryStore.getState().entries).toHaveLength(1);
  });

  it("같은 videoId 재방문 시 맨 앞으로 이동한다", () => {
    const s = useHistoryStore.getState();
    s.addEntry("firstvideoa", "First");
    s.addEntry("secondvideo", "Second");
    s.addEntry("firstvideoa", "First"); // 재방문
    const { entries } = useHistoryStore.getState();
    expect(entries).toHaveLength(2);
    expect(entries[0]?.videoId).toBe("firstvideoa");
  });

  it("같은 videoId 재추가 시 제목을 갱신한다", () => {
    const s = useHistoryStore.getState();
    s.addEntry("videoxxxxxx", ""); // 제목이 아직 없음
    s.addEntry("videoxxxxxx", "Loaded Title"); // 제목 도착
    const { entries } = useHistoryStore.getState();
    expect(entries).toHaveLength(1);
    expect(entries[0]?.title).toBe("Loaded Title");
  });

  it("최근 20개만 유지하고 오래된 항목은 제거한다", () => {
    const s = useHistoryStore.getState();
    for (let i = 0; i < 21; i++) {
      s.addEntry(`video${String(i).padStart(6, "0")}`, `Video ${i}`);
    }
    const { entries } = useHistoryStore.getState();
    expect(entries).toHaveLength(20);
    // 가장 먼저 추가된 video000000은 상한 초과로 밀려남
    expect(entries.some((e) => e.videoId === "video000000")).toBe(false);
    // 가장 최근 video000020은 맨 앞
    expect(entries[0]?.videoId).toBe("video000020");
  });
});

describe("history-store removeEntry", () => {
  it("videoId로 항목을 제거한다", () => {
    const s = useHistoryStore.getState();
    s.addEntry("keepvideoaa", "Keep");
    s.addEntry("dropvideobb", "Drop");
    s.removeEntry("dropvideobb");
    const { entries } = useHistoryStore.getState();
    expect(entries).toHaveLength(1);
    expect(entries[0]?.videoId).toBe("keepvideoaa");
  });

  it("존재하지 않는 videoId 제거는 영향이 없다", () => {
    const s = useHistoryStore.getState();
    s.addEntry("realvideoaa", "Real");
    s.removeEntry("ghostvideo0");
    expect(useHistoryStore.getState().entries).toHaveLength(1);
  });
});

describe("history-store clearAll", () => {
  it("모든 항목을 제거한다", () => {
    const s = useHistoryStore.getState();
    s.addEntry("videoaaaaa1", "A");
    s.addEntry("videobbbbb2", "B");
    s.clearAll();
    expect(useHistoryStore.getState().entries).toHaveLength(0);
  });
});
