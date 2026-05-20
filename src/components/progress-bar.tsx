import { AnimatePresence, motion } from "motion/react";
import { useTranslationStore } from "../stores/translation-store";

/**
 * 번역 진행률 바 — 영상 컨테이너 바로 아래 2px
 *
 * "초기 로딩"(자막 fetch ~ 첫 자막 도착) 동안에만 표시한다. BufferManager는 재생 위치
 * 기준 일부 청크만 lazy 번역하므로 "전체 청크 완료" 같은 전역 진행률은 긴 영상에서
 * 의미가 없다 — 끝까지 도달하지 못해 바가 영영 사라지지 않는다. 재생 중 미번역 구간
 * 알림은 subtitle-overlay의 "번역 준비 중"이 담당한다.
 *
 * - 첫 자막이 도착하면 `isLoading`이 false가 되며 fade-out
 * - 캐시 hit 카운트 표시 (재방문 시 캐시에서 즉시 로드된 청크 수)
 */
export function ProgressBar() {
  const totalChunks = useTranslationStore((s) => s.totalChunks);
  const completedChunks = useTranslationStore((s) => s.completedChunks);
  const cachedChunks = useTranslationStore((s) => s.cachedChunks);
  const isLoading = useTranslationStore((s) => s.isLoading);

  const progress = totalChunks > 0 ? (completedChunks / totalChunks) * 100 : 0;

  return (
    <AnimatePresence>
      {isLoading && totalChunks > 0 ? (
        <motion.div
          key="progress"
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
                background: "var(--progress-active)",
              }}
            />
          </div>

          {/* 상태 텍스트 */}
          <div className="flex justify-between px-3 py-1">
            <span
              className="text-xs"
              style={{ color: "var(--status-translating)" }}
            >
              자막 번역 준비 중...
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
