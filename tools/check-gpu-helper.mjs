import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import { once } from 'node:events';
import { writeFile, mkdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';
const root = new URL('../', import.meta.url);
const executable = process.env.PINMETER_GPU_HELPER ?? fileURLToPath(new URL('src/backend/target/sensors/pinmeter-gpu.exe', root));
const child = spawn(executable, [String(process.pid)], { windowsHide: true, stdio: ['pipe', 'pipe', 'pipe'] });
const exit = once(child, 'exit');
const timeout = setTimeout(() => child.kill(), 110000);
const lines = createInterface({ input: child.stdout });
const iterator = lines[Symbol.asyncIterator]();
const samples = [];
try {
  const continuousSamples = 70;
  for (let i = 0; i < continuousSamples + 2; i++) {
    // A real interruption must still rebuild the load baseline.
    if (i === continuousSamples) await new Promise(resolve => setTimeout(resolve, 4000));
    child.stdin.write('sample\n');
    const line = await iterator.next();
    assert(!line.done, 'helper exited before response');
    assert(line.value.length < 131072);
    const packet = JSON.parse(line.value);
    assert.equal(packet.status, 'normal');
    assert(packet.devices.length > 0 && packet.devices.length <= 16);
    assert.equal(new Set(packet.devices.map(d => d.id)).size, packet.devices.length);
    for (const device of packet.devices) for (const reading of Object.values(device.readings)) {
      assert(['normal', 'warming', 'unsupported', 'failed'].includes(reading.status));
      if (reading.status === 'normal') assert(Number.isFinite(reading.value));
      else assert.equal(reading.value, null);
    }
    samples.push({ at: new Date().toISOString(), ...packet });
    if (i < continuousSamples + 1 && i !== continuousSamples - 1)
      await new Promise(resolve => setTimeout(resolve, 1000));
  }
  assert(samples[0].devices.every(d => d.readings.usage.status === 'warming'));
  assert(samples.slice(1, 20).some(s => s.devices.some(d => d.readings.usage.status === 'normal')));
  const initialIds = samples[0].devices.map(d => d.id).sort();
  for (const sample of samples.slice(1, continuousSamples)) {
    assert.deepEqual(sample.devices.map(d => d.id).sort(), initialIds);
    assert(sample.devices.every(d => d.readings.usage.status === 'normal'), 'stable devices must keep normal usage across periodic inventory checks');
  }
  assert(samples[continuousSamples].devices.every(d => d.readings.usage.status === 'warming'), 'long interruption must rewarm');
  assert(samples[continuousSamples + 1].devices.every(d => d.readings.usage.status === 'normal'), 'usage must recover after rewarm');
  child.stdin.end();
  assert.equal((await exit)[0], 0);
  const output = new URL('src/backend/target/gpu-probe/production-helper.json', root);
  await mkdir(new URL('src/backend/target/gpu-probe/', root), { recursive: true });
  await writeFile(output, JSON.stringify(samples, null, 2) + '\n');
  console.log(JSON.stringify({ result: 'PASS: 70 continuous GPU samples, no periodic rewarm, interrupted baseline recovery, normal EOF exit', samples: samples.length, devices: samples.at(-1).devices }, null, 2));
} finally { clearTimeout(timeout); lines.close(); if (child.exitCode === null) child.kill(); }
