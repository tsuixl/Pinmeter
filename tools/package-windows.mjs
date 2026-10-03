import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import assert from 'node:assert/strict';

export const requiredRuntimeFiles = [
  'Pinmeter.exe',
  'sensors/pinmeter-sensors.exe', 'sensors/pinmeter-sensors.exe.config',
  'sensors/licenses/PawnIO-COPYING.txt',
  'sensors/licenses/PawnIO-NOTICE.txt', 'sensors/licenses/SOURCES.txt',
  'sensors/pinmeter-gpu.exe', 'sensors/pinmeter-gpu.exe.config',
  ...['LibreHardwareMonitorLib.dll', 'HidSharp.dll', 'DiskInfoToolkit.dll',
    'BlackSharp.Core.dll', 'RAMSPDToolkit-NDD.dll', 'System.Memory.dll',
    'System.Buffers.dll', 'System.Numerics.Vectors.dll',
    'System.Runtime.CompilerServices.Unsafe.dll', 'System.Threading.AccessControl.dll',
    'System.Security.AccessControl.dll', 'System.Security.Principal.Windows.dll']
    .map(name => `sensors/${name}`),
  'network/pinmeter-network.exe', 'network-control/pinmeter-network-control.exe',
  'network-control/WinDivert.dll', 'network-control/WinDivert64.sys',
  'licenses/Pinmeter-AGPL-3.0.txt', 'licenses/NOTICE.txt', 'licenses/SOURCE_CODE.txt',
  'licenses/THIRD-PARTY-NOTICES.txt',
  'licenses/one-ip/NOTICE', 'licenses/one-ip/one-ip.LICENSE', 'licenses/Sakani-MIT.txt',
  'licenses/Geist-OFL-1.1.txt', 'licenses/Geist-NOTICE.txt', 'licenses/brands-NOTICE.txt',
  'licenses/HarmonyOS-Sans-LICENSE.txt', 'licenses/HarmonyOS-Sans-NOTICE.txt',
];

const portable = value => value.split(path.sep).join('/');
const hash = file => createHash('sha256').update(fs.readFileSync(file)).digest('hex');

function filesUnder(directory) {
  assert(!fs.lstatSync(directory).isSymbolicLink(), `Linked directory: ${directory}`);
  return fs.readdirSync(directory, { withFileTypes: true }).flatMap(entry => {
    const file = path.join(directory, entry.name);
    assert(!entry.isSymbolicLink(), `Linked resource: ${file}`);
    return entry.isDirectory() ? filesUnder(file) : [file];
  });
}

function destination(value) {
  const normalized = portable(path.normalize(value));
  assert(!path.isAbsolute(value) && normalized !== '..' && !normalized.startsWith('../'),
    `Resource destination escapes package: ${value}`);
  return normalized;
}

export function resourceManifest(hostDirectory, resources, executable) {
  const entries = [{ source: path.resolve(executable), relative: 'Pinmeter.exe' }];
  for (const [sourcePath, targetPath] of Object.entries(resources)) {
    // resolve + relative work identically with and without a trailing separator.
    const source = path.resolve(hostDirectory, sourcePath);
    const info = fs.lstatSync(source);
    assert(!info.isSymbolicLink(), `Linked resource source: ${source}`);
    if (info.isDirectory()) {
      for (const file of filesUnder(source)) {
        entries.push({ source: file, relative: destination(path.join(targetPath, path.relative(source, file))) });
      }
    } else {
      entries.push({ source, relative: destination(targetPath) });
    }
  }
  const names = new Set();
  for (const entry of entries) {
    assert(fs.statSync(entry.source).isFile(), `Missing source: ${entry.source}`);
    const key = entry.relative.toLowerCase();
    assert(path.basename(entry.source).toLowerCase() !== 'pawnio_setup.exe',
      'Retired PawnIO installer must not be distributed');
    assert(!names.has(key), `Duplicate package path: ${entry.relative}`);
    names.add(key);
  }
  return entries;
}

export function verifyPackage(entries, outputDirectory) {
  assert(!filesUnder(outputDirectory).some(file => path.basename(file).toLowerCase() === 'pawnio_setup.exe'),
    'Retired PawnIO installer must not be distributed');
  for (const name of requiredRuntimeFiles) {
    assert(fs.existsSync(path.join(outputDirectory, name)), `Missing runtime file: ${name}`);
  }
  // Compare file names independently from the copy loop, including nested licenses.
  for (const directory of ['sensors', 'network', 'network-control', 'licenses']) {
    const expected = entries.filter(entry => entry.relative.startsWith(`${directory}/`))
      .map(entry => entry.relative).sort();
    const actual = filesUnder(path.join(outputDirectory, directory))
      .map(file => portable(path.relative(outputDirectory, file))).sort();
    assert.deepEqual(actual, expected, `Runtime file names differ in ${directory}`);
  }
  return entries.map(entry => {
    const output = path.join(outputDirectory, entry.relative);
    const sha256 = hash(entry.source);
    assert.equal(hash(output), sha256, `Content differs: ${entry.relative}`);
    return { ...entry, sha256 };
  });
}

// The caller must reserve the numbered directory and hold the workspace build lock.
// Refuse overwrites: an existing runtime is never repaired or replaced in place.
export function packageWindows({ hostDirectory, executable, outputDirectory }) {
  const base = JSON.parse(fs.readFileSync(path.join(hostDirectory, 'tauri.conf.json'), 'utf8'));
  const config = JSON.parse(fs.readFileSync(path.join(hostDirectory, 'tauri.windows.conf.json'), 'utf8'));
  const resources = { ...base.bundle.resources, ...config.bundle.resources };
  const entries = resourceManifest(hostDirectory, resources, executable);
  for (const name of requiredRuntimeFiles) {
    assert(entries.some(entry => entry.relative === name), `Missing runtime source: ${name}`);
  }
  assert(fs.statSync(outputDirectory).isDirectory(), 'Reserve the output directory first');
  assert(!fs.lstatSync(outputDirectory).isSymbolicLink(), 'Output directory must not be linked');
  for (const name of ['Pinmeter.exe', 'sensors', 'network', 'network-control', 'licenses']) {
    assert(!fs.existsSync(path.join(outputDirectory, name)), `Output already contains runtime files: ${name}`);
  }
  for (const entry of entries) {
    const output = path.join(outputDirectory, entry.relative);
    fs.mkdirSync(path.dirname(output), { recursive: true });
    fs.copyFileSync(entry.source, output, fs.constants.COPYFILE_EXCL);
  }
  const verified = verifyPackage(entries, outputDirectory);
  fs.writeFileSync(path.join(outputDirectory, 'package-hashes.json'), JSON.stringify(verified, null, 2) + '\n');
  console.log(`Verified runtime paths and SHA-256 for ${verified.length} files: ${path.join(outputDirectory, 'Pinmeter.exe')}`);
}
