import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';
import assert from 'node:assert/strict';
import { packageWindows, requiredRuntimeFiles, resourceManifest, verifyPackage } from './package-windows.mjs';

const testRoot = process.env.PINMETER_PACKAGE_TEST_DIR;
assert(testRoot, 'Set PINMETER_PACKAGE_TEST_DIR inside the reserved testN directory');
fs.mkdirSync(testRoot, { recursive: true });

function fixture(trailing = '/') {
  const root = fs.mkdtempSync(path.join(testRoot, 'package-'));
  const hostDirectory = path.join(root, 'host');
  const source = path.join(root, 'source');
  const outputDirectory = path.join(root, 'output');
  fs.mkdirSync(hostDirectory); fs.mkdirSync(outputDirectory);
  const names = [...requiredRuntimeFiles];
  for (const name of names) {
    const file = path.join(source, name);
    fs.mkdirSync(path.dirname(file), { recursive: true });
    fs.writeFileSync(file, `fixture: ${name}`);
  }
  const commonResources = { [`../source/licenses${trailing}`]: `licenses${trailing}` };
  const windowsResources = {
    [`../source/sensors${trailing}`]: `sensors${trailing}`,
    '../source/network/pinmeter-network.exe': 'network/pinmeter-network.exe',
    [`../source/network-control${trailing}`]: `network-control${trailing}`,
  };
  const resources = { ...commonResources, ...windowsResources };
  fs.writeFileSync(path.join(hostDirectory, 'tauri.conf.json'), JSON.stringify({bundle:{resources:commonResources}}));
  fs.writeFileSync(path.join(hostDirectory, 'tauri.windows.conf.json'), JSON.stringify({bundle:{resources:windowsResources}}));
  return { hostDirectory, outputDirectory, executable: path.join(source, 'Pinmeter.exe'), resources, names };
}

test('preserves full file names, nested licenses and single-file mapping with either trailing slash form', () => {
  for (const slash of ['', '/', path.sep]) {
    const f = fixture(slash);
    const entries = resourceManifest(f.hostDirectory, f.resources, f.executable);
    assert.deepEqual(entries.map(entry => entry.relative).sort(), f.names.sort());
    packageWindows(f);
    assert.equal(verifyPackage(entries, f.outputDirectory).length, f.names.length);
  }
});

test('rejects a package with the test8 missing-first-character regression', () => {
  const f = fixture(); packageWindows(f);
  const entries = resourceManifest(f.hostDirectory, f.resources, f.executable);
  fs.renameSync(path.join(f.outputDirectory, 'sensors/pinmeter-gpu.exe'), path.join(f.outputDirectory, 'sensors/inmeter-gpu.exe'));
  assert.throws(() => verifyPackage(entries, f.outputDirectory), /Missing runtime file: sensors\/pinmeter-gpu.exe/);
});

test('detects corrupted dependency content and unexpected resource names', () => {
  const f = fixture(); packageWindows(f);
  const entries = resourceManifest(f.hostDirectory, f.resources, f.executable);
  fs.writeFileSync(path.join(f.outputDirectory, 'sensors/HidSharp.dll'), 'corrupted');
  assert.throws(() => verifyPackage(entries, f.outputDirectory), /Content differs: sensors\/HidSharp.dll/);
  fs.writeFileSync(path.join(f.outputDirectory, 'sensors/extra.dll'), 'extra');
  assert.throws(() => verifyPackage(entries, f.outputDirectory), /Runtime file names differ/);
});

test('does not overwrite an existing runtime', () => {
  const f = fixture(); packageWindows(f);
  assert.throws(() => packageWindows(f), /Output already contains runtime files/);
});

test('rejects a retired driver installer in cached resources and delivered files', () => {
  const f = fixture();
  const stale = path.join(path.dirname(f.executable), 'sensors/PawnIO_setup.exe');
  fs.writeFileSync(stale, 'old cached installer');
  assert.throws(() => packageWindows(f), /Retired PawnIO installer/);
  assert(!fs.existsSync(path.join(f.outputDirectory, 'Pinmeter.exe')));
  fs.unlinkSync(stale);
  packageWindows(f);
  const entries = resourceManifest(f.hostDirectory, f.resources, f.executable);
  fs.writeFileSync(path.join(f.outputDirectory, 'sensors/PawnIO_setup.exe'), 'unexpected installer');
  assert.throws(() => verifyPackage(entries, f.outputDirectory), /Retired PawnIO installer/);
});

test('requires common license resources and detects a damaged license in the delivered package', () => {
  const f = fixture();
  const baseConfig = path.join(f.hostDirectory, 'tauri.conf.json');
  const original = fs.readFileSync(baseConfig);
  fs.writeFileSync(baseConfig, JSON.stringify({bundle:{resources:{}}}));
  assert.throws(() => packageWindows(f), /Missing runtime source: licenses\/Pinmeter-AGPL-3.0.txt/);
  assert(!fs.existsSync(path.join(f.outputDirectory, 'Pinmeter.exe')));
  fs.writeFileSync(baseConfig, original);
  packageWindows(f);
  const entries = resourceManifest(f.hostDirectory, f.resources, f.executable);
  fs.writeFileSync(path.join(f.outputDirectory, 'licenses/Pinmeter-AGPL-3.0.txt'), 'truncated license');
  assert.throws(() => verifyPackage(entries, f.outputDirectory), /Content differs: licenses\/Pinmeter-AGPL-3.0.txt/);
});

test('rejects missing source dependencies and escaped destination paths', () => {
  const f = fixture();
  const dependency = path.join(path.dirname(f.executable), 'sensors/LibreHardwareMonitorLib.dll');
  fs.renameSync(dependency, `${dependency}.missing`);
  assert.throws(() => packageWindows(f), /Missing runtime source: sensors\/LibreHardwareMonitorLib.dll/);
  assert.throws(() => resourceManifest(f.hostDirectory, {'../source/sensors/': '../escape/'}, f.executable), /escapes package/);
  assert(!fs.existsSync(path.join(f.outputDirectory, 'Pinmeter.exe')));
});
