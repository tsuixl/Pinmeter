import { defineConfig } from "vite";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
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
  server: {
    port: 1420,
    strictPort: true,
    fs: {
      allow: [
        fileURLToPath(new URL(".", import.meta.url)),
        fileURLToPath(new URL("../shared/fonts", import.meta.url)),
      ],
    },
  },
  build: { target: "es2022" },
  clearScreen: false,
});
