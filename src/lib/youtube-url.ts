/**
 * YouTube URL에서 videoId를 추출한다.
 *
 * 지원 형식:
 * - https://www.youtube.com/watch?v=VIDEO_ID
 * - https://youtu.be/VIDEO_ID
 * - https://www.youtube.com/embed/VIDEO_ID
 * - https://youtube.com/shorts/VIDEO_ID
 */
const YOUTUBE_PATTERNS = [
  // 표준 watch URL
  /(?:youtube\.com\/watch\?.*v=)([a-zA-Z0-9_-]{11})/,
  // 짧은 URL
  /(?:youtu\.be\/)([a-zA-Z0-9_-]{11})/,
  // embed URL
  /(?:youtube\.com\/embed\/)([a-zA-Z0-9_-]{11})/,
  // shorts URL
  /(?:youtube\.com\/shorts\/)([a-zA-Z0-9_-]{11})/,
];

export function extractVideoId(input: string): string | null {
  const trimmed = input.trim();

  // 이미 videoId 형식인 경우 (11자 영숫자+하이픈+언더스코어)
  if (/^[a-zA-Z0-9_-]{11}$/.test(trimmed)) {
    return trimmed;
  }

  for (const pattern of YOUTUBE_PATTERNS) {
    const match = trimmed.match(pattern);
    if (match?.[1]) {
      return match[1];
    }
  }

  return null;
}

export function isValidYouTubeUrl(input: string): boolean {
  return extractVideoId(input) !== null;
}
