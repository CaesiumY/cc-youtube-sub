/**
 * `@tauri-apps/plugin-updater`가 throw하는 영문 Error를 사용자에게 보여줄
 * 한국어 메시지로 매핑한다. 원문은 호출부의 console.error에 그대로 남으므로
 * 여기서는 UI용 메시지만 반환한다.
 *
 * 매칭 우선순위가 중요하다. 예를 들어 tauri-plugin-updater의
 * `Error::ReleaseNotFound`는 "Could not fetch a valid release JSON from the
 * remote" 메시지를 내는데, "fetch"가 포함돼 있다고 네트워크 에러로 분류하면
 * 404(릴리스 없음) 상황을 네트워크 문제로 오인하게 된다. 따라서 릴리스 없음
 * 패턴을 가장 먼저 체크하고, 네트워크 분기는 `fetch` 단독이 아닌 구체적인
 * 문구(`failed to fetch`, `error sending request` 등)로만 잡는다.
 */
export function translateUpdateError(e: unknown): string {
  const raw = String((e as Error)?.message ?? e).toLowerCase();

  // 1. 릴리스 없음 / 404
  //    - tauri-plugin-updater Error::ReleaseNotFound: "Could not fetch a valid
  //      release JSON from the remote" (GitHub Releases가 Draft이거나 없을 때)
  //    - HTTP 404 / "not found" 일반 케이스
  if (
    raw.includes("404") ||
    raw.includes("not found") ||
    raw.includes("release json") ||
    raw.includes("valid release")
  ) {
    return "업데이트 정보를 찾을 수 없습니다 (서버에 릴리스가 없을 수 있습니다)";
  }

  // 2. 서명 검증 실패 (minisign)
  if (
    raw.includes("signature") ||
    raw.includes("verify") ||
    raw.includes("pubkey") ||
    raw.includes("minisign")
  ) {
    return "업데이트 파일 서명 검증에 실패했습니다";
  }

  // 3. 네트워크 연결 실패 (reqwest/연결 오류)
  //    fetch 단독 매칭은 피하고 구체적 문구만 사용.
  if (
    raw.includes("network") ||
    raw.includes("timeout") ||
    raw.includes("dns") ||
    raw.includes("econnrefused") ||
    raw.includes("connection") ||
    raw.includes("error sending request") ||
    raw.includes("failed to fetch") ||
    raw.includes("unable to fetch")
  ) {
    return "네트워크 연결을 확인해주세요";
  }

  // 4. JSON 파싱 실패
  if (
    raw.includes("parse") ||
    raw.includes("unexpected token") ||
    raw.includes("invalid json")
  ) {
    return "업데이트 정보 형식이 올바르지 않습니다";
  }

  return "업데이트 확인에 실패했습니다";
}
