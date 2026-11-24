#!/usr/bin/env node
'use strict';

// Differential fuzzer: random StyleX programs through Babel and a candidate.
//
//   node tools/fuzz/run.cjs --candidate <cjs> --count N --seed S --out DIR
//        [--jobs 8] [--repo DIR] [--minimize 150] [--per-sig 2] [--timeout 20000]
//        [--name label] [--case i]   (--case reruns one case and prints it)
//
// Buckets (one primary per program): identical, both-throw-same,
// both-throw-diff, candidate-only-error, babel-only-error, metadata-diff,
// css-diff, code-diff, babel-crash (does not count against the candidate),
// candidate-crash (process died or timed out), invalid-input (generator bug).
// Secondary kinds: format-only (quoted keys, comments), metadata-order,
// cssproc-diff (candidate processStylexRules on Babel's own metadata).

const fs = require('node:fs');
const path = require('node:path');
const { fork } = require('node:child_process');
const { gen, render, features } = require('./gen.cjs');
const { minimize } = require('./minimize.cjs');

const args = {};
for (let i = 2; i < process.argv.length; i++) {
  const a = process.argv[i];
  if (!a.startsWith('--')) continue;
  const [k, v] = a.includes('=') ? a.slice(2).split(/=(.*)/s) : [a.slice(2), process.argv[i + 1] && !process.argv[i + 1].startsWith('--') ? process.argv[++i] : 'true'];
  args[k] = v;
}
const repo = path.resolve(args.repo || path.resolve(__dirname, '../../../../..'));
const candidate = args.candidate && args.candidate !== 'none' ? path.resolve(args.candidate) : null;
const count = Number(args.count || 1000);
const seed = args.seed ?? String(Date.now());
const jobs = Number(args.jobs || 8);
const minimizeMax = Number(args.minimize ?? 800);
const perSig = Number(args['per-sig'] || 1);
const timeout = Number(args.timeout || 20000);
const root = path.resolve(args.root || path.join(__dirname, 'root'));
const name = args.name || (candidate ? path.basename(candidate).replace(/\.c?js$/, '') : 'babel');
// --corpus <jsonl>: mutate real inputs (records like run-suite.cjs --record writes)
const corpus = args.corpus
  ? fs.readFileSync(path.resolve(args.corpus), 'utf8').split('\n').filter(Boolean).map((l) => JSON.parse(l))
      .filter((r) => !/"__(function|regexp|unserializable)"/.test(JSON.stringify(r.options || {})))
  : null;
const genOpts = corpus ? { corpus } : {};
const out = path.resolve(args.out || path.join(process.cwd(), `fuzz-${name}-${seed}`));

const CRASHY = ['candidate-crash'];
const AGAINST = ['undefined-ref', 'error-loc-diff', 'warning-diff', 'candidate-only-error', 'babel-only-error', 'both-throw-diff', 'metadata-diff', 'css-diff', 'code-diff', 'candidate-crash'];

// ---------------------------------------------------------------------------
// worker pool

class Pool {
  constructor(n) {
    this.workers = [];
    this.queue = [];
    this.nextId = 1;
    for (let i = 0; i < n; i++) this.workers.push(this.spawn());
  }
  spawn() {
    const w = { proc: null, busy: null, ready: false };
    w.proc = fork(path.join(__dirname, 'worker.cjs'), [], {
      env: { ...process.env, STYLEX_FUZZ_CFG: JSON.stringify({ repo, candidate, root }) },
      stdio: ['ignore', 'ignore', 'pipe', 'ipc'],
      serialization: 'advanced',
    });
    w.stderr = '';
    w.proc.stderr.on('data', (d) => {
      w.stderr = (w.stderr + d).slice(-4000);
    });
    w.proc.on('message', (m) => {
      if (m.ready) {
        w.ready = true;
        this.pump();
        return;
      }
      const job = w.busy;
      w.busy = null;
      clearTimeout(job.timer);
      job.resolve(m.result);
      this.pump();
    });
    w.proc.on('exit', (code, signal) => {
      const job = w.busy;
      const idx = this.workers.indexOf(w);
      if (idx >= 0 && !this.closing) this.workers[idx] = this.spawn();
      if (job) {
        clearTimeout(job.timer);
        job.resolve({
          primary: 'candidate-crash',
          kinds: ['candidate-crash'],
          crash: job.timedOut ? `timeout after ${timeout} ms` : `exit code ${code} signal ${signal}`,
          stderr: w.stderr.split('\n').slice(-8).join('\n'),
          tags: features(job.spec),
        });
      }
      this.pump();
    });
    return w;
  }
  eval(spec, full = false) {
    return new Promise((resolve) => {
      this.queue.push({ id: this.nextId++, spec, full, resolve });
      this.pump();
    });
  }
  pump() {
    for (const w of this.workers) {
      if (!w.ready || w.busy || !this.queue.length) continue;
      const job = this.queue.shift();
      w.busy = job;
      job.timer = setTimeout(() => {
        job.timedOut = true;
        w.proc.kill('SIGKILL');
      }, timeout);
      w.proc.send({ type: 'eval', id: job.id, spec: job.spec, full: job.full });
    }
  }
  close() {
    this.closing = true;
    for (const w of this.workers) w.proc.kill();
  }
}

