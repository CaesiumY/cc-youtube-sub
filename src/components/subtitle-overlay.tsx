import { AnimatePresence, motion } from "motion/react";
import { useMemo } from "react";
import { findSubtitleAt } from "../lib/subtitle-matcher";
import { isTauri } from "../lib/tauri-commands";
import { usePlayerStore } from "../stores/player-store";
import { useTranslationStore } from "../stores/translation-store";

/**
 * 영상 위 자막 오버레이 — YouTube 컨트롤 바 바로 위에 표시
 *
 * Phase 3 개선:
 * - shimmer 애니메이션: seek 후 캐시 miss 시 로딩 표시
 * - 에러 메시지: 번역 실패 시 오버레이 내부에 표시
 * - fade-in/out: AnimatePresence로 자연스러운 자막 전환
 * - T키: 원본 텍스트 토글, +/-: 폰트 크기 조절
 */
export function SubtitleOverlay() {
  const currentTime = usePlayerStore((s) => s.currentTime);
  const showOriginal = usePlayerStore((s) => s.showOriginal);
  const subtitleSize = usePlayerStore((s) => s.subtitleSize);
  const translations = useTranslationStore((s) => s.translations);
  const isLoading = useTranslationStore((s) => s.isLoading);
  const totalChunks = useTranslationStore((s) => s.totalChunks);
  const error = useTranslationStore((s) => s.error);

  const currentEntry = useMemo(
    () => findSubtitleAt(translations, currentTime),
    [translations, currentTime],
  );

  if (totalChunks === 0 && !isLoading) return null;

  return (
    <div className="pointer-events-none absolute inset-x-0 bottom-16 z-10 flex justify-center px-4">
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
              {!isTauri() && (
                <span className="mr-1.5 rounded bg-amber-500/80 px-1 py-0.5 text-[0.6rem] font-bold uppercase text-black">
                  mock
                </span>
              )}
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
        ) : error ? (
          <motion.div
            key="error"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.2 }}
            className="max-w-[85%] rounded-md px-4 py-2"
            style={{
              backgroundColor: "rgba(127, 29, 29, 0.85)",
            }}
          >
            <p className="text-center text-sm text-red-200">{error}</p>
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
            <p className="text-center text-sm text-zinc-400">
              <span className="inline-block animate-pulse">
                번역 준비 중...
              </span>
            </p>
          </motion.div>
        ) : null}
      </AnimatePresence>
    </div>
  );
}
