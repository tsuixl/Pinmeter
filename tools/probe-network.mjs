import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { writeFileSync } from 'node:fs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const helper = resolve(root, 'src/backend/target/network/pinmeter-network.exe');
const child = spawn(helper, [String(process.pid), ...(process.argv.includes('--authorize') ? ['--elevate'] : [])], { windowsHide: true });
const records = [];
const reuse = process.argv.includes('--reuse');
const timeout = setTimeout(() => { child.kill(); process.exitCode = 1; }, 130_000);
let samples = 0, stopped = false;
const lines = createInterface({ input: child.stdout });
lines.on('line', line => {
  const packet = JSON.parse(line);
  records.push(packet);
  console.log(JSON.stringify({ status: packet.status, sequence: packet.sequence, rows: packet.rows?.length, lost: packet.lost, detail: packet.detail }));
  if (reuse && packet.status === 'stopped') {
    stopped = true;
    setTimeout(() => child.stdin.write('sample\n'), 2000);
    return;
  }
  if (reuse && stopped && samples === 3 && packet.sequence !== '1') {
    process.exitCode = 1; child.stdin.end(); return;
  }
  if (packet.status !== 'normal' || ++samples >= 12) { child.stdin.end(); return; }
  if (reuse && samples === 3) { child.stdin.write('stop\n'); return; }
  // Known initiating PID and HTTPS traffic; raw records are kept locally for comparison.
  void fetch('https://api.nuget.org/v3-flatcontainer/microsoft.diagnostics.tracing.traceevent/index.json').then(r => r.arrayBuffer()).catch(() => {});
  setTimeout(() => { if (!child.stdin.destroyed) child.stdin.write('sample\n'); }, 1000);
});
child.on('exit', code => {
  clearTimeout(timeout);
  writeFileSync(resolve(root, `src/backend/target/network/${reuse ? 'reuse-probe' : 'probe'}.json`), JSON.stringify({ pid: process.pid, broker: child.pid, code, records }, null, 2));
  if (records.length === 0 || code !== 0 || records.some(r => !['normal', 'stopped'].includes(r.status)) || (reuse && !stopped)) process.exitCode = 1;
});
child.stdin.on('error', () => {});
child.stdin.write('sample\n');
