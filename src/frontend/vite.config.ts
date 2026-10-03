import { defineConfig } from "vite";
import { execFile, execFileSync } from "node:child_process";
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
  // Opt-in, loopback development fixture. Production never exposes font-probe.
  plugins:
    process.env.PINMETER_FONT_PROBE === "1"
      ? [
          {
            name: "pinmeter-font-probe-fixture",
            apply: "serve",
            configureServer(server) {
              server.middlewares.use(
                "/__test_font_catalog",
                (_request, response) => {
                  const executable = fileURLToPath(
                    new URL(
                      `../backend/target/debug/examples/font-probe${process.platform === "win32" ? ".exe" : ""}`,
                      import.meta.url,
                    ),
                  );
                  execFile(
                    executable,
                    ["--json"],
                    {
                      windowsHide: true,
                      timeout: 15000,
                      maxBuffer: 8 * 1024 * 1024,
                    },
                    (error, stdout) => {
                      response.statusCode = error ? 500 : 200;
                      response.setHeader(
                        "Content-Type",
                        "application/json; charset=utf-8",
                      );
                      response.end(
                        error
                          ? JSON.stringify({
                              error: "本机字体探针不可用，请先编译 font-probe",
                            })
                          : stdout,
                      );
                    },
                  );
                },
              );
            },
          },
        ]
      : [],
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
