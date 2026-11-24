#!/usr/bin/env node
'use strict';

// Per-file transform time and throughput for several StyleX compilers.
//
//   node tools/perf/run.cjs [--candidate <cjs>] [--cores 10,11] [--out DIR] [--max-load 30]
//        [--compilers candidate,babel] [--skip-huge]
//
// Inputs: packages/benchmarks fixtures (perf + size) and the StyleX files the
// example apps fed the plugin (tools/apps/results/*/<app>.babel.calls.jsonl).
// Each compiler runs in its own process pinned with taskset. Timings refuse to
// start while the 1-minute load average is over --max-load.

const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const M = path.resolve(__dirname, '../../../../..');
const args = {};
for (let i = 2; i < process.argv.length; i++) {
  const a = process.argv[i];
  if (a.startsWith('--')) args[a.slice(2)] = process.argv[i + 1] && !process.argv[i + 1].startsWith('--') ? process.argv[++i] : 'true';
}

const COMPILERS = {
  candidate: () => path.resolve(args.candidate || path.join(__dirname, '../../js/index.js')),
  babel: () => 'babel',
};

function pluginsFor(file) {
  if (/\.tsx$/.test(file)) return ['typescript', 'jsx'];
  if (/\.(ts|mts|cts)$/.test(file)) return ['typescript'];
  return [['flow', { all: true }], 'jsx'];
}

