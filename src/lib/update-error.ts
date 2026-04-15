/**
 * `@tauri-apps/plugin-updater`가 throw하는 영문 Error를 사용자에게 보여줄
 * 한국어 메시지로 매핑한다. 원문은 호출부의 console.error에 그대로 남으므로
 * 여기서는 UI용 메시지만 반환한다.
 */
export function translateUpdateError(e: unknown): string {
  const raw = String((e as Error)?.message ?? e).toLowerCase();

  if (raw.includes("404") || raw.includes("not found")) {
    return "업데이트 정보를 찾을 수 없습니다 (서버에 릴리스가 없을 수 있습니다)";
  }
  if (
    raw.includes("network") ||
    raw.includes("fetch") ||
    raw.includes("timeout") ||
    raw.includes("dns") ||
    raw.includes("econnrefused")
  ) {
    return "네트워크 연결을 확인해주세요";
  }
  if (
    raw.includes("signature") ||
    raw.includes("verify") ||
    raw.includes("pubkey")
  ) {
    return "업데이트 파일 서명 검증에 실패했습니다";
  }
  if (
    raw.includes("parse") ||
    raw.includes("json") ||
    raw.includes("unexpected token")
  ) {
    return "업데이트 정보 형식이 올바르지 않습니다";
  }
  return "업데이트 확인에 실패했습니다";
}
