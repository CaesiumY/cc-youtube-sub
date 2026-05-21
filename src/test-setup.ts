import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

// vitest `globals`를 켜지 않으므로(기존 명시 import 컨벤션 유지)
// 각 테스트 후 렌더된 DOM을 직접 정리한다.
afterEach(() => {
  cleanup();
});
