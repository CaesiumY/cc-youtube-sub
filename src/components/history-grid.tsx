import { useHistoryStore } from "../stores/history-store";
import { HistoryCard } from "./history-card";

/**
 * 홈 화면 — 최근 본 영상 히스토리 그리드.
 *
 * 항목이 없으면 렌더하지 않아 첫 사용 시 홈 화면을 깔끔하게 유지한다.
 */
export function HistoryGrid() {
  const entries = useHistoryStore((s) => s.entries);
  const clearAll = useHistoryStore((s) => s.clearAll);

  if (entries.length === 0) return null;

  return (
    <div className="w-full">
      <div className="mb-2 flex items-center justify-between">
        <p className="text-sm font-medium text-muted-foreground">
          최근 본 영상
        </p>
        <button
          type="button"
          onClick={clearAll}
          className="text-xs text-muted-foreground transition-colors hover:text-foreground"
        >
          전체 지우기
        </button>
      </div>
      <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
        {entries.map((entry) => (
          <HistoryCard key={entry.videoId} entry={entry} />
        ))}
      </div>
    </div>
  );
}