// ---------------------------------------------------------------------------
// signatures

function maskMsg(s) {
  return String(s ?? '')
    .replace(/"[^"]*"|'[^']*'|`[^`]*`/g, 'Q')
    .replace(/\b\d+(\.\d+)?\b/g, 'N')
    .replace(/\(N:N\)/g, '')
    .slice(0, 200);
}

const NOISE_TAG = /^(prop:|syntax:plain|import:ns|file:component|value:number|consts$|opt:moduleResolution:commonJS|flow:retAnn|ts:retAnn)/;

function coreTags(tags) {
  return (tags || []).filter((t) => !NOISE_TAG.test(t));
}

function preSig(r) {
  if (r.primary === 'identical' || r.primary === 'both-throw-same' || r.primary === 'invalid-input') return null;
  const parts = [r.primary];
  if (r.babelError) parts.push(`B:${maskMsg(r.babelError)}`);
  if (r.candError) parts.push(`C:${maskMsg(r.candError)}`);
  if (r.crash) parts.push(r.crash.startsWith('timeout') ? 'timeout' : 'died');
  if (!r.babelError && !r.candError && !r.crash) {
    // coarse on purpose: the minimizer separates causes, this only picks examples
    const keep = /^(opt:(dev|test|debug)=true|file:unnamed|file:theme|file:stylex|api:(?!create$)|create:dynamic|stylex\.types|when)/;
    const uses = [...new Set((r.tags || []).filter((t) => t.startsWith('use:')).map((t) => t.replace(/:jsx$/, '')))];
    parts.push(...(r.tags || []).filter((t) => keep.test(t)), ...uses);
  }
  return parts.join(' | ');
}

function maskCode(s) {
  return String(s)
    .replace(/\b(?:x|s|my-|css)(?=[a-z0-9]*\d)[a-z0-9]{5,9}\b/g, 'H')
    .replace(/\bx[a-z0-9]{5,9}\b/g, 'H')
    .replace(/\bk[A-Za-z0-9]{4,5}\b/g, 'K')
    .replace(/:\d+"/g, ':N"')
    .replace(/@keyframes H-B\{.*?\}\}/g, '@keyframes H-B{BODY}')
    .replace(/\{(-?[a-z][a-z-]*):/g, '{P:')
    .replace(/\b\d+(\.\d+)?(e[+-]?\d+)?(?=px|em|rem|%|\b)/g, 'N')
    .replace(/"constVal":("[^"]*"|\{[^}]*\}|\[[^\]]*\]|[\w.-]+)/g, '"constVal":V')
    .replace(/props\.disabled|size==="lg"|x>1|!hidden|\bisDark\b|\bactive\b(?=[?&])/g, 'COND')
    .replace(/\(\[?[^()\]]*COND[^()\]]*\]?\)/g, '(CONDS)')
    .replace(/\b[A-Z][A-Za-z]*__[\w.]+/g, 'DEVCLASS')
    .replace(/\b(root|base|active|container|title|button|icon|row|col|hidden|wrapper|label)\d*\b/g, 'NS')
    .replace(/\b(styles|vars|theme|tokens)\d*\b/g, 'ID')
    .replace(/\b_[A-Za-z]\w*\b/g, '_UID')
    .replace(/(K:null,)+/g, 'K:null*,')
    .replace(/("P-K":null,)+/g, '"P-K":null*,')
    .replace(/"[a-zA-Z-]+-K"/g, '"P-K"')
    .replace(/\b[a-zA-Z]+-K\b/g, 'P-K')
    .slice(0, 160);
}

