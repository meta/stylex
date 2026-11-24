// Expected ESM resolutions for real package layouts, from Babel's own resolver.
import fs from 'fs'; import path from 'path'; import url from 'url'; import { createRequire } from 'module';
const repo = process.argv[2];
const req = createRequire(path.join(repo, 'packages/@stylexjs/babel-plugin/package.json'));
const { moduleResolve } = req('@dual-bundle/import-meta-resolve');
const nm = path.join(repo, 'node_modules');
const base = path.join(repo, 'packages/@stylexjs/babel-plugin/src/index.js');
const specs = new Set();
for (const e of fs.readdirSync(nm)) {
  const dirs = e.startsWith('@') ? fs.readdirSync(path.join(nm, e)).map((x) => `${e}/${x}`) : [e];
  for (const name of dirs) {
    if (name.startsWith('.')) continue;
    specs.add(name);
    let pj; try { pj = JSON.parse(fs.readFileSync(path.join(nm, name, 'package.json'), 'utf8')); } catch { continue; }
    if (pj.exports && typeof pj.exports === 'object' && !Array.isArray(pj.exports)) {
      for (const k of Object.keys(pj.exports)) if (k.startsWith('./') ) specs.add(name + '/' + k.slice(2).replace('*', 'index'));
    }
    if (pj.main) specs.add(name + '/' + pj.main.replace(/^\.\//, ''));
    specs.add(name + '/package.json');
  }
}
const out = [];
for (const s of specs) {
  let r = null;
  try { r = url.fileURLToPath(moduleResolve(s, url.pathToFileURL(base))); } catch {}
  out.push([s, r]);
}
fs.writeFileSync(process.argv[3], JSON.stringify({ base, cases: out }));
console.log(out.length, 'cases', out.filter((x) => x[1]).length, 'resolved');
