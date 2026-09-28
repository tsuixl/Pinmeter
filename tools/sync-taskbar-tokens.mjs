import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const css = fs.readFileSync(path.join(root, 'src/frontend/src/shared/ui/sakani/tokens.css'), 'utf8');
const declarations = text => Object.fromEntries([...text.matchAll(/(--[\w-]+):\s*([^;]+);/g)].map(m => [m[1], m[2].trim()]));
const light = declarations(css.split('.dark {')[0]);
const dark = { ...light, ...declarations(css.split('.dark {')[1].split('}')[0]) };
const resolve = (vars, key) => vars[key].startsWith('var(') ? resolve(vars, vars[key].slice(4,-1)) : vars[key];
const palette = vars => Object.fromEntries(['bg-surface','bg-subtle','fg-default','fg-muted','border-focus','bg-inverse','fg-on-inverse'].map(key => [key, resolve(vars, '--color-'+key)]));
const result = JSON.stringify({ source: 'Sakani 0c9a97359001299a1e917ef67fdd0d5af99023a7', light: palette(light), dark: palette(dark), fontSize: 14, lineHeight: 20, outerPadding: Number.parseFloat(resolve(light,'--space-2')), groupGap: Number.parseFloat(resolve(light,'--space-12')), radius: Number.parseFloat(resolve(light,'--radius-md')) }, null, 2)+'\n';
const target = path.join(root, 'src/shared/design-tokens/taskbar.json');
if (process.argv.includes('--check')) {
  if (fs.readFileSync(target,'utf8') !== result) throw new Error('Taskbar tokens drifted from Sakani');
} else { fs.mkdirSync(path.dirname(target),{recursive:true}); fs.writeFileSync(target,result); }
