import { useEffect, useState } from "react";
import { BackendSelector } from "../components/backend-selector";
import { ModelSelector } from "../components/model-selector";
import { UrlInput } from "../components/url-input";
import {
  type AppError,
  checkEnvironment,
  isTauri,
} from "../lib/tauri-commands";
import { useSettingsStore } from "../stores/settings-store";
import { useUpdateStore } from "../stores/update-store";

type EnvStatus = "checking" | "ok" | "not_installed" | "execution_failed";

const INSTALL_HINTS: Record<
  "claude" | "codex",
  { name: string; command: string }
> = {
  claude: {
    name: "Claude CLI",
    command: "npm install -g @anthropic-ai/claude-code",
  },
  codex: { name: "Codex CLI", command: "npm install -g @openai/codex" },
};

export function HomeView() {
  const backend = useSettingsStore((s) => s.backend);
  const [envStatus, setEnvStatus] = useState<EnvStatus>(
    isTauri() ? "checking" : "ok",
  );
  const updateStatus = useUpdateStore((s) => s.status);
  const isCheckingUpdate = updateStatus === "checking";

  // 백엔드 변경 또는 Home 진입 시 환경 검증.
  // 브라우저 mock 환경에서는 항상 "ok" 처리(mock-tauri::checkEnvironment가 성공 반환).
  useEffect(() => {
    if (!isTauri()) {
      setEnvStatus("ok");
      return;
    }
    let cancelled = false;
    setEnvStatus("checking");
    checkEnvironment(backend)
      .then(() => {
        if (!cancelled) setEnvStatus("ok");
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        let parsed: unknown;
        try {
          parsed = typeof err === "string" ? JSON.parse(err) : err;
        } catch {
          parsed = err;
        }
        const appErr = parsed as AppError | undefined;
        if (appErr?.kind === "EnvironmentCheck") {
          setEnvStatus(
            appErr.message?.startsWith("NOT_INSTALLED")
              ? "not_installed"
              : "execution_failed",
          );
        } else {
          console.error(
            "[HomeView] unexpected checkEnvironment error:",
            parsed,
          );
          setEnvStatus("execution_failed");
        }
      });
    return () => {
      cancelled = true;
    };
  }, [backend]);

  const handleCheckUpdate = () => {
    useUpdateStore.getState().checkForUpdate("manual");
  };

  const hint = INSTALL_HINTS[backend];
  const isBlocked = envStatus !== "ok" && envStatus !== "checking";

  return (
    <div className="flex h-full flex-col items-center justify-center p-8">
      <div className="flex w-full max-w-xl flex-col items-center gap-4">
        <UrlInput
          disabled={isBlocked || envStatus === "checking"}
          placeholderOverride={
            envStatus === "checking" ? "환경 확인 중..." : undefined
          }
        />
        <div className="flex flex-wrap items-center justify-center gap-2">
          <BackendSelector />
          <ModelSelector />
        </div>
        {isBlocked && (
          <div className="w-full rounded-xl border border-destructive bg-destructive/10 p-4 text-sm">
            <p className="font-medium text-destructive">
              {hint.name}가 설치되지 않았습니다.
            </p>
            <p className="mt-1 text-muted-foreground">
              다음 명령으로 설치 후 다시 선택하세요:
            </p>
            <pre className="mt-2 rounded-md bg-card px-3 py-2 font-mono text-xs">
              {hint.command}
            </pre>
            {envStatus === "execution_failed" && (
              <p className="mt-2 text-xs text-muted-foreground">
                (CLI는 발견되었지만 실행에 실패했습니다. 설치를 다시
                확인해주세요.)
              </p>
            )}
          </div>
        )}
      </div>
      <button
        type="button"
        onClick={handleCheckUpdate}
        disabled={isCheckingUpdate}
        className="mt-8 text-xs text-zinc-500 transition-colors hover:text-zinc-300 disabled:cursor-not-allowed disabled:opacity-60"
      >
        {isCheckingUpdate ? "업데이트 확인 중..." : "업데이트 확인"}
      </button>
    </div>
  );
}
