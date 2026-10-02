import test from 'node:test';
import assert from 'node:assert/strict';
import { updateManifest, verifyUpdateSignature } from './prepare-update.mjs';
import { generateKeyPairSync, sign, createHash } from 'node:crypto';
const source = { version: '0.1.2', filename: 'Pinmeter setup.exe', signature: 'test-signature',
  releases: [{version:'0.1.2',date:'2026-10-02',summary:'更新',sections:[]}],
  baseUrl: 'https://github.com/tsuixl/Pinmeter/releases/download/v0.1.2' };
test('清单绑定同版本公告、签名和转义后的安装包名称', () => {
  const value = updateManifest(source);
  assert.equal(value.version,'0.1.2'); assert.equal(value.release_notes[0].summary,'更新');
  assert.match(value.platforms['windows-x86_64'].url,/Pinmeter%20setup\.exe$/);
});
test('拒绝缺失公告、签名和非官方地址', () => {
  for (const patch of [{releases:[]},{signature:''},{version:'0.1.2-beta.1'},{filename:'../setup.exe'},
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
  assert.doesNotThrow(()=>verifyUpdateSignature(bytes,publicText,encoded));
  assert.throws(()=>verifyUpdateSignature(Buffer.from('tampered'),publicText,encoded));
  assert.throws(()=>verifyUpdateSignature(bytes,publicText,Buffer.from(text.replace('timestamp:1','timestamp:2')).toString('base64')));
});
