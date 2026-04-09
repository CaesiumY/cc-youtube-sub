import { cn } from "../lib/utils";
import { type ModelAlias, useSettingsStore } from "../stores/settings-store";

const MODELS: { alias: ModelAlias; label: string; icon: string }[] = [
  { alias: "haiku", label: "Haiku", icon: "⚡" },
  { alias: "sonnet", label: "Sonnet", icon: "⭐" },
];

export function ModelSelector() {
  const { selectedModel, setSelectedModel } = useSettingsStore();

  return (
    <div className="flex items-center justify-center gap-1 rounded-xl border border-input bg-card p-1">
      {MODELS.map(({ alias, label, icon }) => (
        <button
          key={alias}
          type="button"
          onClick={() => setSelectedModel(alias)}
          className={cn(
            "rounded-lg px-4 py-1.5 text-sm font-medium transition-all",
            selectedModel === alias
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
