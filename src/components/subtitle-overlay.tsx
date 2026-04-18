import { AnimatePresence, motion } from "motion/react";
import { useEffect, useMemo, useRef, useState } from "react";
import { findSubtitleAt } from "../lib/subtitle-matcher";
import { isTauri } from "../lib/tauri-commands";
import { usePlayerStore } from "../stores/player-store";
import { useTranslationStore } from "../stores/translation-store";

/**
 * 영상 영역 너비 대비 자막 폰트 스케일을 계산.
 * 640px 컨테이너를 기준 1.0으로 두고 0.9~1.6 사이로 clamp.
 */
const BASE_WIDTH = 640;
const SCALE_MIN = 0.9;
const SCALE_MAX = 1.6;

/**
 * 자막 매칭에서 등장 시점만 앞당기는 offset (초).
 *
 * - `leadSec`: 자막 start 시점을 앞당겨 **미리 등장**하게 함. ASR start가 실제 발음보다
 *   뒤에 찍히는 구조적 지연 상쇄용.
 * - `lingerSec` (0): 자막은 **raw end 순간에 사라짐**. 즉 발음이 끝나면 자막도 사라짐.
 *   학습 용도에는 이 조합이 자연스러움 (다음 자막이 뜰 때 자연스럽게 교체).
 *
 * 이전 구현은 `currentTime + 0.5`로 시간 자체를 앞당겼는데, 그러면 퇴장도 같이
 * 앞당겨져 발음이 끝나기 전에 자막이 사라지는 문제가 있었다.
 */
const SUBTITLE_LEAD_SEC = 0.5;
const SUBTITLE_LINGER_SEC = 0;

function computeScale(width: number): number {
  if (width <= 0) return 1;
  const raw = width / BASE_WIDTH;
  return Math.max(SCALE_MIN, Math.min(SCALE_MAX, raw));
}

/**
 * 영상 위 자막 오버레이 — YouTube 컨트롤 바 바로 위에 표시.
 *
 * - 원문 + 번역을 기본 2줄로 동시 표시 (T키로 원문 hide)
 * - 폰트는 영상 영역 width에 비례해 자동 스케일
 * - fade-in/out: AnimatePresence로 자연스러운 자막 전환
 */
export function SubtitleOverlay() {
  const currentTime = usePlayerStore((s) => s.currentTime);
  const showOriginal = usePlayerStore((s) => s.showOriginal);
  const subtitleSize = usePlayerStore((s) => s.subtitleSize);
  const translations = useTranslationStore((s) => s.translations);
  const isLoading = useTranslationStore((s) => s.isLoading);
  const totalChunks = useTranslationStore((s) => s.totalChunks);
  const error = useTranslationStore((s) => s.error);

  const rootRef = useRef<HTMLDivElement>(null);
  const [containerWidth, setContainerWidth] = useState(BASE_WIDTH);

  useEffect(() => {
    const target = rootRef.current?.parentElement;
    if (!target) return;
    setContainerWidth(target.getBoundingClientRect().width);
    const ro = new ResizeObserver((entries) => {
      const rect = entries[0]?.contentRect;
      if (rect) setContainerWidth(rect.width);
    });
    ro.observe(target);
    return () => ro.disconnect();
  }, []);

  const currentEntry = useMemo(
    () =>
      findSubtitleAt(translations, currentTime, {
        leadSec: SUBTITLE_LEAD_SEC,
        lingerSec: SUBTITLE_LINGER_SEC,
      }),
    [translations, currentTime],
  );

  const scale = computeScale(containerWidth);
  const translatedFontPx = 16 * subtitleSize * scale;
  const originalFontSize = "0.72em";

  if (totalChunks === 0 && !isLoading) return null;

  return (
    <div
      ref={rootRef}
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
            className="max-w-[90%] rounded-lg px-5 py-3 backdrop-blur-sm"
            style={{
              backgroundColor: "var(--subtitle-bg, rgba(0, 0, 0, 0.82))",
            }}
          >
            <p
              className="text-center font-semibold leading-relaxed"
              style={{
                color: "var(--subtitle-text, #fafafa)",
                fontSize: `${translatedFontPx}px`,
                lineHeight: "var(--leading-subtitle, 1.6)",
                textShadow: "0 1px 2px rgba(0, 0, 0, 0.6)",
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
                className="mt-1.5 text-center"
                style={{
                  color: "var(--subtitle-original, #b8b8b8)",
                  fontSize: originalFontSize,
                  textShadow: "0 1px 2px rgba(0, 0, 0, 0.6)",
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
            className="max-w-[90%] rounded-lg px-5 py-3"
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
            className="rounded-lg px-5 py-3 backdrop-blur-sm"
            style={{
              backgroundColor: "var(--subtitle-bg, rgba(0, 0, 0, 0.82))",
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
