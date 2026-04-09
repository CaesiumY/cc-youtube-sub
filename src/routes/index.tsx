import { ModelSelector } from "../components/model-selector";
import { UrlInput } from "../components/url-input";

export function HomeView() {
  return (
    <div className="flex h-full items-center justify-center p-8">
      <div className="flex w-full max-w-xl flex-col items-center gap-4">
        <UrlInput />
        <ModelSelector />
      </div>
    </div>
  );
}
