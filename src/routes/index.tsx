import { ModelSelector } from "../components/model-selector";
import { UrlInput } from "../components/url-input";
import { useUpdateStore } from "../stores/update-store";

export function HomeView() {
  const status = useUpdateStore((s) => s.status);
  const isChecking = status === "checking";

  const handleCheckUpdate = () => {
    useUpdateStore.getState().checkForUpdate("manual");
  };

  return (
    <div className="flex h-full flex-col items-center justify-center p-8">
      <div className="flex w-full max-w-xl flex-col items-center gap-4">
        <UrlInput />
        <ModelSelector />
      </div>
      <button
        type="button"
        onClick={handleCheckUpdate}
        disabled={isChecking}
        className="mt-8 text-xs text-zinc-500 transition-colors hover:text-zinc-300 disabled:cursor-not-allowed disabled:opacity-60"
      >
        {isChecking ? "업데이트 확인 중..." : "업데이트 확인"}
      </button>
    </div>
  );
}
