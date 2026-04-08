import { AnimatePresence, motion } from "motion/react";
import { useTranslationStore } from "../stores/translation-store";

/**
 * 번역 진행률 바 — 영상 컨테이너 바로 아래 2px
 *
 * - completedChunks / totalChunks 비율로 너비 계산
 * - "청크 2/10 번역 중..." 상태 텍스트
 * - 캐시 hit 카운트 표시
 * - 완료 시 fade-out
 */
export function ProgressBar() {
  const totalChunks = useTranslationStore((s) => s.totalChunks);
  const completedChunks = useTranslationStore((s) => s.completedChunks);
  const cachedChunks = useTranslationStore((s) => s.cachedChunks);
  const isLoading = useTranslationStore((s) => s.isLoading);

  if (totalChunks === 0) return null;

  const progress = (completedChunks / totalChunks) * 100;
  const isDone = completedChunks === totalChunks;

  return (
    <AnimatePresence>
      {!isDone || isLoading ? (
        <motion.div
          initial={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.5, delay: 1 }}
        >
          {/* 진행률 바 */}
          <div
            className="h-[2px] w-full"
            style={{ background: "var(--progress-track)" }}
          >
            <div
              className="h-full transition-[width] duration-300 ease-linear"
              style={{
                width: `${progress}%`,
                background: isLoading
                  ? "var(--progress-active)"
                  : "var(--progress-fill)",
              }}
            />
          </div>

          {/* 상태 텍스트 */}
          <div className="flex justify-between px-3 py-1">
            <span
              className="text-xs"
              style={{ color: "var(--status-translating)" }}
            >
              {isDone
                ? "번역 완료"
                : `청크 ${completedChunks}/${totalChunks} 번역 중...`}
            </span>
            {cachedChunks > 0 && (
              <span
                className="text-xs"
                style={{ color: "var(--status-cached)" }}
              >
                캐시에서 로드됨 {cachedChunks}개
              </span>
            )}
          </div>
        </motion.div>
      ) : null}
    </AnimatePresence>
  );
}
