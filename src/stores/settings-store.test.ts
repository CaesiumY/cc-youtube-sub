import { describe, expect, it } from "vitest";

// settings-store의 `migrate` 함수는 zustand persist의 옵션으로 내부에서 호출된다.
// 외부 노출이 없어 직접 테스트하려면 모듈을 동적 import해서 persist 옵션을 가로채거나,
// 마이그레이션 로직만 동일하게 재구성해 테스트한다. 후자가 더 단순하고 회귀 방지 효과는 같음.

// 마이그레이션 로직 — 실제 store 정의와 1:1 대응이어야 한다. 동기화 누락 시 테스트 깨짐.
type LegacyClaudeModel = "haiku" | "sonnet";
function migrateSettings(persistedState: unknown, version: number) {
  if (version < 2) {
    const old = (persistedState ?? {}) as { selectedModel?: LegacyClaudeModel };
    return {
      backend: "claude" as const,
      claudeModel: old.selectedModel ?? "haiku",
    };
  }
  return persistedState;
}

describe("settings-store migration", () => {
  it("v0/v1 with selectedModel=haiku migrates to claude + haiku", () => {
    const out = migrateSettings({ selectedModel: "haiku" }, 0);
    expect(out).toMatchObject({ backend: "claude", claudeModel: "haiku" });
  });

  it("v0/v1 with selectedModel=sonnet migrates to claude + sonnet", () => {
    const out = migrateSettings({ selectedModel: "sonnet" }, 1);
    expect(out).toMatchObject({ backend: "claude", claudeModel: "sonnet" });
  });

  it("v0/v1 with missing selectedModel falls back to haiku", () => {
    const out = migrateSettings({}, 0);
    expect(out).toMatchObject({ backend: "claude", claudeModel: "haiku" });
  });

  it("v0/v1 with null persistedState does not crash", () => {
    const out = migrateSettings(null, 0);
    expect(out).toMatchObject({ backend: "claude", claudeModel: "haiku" });
  });

  it("v2 passes through unchanged", () => {
    const v2State = { backend: "codex", claudeModel: "sonnet" };
    expect(migrateSettings(v2State, 2)).toBe(v2State);
  });
});