function collectCases() {
  const cases = [];
  const B = path.join(M, 'packages/benchmarks');
  const haste = { unstable_moduleResolution: { type: 'haste' }, classNamePrefix: 'x' };
  for (const sub of ['perf/fixtures', 'size/fixtures']) {
    for (const f of fs.readdirSync(path.join(B, sub)).sort()) {
      if (!f.endsWith('.js')) continue;
      const file = path.join(B, sub, f);
      const huge = fs.statSync(file).size > 1e6;
      if (huge && args['skip-huge']) continue;
      cases.push({ group: 'benchmarks', name: `${sub.split('/')[0]}/${f}`, filename: file, code: fs.readFileSync(file, 'utf8'), options: haste, parserPlugins: pluginsFor(file), huge });
    }
  }
  // example app sources as the unplugin/Next.js saw them
  const seen = new Set();
  const resDir = args['apps-results'] ? path.dirname(path.resolve(args['apps-results'])) : path.join(__dirname, '../apps/results');
  if (fs.existsSync(resDir)) {
    const dirs = args['apps-results'] ? [path.basename(path.resolve(args['apps-results']))] : fs.readdirSync(resDir);
    for (const cand of dirs) {
      if (!fs.existsSync(path.join(resDir, cand))) continue;
      for (const f of fs.readdirSync(path.join(resDir, cand))) {
        const m = f.match(/^(.*)\.babel\.calls\.jsonl$/);
        if (!m) continue;
        for (const line of fs.readFileSync(path.join(resDir, cand, f), 'utf8').split('\n')) {
          if (!line) continue;
          const r = JSON.parse(line);
          if (!/@stylexjs\/stylex/.test(r.code)) continue;
          const short = r.filename.replace(/^.*\/(example-[^/]+|packages\/[^/]+)\//, '$1/');
          if (seen.has(short)) continue;
          seen.add(short);
          cases.push({ group: `app:${m[1]}`, name: short, filename: r.filename, code: r.code, options: r.options, parserPlugins: r.parserPlugins });
        }
      }
      break; // one results dir is enough for inputs
    }
  }
  return cases;
}

// ---------------------------------------------------------------------------
// worker: time one compiler over all cases

function worker(compiler, casesFile, outFile, only) {
  const all = JSON.parse(fs.readFileSync(casesFile, 'utf8'));
  const cases = all.filter((c) => (only === 'huge' ? c.huge : !c.huge));
  const pluginRoot = path.join(M, 'packages/@stylexjs/babel-plugin');
  let fn;
  if (compiler === 'babel') {
    const req = (m) => require(require.resolve(m, { paths: [pluginRoot] }));
    const babel = req('@babel/core');
    const plugin = require(path.join(pluginRoot, 'lib/index.js'));
    fn = (c) =>
      babel.transformSync(c.code, {
        babelrc: false, configFile: false, filename: c.filename, sourceType: 'module',
        parserOpts: { plugins: c.parserPlugins }, plugins: [[plugin.default ?? plugin, c.options]],
      }).code;
  } else {
    const mod = require(compiler);
    fn = (c) => mod.transformSync(c.code, { ...c.options, filename: c.filename, __parserPlugins: c.parserPlugins, __sourceType: 'module' }).code;
  }
  const res = [];
  for (const c of cases) {
    const r = { name: c.name, group: c.group, bytes: Buffer.byteLength(c.code) };
    try {
      const w0 = Date.now();
      fn(c);
      const slow = c.huge || Date.now() - w0 > 1000; // pathological files: one timed run
      if (!slow) fn(c);
      const times = [];
      const budget = slow ? 0 : 300; // ms
      const minReps = slow ? 1 : 5;
      const t0 = Date.now();
      while (times.length < minReps || (Date.now() - t0 < budget && times.length < 200)) {
        const s = process.hrtime.bigint();
        fn(c);
        times.push(Number(process.hrtime.bigint() - s) / 1e3);
      }
      times.sort((a, b) => a - b);
      r.us = times[Math.floor(times.length / 2)];
      r.reps = times.length;
      r.ok = true;
    } catch (e) {
      r.ok = false;
      r.error = String(e && e.message).split('\n')[0].slice(0, 200);
    }
    res.push(r);
  }
  fs.writeFileSync(outFile, JSON.stringify(res));
}

if (args.worker) {
  worker(args.worker, args.cases, args.result, args.only);
  process.exit(0);
}

// ---------------------------------------------------------------------------

const out = path.resolve(args.out || path.join(__dirname, 'results', new Date().toISOString().replace(/[:.]/g, '-')));
fs.mkdirSync(out, { recursive: true });
const cores = args.cores || '10,11';
const maxLoad = Number(args['max-load'] || 30);
const names = (args.compilers || Object.keys(COMPILERS).join(',')).split(',');
const cases = collectCases();
const casesFile = path.join(out, 'cases.json');
fs.writeFileSync(casesFile, JSON.stringify(cases));

const results = {};
const loads = [];
if (args['summarize-only']) {
  const prev = JSON.parse(fs.readFileSync(path.join(out, 'summary.json'), 'utf8'));
  loads.push(...prev.loads);
  for (const name of names) {
    const merged = [];
    for (const only of ['small', 'huge']) {
      const f = path.join(out, `${name}.${only}.json`);
      if (fs.existsSync(f)) merged.push(...JSON.parse(fs.readFileSync(f, 'utf8')));
      else for (const c of cases.filter((c) => (only === 'huge' ? c.huge : !c.huge))) merged.push({ name: c.name, ok: false, error: prev.rows.find((r) => r.name === name)?.hugeMs == null ? `timeout after ${args['huge-timeout'] || 120} s` : 'missing' });
    }
    const byName = new Map(merged.map((x) => [x.name, x]));
    results[name] = cases.map((c) => byName.get(c.name));
  }
}
for (const name of args['summarize-only'] ? [] : names) {
  for (let wait = 0; os.loadavg()[0] > maxLoad; wait++) {
    if (wait === 0) process.stderr.write(`load ${os.loadavg()[0].toFixed(1)} > ${maxLoad}, waiting\n`);
    if (wait > 60) throw new Error('load stayed high for 10 minutes');
    spawnSync('sleep', ['10']);
  }
  loads.push(os.loadavg()[0]);
  process.stderr.write(`${name}: timing ${cases.length} files on cores ${cores} (load ${os.loadavg()[0].toFixed(1)})\n`);
  const merged = [];
  for (const only of ['small', 'huge']) {
    const subset = cases.filter((c) => (only === 'huge' ? c.huge : !c.huge));
    if (!subset.length) continue;
    const resultFile = path.join(out, `${name}.${only}.json`);
    const limit = only === 'huge' ? Number(args['huge-timeout'] || 120) * 1000 : 1800000;
    const r = spawnSync('taskset', ['-c', cores, process.execPath, __filename, '--worker', COMPILERS[name](), '--cases', casesFile, '--result', resultFile, '--only', only], { stdio: 'inherit', timeout: limit });
    if (r.status !== 0) {
      const why = r.error && r.error.code === 'ETIMEDOUT' ? `timeout after ${limit / 1000} s` : `worker failed (${r.status} ${r.signal})`;
      process.stderr.write(`${name} ${only}: ${why}\n`);
      for (const c of subset) merged.push({ name: c.name, group: c.group, bytes: Buffer.byteLength(c.code), ok: false, error: why });
      continue;
    }
    merged.push(...JSON.parse(fs.readFileSync(resultFile, 'utf8')));
  }
  const byName = new Map(merged.map((x) => [x.name, x]));
  results[name] = cases.map((c) => byName.get(c.name));
}

// common set: files every compiler accepts (huge file reported separately)
const okAll = cases.filter((c, i) => Object.values(results).every((r) => r[i] && r[i].ok));
const idx = new Map(cases.map((c, i) => [c.name, i]));
const rows = [];
for (const [name, r] of Object.entries(results)) {
  const small = okAll.filter((c) => !c.huge);
  const totUs = small.reduce((s, c) => s + r[idx.get(c.name)].us, 0);
  const bytes = small.reduce((s, c) => s + Buffer.byteLength(c.code), 0);
  const perFile = small.map((c) => r[idx.get(c.name)].us).sort((a, b) => a - b);
  const huge = cases.map((c, i) => [c, r[i]]).filter(([c, x]) => c.huge && x.ok);
  rows.push({
    name,
    ok: r.filter((x) => x.ok).length,
    total: r.length,
    medianUs: perFile[Math.floor(perFile.length / 2)],
    totalMs: totUs / 1000,
    mbps: bytes / totUs,
    hugeMs: huge.length ? huge[0][1].us / 1000 : null,
    geoVsBabel: results.babel ? Math.exp(small.reduce((acc, c) => acc + Math.log(results.babel[idx.get(c.name)].us / r[idx.get(c.name)].us), 0) / small.length) : null,
  });
}
const babelRow = rows.find((r) => r.name === 'babel');
const lines = [
  `# perf ${new Date().toISOString()}`,
  '',
  `${cases.length} files (${cases.filter((c) => c.group === 'benchmarks').length} benchmark fixtures, ${cases.filter((c) => c.group.startsWith('app:')).length} example-app files). ` +
    `Pinned to cores ${cores}. Load average at start of each compiler: ${loads.map((l) => l.toFixed(1)).join(', ')}. ` +
    `Common set (accepted by all, huge file excluded): ${okAll.filter((c) => !c.huge).length} files, ${okAll.filter((c) => !c.huge).reduce((s, c) => s + c.code.length, 0)} bytes.`,
  '',
  '| compiler | files ok | median per file (us) | common set total (ms) | throughput (MB/s) | speedup vs Babel (geomean per file) | lotsOfStyles.js 7.3 MB (ms) |',
  '|---|---|---|---|---|---|---|',
  ...rows.map((r) => `| ${r.name} | ${r.ok}/${r.total} | ${r.medianUs?.toFixed(0)} | ${r.totalMs.toFixed(1)} | ${r.mbps.toFixed(2)} | ${r.geoVsBabel != null ? `${r.geoVsBabel.toFixed(1)}x` : '-'} | ${r.hugeMs != null ? r.hugeMs.toFixed(0) : (results[r.name].find((x, i) => cases[i].huge) || {}).error || 'fail'} |`),
  '',
  '## Per file (us, median)',
  '',
  `| file | bytes | ${Object.keys(results).join(' | ')} |`,
  `|---|---|${Object.keys(results).map(() => '---').join('|')}|`,
  ...cases.map((c, i) => {
    return `| ${c.name} | ${Buffer.byteLength(c.code)} | ${Object.values(results).map((r) => (r[i] && r[i].ok ? r[i].us.toFixed(0) : `err`)).join(' | ')} |`;
  }),
  '',
  'Errors:',
  '',
  ...Object.entries(results).flatMap(([n, r]) => r.filter((x) => !x.ok).map((x) => `- ${n} ${x.name}: ${x.error}`)),
];
fs.writeFileSync(path.join(out, 'summary.md'), lines.join('\n') + '\n');
fs.writeFileSync(path.join(out, 'summary.json'), JSON.stringify({ date: new Date().toISOString(), cores, loads, rows }, null, 2));
console.log(lines.slice(0, 12).join('\n'));
console.log(`out: ${out}`);
