// Run explicitly with administrator rights. Only the copied probe executable is blocked.
const fs = require("node:fs");
const path = require("node:path");
const http = require("node:http");
const readline = require("node:readline");
const { spawn, spawnSync } = require("node:child_process");
const assert = require("node:assert/strict");
const root = path.resolve(__dirname, "..");
const base = path.resolve(process.env.PINMETER_BLOCK_OUTPUT || path.join(root, "src/backend/target/network-control-proxy-check"));
fs.mkdirSync(base, { recursive: true });
const resultPath = path.join(base, "result.json");
const helperSource = process.env.PINMETER_CONTROL_HELPER || path.join(
  root,
  "src/backend/target/network-control/pinmeter-network-control.exe",
);
const curlSource = path.join(process.env.WINDIR, "System32/curl.exe");
const folder = fs.mkdtempSync(path.join(base, "run-"));
// A unique installation identity prevents cleanup from touching the user's Pinmeter rules.
const helperFolder = path.join(folder, "helper-install/network-control");
fs.mkdirSync(helperFolder, { recursive: true });
const helper = path.join(helperFolder, "pinmeter-network-control.exe");
fs.copyFileSync(helperSource, helper);
const { createHash } = require("node:crypto");
const hash = (file) => createHash("sha256").update(fs.readFileSync(file)).digest("hex");
assert.equal(hash(helper), hash(helperSource), "Probe must use the requested helper");
const probe = path.join(folder, "network-probe.exe");
fs.copyFileSync(curlSource, probe);
const proxy = process.argv[2]; // Optional explicit local HTTP proxy, e.g. http://127.0.0.1:10808.
// Optional exact run name: recover only this previous isolated installation before testing.
const recoverRun = process.argv[3];
if (recoverRun) assert.match(recoverRun, /^run-[A-Za-z0-9]{6}$/);
if (proxy) {
  const url = new URL(proxy);
  assert(
    ["127.0.0.1", "[::1]"].includes(url.hostname) &&
      url.protocol === "http:" &&
      !url.username &&
      !url.password,
    "Only a local test proxy is supported",
  );
}
const log = [];
const record = (name, value) => {
  log.push({ name, value });
  fs.writeFileSync(resultPath, JSON.stringify(log, null, 2));
  if (["error", "cleanup-error", "passed"].includes(name))
    console.log(name + ": " + JSON.stringify(value));
};
const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
function inspect(target = helper) {
  const r = spawnSync(target, ["--inspect-platform"], {
    encoding: "utf8",
    windowsHide: true,
    timeout: 10000,
  });
  assert.equal(r.status, 0, r.stderr);
  return JSON.parse(r.stdout);
}
function connect() {
  const child = spawn(helper, [String(process.pid)], {
    windowsHide: true,
    stdio: ["pipe", "pipe", "pipe"],
  });
  const lines = readline.createInterface({ input: child.stdout });
  let pending;
  let failure;
  function fail(error) {
    failure = error;
    if (pending) {
      const next = pending;
      pending = null;
      next(error);
    }
  }
  child.on("error", fail);
  child.stdin.on("error", fail);
  child.stderr.resume();
  lines.on("line", (line) => {
    if (pending) {
      const next = pending;
      pending = null;
      try {
        next(null, JSON.parse(line));
      } catch (error) {
        next(error);
      }
    }
  });
  child.on("exit", (code) => {
    fail(Error("Helper exited: " + code));
  });
  return {
    async request(rules, command = "apply") {
      if (failure) throw failure;
      const response = await new Promise((resolve, reject) => {
        const timer = setTimeout(() => {
          pending = null;
          reject(Error("Helper timed out"));
        }, 15000);
        pending = (error, data) => {
          clearTimeout(timer);
          error ? reject(error) : resolve(data);
        };
        child.stdin.write(JSON.stringify({ command, rules }) + "\n");
      });
      assert.equal(response.ok, true, response.error);
      assert.equal(
        response.report.rules[0].status,
        "applied",
        JSON.stringify(response),
      );
      return response.report;
    },
    async close() {
      if (child.exitCode !== null || child.signalCode !== null || !child.pid)
        return;
      const exited = new Promise((resolve) => child.once("exit", resolve));
      if (!child.stdin.destroyed)
        child.stdin.end(JSON.stringify({ command: "shutdown" }) + "\n");
      const timer = setTimeout(() => child.kill(), 5000);
      await exited;
      clearTimeout(timer);
    },
  };
}
async function transfer(executable, url, viaProxy) {
  return new Promise((resolve, reject) => {
    const args = [
      "--max-time",
      "5",
      "-s",
      "-o",
      "NUL",
      "-w",
      "%{http_code} %{size_download}",
      "--noproxy",
      viaProxy ? "" : "*",
    ];
    if (viaProxy) args.push("--proxy", viaProxy);
    args.push(url);
    const child = spawn(executable, args, { windowsHide: true });
    let output = "";
    child.stdout.on("data", (b) => (output += b));
    child.on("error", reject);
    child.once("exit", (code) => resolve({ code, output }));
  });
}
const rule = {
  id: probe.toLowerCase(),
  path: probe,
  name: "network-probe",
  download: null,
  upload: null,
  blocked: false,
};
let connection,
  stream,
  cleanScope = false,
  succeeded = false;
