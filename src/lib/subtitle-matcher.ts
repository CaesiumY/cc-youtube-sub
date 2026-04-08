import type { TranslationEntry } from "./tauri-commands";

/**
 * 이진 검색으로 현재 재생 시간에 해당하는 자막을 찾는다.
 *
 * @param translations - 시간순 정렬된 번역 배열
 * @param time - 현재 재생 시간 (초)
 * @returns 매칭되는 TranslationEntry 또는 null
 *
 * 조건: entry.start <= time < entry.end
 */
export function findSubtitleAt(
  translations: TranslationEntry[],
  time: number,
): TranslationEntry | null {
  if (translations.length === 0) return null;

  let low = 0;
  let high = translations.length - 1;

  while (low <= high) {
    const mid = (low + high) >>> 1;
    const entry = translations[mid]!;

    if (time < entry.start) {
      high = mid - 1;
    } else if (time >= entry.end) {
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
