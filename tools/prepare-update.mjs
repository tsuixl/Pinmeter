import fs from 'node:fs';
import path from 'node:path';
import { createHash, createPublicKey, verify } from 'node:crypto';
import { fileURLToPath } from 'node:url';

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
}
export function updateManifest({ version, filename, signature, releases, baseUrl }) {
  if (!/^\d+\.\d+\.\d+$/.test(version)) throw new Error('更新版本必须为正式三段版本号');
  if (!filename.endsWith('.exe') || filename !== path.basename(filename)) throw new Error('更新文件必须为 NSIS EXE');
  if (!signature.trim()) throw new Error('缺少更新签名');
  const current = releases.find(release => release.version === version);
  if (!current || !Array.isArray(current.sections)) throw new Error('缺少同版本更新公告');
  const url = new URL(baseUrl);
  if (url.protocol !== 'https:' || url.hostname !== 'github.com' || url.username || url.password || url.port || url.search || url.hash ||
      !url.pathname.startsWith('/tsuixl/Pinmeter/releases/download/')) throw new Error('发行地址必须属于 Pinmeter 官方 GitHub Release');
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
  verifyUpdateSignature(bytes, config.plugins.updater.pubkey, manifest.platforms['windows-x86_64'].signature);
  if (bytes.length < 1024 || bytes[0] !== 0x4d || bytes[1] !== 0x5a) throw new Error('安装包不是有效的 Windows 可执行文件');
  fs.mkdirSync(outputDirectory, { recursive: true });
  fs.writeFileSync(path.join(outputDirectory, 'latest.json'), JSON.stringify(manifest, null, 2) + '\n');
  fs.writeFileSync(path.join(outputDirectory, 'release-notes.json'), JSON.stringify(releases, null, 2) + '\n');
  fs.writeFileSync(path.join(outputDirectory, 'update-artifacts.json'), JSON.stringify({ installer,
    sha256: createHash('sha256').update(bytes).digest('hex'), signature: installer + '.sig', version: config.version,
    published: false }, null, 2) + '\n');
  console.log('已生成待发布更新清单：' + path.join(outputDirectory, 'latest.json'));
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  if (process.argv.length !== 4) throw new Error('用法：node tools/prepare-update.mjs <安装包> <输出目录>');
  prepareUpdate({ installer: path.resolve(process.argv[2]), outputDirectory: path.resolve(process.argv[3]) });
}