function finalSig(r) {
  const parts = [r.primary];
  if (r.babelError) parts.push(`B:${maskMsg(r.babelError)}`);
  if (r.candError) parts.push(`C:${maskMsg(r.candError)}`);
  if (r.crash) parts.push(r.crash.startsWith('timeout') ? 'timeout' : 'died');
  if (r.candError && /parse error/.test(r.candError)) {
    // a candidate parse error hides whatever Babel did
    return ['candidate-parse-error', `C:${maskMsg(r.candError)}`, ...(r.tags || []).filter((t) => /^(syntax:|flow:|ts:)/.test(t))].join(' | ');
  }
  if (r.primary === 'undefined-ref') {
    const refs = [...new Set((r.undefinedRefs || []).map((n) => n.replace(/\d+$/, '')))].sort();
    return ['undefined-ref', `refs:${refs.join(',')}`, ...(r.tags || []).filter((t) => /^(create:(paren|dynamic|inFunction|exportDefault)|arg:(spread|dynMember|nsComputed|nsOptional)|use:)/.test(t))].join(' | ');
  }
  if (r.primary === 'error-loc-diff') return ['error-loc-diff', `B:${maskMsg(r.babelError)}`, ...(r.tags || []).filter((t) => /^(api:|syntax:|invalid:)/.test(t))].join(' | ');
  if (r.primary === 'warning-diff') return ['warning-diff', ...(r.warnDiff || []).map(maskMsg)].join(' | ');
  if (r.diffHead) {
    const uniq = (xs) => [...new Set(xs.map(maskCode))].sort().join(' ');
    parts.push(`b:${uniq(r.diffHead.babel)}`);
    parts.push(`c:${uniq(r.diffHead.candidate)}`);
  }
  if (!r.babelError && !r.candError && !r.crash && !r.diffHead) parts.push(coreTags(r.tags).join(','));
  return parts.join(' | ');
}

function describe(f) {
  if (f.candError || f.babelError || f.crash) {
    return [f.babelError && `babel: ${f.babelError}`, f.candError && `candidate: ${f.candError}`, f.crash].filter(Boolean).join(' / ');
  }
  const d = f.diffHead;
  const tags = coreTags(f.tags).filter((t) => !/^(file:|syntax:|import:)/.test(t)).join(', ');
  if (!d) return tags;
  return `${tags}. babel \`${(d.babel[0] ?? '').slice(0, 70)}\` vs candidate \`${(d.candidate[0] ?? '').slice(0, 70)}\``;
}

function sigId(sig) {
  let h = 2166136261;
  for (const c of sig) h = Math.imul(h ^ c.charCodeAt(0), 16777619);
  return (h >>> 0).toString(36);
}

// ---------------------------------------------------------------------------

async function runOne(pool, i) {
  const spec = gen(seed, i, genOpts);
  const r = await pool.eval(spec, true);
  console.log(JSON.stringify({ case: i, primary: r.primary, kinds: r.kinds, babelError: r.babelError, candError: r.candError, crash: r.crash }, null, 1));
  if (r.input) {
    console.log('--- input', JSON.stringify(r.input.options), JSON.stringify(r.input.parserPlugins), r.input.filename);
    console.log(r.input.code);
  }
  if (r.babel?.ok) console.log('--- babel\n' + r.babel.code + '\n--- babel css\n' + r.babel.css);
  if (r.candidate?.ok) console.log('--- candidate\n' + r.candidate.code + '\n--- candidate css\n' + r.candidate.css);
}

