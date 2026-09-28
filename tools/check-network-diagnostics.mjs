// Read-only system checks. Never creates a block, opens WinDivert or changes VPN settings.
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { spawn, spawnSync } from "node:child_process";
import { createInterface } from "node:readline";
import { setTimeout as delay } from "node:timers/promises";

const output = path.resolve(process.argv[2]);
const sourceHelper = path.resolve(
  process.argv[3] ??
    "src/backend/target/network-control/pinmeter-network-control.exe",
);
assert(
  /^test[1-9][0-9]*$/.test(path.basename(output)),
  "Use a reserved numbered directory",
);
const artifacts = fs.mkdtempSync(path.join(output, "diagnostics-tests-"));
const helper = path.join(
  artifacts,
  "isolated/network-control/pinmeter-network-control.exe",
);
fs.mkdirSync(path.dirname(helper), { recursive: true });
fs.copyFileSync(sourceHelper, helper);
const platform = spawnSync(helper, ["--inspect-platform"], {
  encoding: "utf8",
  windowsHide: true,
});
assert.equal(platform.status, 0, platform.stderr);
const initialPlatform = JSON.parse(platform.stdout.replace(/^\uFEFF/, ""));
assert.equal(initialPlatform.owned_rules, 0);
if (initialPlatform.admin) assert.equal(initialPlatform.wfp_owned_rules, 0);
const source = path.resolve("src/backend/platform/network-control");
const harness = path.join(artifacts, "diagnostics-tests.exe");
const compiled = spawnSync(
  path.join(process.env.WINDIR, "Microsoft.NET/Framework64/v4.0.30319/csc.exe"),
  [
    "/nologo",
    "/target:exe",
    "/platform:x64",
    "/main:DiagnosticsTests",
    `/out:${harness}`,
    "/reference:System.Web.Extensions.dll",
    "/reference:Microsoft.CSharp.dll",
    ...[
      "Program.cs",
      "PacketEngine.cs",
      "WfpBlock.cs",
      "Diagnostics.cs",
      "tests/DiagnosticsTests.cs",
    ].map((f) => path.join(source, f)),
  ],
  { encoding: "utf8", windowsHide: true },
);
assert.equal(compiled.status, 0, compiled.stdout + compiled.stderr);
const results = [];
for (const mode of [
  "rotation",
  "queue",
  "unwritable",
  "write-failure",
  "retention",
]) {
  const tested = spawnSync(harness, [mode, path.join(artifacts, mode)], {
    encoding: "utf8",
    windowsHide: true,
    timeout: 15000,
  });
  assert.equal(tested.status, 0, tested.stdout + tested.stderr);
  results.push({ mode, passed: true });
}
for (const mode of ["shutdown", "eof"]) {
  const child = spawn(helper, [String(process.pid)], { windowsHide: true });
  const lines = [];
  let exit,
    stderr = "";
  child.on("exit", (code) => {
    exit = code;
  });
  child.stderr.on("data", (bytes) => {
    stderr += bytes;
  });
  createInterface({ input: child.stdout }).on("line", (line) =>
    lines.push(JSON.parse(line)),
  );
  const request = async (command) => {
    child.stdin.write(JSON.stringify({ command, rules: [] }) + "\n");
    const deadline = Date.now() + 10000;
    while (!lines.length) {
      assert.equal(exit, undefined, `Unexpected exit: ${stderr}`);
      assert(Date.now() < deadline, `No reply to ${command}`);
      await delay(20);
    }
    return lines.shift();
  };
  try {
    const before = await request("inspect");
    assert.equal(before.ok, true);
    assert.equal(before.report.engine.engine_present, false);
    assert.equal(before.report.diagnostics.error, "");
    const applied = await request("apply");
    assert.equal(
      applied.ok,
      before.report.available,
      "Empty apply must follow actual privilege",
    );
    // Covers a full background inventory interval, independent of an empty config.
    await delay(16000);
    const after = await request("inspect");
    assert.equal(after.report.engine.engine_present, false);
    assert.deepEqual(after.report.rules, []);
    // Fresh isolated owner verified empty above. This only tests the no-op cleanup path.
    if (before.report.available) {
      assert.equal((await request("release-all")).ok, true);
      const flushed = fs.readFileSync(before.report.diagnostics.path, "utf8");
      assert(
        flushed.includes('"kind":"cleanup-complete"'),
        "Cleanup evidence missing when reply arrived",
      );
    }
    if (mode === "shutdown") child.stdin.write('{"command":"shutdown"}\n');
    child.stdin.end();
    const deadline = Date.now() + 5000;
    while (exit === undefined && Date.now() < deadline) await delay(20);
    assert.equal(exit, 0, stderr);
    const records = fs
      .readFileSync(before.report.diagnostics.path, "utf8")
      .trim()
      .split(/\r?\n/)
      .map(JSON.parse);
    assert(records.some((r) => r.kind === "helper-stop"));
    assert(records.some((r) => r.kind.startsWith("firewall-inventory")));
    assert(records.some((r) => r.kind.startsWith("wfp-inventory")));
    assert(
      !records.some((r) =>
        [
          "engine-create",
          "network-open",
          "flow-open",
          "wfp-apply",
          "firewall-apply",
        ].includes(r.kind),
      ),
    );
    if (before.report.available)
      assert(
        records.some((r) => r.kind === "cleanup-complete" && r.data.verified),
      );
    fs.copyFileSync(
      before.report.diagnostics.path,
      path.join(artifacts, `${mode}.jsonl`),
    );
    results.push({
      mode,
      passed: true,
      admin: before.report.available,
      emptyApplyVerified: applied.ok,
      inventory: records
        .filter((r) => r.kind.includes("inventory"))
        .map((r) => ({ kind: r.kind, data: r.data })),
    });
  } finally {
    if (exit === undefined) child.kill(); // Only this test's own child; no system rules were created.
  }
}
fs.writeFileSync(
  path.join(artifacts, "results.json"),
  JSON.stringify(results, null, 2) + "\n",
);
console.log(JSON.stringify(results, null, 2));
