import { AlertTriangle } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import type { EnvErrorKind } from "../lib/tauri-commands";
import type { BackendType } from "../stores/settings-store";

interface ErrorModalProps {
  open: boolean;
  errorKind?: EnvErrorKind;
  backend?: BackendType;
}

const BACKEND_INFO: Record<
  BackendType,
  { displayName: string; command: string; cliBinary: string }
> = {
  claude: {
    displayName: "Claude Code CLI",
    command: "npm install -g @anthropic-ai/claude-code",
    cliBinary: "claude",
  },
  codex: {
    displayName: "OpenAI Codex CLI",
    command: "npm install -g @openai/codex",
    cliBinary: "codex",
  },
};

/**
 * 백엔드 CLI 환경 검증 실패 시 전체 화면을 가리는 모달.
 * 에러 종류와 백엔드에 따라 다른 안내 메시지를 표시한다.
 * - not_installed: CLI가 설치되지 않음 → npm install 안내
 * - execution_failed: CLI는 있지만 실행 실패 → PATH/재시작 안내
 */
export function ErrorModal({
  open,
  errorKind = "not_installed",
  backend = "claude",
}: ErrorModalProps) {
  const info = BACKEND_INFO[backend];

  return (
    <AnimatePresence>
      {open && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
        >
          <motion.div
            initial={{ scale: 0.95, opacity: 0 }}
            animate={{ scale: 1, opacity: 1 }}
            exit={{ scale: 0.95, opacity: 0 }}
            transition={{ duration: 0.2 }}
            className="mx-4 flex max-w-md flex-col items-center gap-4 rounded-xl bg-zinc-900 px-8 py-8 shadow-2xl ring-1 ring-white/10"
          >
            <div className="flex h-12 w-12 items-center justify-center rounded-full bg-red-500/10">
              <AlertTriangle className="h-6 w-6 text-red-400" />
            </div>

            {errorKind === "not_installed" ? (
              <>
                <h2 className="text-lg font-semibold text-white">
                  {info.displayName}가 필요합니다
                </h2>

                <p className="text-center text-sm leading-relaxed text-zinc-400">
                  자막 번역 기능을 사용하려면 {info.displayName}가 설치되어
                  있어야 합니다. 아래 명령어로 설치한 후 앱을 다시 실행해
                  주세요.
                </p>

                <code className="w-full rounded-lg bg-zinc-800 px-4 py-3 text-center font-mono text-sm text-emerald-400 select-all">
                  {info.command}
                </code>

                <p className="text-xs text-zinc-500">
                  설치 후 터미널에서{" "}
                  <code className="rounded bg-zinc-800 px-1.5 py-0.5 text-zinc-300">
                    {info.cliBinary} --version
                  </code>{" "}
                  으로 확인할 수 있습니다.
                </p>
              </>
            ) : (
              <>
                <h2 className="text-lg font-semibold text-white">
                  {info.displayName} 실행 오류
                </h2>

                <p className="text-center text-sm leading-relaxed text-zinc-400">
                  {info.displayName}가 설치되어 있지만 실행할 수 없습니다. 다음
                  사항을 확인해 주세요.
                </p>

                <ul className="w-full space-y-2 text-sm text-zinc-400">
                  <li className="flex gap-2">
                    <span className="text-zinc-500">1.</span>
                    PC를 재시작하여 PATH 환경변수를 갱신하세요.
                  </li>
                  <li className="flex gap-2">
                    <span className="text-zinc-500">2.</span>
                    터미널에서{" "}
                    <code className="rounded bg-zinc-800 px-1.5 py-0.5 text-zinc-300">
                      {info.cliBinary} --version
                    </code>{" "}
                    이 정상 동작하는지 확인하세요.
                  </li>
                  <li className="flex gap-2">
                    <span className="text-zinc-500">3.</span>
                    문제가 지속되면 CLI를 재설치해 보세요.
                  </li>
                </ul>

                <code className="w-full rounded-lg bg-zinc-800 px-4 py-3 text-center font-mono text-sm text-emerald-400 select-all">
                  {info.command}
                </code>
              </>
            )}
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
