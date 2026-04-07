import {
  createHashHistory,
  createRootRoute,
  createRoute,
  createRouter,
} from "@tanstack/react-router";
import { RootLayout } from "./routes/__root";
import { HomeView } from "./routes/index";
import { PlayerView } from "./routes/watch.$videoId";

// Tauri 파일 프로토콜 호환 — hash history 필수
const hashHistory = createHashHistory();

const rootRoute = createRootRoute({
  component: RootLayout,
});

const homeRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  component: HomeView,
});

const watchRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/watch/$videoId",
  component: PlayerView,
});

const routeTree = rootRoute.addChildren([homeRoute, watchRoute]);

export const router = createRouter({
  routeTree,
  history: hashHistory,
});

// 타입 안전한 라우터 등록
declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
