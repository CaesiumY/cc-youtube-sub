import { UrlInput } from "../components/url-input";

export function HomeView() {
  return (
    <div className="flex h-full items-center justify-center p-8">
      <div className="w-full max-w-xl">
        <UrlInput />
      </div>
    </div>
  );
}
