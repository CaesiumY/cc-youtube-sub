import { cn } from "../lib/utils";
import { type ClaudeModel, useSettingsStore } from "../stores/settings-store";

const CLAUDE_MODELS: { alias: ClaudeModel; label: string; icon: string }[] = [
  { alias: "haiku", label: "Haiku", icon: "⚡" },
  { alias: "sonnet", label: "Sonnet", icon: "⭐" },
];

/**
 * 모델 선택 UI.
 *
 * - Claude: haiku/sonnet 토글
 * - Codex: 모델 선택이 없다(카탈로그 명령 부재). 항상 빠른 응답 모드로 동작하므로
 *   선택 불가한 정적 안내 배지를 표시한다.
 */
export function ModelSelector() {
  const { backend, claudeModel, setClaudeModel } = useSettingsStore();

  if (backend === "codex") {
    return (
      <div className="flex items-center justify-center gap-1 rounded-xl border border-input bg-card px-4 py-1.5">
        <span className="text-sm font-medium text-muted-foreground">
          ⚡ 빠른 응답 모드
        </span>
      </div>
    );
  }

  return (
    <div className="flex items-center justify-center gap-1 rounded-xl border border-input bg-card p-1">
      {CLAUDE_MODELS.map(({ alias, label, icon }) => (
        <button
          key={alias}
          type="button"
          onClick={() => setClaudeModel(alias)}
          className={cn(
            "rounded-lg px-4 py-1.5 text-sm font-medium transition-all",
            claudeModel === alias
              ? "bg-foreground text-background shadow-sm"
              : "text-muted-foreground hover:text-foreground",
          )}
        >
          {icon} {label}
        </button>
      ))}
    </div>
  );
}