const server = http.createServer((req, res) => {
  res.writeHead(200, {
    "Content-Type": "application/octet-stream",
    "Cache-Control": "no-store",
  });
  if (req.url === "/slow") {
    const interval = setInterval(() => res.write(Buffer.alloc(8192)), 50);
    res.once("close", () => clearInterval(interval));
  } else res.end(Buffer.alloc(32768));
});
(async () => {
  try {
    record("run", path.basename(folder));
    record("source", { helperSource: path.resolve(helperSource), sha256: hash(helper) });
    const initial = inspect();
    record("initial", initial);
    assert.equal(
      initial.admin,
      true,
      "Run explicitly with administrator rights",
    );
    assert.equal(
      initial.owned_rules,
      0,
      "Refuse to change an installation with existing firewall rules",
    );
    assert.equal(
      initial.wfp_owned_rules,
      0,
      "Refuse to change an installation with existing WFP rules",
    );
    cleanScope = true;
    if (recoverRun) {
      const previousFolder = path.join(base, recoverRun);
      const previousHelperFolder = path.join(
        previousFolder,
        "helper-install/network-control",
      );
      assert.equal(
        fs.realpathSync(previousHelperFolder).toLowerCase(),
        previousHelperFolder.toLowerCase(),
        "Refuse redirected recovery directory",
      );
      assert(
        fs.statSync(path.join(previousFolder, "network-probe.exe")).isFile(),
      );
      const previousHelper = path.join(
        previousHelperFolder,
        "pinmeter-network-control.exe",
      );
      fs.copyFileSync(helperSource, previousHelper);
      const cleanup = spawnSync(previousHelper, ["--cleanup"], {
        encoding: "utf8",
        windowsHide: true,
        timeout: 15000,
      });
      record("recovery-cleanup", {
        run: recoverRun,
        code: cleanup.status,
        error: cleanup.stderr,
      });
      const previous = inspect(previousHelper);
      record("recovery-final", previous);
      assert.equal(cleanup.status, 0, "Previous test cleanup failed");
      assert.equal(previous.owned_rules, 0);
      assert.equal(previous.wfp_owned_rules, 0);
    }
    await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
    const local = "http://127.0.0.1:" + server.address().port;
    const remote = "https://speed.cloudflare.com/__down?bytes=32768";
    const baseline = await transfer(probe, local, null);
    record("loopback-baseline", baseline);
    assert.equal(baseline.code, 0);
    if (proxy) {
      const r = await transfer(probe, remote, proxy);
      record("proxy-baseline", r);
      assert.equal(r.code, 0);
    }
    let received = 0;
    stream = spawn(
      probe,
      [
        "--max-time",
        "30",
        "-s",
        "--no-buffer",
        "--noproxy",
        "*",
        local + "/slow",
      ],
      { windowsHide: true },
    );
    stream.stdout.on("data", (b) => (received += b.length));
    let streamError;
    stream.on("error", (error) => {
      streamError = error;
    });
    stream.stderr.resume();
    for (let i = 0; i < 50 && received < 32768; i++) await delay(100);
    if (streamError) throw streamError;
    assert(received >= 32768, "Existing stream never started");
    connection = connect();
    record("block", await connection.request([{ ...rule, blocked: true }]));
    await delay(500);
    const settled = received;
    await delay(1500);
    record("existing-loopback-stream", {
      bytesAfterGrace: settled,
      bytesLater: received,
      exitCode: stream.exitCode,
    });
    assert.equal(
      received,
      settled,
      "Existing loopback stream still receiving after block",
    );
    stream.kill();
    const blocked = await transfer(probe, local, null);
    record("loopback-blocked", blocked);
    assert.notEqual(blocked.code, 0);
    assert.match(blocked.output, /^000 0$/);
    if (proxy) {
      const r = await transfer(probe, remote, proxy);
      record("proxy-blocked", r);
      assert.notEqual(r.code, 0);
      assert.match(r.output, /^000 0$/);
    }
    const unaffected = await transfer(curlSource, local, null);
    record("other-executable", unaffected);
    assert.equal(unaffected.code, 0);
    await connection.close();
    connection = null;
    const persistent = await transfer(probe, local, null);
    record("after-helper-exit", persistent);
    assert.notEqual(persistent.code, 0);
    record("persisted-rules", inspect());
    connection = connect();
    record("release-all", await connection.request([rule], "release-all"));
    const released = inspect();
    record("release-all-rules", released);
    assert.equal(released.owned_rules, 0);
    assert.equal(released.wfp_owned_rules, 0);
    const restored = await transfer(probe, local, null);
    record("loopback-restored", restored);
    assert.equal(restored.code, 0);
    if (proxy) {
      const r = await transfer(probe, remote, proxy);
      record("proxy-restored", r);
      assert.equal(r.code, 0);
    }
    succeeded = true;
  } catch (error) {
    record("error", String(error.stack || error));
  } finally {
    if (stream && stream.exitCode === null) stream.kill();
    if (connection) await connection.close();
    server.closeAllConnections();
    server.close();
    if (cleanScope) {
      const cleanup = spawnSync(helper, ["--cleanup"], {
        windowsHide: true,
        encoding: "utf8",
        timeout: 15000,
      });
      record("cleanup", { code: cleanup.status, error: cleanup.stderr });
      if (cleanup.status !== 0) succeeded = false;
      try {
        const final = inspect();
        record("final", final);
        if (final.owned_rules !== 0 || final.wfp_owned_rules !== 0)
          succeeded = false;
      } catch (error) {
        record("cleanup-error", String(error));
        succeeded = false;
      }
    }
    record("passed", succeeded);
    process.exitCode = succeeded ? 0 : 1;
  }
})();
