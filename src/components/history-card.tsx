import { useNavigate } from "@tanstack/react-router";
import { X, Youtube } from "lucide-react";
import { useState } from "react";
import { cn } from "../lib/utils";
import { getThumbnailUrl } from "../lib/youtube-url";
import { type HistoryEntry, useHistoryStore } from "../stores/history-store";

interface HistoryCardProps {
  entry: HistoryEntry;
}

/**
 * 홈 화면 히스토리 그리드의 카드 1개 — 썸네일 + 제목.
 *
 * 클릭 가능한 카드 `button`과 삭제 `button`을 중첩하지 않고 형제로 두어
 * HTML 인터랙티브 요소 중첩을 피한다. 형제이므로 삭제 클릭이 카드로
 * 전파되지 않아 `stopPropagation`이 필요 없다.
 */
export function HistoryCard({ entry }: HistoryCardProps) {
  const navigate = useNavigate();
  const removeEntry = useHistoryStore((s) => s.removeEntry);
  const [thumbnailFailed, setThumbnailFailed] = useState(false);

  const handleOpen = () => {
    navigate({ to: "/watch/$videoId", params: { videoId: entry.videoId } });
  };

  return (
    <div className="group relative">
      <button
        type="button"
        onClick={handleOpen}
        className={cn(
          "flex w-full flex-col overflow-hidden rounded-xl border border-input bg-card text-left",
          "transition-colors hover:border-ring focus:outline-none focus:ring-2 focus:ring-ring",
        )}
      >
        <div className="flex aspect-video w-full items-center justify-center bg-muted">
          {thumbnailFailed ? (
            <Youtube size={32} className="text-muted-foreground" />
          ) : (
            <img
              src={getThumbnailUrl(entry.videoId)}
              alt=""
              loading="lazy"
              onError={() => setThumbnailFailed(true)}
              className="h-full w-full object-cover"
            />
          )}
        </div>
        <p
          className={cn(
            "line-clamp-2 px-3 py-2 text-sm",
            entry.title ? "text-foreground" : "text-muted-foreground",
          )}
        >
          {entry.title || "제목 없음"}
        </p>
      </button>
      <button
        type="button"
        onClick={() => removeEntry(entry.videoId)}
        aria-label="히스토리에서 삭제"
        className={cn(
          "absolute top-1.5 right-1.5 rounded-full bg-background/80 p-1 text-muted-foreground",
          "opacity-0 transition-opacity hover:bg-background hover:text-foreground",
          "group-hover:opacity-100 focus:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring",
        )}
      >
        <X size={14} />
      </button>
    </div>
  );
}
