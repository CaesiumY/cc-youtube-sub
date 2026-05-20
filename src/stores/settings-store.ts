import { create } from "zustand";
import { persist } from "zustand/middleware";

export type BackendType = "claude" | "codex";
export type ClaudeModel = "haiku" | "sonnet";

/**
 * 설정 스토어.
 *
 * 모델 선택은 Claude 백엔드에만 존재한다(haiku/sonnet). Codex CLI는 모델 카탈로그
 * 명령이 없고, 속도는 `model_reasoning_effort`로 조절되며 본 앱은 자막 번역 특성상
 * 항상 빠른 모드(low effort)로 고정한다 — 따라서 Codex용 모델 선택 상태는 없다.
 */
interface SettingsState {
  backend: BackendType;
  claudeModel: ClaudeModel;
  setBackend: (backend: BackendType) => void;
  setClaudeModel: (model: ClaudeModel) => void;
}

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set) => ({
      backend: "claude",
      claudeModel: "haiku",
      setBackend: (backend) => set({ backend }),
      setClaudeModel: (claudeModel) => set({ claudeModel }),
    }),
    {
      name: "yt-subtitle-settings",
      version: 2,
      migrate: (persistedState, version) => {
        // v0/v1 (백엔드 도입 전): `selectedModel: ClaudeModel` 만 저장하던 시절
        if (version < 2) {
          const old = (persistedState ?? {}) as { selectedModel?: ClaudeModel };
          return {
            backend: "claude" as BackendType,
            claudeModel: old.selectedModel ?? "haiku",
          } as SettingsState;
        }
        return persistedState as SettingsState;
      },
    },
  ),
);

/**
 * 현재 백엔드의 활성 모델 alias를 반환 (Tauri command의 `model` 인자에 전달).
 *
 * Codex는 모델 선택이 없으므로 `undefined`를 반환한다 — 이 경우 `translate_chunk`/
 * `init_buffer`의 `model` 인자에 `None`이 흘러가 codex adapter가 `--model`을 생략하고
 * codex 기본 모델을 사용한다.
 */
export function getActiveModel(state: SettingsState): string | undefined {
  return state.backend === "claude" ? state.claudeModel : undefined;
}
