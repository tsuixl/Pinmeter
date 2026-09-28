// Run in an elevated Windows terminal. Uses real ETW; changes no network configuration.
import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { createInterface } from 'node:readline';
import { resolve } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';

const executable = resolve(process.argv[2] ?? 'src/backend/target/network/pinmeter-network.exe');
function sessions() {
  const result = spawnSync('logman.exe', ['query', '-ets'], { encoding: 'utf8', windowsHide: true });
  assert.equal(result.status, 0, result.stderr);
  return new Set(result.stdout.match(/Pinmeter-network-[a-f0-9]{32}/gi) ?? []);
}

const before = sessions();
const results = [];
for (const eof of [false, true]) {
  const id = randomUUID().replaceAll('-', '');
  const name = `Pinmeter-network-${id}`;
  const child = spawn(executable, [String(process.pid), '--session', id], { windowsHide: true });
  let exit;
  child.on('exit', code => { exit = code; });
  const packets = [];
  let stderr = '';
  child.stderr.on('data', data => { stderr += data; });
  createInterface({ input: child.stdout }).on('line', line => packets.push(JSON.parse(line)));
  const request = async command => {
    child.stdin.write(`${command}\n`);
    const deadline = Date.now() + 8000;
    while (!packets.length) {
      assert.equal(exit, undefined, `Helper exited: ${exit}; ${stderr}`);
      assert(Date.now() < deadline, `Timed out: ${command}; ${stderr}`);
      await delay(20);
    }
    return packets.shift();
  };
  try {
    for (let cycle = 0; cycle < 3; cycle++) {
      const sample = await request('sample');
      assert.equal(sample.status, 'normal', JSON.stringify(sample));
      assert.equal(sample.sequence, '1');
      assert(sessions().has(name), 'ETW session was not created');
      assert.equal((await request('stop')).status, 'stopped');
      assert(!sessions().has(name), 'Paused session remained running');
    }
    assert.equal((await request('sample')).status, 'normal');
    const start = Date.now();
    if (!eof) assert.equal((await request('shutdown')).status, 'shutdown_complete');
    child.stdin.end();
    while (exit === undefined && Date.now() - start < 8000) await delay(20);
    assert.equal(exit, 0, stderr || 'Helper did not exit cleanly');
    assert(!sessions().has(name), 'ETW session survived helper exit');
    results.push({ mode: eof ? 'EOF' : 'shutdown', cycles: 3, exit, cleanupMs: Date.now() - start });
  } finally {
    if (exit === undefined) {
      child.stdin.end();
      for (let i = 0; exit === undefined && i < 80; i++) await delay(100);
      if (exit === undefined) child.kill();
    }
    // Test failure recovery is limited to this freshly generated session name.
    if (sessions().has(name)) spawnSync('logman.exe', ['stop', name, '-ets'], { windowsHide: true });
  }
}
assert.deepEqual(sessions(), before, 'ETW session set changed after lifecycle checks');
console.log(JSON.stringify({ passed: true, results, preservedExistingSessions: before.size }, null, 2));
