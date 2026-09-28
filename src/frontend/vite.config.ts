import { defineConfig } from "vite";
import { execFileSync } from "node:child_process";
const revision = (() => {
  try {
    return execFileSync("git", ["describe", "--always", "--dirty"], {
      encoding: "utf8",
    }).trim();
  } catch {
    return "源码包构建";
  }
})();

export default defineConfig({
  define: {
    __BUILD_REVISION__: JSON.stringify(revision),
    __BUILD_TIME__: JSON.stringify(new Date().toISOString()),
  },
  server: { port: 1420, strictPort: true },
  build: { target: "es2022" },
  clearScreen: false,
});