function writeRepro(dir, spec, r) {
  fs.mkdirSync(dir, { recursive: true });
  const input = r.input || {};
  const ext = input.parserPlugins?.some((p) => (Array.isArray(p) ? p[0] : p) === 'typescript') ? (input.parserPlugins.includes('jsx') ? '.tsx' : '.ts') : '.js';
  fs.writeFileSync(path.join(dir, `input${ext}`), input.code ?? '');
  fs.writeFileSync(path.join(dir, 'call.json'), JSON.stringify({ filename: input.filename, options: input.options, parserPlugins: input.parserPlugins, processConfig: input.processConfig }, null, 2));
  fs.writeFileSync(path.join(dir, 'spec.json'), JSON.stringify(spec));
  const side = (s, label) => {
    if (!s) return;
    if (s.ok) {
      fs.writeFileSync(path.join(dir, `${label}.out.js`), s.code ?? '');
      fs.writeFileSync(path.join(dir, `${label}.css`), s.css ?? '');
      fs.writeFileSync(path.join(dir, `${label}.metadata.json`), JSON.stringify(s.metadata, null, 1));
      if (s.cssOnBabelMetadata != null) fs.writeFileSync(path.join(dir, `${label}.css-on-babel-metadata.css`), s.cssOnBabelMetadata);
    } else {
      fs.writeFileSync(path.join(dir, `${label}.error.txt`), `${s.error}\n\n${s.stack ?? ''}`);
    }
  };
  side(r.babel, 'babel');
  side(r.candidate, 'candidate');
  if (r.crash) fs.writeFileSync(path.join(dir, 'candidate.crash.txt'), `${r.crash}\n${r.stderr ?? ''}`);
  if (r.babelStack) fs.writeFileSync(path.join(dir, 'babel.crash.txt'), `${r.babelError}\n${r.babelStack}`);
}

