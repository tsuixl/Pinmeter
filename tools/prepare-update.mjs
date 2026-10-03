import fs from 'node:fs';
import path from 'node:path';
import { createHash, createPublicKey, verify } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
// Verify the Tauri/minisign artifact against the public key embedded in the app.
// Runtime verification remains the official updater plugin's responsibility.
export function verifyUpdateSignature(bytes, publicKey, signature) {
  const keyLines = Buffer.from(publicKey.trim(), 'base64').toString('utf8').trim().split(/\r?\n/);
  const lines = Buffer.from(signature.trim(), 'base64').toString('utf8').trim().split(/\r?\n/);
  const key = Buffer.from(keyLines[1] || '', 'base64');
  const sig = Buffer.from(lines[1] || '', 'base64');
  const global = Buffer.from(lines[3] || '', 'base64');
  if (key.length !== 42 || sig.length !== 74 || global.length !== 64 || !lines[2]?.startsWith('trusted comment: ') ||
      !key.subarray(2,10).equals(sig.subarray(2,10))) throw new Error('更新签名格式或公钥标识不匹配');
  const algorithm = sig.subarray(0,2).toString('ascii');
  if (algorithm !== 'ED' && algorithm !== 'Ed') throw new Error('不支持的更新签名算法');
  const publicObject = createPublicKey({key: Buffer.concat([Buffer.from('302a300506032b6570032100','hex'),key.subarray(10)]),type:'spki',format:'der'});
  const message = algorithm === 'ED' ? createHash('blake2b512').update(bytes).digest() : bytes;
  if (!verify(null,message,publicObject,sig.subarray(10)) ||
      !verify(null,Buffer.concat([sig.subarray(10),Buffer.from(lines[2].slice(17))]),publicObject,global)) throw new Error('更新包签名验证失败');
  return lines[2].slice(17);
}
export function verifySignedVersion(comment, version) {
  const versions = comment.split('\t').filter(field => field.startsWith('version:'));
  if (versions.length > 1 || (versions.length === 1 && versions[0].slice(8).replace(/^v/, '') !== version)) {
    throw new Error('更新签名中的版本与目标版本不一致');
  }
}
export function verifyInstallerVersion(installer, version) {
  if (process.platform !== 'win32') throw new Error('Windows 安装器版本核对必须在 Windows 构建环境执行');
  // The path is data, never interpolated into PowerShell source.
  const script = "$ErrorActionPreference='Stop'; $v=[Diagnostics.FileVersionInfo]::GetVersionInfo($env:PINMETER_INSPECT_INSTALLER); [Console]::Write($v.ProductVersion)";
  const actual = execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', script], {
    encoding: 'utf8', windowsHide: true, timeout: 15000,
    env: { ...process.env, PINMETER_INSPECT_INSTALLER: path.resolve(installer) },
  }).trim();
  if (actual !== version && actual !== version + '.0') {
    throw new Error(`安装器实际产品版本 ${actual || '缺失'} 与目标 ${version} 不一致`);
  }
  return actual;
}
export function updateManifest({ version, filename, signature, releases, baseUrl }) {
  if (!/^\d+\.\d+\.\d+$/.test(version)) throw new Error('更新版本必须为正式三段版本号');
  if (filename !== `Pinmeter_${version}_x64-setup.exe`) throw new Error('安装器名称必须与目标版本和 x64 架构一致');
  if (!signature.trim()) throw new Error('缺少更新签名');
  const current = releases.find(release => release.version === version);
  if (!current || !Array.isArray(current.sections)) throw new Error('缺少同版本更新公告');
  const url = new URL(baseUrl);
  if (url.protocol !== 'https:' || url.hostname !== 'github.com' || url.username || url.password || url.port || url.search || url.hash ||
      url.pathname.replace(/\/$/, '') !== '/tsuixl/Pinmeter/releases/download/v' + version) throw new Error('发行地址必须属于同版本 Pinmeter 官方 GitHub Release');
  const selected = releases.filter(release => /^\d+\.\d+\.\d+$/.test(release.version) &&
    release.version.localeCompare(version, undefined, { numeric: true }) <= 0).slice(0, 50);
  return {
    version,
    pub_date: new Date(current.date + 'T00:00:00+08:00').toISOString(),
    notes: current.summary,
    release_notes: selected,
    platforms: { 'windows-x86_64': { url: url.href.replace(/\/$/, '') + '/' + encodeURIComponent(filename), signature: signature.trim() } },
  };
}
export function prepareUpdate({ installer, outputDirectory }) {
  const config = JSON.parse(fs.readFileSync(path.join(root, 'src/backend/host/tauri.conf.json'), 'utf8'));
  const releases = JSON.parse(fs.readFileSync(path.join(root, 'src/shared/updates/releases.json'), 'utf8'));
  const manifest = updateManifest({ version: config.version, filename: path.basename(installer),
    signature: fs.readFileSync(installer + '.sig', 'utf8'), releases,
    baseUrl: 'https://github.com/tsuixl/Pinmeter/releases/download/v' + config.version });
  const bytes = fs.readFileSync(installer);
  const comment = verifyUpdateSignature(bytes, config.plugins.updater.pubkey, manifest.platforms['windows-x86_64'].signature);
  verifySignedVersion(comment, config.version);
  if (bytes.length < 1024 || bytes[0] !== 0x4d || bytes[1] !== 0x5a) throw new Error('安装包不是有效的 Windows 可执行文件');
  const productVersion = verifyInstallerVersion(installer, config.version);
  if (!bytes.equals(fs.readFileSync(installer))) throw new Error('版本核对期间安装器内容发生变化，拒绝生成清单');
  fs.mkdirSync(outputDirectory, { recursive: true });
  fs.writeFileSync(path.join(outputDirectory, 'latest.json'), JSON.stringify(manifest, null, 2) + '\n');
  fs.writeFileSync(path.join(outputDirectory, 'release-notes.json'), JSON.stringify(releases, null, 2) + '\n');
  fs.writeFileSync(path.join(outputDirectory, 'update-artifacts.json'), JSON.stringify({ installer,
    sha256: createHash('sha256').update(bytes).digest('hex'), signature: installer + '.sig', version: config.version,
    productVersion, published: false }, null, 2) + '\n');
  console.log('已生成待发布更新清单：' + path.join(outputDirectory, 'latest.json'));
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  if (process.argv.length !== 4) throw new Error('用法：node tools/prepare-update.mjs <安装包> <输出目录>');
  prepareUpdate({ installer: path.resolve(process.argv[2]), outputDirectory: path.resolve(process.argv[3]) });
}
