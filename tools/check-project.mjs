import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';
import assert from 'node:assert/strict';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const read=(p)=>fs.readFileSync(path.join(root,p),'utf8');
const tracked=execFileSync('git',['ls-files'],{cwd:root,encoding:'utf8'}).trim().split(/\r?\n/);
let links=0;
for(const file of tracked.filter(p=>p.endsWith('.md'))){
  for(const match of read(file).matchAll(/\]\(([^)]+)\)/g)){
    let target=match[1].split('#')[0];if(!target || /^[a-z]+:/i.test(target))continue;
    target=decodeURIComponent(target);assert(fs.existsSync(path.resolve(root,path.dirname(file),target)),`Broken link: ${file} -> ${target}`);links++;
  }
}
for(const entry of fs.readdirSync(path.join(root,'docs/development'),{withFileTypes:true}).filter(x=>x.isDirectory())){
  const files=fs.readdirSync(path.join(root,'docs/development',entry.name)).filter(x=>x.endsWith('.md')).sort();
  assert.deepEqual(files,['design.md','execution.md']);
  const execution=read(`docs/development/${entry.name}/execution.md`);
  assert.deepEqual([...execution.matchAll(/^## (.+)$/gm)].map(m=>m[1].trim()),['任务计划','进度']);
}
for(const file of tracked){
  assert(!/(^|\/)(node_modules|target|dist|test-results)\//.test(file),`Tracked build output: ${file}`);
  if(file.startsWith('src/backend/core/') && /\.(rs|toml)$/.test(file))assert(!/(?:tauri|windows|sysinfo)\s*(?:::|=)/.test(read(file)),`Core dependency leak: ${file}`);
  if(file.startsWith('src/backend/platform/') && /\.(rs|toml)$/.test(file))assert(!/(?:tauri|pinmeter_host|pinmeter-host)\s*(?:::|=)/.test(read(file)),`Platform dependency leak: ${file}`);
  if(file.startsWith('src/frontend/src/') && /\.tsx?$/.test(file) && !file.includes('/shared/client/'))assert(!/from\s+["']@tauri-apps/.test(read(file)),`View bypasses client: ${file}`);
}
const ignored=['src/frontend/node_modules/a','src/frontend/dist/index.html','src/backend/target/a','src/frontend/test-results/a.png'];
const kept=['src/backend/Cargo.lock','src/frontend/package-lock.json','src/frontend/src/shared/contracts/monitor.ts','src/backend/host/icons/icon.png','.env.example'];
for(const file of ignored){execFileSync('git',['check-ignore','--no-index',file],{cwd:root});}
for(const file of kept){let found=false;try{execFileSync('git',['check-ignore','--no-index',file],{cwd:root});found=true;}catch(e){if(e.status!==1)throw e;}assert(!found,`Source ignored: ${file}`);}
console.log(`PASS: ${links} local Markdown links, feature documents, tracked outputs, layer boundaries, ignore/retain rules.`);