async function main() {
  const pool = new Pool(args.case != null ? 1 : jobs);
  if (args.case != null) {
    await runOne(pool, Number(args.case));
    pool.close();
    return;
  }
  fs.mkdirSync(out, { recursive: true });
  const t0 = Date.now();
  const results = new Array(count);
  let done = 0;
  await Promise.all(
    Array.from({ length: count }, (_, i) =>
      pool.eval(gen(seed, i, genOpts)).then((r) => {
        results[i] = r;
        if (++done % 1000 === 0) process.stderr.write(`  ${done}/${count}\n`);
      }),
    ),
  );
  const fuzzSecs = (Date.now() - t0) / 1000;

  const counts = {};
  const kindCounts = {};
  const groups = new Map();
  const cases = fs.createWriteStream(path.join(out, 'cases.jsonl'));
  results.forEach((r, i) => {
    counts[r.primary] = (counts[r.primary] || 0) + 1;
    for (const k of r.kinds || []) kindCounts[k] = (kindCounts[k] || 0) + 1;
    const ps = preSig(r);
    cases.write(JSON.stringify({ case: i, primary: r.primary, kinds: r.kinds, babelError: r.babelError, candError: r.candError, crash: r.crash, sig: ps, tags: r.tags }) + '\n');
    if (ps) {
      if (!groups.has(ps)) groups.set(ps, []);
      groups.get(ps).push(i);
    }
  });
  cases.end();

  // minimize up to perSig examples per pre-signature, biggest groups first
  const t1 = Date.now();
  const failures = new Map();
  const creditedGroups = new Set();
  const jobsList = [];
  for (const [ps, idxs] of [...groups].sort((a, b) => b[1].length - a[1].length)) {
    for (const i of idxs.slice(0, perSig)) jobsList.push([ps, i, idxs.length]);
  }
  const selected = jobsList.slice(0, minimizeMax);
  await Promise.all(
    selected.map(async ([ps, i, groupSize]) => {
      const orig = results[i];
      const spec0 = gen(seed, i, genOpts);
      const parseErr = /parse error/.test(orig.candError || '');
      const same = (r) =>
        (parseErr && /parse error/.test(r.candError || '')) ||
        r.primary === orig.primary &&
        (orig.babelError == null || maskMsg(r.babelError) === maskMsg(orig.babelError)) &&
        (orig.candError == null || maskMsg(r.candError) === maskMsg(orig.candError)) &&
        (orig.crash == null || (r.crash ?? '').slice(0, 7) === orig.crash.slice(0, 7));
      const { spec, evals } = orig.primary === 'candidate-crash' && orig.crash?.startsWith('timeout')
        ? { spec: spec0, evals: 0 }
        : await minimize(spec0, async (s) => same(await pool.eval(s)), { budget: 250 });
      const full = await pool.eval(spec, true);
      const r = same(full) ? full : await pool.eval(spec0, true);
      const sig = finalSig(r);
      const id = sigId(sig);
      if (!failures.has(id)) {
        failures.set(id, { id, sig, primary: sig.startsWith('candidate-parse-error') ? 'candidate-parse-error' : r.primary, babelError: r.babelError, candError: r.candError, crash: r.crash, tags: r.tags, diffHead: r.diffHead, undefinedRefs: r.undefinedRefs, examples: [], groupCases: 0, preSigs: new Set() });
        writeRepro(path.join(out, 'repros', id), spec, r);
      }
      const f = failures.get(id);
      f.examples.push({ case: i, evals });
      // a group counts once, toward the first failure its examples reach
      if (!creditedGroups.has(ps)) {
        creditedGroups.add(ps);
        f.preSigs.add(ps);
        f.groupCases += groupSize;
      }
    }),
  );
  pool.close();
  const minSecs = (Date.now() - t1) / 1000;

  const total = count;
  const against = AGAINST.reduce((s, k) => s + (counts[k] || 0), 0);
  const valid = total - (counts['invalid-input'] || 0) - (counts['babel-crash'] || 0);
  const agree = (counts.identical || 0) + (counts['both-throw-same'] || 0);
  const summary = {
    name,
    candidate,
    seed,
    count,
    repo,
    date: new Date().toISOString(),
    fuzzSecs,
    minSecs,
    counts,
    kindCounts,
    agreeRate: valid ? agree / valid : 0,
    against,
    distinctFailures: failures.size,
    unminimizedGroups: [...groups].filter(([ps]) => !creditedGroups.has(ps)).map(([ps, idxs]) => ({ sig: ps, cases: idxs.length, example: idxs[0] })),
    failures: [...failures.values()]
      .map((f) => ({ ...f, preSigs: [...f.preSigs] }))
      .sort((a, b) => b.groupCases - a.groupCases),
  };
  fs.writeFileSync(path.join(out, 'summary.json'), JSON.stringify(summary, null, 2));
  const order = ['identical', 'undefined-ref', 'error-loc-diff', 'warning-diff', 'both-throw-same', 'both-throw-diff', 'candidate-only-error', 'babel-only-error', 'code-diff', 'metadata-diff', 'css-diff', 'candidate-crash', 'babel-crash', 'invalid-input', 'harness-error'];
  const md = [
    `# fuzz: ${name} vs Babel`,
    '',
    `seed ${seed}, ${count} programs, ${summary.date}. Fuzz ${fuzzSecs.toFixed(0)} s, minimize ${minSecs.toFixed(0)} s.`,
    '',
    `Agreement (identical or same error) on valid programs: **${(summary.agreeRate * 100).toFixed(1)}%**. ` +
      `Distinct failures after minimizing: ${failures.size}.`,
    '',
    '| bucket | programs |',
    '|---|---|',
    ...order.filter((k) => counts[k]).map((k) => `| ${k} | ${counts[k]} |`),
    '',
    'Secondary kinds (a program can have several):',
    '',
    '| kind | programs |',
    '|---|---|',
    ...Object.entries(kindCounts).sort((a, b) => b[1] - a[1]).map(([k, v]) => `| ${k} | ${v} |`),
    '',
    '## Distinct failures',
    '',
    '| id | cases | bucket | detail |',
    '|---|---|---|---|',
    ...summary.failures.map((f) => `| [${f.id}](repros/${f.id}) | ${f.groupCases} | ${f.primary} | ${describe(f).replace(/\|/g, '/').replace(/\n/g, ' ').slice(0, 200)} |`),
    '',
    summary.unminimizedGroups.length ? `Not minimized (over the --minimize budget): ${summary.unminimizedGroups.length} groups, ${summary.unminimizedGroups.reduce((n, g) => n + g.cases, 0)} programs.` : '',
    '',
  ];
  fs.writeFileSync(path.join(out, 'summary.md'), md.join('\n'));
  console.log(md.slice(0, 22).join('\n'));
  console.log(`out: ${out}`);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
