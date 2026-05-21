import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useHistoryStore } from "../stores/history-store";
import { HistoryGrid } from "./history-grid";

// HistoryCard가 useNavigate를 쓰므로 라우터 모듈을 stub으로 대체한다.
vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => vi.fn(),
}));

beforeEach(() => {
  useHistoryStore.getState().clearAll();
});

describe("HistoryGrid", () => {
  it("히스토리가 비어 있으면 아무것도 렌더하지 않는다", () => {
    const { container } = render(<HistoryGrid />);
    expect(container).toBeEmptyDOMElement();
  });

  it("히스토리가 있으면 헤더와 카드 제목을 렌더한다", () => {
    useHistoryStore.getState().addEntry("video123abc", "테스트 영상");
    render(<HistoryGrid />);
    expect(screen.getByText("최근 본 영상")).toBeInTheDocument();
    expect(screen.getByText("테스트 영상")).toBeInTheDocument();
  });

  it("항목 수만큼 카드를 렌더한다", () => {
    const s = useHistoryStore.getState();
    s.addEntry("videoaaaaa1", "영상 1");
    s.addEntry("videobbbbb2", "영상 2");
    s.addEntry("videoccccc3", "영상 3");
    render(<HistoryGrid />);
    expect(screen.getByText("영상 1")).toBeInTheDocument();
    expect(screen.getByText("영상 2")).toBeInTheDocument();
    expect(screen.getByText("영상 3")).toBeInTheDocument();
  });

  it("'전체 지우기'를 누르면 그리드가 사라진다", () => {
    useHistoryStore.getState().addEntry("video123abc", "테스트 영상");
    const { container } = render(<HistoryGrid />);
    fireEvent.click(screen.getByRole("button", { name: "전체 지우기" }));
    expect(container).toBeEmptyDOMElement();
    expect(useHistoryStore.getState().entries).toHaveLength(0);
  });
});
