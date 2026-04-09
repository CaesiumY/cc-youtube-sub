import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

// 디버그 빌드 바이너리 경로 (pnpm tauri build --debug 필요)
const APP_PATH = path.resolve(
  __dirname,
  "src-tauri/target/debug/cc-youtube-sub.exe",
);

export const config = {
  runner: "local",
  specs: ["./e2e/**/*.e2e.ts"],
  maxInstances: 1,

  capabilities: [
    {
      "tauri:options": {
        application: APP_PATH,
      },
    },
  ],

  logLevel: "info" as const,
  framework: "mocha",
  reporters: ["spec"],

  mochaOpts: {
    ui: "bdd",
    timeout: 120_000,
  },
};
