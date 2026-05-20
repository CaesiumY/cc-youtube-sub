import { cn } from "../lib/utils";
import { type BackendType, useSettingsStore } from "../stores/settings-store";

const BACKENDS: { type: BackendType; label: string; icon: string }[] = [
  { type: "claude", label: "Claude", icon: "🅒" },
  { type: "codex", label: "Codex", icon: "🅞" },
];

export function BackendSelector() {
  const backend = useSettingsStore((s) => s.backend);
  const setBackend = useSettingsStore((s) => s.setBackend);

  return (
    <div className="flex items-center justify-center gap-1 rounded-xl border border-input bg-card p-1">
      {BACKENDS.map(({ type, label, icon }) => (
        <button
          key={type}
          type="button"
          onClick={() => setBackend(type)}
          className={cn(
            "rounded-lg px-4 py-1.5 text-sm font-medium transition-all",
            backend === type
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
