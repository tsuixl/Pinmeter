import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { basename, dirname, resolve, delimiter } from 'node:path';
import { homedir } from 'node:os';
import fs from 'node:fs';
import { createHash } from 'node:crypto';
import { packageWindows } from './package-windows.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const cli = resolve(root, 'src/frontend/node_modules/@tauri-apps/cli/tauri.js');
const args = process.argv.slice(2);
if (process.platform === 'win32' && ['dev', 'build', 'prepare', 'check-tools'].includes(args[0]) && !process.env.PINMETER_DESKTOP_WORKER) {
  const coordinator = spawnSync('powershell.exe', ['-NoProfile', '-OutputFormat', 'Text', '-ExecutionPolicy', 'Bypass', '-File', resolve(root, 'tools/desktop-windows.ps1')], {
    stdio: 'inherit', windowsHide: true,
    env: { ...process.env, PINMETER_DESKTOP_ARGS: JSON.stringify(args) },
  });
  if (coordinator.error) console.error(coordinator.error);
  process.exit(coordinator.status ?? 1);
}
const packageOption = args.find(arg => arg.startsWith('--package-dir='));
const packageDirectory = process.env.PINMETER_PACKAGE_DIRECTORY || (packageOption ? resolve(root, packageOption.slice('--package-dir='.length)) : null);
if (packageOption) {
  args.splice(args.indexOf(packageOption), 1);
  if (process.platform !== 'win32' || args[0] !== 'build' ||
      args.some(arg => /^--(debug|target|profile)(=|$)/.test(arg)) ||
      !/^test[1-9][0-9]*$/.test(basename(packageDirectory)) ||
      dirname(packageDirectory) !== resolve(root, 'src/backend/target')) {
    throw new Error('--package-dir requires a native Windows release build and a reserved src/backend/target/testN directory');
  }
}
const logFile = packageDirectory ? resolve(packageDirectory, 'build.log') : null;
const logFd = logFile ? fs.openSync(logFile, 'a') : null;
function run(command, arguments_, options = {}) {
  console.log(`Running: ${command} ${arguments_.join(' ')}`);
  const result = spawnSync(command, arguments_, {
    stdio: logFd === null ? 'inherit' : ['inherit', logFd, logFd], windowsHide: true, ...options,
  });
  if (result.error) console.error(result.error);
  if (result.status !== 0 && logFile) console.error(fs.readFileSync(logFile, 'utf8').slice(-16000));
  return result;
}
if (args[0] === 'check-tools') {
  if (!packageDirectory || !process.env.PINMETER_DESKTOP_WORKER) throw new Error('Run check-tools through the Windows coordinator');
  const checks = [
    [process.execPath, ['--test', resolve(root, 'tools/package-windows.test.mjs')]],
    ['powershell.exe', ['-NoProfile', '-OutputFormat', 'Text', '-ExecutionPolicy', 'Bypass', '-File', resolve(root, 'src/backend/platform/tests/autostart-cleanup.tests.ps1')]],
    ['powershell.exe', ['-NoProfile', '-OutputFormat', 'Text', '-ExecutionPolicy', 'Bypass', '-File', resolve(root, 'tools/check-uninstall.ps1')]],
    ['powershell.exe', ['-NoProfile', '-OutputFormat', 'Text', '-ExecutionPolicy', 'Bypass', '-File', resolve(root, 'tools/build-lock.test.ps1')]],
  ];
  for (const [command, arguments_] of checks) {
    const result = run(command, arguments_);
    if (result.status !== 0) process.exit(result.status ?? 1);
  }
  console.log(`PASS: package and build lock checks. Log: ${logFile}`);
  process.exit(0);
}
function sourceManifest() {
  const list = spawnSync('git', ['ls-files', '-z', '--cached', '--others', '--exclude-standard', 'LICENSE', 'README.md', 'src', 'tools'], { cwd: root, encoding: 'utf8' });
  if (list.status !== 0) throw new Error('Cannot enumerate build sources');
  return [...new Set(list.stdout.split('\0').filter(Boolean))].sort().map(file => ({
    path: file, sha256: createHash('sha256').update(fs.readFileSync(resolve(root, file))).digest('hex'),
  }));
}
const sources = packageDirectory ? sourceManifest() : null;
if (sources) fs.writeFileSync(resolve(packageDirectory, 'source-manifest.json'), JSON.stringify(sources, null, 2));
if (process.platform === 'win32' && ['dev', 'build', 'prepare'].includes(args[0])) {
  const sensors = run('powershell.exe', ['-NoProfile', '-OutputFormat', 'Text', '-ExecutionPolicy', 'Bypass', '-File', resolve(root, 'tools/build-sensors.ps1')]);
  if (sensors.status !== 0) process.exit(sensors.status ?? 1);
  const network = run('powershell.exe', ['-NoProfile', '-OutputFormat', 'Text', '-ExecutionPolicy', 'Bypass', '-File', resolve(root, 'tools/build-network.ps1')]);
  if (network.status !== 0) process.exit(network.status ?? 1);
  const control = run("powershell.exe", ["-NoProfile", "-OutputFormat", "Text", "-ExecutionPolicy", "Bypass", "-File", resolve(root, "tools/build-network-control.ps1")]);
  if (control.status !== 0) process.exit(control.status ?? 1);
}
if (args[0] === 'prepare') process.exit(0);
const result = run(process.execPath, [cli, ...args], {
  cwd: resolve(root, 'src/backend/host'),
  env: { ...process.env, PATH: resolve(homedir(), '.cargo/bin') + delimiter + process.env.PATH },
});
if (result.error) console.error(result.error);
if (result.status === 0 && packageDirectory) {
  if (JSON.stringify(sourceManifest()) !== JSON.stringify(sources)) throw new Error('Sources changed during the build; package refused');
  packageWindows({
    hostDirectory: resolve(root, 'src/backend/host'),
    executable: resolve(root, 'src/backend', process.env.CARGO_TARGET_DIR || 'target', 'release/pinmeter-host.exe'),
    outputDirectory: packageDirectory,
  });
  const commit = spawnSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' });
  fs.writeFileSync(resolve(packageDirectory, 'build-source.json'), JSON.stringify({
    commit: commit.stdout.trim(), sourceManifest: 'source-manifest.json',
    builtAt: new Date().toISOString(), cargoTargetDirectory: process.env.CARGO_TARGET_DIR || 'target',
    command: ['node', 'tools/desktop.mjs', ...args],
  }, null, 2));
}
process.exit(result.status ?? 1);
