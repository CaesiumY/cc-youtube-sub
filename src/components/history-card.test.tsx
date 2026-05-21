import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { getThumbnailUrl } from "../lib/youtube-url";
import { type HistoryEntry, useHistoryStore } from "../stores/history-store";
import { HistoryCard } from "./history-card";

// vi.mock 팩토리는 hoist되므로 spy를 vi.hoisted로 끌어올려 참조한다.
const { navigateMock } = vi.hoisted(() => ({ navigateMock: vi.fn() }));

vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => navigateMock,
}));

function makeEntry(overrides: Partial<HistoryEntry> = {}): HistoryEntry {
  return {
    videoId: "video123abc",
    title: "테스트 영상",
    addedAt: Date.now(),
    ...overrides,
  };
}

beforeEach(() => {
  navigateMock.mockClear();
  useHistoryStore.getState().clearAll();
});

describe("HistoryCard", () => {
  it("영상 제목을 표시한다", () => {
    render(<HistoryCard entry={makeEntry({ title: "리액트 강의" })} />);
    expect(screen.getByText("리액트 강의")).toBeInTheDocument();
  });

  it("제목이 비어 있으면 '제목 없음'을 표시한다", () => {
    render(<HistoryCard entry={makeEntry({ title: "" })} />);
    expect(screen.getByText("제목 없음")).toBeInTheDocument();
  });

  it("썸네일 src가 videoId 기반 URL이다", () => {
    const { container } = render(
      <HistoryCard entry={makeEntry({ videoId: "abcdefghij1" })} />,
    );
    const img = container.querySelector("img");
    expect(img?.getAttribute("src")).toBe(getThumbnailUrl("abcdefghij1"));
  });

  it("카드를 클릭하면 해당 영상으로 이동한다", () => {
    render(<HistoryCard entry={makeEntry({ videoId: "abcdefghij1" })} />);
    fireEvent.click(screen.getByRole("button", { name: "테스트 영상" }));
    expect(navigateMock).toHaveBeenCalledWith({
      to: "/watch/$videoId",
      params: { videoId: "abcdefghij1" },
    });
  });

  it("삭제 버튼을 누르면 store에서 제거하고 영상으로 이동하지 않는다", () => {
    useHistoryStore.getState().addEntry("video123abc", "테스트 영상");
    render(<HistoryCard entry={makeEntry()} />);
    fireEvent.click(screen.getByRole("button", { name: "히스토리에서 삭제" }));
    expect(useHistoryStore.getState().entries).toHaveLength(0);
    expect(navigateMock).not.toHaveBeenCalled();
  });

  it("썸네일 로드 실패 시 플레이스홀더로 전환한다", () => {
    const { container } = render(<HistoryCard entry={makeEntry()} />);
    const img = container.querySelector("img");
    expect(img).not.toBeNull();
    if (img) fireEvent.error(img);
    expect(container.querySelector("img")).toBeNull();
  });
});
