import type { TranslationEntry } from "./tauri-commands";

export interface FindSubtitleOptions {
  /**
   * 자막 등장을 현재 시간보다 앞당기는 offset (초).
   * ASR `start`가 실제 발음보다 뒤에 찍히는 구조적 지연을 상쇄.
   *
   * 예: `leadSec=0.5` 일 때 `entry.start = 3.0`이면 `time >= 2.5`부터 매칭.
   */
  leadSec?: number;
  /**
   * 자막 퇴장을 `entry.end` 이후로 늦추는 offset (초).
   * 0이면 `entry.end` 순간 즉시 사라짐. 학습 용도로는 linger 없이 두는 게
   * 자연스러움 (다음 자막이 시작되면 그것이 표시되므로).
   */
  lingerSec?: number;
}

/**
 * 이진 검색으로 현재 재생 시간에 해당하는 자막을 찾는다.
 *
 * 매칭 윈도우: `entry.start - leadSec <= time < entry.end + lingerSec`
 *
 * - `leadSec > 0`: 자막을 미리 표시 (ASR 지연 상쇄)
 * - `lingerSec > 0`: 자막을 end 이후에도 잠시 유지 (기본 0)
 * - 둘 다 0이면 고전 조건: `entry.start <= time < entry.end`
 *
 * 학습 자막 기본값: `leadSec=0.5`, `lingerSec=0` — 미리 보이고 제 시간에 사라짐.
 *
 * @param translations - 시간순 정렬된 번역 배열
 * @param time - 현재 재생 시간 (초)
 * @param opts - lead/linger 설정
 * @returns 매칭되는 TranslationEntry 또는 null
 */
export function findSubtitleAt(
  translations: TranslationEntry[],
  time: number,
  opts?: FindSubtitleOptions,
): TranslationEntry | null {
  if (translations.length === 0) return null;

  const lead = opts?.leadSec ?? 0;
  const linger = opts?.lingerSec ?? 0;

  let low = 0;
  let high = translations.length - 1;

  while (low <= high) {
    const mid = (low + high) >>> 1;
    const entry = translations[mid];
    if (!entry) break;

    if (time + lead < entry.start) {
      high = mid - 1;
    } else if (time >= entry.end + linger) {
      low = mid + 1;
    } else {
      return entry;
    }
  }

  return null;
}

/**
 * 자막 배열에서 chunk_hash를 생성한다.
 * SHA-256은 Rust 백엔드에서 처리하지만, 프론트엔드에서도
 * 동일한 입력 문자열을 만들어야 캐시 키가 일치한다.
 */
export function buildChunkHashInput(lines: { text: string }[]): string {
  return lines.map((l) => l.text).join(" ");
}
