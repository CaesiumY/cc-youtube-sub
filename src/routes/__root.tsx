import { Outlet, useMatches } from "@tanstack/react-router";
import { AnimatePresence, motion } from "motion/react";
import { UpdateBanner } from "../components/update-banner";

export function RootLayout() {
  const matches = useMatches();
  const matchKey = matches[matches.length - 1]?.id ?? "root";

  return (
    <div className="h-full w-full overflow-hidden bg-background text-foreground">
      <UpdateBanner />
      <AnimatePresence mode="wait">
        <motion.div
          key={matchKey}
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.25 }}
          className="h-full w-full"
        >
          <Outlet />
        </motion.div>
      </AnimatePresence>
    </div>
  );
}
