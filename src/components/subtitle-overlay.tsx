import { AnimatePresence, motion } from "motion/react";
import { useMemo } from "react";
import { findSubtitleAt } from "../lib/subtitle-matcher";
import { usePlayerStore } from "../stores/player-store";
import { useTranslationStore } from "../stores/translation-store";

/**
 * 영상 위 자막 오버레이 — YouTube 컨트롤 바 바로 위에 표시
 *
 * - 현재 재생 시간에 해당하는 번역 자막을 이진 검색으로 찾아 표시
 * - AnimatePresence로 fade-in/out 전환
 * - T키로 원본 텍스트 토글, +/-로 폰트 크기 조절
 * - 번역 대기 중이면 "번역 준비 중..." 표시
 */
export function SubtitleOverlay() {
  const currentTime = usePlayerStore((s) => s.currentTime);
  const showOriginal = usePlayerStore((s) => s.showOriginal);
  const subtitleSize = usePlayerStore((s) => s.subtitleSize);
  const translations = useTranslationStore((s) => s.translations);
  const isLoading = useTranslationStore((s) => s.isLoading);
  const totalChunks = useTranslationStore((s) => s.totalChunks);

  const currentEntry = useMemo(
    () => findSubtitleAt(translations, currentTime),
    [translations, currentTime],
  );

  // 아직 자막을 로드하지 않은 상태
  if (totalChunks === 0 && !isLoading) return null;

  return (
    <div
      className="pointer-events-none absolute inset-x-0 bottom-16 z-10 flex justify-center px-4"
    >
      <AnimatePresence mode="wait">
        {currentEntry ? (
          <motion.div
            key={`${currentEntry.start}-${currentEntry.end}`}
            initial={{ opacity: 0, y: 4 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -4 }}
            transition={{ duration: 0.15 }}
            className="max-w-[85%] rounded-md px-4 py-2"
            style={{
              backgroundColor: "var(--subtitle-bg, rgba(0, 0, 0, 0.75))",
            }}
          >
            <p
              className="text-center font-medium leading-relaxed"
              style={{
                color: "var(--subtitle-text, #fafafa)",
                fontSize: `${subtitleSize}rem`,
                lineHeight: "var(--leading-subtitle, 1.6)",
              }}
            >
              {currentEntry.translated}
            </p>
            {showOriginal && (
              <p
                className="mt-1 text-center"
                style={{
                  color: "var(--subtitle-original, #8a8a8a)",
                  fontSize: "var(--subtitle-original-size, 0.875rem)",
                }}
              >
                {currentEntry.original}
              </p>
            )}
          </motion.div>
        ) : isLoading ? (
          <motion.div
            key="loading"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.2 }}
            className="rounded-md px-4 py-2"
            style={{
              backgroundColor: "var(--subtitle-bg, rgba(0, 0, 0, 0.75))",
            }}
          >
            <p
              className="text-center text-sm"
              style={{
                color: "var(--subtitle-original, #8a8a8a)",
              }}
            >
              번역 준비 중...
            </p>
          </motion.div>
        ) : null}
      </AnimatePresence>
    </div>
  );
}
