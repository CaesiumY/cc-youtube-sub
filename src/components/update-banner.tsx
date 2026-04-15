import { AlertTriangle, Download, RefreshCw, X, Zap } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { useEffect, useRef } from "react";
import { useUpdateStore } from "../stores/update-store";

export function UpdateBanner() {
  const {
    status,
    version,
    progress,
    dismissed,
    error,
    lastTriggeredBy,
    dismiss,
  } = useUpdateStore();
  const downloadAndInstall = useUpdateStore((s) => s.downloadAndInstall);
  const relaunch = useUpdateStore((s) => s.relaunch);
  const checkForUpdate = useUpdateStore((s) => s.checkForUpdate);

  // 앱 시작 시 1회 자동 확인. 실패해도 UI에는 노출하지 않는다(auto 트리거).
  const checkedRef = useRef(false);
  useEffect(() => {
    if (!checkedRef.current) {
      checkedRef.current = true;
      checkForUpdate("auto");
    }
  }, [checkForUpdate]);

  const isErrorVisible = status === "error" && lastTriggeredBy === "manual";
  const visible =
    !dismissed &&
    (status === "available" ||
      status === "downloading" ||
      status === "ready" ||
      isErrorVisible);

  const handleUpdate = () => {
    downloadAndInstall();
  };

  const handleRetry = () => {
    checkForUpdate("manual");
  };

  const handleRelaunch = async () => {
    const canRelaunch = await relaunch();
    if (!canRelaunch) {
      const confirmed = window.confirm(
        "번역이 진행 중입니다. 재시작하면 진행 중인 번역이 손실됩니다. 계속하시겠습니까?",
      );
      if (confirmed) {
        const { relaunch: forceRelaunch } = await import(
          "@tauri-apps/plugin-process"
        );
        await forceRelaunch();
      }
    }
  };

  const background = isErrorVisible
    ? "oklch(0.32 0.12 25)"
    : "oklch(0.3 0.08 250)";

  return (
    <AnimatePresence>
      {visible && (
        <motion.div
          initial={{ y: -60, opacity: 0 }}
          animate={{ y: 0, opacity: 1 }}
          exit={{ y: -60, opacity: 0 }}
          transition={{ duration: 0.3 }}
          className="fixed top-0 left-0 right-0 z-40 flex items-center justify-center gap-3 px-4 py-2.5 text-sm"
          style={{ background }}
        >
          {status === "available" && (
            <>
              <Zap className="h-4 w-4 shrink-0 text-blue-300" />
              <span className="text-zinc-200">
                v{version} 업데이트 사용 가능
              </span>
              <button
                type="button"
                onClick={handleUpdate}
                className="rounded-md bg-blue-500 px-3 py-1 text-xs font-medium text-white transition-colors hover:bg-blue-400"
              >
                지금 업데이트
              </button>
            </>
          )}

          {status === "downloading" && (
            <>
              <Download className="h-4 w-4 shrink-0 animate-pulse text-blue-300" />
              <span className="text-zinc-200">다운로드 중...</span>
              <div className="h-1.5 w-32 overflow-hidden rounded-full bg-zinc-700">
                <motion.div
                  className="h-full rounded-full bg-blue-400"
                  initial={{ width: 0 }}
                  animate={{ width: `${progress}%` }}
                  transition={{ duration: 0.3 }}
                />
              </div>
              <span className="text-xs text-zinc-400">{progress}%</span>
            </>
          )}

          {status === "ready" && (
            <>
              <RefreshCw className="h-4 w-4 shrink-0 text-emerald-300" />
              <span className="text-zinc-200">다운로드 완료</span>
              <button
                type="button"
                onClick={handleRelaunch}
                className="rounded-md bg-emerald-500 px-3 py-1 text-xs font-medium text-white transition-colors hover:bg-emerald-400"
              >
                재시작하여 적용
              </button>
            </>
          )}

          {isErrorVisible && (
            <>
              <AlertTriangle className="h-4 w-4 shrink-0 text-red-300" />
              <span className="text-zinc-200">
                {error ?? "업데이트 확인에 실패했습니다"}
              </span>
              <button
                type="button"
                onClick={handleRetry}
                className="rounded-md bg-red-500/80 px-3 py-1 text-xs font-medium text-white transition-colors hover:bg-red-500"
              >
                다시 확인
              </button>
            </>
          )}

          {status !== "downloading" && (
            <button
              type="button"
              onClick={dismiss}
              className="absolute right-3 rounded p-0.5 text-zinc-400 transition-colors hover:text-zinc-200"
            >
              <X className="h-3.5 w-3.5" />
            </button>
          )}
        </motion.div>
      )}
    </AnimatePresence>
  );
}
