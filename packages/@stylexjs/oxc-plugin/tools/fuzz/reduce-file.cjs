#!/usr/bin/env node
'use strict';

// Shrink a real source file while Babel and a candidate still disagree the
// same way. Text-splice reducer over Babel AST nodes (statements, object
// properties, array elements, JSX children and attributes, specifiers).
//
//   node tools/fuzz/reduce-file.cjs --candidate js/index.js --dir <apps module repro dir>
//        [--file input.tsx --options options.json] [--pattern REGEX] [--out DIR] [--budget 3000]
//
// Default predicate: both compile, and the first Babel-only and candidate-only
// output chunks (hashes masked) stay the same. With --pattern, the predicate is
// "some Babel-only output chunk matches REGEX, and both compile".

const fs = require('node:fs');
const path = require('node:path');
const { setup } = require('./compare.cjs');

const args = {};
for (let i = 2; i < process.argv.length; i++) {
  const a = process.argv[i];
  if (a.startsWith('--')) args[a.slice(2)] = process.argv[i + 1] && !process.argv[i + 1].startsWith('--') ? process.argv[++i] : 'true';
}
const dir = args.dir ? path.resolve(args.dir) : null;
const inputFile = args.file ? path.resolve(args.file) : fs.readdirSync(dir).map((f) => path.join(dir, f)).find((f) => /^input\./.test(path.basename(f)));
const call = dir && fs.existsSync(path.join(dir, 'call.json')) ? JSON.parse(fs.readFileSync(path.join(dir, 'call.json'), 'utf8')) : {};
const options = args.options ? JSON.parse(fs.readFileSync(args.options, 'utf8')) : call.options || JSON.parse(fs.readFileSync(path.join(dir, 'options.json'), 'utf8'));
const filename = args.filename || options.__filename || call.filename || inputFile;
delete options.__filename;
const ext = path.extname(inputFile);
const parserPlugins = /\.tsx$/.test(ext) ? ['typescript', 'jsx'] : /\.ts$/.test(ext) ? ['typescript'] : [['flow', { all: true }], 'jsx'];
const repo = path.resolve(__dirname, '../../../../..');
const engine = setup({ repo, candidate: path.resolve(args.candidate), root: path.dirname(filename) });
const req = (m) => require(require.resolve(m, { paths: [path.join(repo, 'packages/@stylexjs/babel-plugin')] }));
const parser = req('@babel/parser');
const budget = Number(args.budget || 3000);
const out = path.resolve(args.out || path.join(dir || path.dirname(inputFile), 'reduced'));

function mask(s) {
  return String(s).replace(/\bx[a-z0-9]{4,9}\b/g, 'H').replace(/\bk[A-Za-z0-9]{4,6}\b/g, 'K');
}

function diffChunks(code, input) {
  const a = engine.run(engine.realPlugin, input);
  const b = engine.run(engine.adapter, input);
  if (!a.ok || !b.ok) return null;
  const ca = String(engine.canon(a.code, input) ?? a.code).split(/(?<=[;{},])/);
  const cb = String(engine.canon(b.code, input) ?? b.code).split(/(?<=[;{},])/);
  const sa = new Set(ca);
  const sb = new Set(cb);
  return { onlyBabel: ca.filter((l) => !sb.has(l)), onlyCand: cb.filter((l) => !sa.has(l)), a, b, metaSame: JSON.stringify(a.metadata) === JSON.stringify(b.metadata), cssSame: a.css === b.css };
}

const mkInput = (code) => ({ code, filename, options, parserPlugins, sourceType: 'module', processConfig: { useLayers: false } });
const orig = fs.readFileSync(inputFile, 'utf8');
const d0 = diffChunks(orig, mkInput(orig));
if (!d0) throw new Error('original does not compile on both sides');
const pattern = args.pattern ? new RegExp(args.pattern) : null;
const key0 = `${mask(d0.onlyBabel[0] ?? '')}|${mask(d0.onlyCand[0] ?? '')}`;
if (!pattern && !d0.onlyBabel.length && !d0.onlyCand.length) throw new Error('no code difference to keep');
console.error(`original: ${orig.length} bytes. keep: ${pattern ? pattern : key0.slice(0, 200)}`);

function holds(code) {
  const d = diffChunks(code, mkInput(code));
  if (!d) return false;
  if (pattern) return d.onlyBabel.some((l) => pattern.test(l));
  return `${mask(d.onlyBabel[0] ?? '')}|${mask(d.onlyCand[0] ?? '')}` === key0;
}

function spans(code) {
  let ast;
  try {
    ast = parser.parse(code, { sourceType: 'module', plugins: parserPlugins });
  } catch {
    return [];
  }
  const list = [];
  const visit = (node, parentKey) => {
    if (!node || typeof node !== 'object') return;
    if (Array.isArray(node)) {
      for (const n of node) {
        if (n && typeof n.start === 'number' && ['body', 'properties', 'elements', 'children', 'attributes', 'specifiers', 'arguments', 'declarations'].includes(parentKey)) {
          list.push([n.start, n.end, parentKey]);
        }
        visit(n, parentKey);
      }
      return;
    }
    for (const [k, v] of Object.entries(node)) {
      if (k === 'loc' || k === 'leadingComments' || k === 'trailingComments' || k === 'innerComments' || k === 'extra') continue;
      if (v && typeof v === 'object') visit(v, k);
    }
  };
  visit(ast.program, 'program');
  // comments go too
  for (const c of ast.comments || []) list.push([c.start, c.end, 'comment']);
  return list.sort((a, b) => b[1] - b[0] - (a[1] - a[0]));
}

function cut(code, [s, e, kind]) {
  let end = e;
  if (['properties', 'elements', 'specifiers', 'arguments', 'declarations'].includes(kind)) {
    const m = code.slice(end).match(/^\s*,/);
    if (m) end += m[0].length;
  }
  return code.slice(0, s) + code.slice(end);
}

let cur = orig;
let evals = 0;
let progress = true;
while (progress && evals < budget) {
  progress = false;
  for (const sp of spans(cur)) {
    if (evals >= budget) break;
    const next = cut(cur, sp);
    if (next === cur) continue;
    evals++;
    if (holds(next)) {
      cur = next;
      progress = true;
      break;
    }
  }
}
cur = cur.replace(/\n\s*\n+/g, '\n');
const d = diffChunks(cur, mkInput(cur));
fs.mkdirSync(out, { recursive: true });
fs.writeFileSync(path.join(out, `input${ext}`), cur);
fs.writeFileSync(path.join(out, 'call.json'), JSON.stringify({ filename, options, parserPlugins }, null, 2));
fs.writeFileSync(path.join(out, 'babel.out.js'), d.a.code);
fs.writeFileSync(path.join(out, 'candidate.out.js'), d.b.code);
console.log(`reduced ${orig.length} -> ${cur.length} bytes in ${evals} evals. out: ${out}`);
console.log(cur);
console.log('--- babel only:', d.onlyBabel.slice(0, 4).join(' '));
console.log('--- candidate only:', d.onlyCand.slice(0, 4).join(' '));
