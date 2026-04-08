import { AlertTriangle } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";

interface ErrorModalProps {
  open: boolean;
}

/**
 * Claude CLI 미설치 시 전체 화면을 가리는 모달.
 * 앱 차단 — 자막 번역 기능이 작동할 수 없으므로 사용자 조치가 필요.
 */
export function ErrorModal({ open }: ErrorModalProps) {
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

            <h2 className="text-lg font-semibold text-white">
              Claude Code CLI가 필요합니다
            </h2>

            <p className="text-center text-sm leading-relaxed text-zinc-400">
              자막 번역 기능을 사용하려면 Claude Code CLI가 설치되어 있어야
              합니다. 아래 명령어로 설치한 후 앱을 다시 실행해 주세요.
            </p>

            <code className="w-full rounded-lg bg-zinc-800 px-4 py-3 text-center font-mono text-sm text-emerald-400 select-all">
              npm install -g @anthropic-ai/claude-code
            </code>

            <p className="text-xs text-zinc-500">
              설치 후 터미널에서{" "}
              <code className="rounded bg-zinc-800 px-1.5 py-0.5 text-zinc-300">
                claude --version
              </code>{" "}
              으로 확인할 수 있습니다.
            </p>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
