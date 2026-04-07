import { useNavigate } from "@tanstack/react-router";
import { Link } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { cn } from "../lib/utils";
import { extractVideoId } from "../lib/youtube-url";

export function UrlInput() {
  const [value, setValue] = useState("");
  const [error, setError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const navigate = useNavigate();

  const handleSubmit = useCallback(
    (input: string) => {
      const videoId = extractVideoId(input);
      if (videoId) {
        setError(null);
        navigate({ to: "/watch/$videoId", params: { videoId } });
      } else if (input.trim()) {
        setError("올바른 YouTube URL을 입력해주세요");
      }
    },
    [navigate],
  );

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") {
      handleSubmit(value);
    }
  };

  const handlePaste = (e: React.ClipboardEvent<HTMLInputElement>) => {
    const pasted = e.clipboardData.getData("text");
    // 붙여넣기 시 자동 제출 시도
    const videoId = extractVideoId(pasted);
    if (videoId) {
      e.preventDefault();
      setValue(pasted);
      navigate({ to: "/watch/$videoId", params: { videoId } });
    }
  };

  // Cmd/Ctrl+V 글로벌 붙여넣기 감지 (입력 필드에 포커스가 없어도)
  useEffect(() => {
    const handleGlobalPaste = (e: ClipboardEvent) => {
      if (document.activeElement === inputRef.current) return; // 이미 입력 필드에 포커스
      const pasted = e.clipboardData?.getData("text");
      if (pasted) {
        const videoId = extractVideoId(pasted);
        if (videoId) {
          navigate({ to: "/watch/$videoId", params: { videoId } });
        }
      }
    };
    window.addEventListener("paste", handleGlobalPaste);
    return () => window.removeEventListener("paste", handleGlobalPaste);
  }, [navigate]);

  // 자동 포커스
  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  return (
    <div className="flex flex-col items-center gap-3">
      <div className="relative w-full">
        <Link
          size={18}
          className="absolute top-1/2 left-4 -translate-y-1/2 text-muted-foreground"
        />
        <input
          ref={inputRef}
          type="text"
          value={value}
          onChange={(e) => {
            setValue(e.target.value);
            setError(null);
          }}
          onKeyDown={handleKeyDown}
          onPaste={handlePaste}
          placeholder="YouTube URL을 붙여넣으세요..."
          className={cn(
            "w-full rounded-2xl border bg-card py-4 pr-4 pl-11 text-base text-foreground outline-none transition-all",
            "placeholder:text-muted-foreground",
            "focus:ring-2 focus:ring-ring",
            error ? "border-destructive" : "border-input",
          )}
          spellCheck={false}
          autoComplete="off"
        />
      </div>
      {error && <p className="text-sm text-destructive">{error}</p>}
      <p className="text-xs text-muted-foreground">
        Enter를 눌러 이동하거나, URL을 붙여넣으면 자동으로 이동합니다
      </p>
    </div>
  );
}
