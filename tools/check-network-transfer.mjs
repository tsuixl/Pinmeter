import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { writeFileSync } from 'node:fs';
import net from 'node:net';
import dgram from 'node:dgram';
import { setTimeout as delay } from 'node:timers/promises';

const file = fileURLToPath(import.meta.url);
const mode = process.argv[2];
if (mode?.startsWith('traffic-')) {
  const udp = mode.includes('udp');
  const ipv6 = mode.endsWith('6');
  const host = ipv6 ? '::1' : '127.0.0.1';
  let received = 0, sent = 0, packets = 0;
  const payload = Buffer.alloc(udp ? 1200 : 65536, 1);
  const server = udp ? dgram.createSocket(ipv6 ? 'udp6' : 'udp4') : net.createServer(socket => socket.on('data', data => { received += data.length; }));
  if (udp) server.on('message', data => { received += data.length; });
  server.on('error', error => { console.log(JSON.stringify({ error: String(error), pid: process.pid, mode })); process.exit(1); });
  await new Promise(r => udp ? server.bind(0, host, r) : server.listen(0, host, r));
  const port = server.address().port;
  const socket = udp ? dgram.createSocket(ipv6 ? 'udp6' : 'udp4') : net.connect(port, host);
  if (!udp) await new Promise(r => {
    const timeout = setTimeout(() => {
      console.log(JSON.stringify({ mode, pid: process.pid, error: 'Local TCP connection timed out; transport not verified' }));
      process.exit(1);
    }, 5000);
    socket.once('error', error => { console.log(JSON.stringify({ mode, pid: process.pid, error: String(error) })); process.exit(1); });
    socket.once('connect', () => { clearTimeout(timeout); r(); });
  });
  console.log(JSON.stringify({ ready: true, mode, pid: process.pid }));
  const start = Date.now();
  while (Date.now() - start < 60_000) {
    await new Promise((r, reject) => udp ? socket.send(payload, port, host, e => e ? reject(e) : r()) : socket.write(payload, e => e ? reject(e) : r()));
    sent += payload.length; packets++;
    await delay(udp ? 20 : 200);
  }
  await delay(500);
  console.log(JSON.stringify({ mode, pid: process.pid, received, sent, packets }));
  // Keep identity alive until the delayed ETW buffers have been consumed.
  process.stdin.resume();
  process.stdin.on('end', () => process.exit(0));
} else {
  const root = resolve(dirname(file), '..');
  const helper = spawn(resolve(root, 'src/backend/target/network/pinmeter-network.exe'), [String(process.pid), '--elevate'], { windowsHide: true });
  const packets = [], transfers = [], children = [];
  let started = false, ending = false, timer;
  const deadline = setTimeout(() => { helper.kill(); children.forEach(c => c.kill()); process.exitCode = 1; }, 200_000);
  createInterface({ input: helper.stdout }).on('line', line => {
    const packet = JSON.parse(line); packets.push(packet);
    if (packet.status !== 'normal') { console.log(packet); helper.stdin.end(); return; }
    if (!started) {
      started = true;
      for (const transport of ['tcp4', 'tcp6', 'udp4', 'udp6']) {
        const child = spawn(process.execPath, [file, `traffic-${transport}`], { windowsHide: true }); children.push(child);
        createInterface({ input: child.stdout }).on('line', line => {
          const result = JSON.parse(line);
          if (!result.ready) transfers.push(result);
          console.log(JSON.stringify(result));
          if (transfers.length === 4) setTimeout(() => { ending = true; clearTimeout(timer); helper.stdin.end(); }, 6000);
        });
      }
    }
    if (!ending) timer = setTimeout(() => helper.stdin.write('sample\n'), 1000);
  });
  helper.stdin.on('error', () => {});
  helper.on('exit', code => {
    clearTimeout(timer); clearTimeout(deadline); children.forEach(c => c.stdin.end());
    const comparison = transfers.map(t => {
      const rows = packets.flatMap(p => p.rows ?? []).filter(r => r.pid === t.pid);
      const observed_download = rows.reduce((s,r) => s + Number(r.download), 0);
      const observed_upload = rows.reduce((s,r) => s + Number(r.upload), 0);
      return { ...t, observed_download, observed_upload, verified: !t.error && t.received > 0 && observed_download === t.received && observed_upload === t.sent };
    });
    const report = { code, windows: packets.length, lost: packets.at(-1)?.lost, comparison, packets };
    writeFileSync(resolve(root, 'src/backend/target/network/transfer-check.json'), JSON.stringify(report, null, 2));
    console.log(JSON.stringify({ code, windows: packets.length, lost: report.lost, comparison }));
    if (code !== 0 || comparison.length !== 4 || comparison.some(c => !c.verified)) process.exitCode = 1;
  });
  helper.stdin.write('sample\n');
}
