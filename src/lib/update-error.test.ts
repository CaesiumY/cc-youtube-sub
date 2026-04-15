import { describe, expect, it } from "vitest";
import { translateUpdateError } from "./update-error";

describe("translateUpdateError", () => {
  describe("릴리스 없음 / 404", () => {
    // tauri-plugin-updater v2.10.1의 Error::ReleaseNotFound Display 메시지 원문
    // (참고: crate src/error.rs:25)
    const RELEASE_NOT_FOUND_MSG =
      "Could not fetch a valid release JSON from the remote";

    it("ReleaseNotFound 메시지를 '업데이트 정보를 찾을 수 없습니다'로 매핑", () => {
      expect(translateUpdateError(new Error(RELEASE_NOT_FOUND_MSG))).toMatch(
        /찾을 수 없습니다/,
      );
    });

    it("GitHub Draft 릴리스 404 재현: '네트워크'로 오분류되면 안 된다", () => {
      // 회귀 테스트: "fetch" 단어가 포함되어 있어도 네트워크로 빠지면 안 됨
      const result = translateUpdateError(new Error(RELEASE_NOT_FOUND_MSG));
      expect(result).not.toMatch(/네트워크/);
    });

    it("명시적 HTTP 404", () => {
      expect(translateUpdateError(new Error("HTTP 404 Not Found"))).toMatch(
        /찾을 수 없습니다/,
      );
    });

    it("lowercase 'not found'", () => {
      expect(translateUpdateError(new Error("resource not found"))).toMatch(
        /찾을 수 없습니다/,
      );
    });
  });

  describe("네트워크 에러", () => {
    it("Network Error", () => {
      expect(translateUpdateError(new Error("Network Error"))).toMatch(
        /네트워크 연결/,
      );
    });

    it("request timeout", () => {
      expect(translateUpdateError(new Error("request timeout"))).toMatch(
        /네트워크 연결/,
      );
    });

    it("DNS lookup failure", () => {
      expect(translateUpdateError(new Error("dns lookup failed"))).toMatch(
        /네트워크 연결/,
      );
    });

    it("ECONNREFUSED", () => {
      expect(
        translateUpdateError(new Error("connect ECONNREFUSED 127.0.0.1:443")),
      ).toMatch(/네트워크 연결/);
    });

    it("reqwest 'error sending request'", () => {
      // reqwest crate가 연결 실패 시 던지는 전형적 문구
      expect(
        translateUpdateError(
          new Error("error sending request for url (https://example.com)"),
        ),
      ).toMatch(/네트워크 연결/);
    });
  });

  describe("서명 검증 실패", () => {
    it("signature mismatch", () => {
      expect(
        translateUpdateError(new Error("signature verification failed")),
      ).toMatch(/서명 검증/);
    });

    it("minisign 에러", () => {
      expect(
        translateUpdateError(new Error("minisign: unknown pubkey")),
      ).toMatch(/서명 검증/);
    });
  });

  describe("JSON 파싱 실패", () => {
    it("unexpected token", () => {
      expect(
        translateUpdateError(
          new Error("Unexpected token '<' in JSON at position 0"),
        ),
      ).toMatch(/형식이 올바르지 않습니다/);
    });

    it("일반 parse 실패", () => {
      expect(
        translateUpdateError(new Error("failed to parse response body")),
      ).toMatch(/형식이 올바르지 않습니다/);
    });
  });

  describe("기본값", () => {
    it("매칭되지 않는 에러", () => {
      expect(translateUpdateError(new Error("some unexpected failure"))).toBe(
        "업데이트 확인에 실패했습니다",
      );
    });

    it("문자열 에러 (Error 인스턴스가 아닌 경우)", () => {
      expect(translateUpdateError("plain string error")).toBe(
        "업데이트 확인에 실패했습니다",
      );
    });

    it("null 처리", () => {
      expect(translateUpdateError(null)).toBe("업데이트 확인에 실패했습니다");
    });

    it("undefined 처리", () => {
      expect(translateUpdateError(undefined)).toBe(
        "업데이트 확인에 실패했습니다",
      );
    });
  });
});
