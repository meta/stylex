#!/usr/bin/env node
'use strict';
// Training run for a profile-guided build (tools/perf/pgo.sh). Compiles every input it is given
// a few times through js/index.js, with and without source maps, and runs processStylexRules on
// the results. Errors are expected (the suite has error cases) and ignored.
//
//   OXS_ADDON=<instrumented addon> node tools/perf/pgo-train.cjs <inputs.jsonl | dir>...
//
// A .jsonl holds records like the harness writes ({code, filename, options, parserPlugins}).
// A directory contributes every .js, .jsx, .ts and .tsx file under it (default options).
const fs = require('node:fs');
const path = require('node:path');
const oxs = require(path.join(__dirname, '../../js/index.js'));

const inputs = [];
function addDir(dir) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    if (e.name === 'node_modules' || e.name.startsWith('.')) continue;
    const p = path.join(dir, e.name);
    if (e.isDirectory()) addDir(p);
    else if (/\.(js|jsx|ts|tsx)$/.test(e.name) && fs.statSync(p).size < 1e6) {
      const plugins = /\.tsx?$/.test(p) ? ['typescript', 'jsx'] : ['jsx'];
      inputs.push({ code: fs.readFileSync(p, 'utf8'), filename: p, options: { unstable_moduleResolution: { type: 'haste' } }, parserPlugins: plugins });
    }
  }
}
for (const a of process.argv.slice(2)) {
  if (!fs.existsSync(a)) continue;
  if (fs.statSync(a).isDirectory()) addDir(a);
  else for (const line of fs.readFileSync(a, 'utf8').split('\n')) if (line) inputs.push(JSON.parse(line));
}
let ok = 0;
for (let round = 0; round < 3; round++) {
  for (const r of inputs) {
    for (const maps of [false, true]) {
      if (maps && round > 0) continue;
      try {
        const opts = { ...r.options, ...(r.filename ? { filename: r.filename } : {}), __parserPlugins: r.parserPlugins, ...(maps ? { sourceMaps: true } : {}) };
        if (round === 2) opts.classNamePrefix = 'p';
        const out = oxs.transformSync(r.code, opts);
        if (round === 0) oxs.processStylexRules(out.metadata.stylex, { useLayers: maps });
        ok++;
      } catch {}
    }
  }
}
console.log(`pgo-train: ${inputs.length} inputs, ${ok} compiles`);
