import test from 'node:test';
import assert from 'node:assert/strict';
import { updateManifest, verifyUpdateSignature, verifySignedVersion, verifyInstallerVersion } from './prepare-update.mjs';
import { generateKeyPairSync, sign, createHash } from 'node:crypto';
const source = { version: '0.1.2', filename: 'Pinmeter_0.1.2_x64-setup.exe', signature: 'test-signature',
  releases: [{version:'0.1.2',date:'2026-10-02',summary:'更新',sections:[]}],
  baseUrl: 'https://github.com/tsuixl/Pinmeter/releases/download/v0.1.2' };
test('清单绑定同版本公告、签名、发行标签和安装器名称', () => {
  const value = updateManifest(source);
  assert.equal(value.version,'0.1.2'); assert.equal(value.release_notes[0].summary,'更新');
  assert.match(value.platforms['windows-x86_64'].url,/v0\.1\.2\/Pinmeter_0\.1\.2_x64-setup\.exe$/);
});
test('拒绝缺失公告、签名和非官方地址', () => {
  for (const patch of [{releases:[]},{signature:''},{version:'0.1.2-beta.1'},{filename:'../setup.exe'},
    {filename:'Pinmeter_0.1.1_x64-setup.exe'}, {filename:'Pinmeter_0.1.2_arm64-setup.exe'},
    {baseUrl:'https://github.com/tsuixl/Pinmeter/releases/download/v0.1.1'},
    {baseUrl:'http://github.com/tsuixl/Pinmeter/releases/download/v1'},{baseUrl:'https://github.com/other/repo/releases/download/v1'}]) {
    assert.throws(() => updateManifest({...source,...patch}));
  }
});
test('发行前核对实际包和签名，篡改内容及公告签名均拒绝', () => {
  const {publicKey,privateKey}=generateKeyPairSync('ed25519');
  const key=Buffer.concat([Buffer.from('Ed'),Buffer.alloc(8,1),publicKey.export({type:'spki',format:'der'}).subarray(-32)]);
  const publicText=Buffer.from('untrusted comment: test\n'+key.toString('base64')+'\n').toString('base64');
  const bytes=Buffer.from('Pinmeter test package');
  const raw=sign(null,createHash('blake2b512').update(bytes).digest(),privateKey);
  const sig=Buffer.concat([Buffer.from('ED'),Buffer.alloc(8,1),raw]);
  const comment='timestamp:1';
  const global=sign(null,Buffer.concat([raw,Buffer.from(comment)]),privateKey);
  const text=['untrusted comment: test',sig.toString('base64'),'trusted comment: '+comment,global.toString('base64')].join('\n');
  const encoded=Buffer.from(text).toString('base64');
  assert.equal(verifyUpdateSignature(bytes,publicText,encoded),comment);
  assert.throws(()=>verifyUpdateSignature(Buffer.from('tampered'),publicText,encoded));
  assert.throws(()=>verifyUpdateSignature(bytes,publicText,Buffer.from(text.replace('timestamp:1','timestamp:2')).toString('base64')));
});

test('拒绝签名旧版本与重复版本字段，兼容无版本旧签名但仍需 PE 核对', () => {
  assert.doesNotThrow(() => verifySignedVersion('timestamp:1\tfile:setup.exe\tversion:0.1.2', '0.1.2'));
  assert.doesNotThrow(() => verifySignedVersion('timestamp:1\tversion:v0.1.2', '0.1.2'));
  assert.doesNotThrow(() => verifySignedVersion('timestamp:1\tfile:setup.exe', '0.1.2'));
  assert.throws(() => verifySignedVersion('timestamp:1\tversion:0.1.1', '0.1.2'));
  assert.throws(() => verifySignedVersion('version:0.1.2\tversion:0.1.1', '0.1.2'));
});

test('读取真实 PE 产品版本，不因文件是 EXE 就接受目标版本', {skip:process.platform !== 'win32'}, () => {
  assert.throws(() => verifyInstallerVersion(process.execPath, '0.1.2'), /实际产品版本.*不一致/);
});
