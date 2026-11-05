/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

// Compares whole-file transforms (parse, compile, print, CSS metadata) of the
// current Babel plugin and the native compiler on the repository's benchmark
// fixtures. A fixture only reports a ratio when both engines produce the same
// StyleX metadata and the same program (compared after reprinting both outputs
// with Babel), so the timings describe comparable work.
//
// node scripts/benchmark.cjs [--iterations=N] [--json=out.json] [--only=name]

const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const { isDeepStrictEqual } = require('node:util');

const packageRoot = path.resolve(__dirname, '..');
const repository = path.resolve(packageRoot, '../../..');
const fixtures = path.join(repository, 'packages/benchmarks');
const args = Object.fromEntries(
  process.argv.slice(2).map((arg) => {
    const [key, value = 'true'] = arg.replace(/^--/, '').split('=');
    return [key, value];
  }),
);

const baseOptions = {
  classNamePrefix: 'x',
  stylexSheetName: '<>',
  unstable_moduleResolution: { type: 'haste' },
};

function file(...parts) {
  const filename = path.join(fixtures, ...parts);
  return { filename, source: fs.readFileSync(filename, 'utf8') };
}

function cases() {
  const perf = [
    'create-basic',
    'create-complex',
    'createTheme-basic',
    'createTheme-complex',
    'colors.stylex',
    'sizes.stylex',
  ].map((name) => file('perf/fixtures', `${name}.js`));
  const dynamic = file('size/fixtures/lotsOfStylesDynamic.js');
  const large = file('size/fixtures/lotsOfStyles.js');
  const callbacks = {
    filename: path.join(fixtures, 'perf/fixtures/callbacks.js'),
    source: [
      "import * as stylex from '@stylexjs/stylex';",
      ...Array.from(
        { length: 40 },
        (_, index) =>
          `export const s${index} = stylex.create({ root: { color: stylex.env.mix('red', ${index}), padding: ${index} } });`,
      ),
    ].join('\n'),
    options: {
      env: {
        mix: (color, amount) =>
          `color-mix(in srgb, ${color} ${amount}%, white)`,
      },
      unstable_moduleResolution: {
        type: 'custom',
        filePathResolver: () => undefined,
        getCanonicalFilePath: (filename) => path.basename(filename),
      },
    },
  };
  const noStylex = {
    filename: path.join(fixtures, 'perf/fixtures/plain.js'),
    source: fs.readFileSync(
      require.resolve('@babel/core/lib/transformation/file/file.js'),
      'utf8',
    ),
  };
  return [
    { name: 'tiny (create-basic)', files: [perf[0]] },
    { name: 'complex (create-complex)', files: [perf[1]] },
    { name: 'theme (createTheme-complex)', files: [perf[3]] },
    { name: 'dynamic (lotsOfStylesDynamic)', files: [dynamic] },
    { name: 'large (lotsOfStyles 7 MB)', files: [large], iterations: 3 },
    { name: 'multi-file (all perf fixtures)', files: perf },
    { name: 'callbacks (env functions + custom resolver)', files: [callbacks] },
    { name: 'no StyleX (plain module)', files: [noStylex] },
  ].filter((entry) => args.only == null || entry.name.includes(args.only));
}

function loadEngine(engine) {
  if (engine === 'babel') {
    const babel = require('@babel/core');
    const plugin = require('@stylexjs/babel-plugin');
    return ({ filename, source, options }) => {
      const result = babel.transformSync(source, {
        filename,
        babelrc: false,
        configFile: false,
        parserOpts: { plugins: ['jsx', ['flow', { all: true }]] },
        plugins: [[plugin, { ...baseOptions, ...options }]],
      });
      return { code: result.code, metadata: result.metadata.stylex };
    };
  }
  const native = require(packageRoot);
  return ({ filename, source, options }) => {
    const result = native.transformSync(source, {
      ...baseOptions,
      ...options,
      filename,
    });
    return { code: result.code, metadata: result.metadata.stylex };
  };
}

function normalizeCode(code) {
  const babel = require('@babel/core');
  const generate = require('@babel/generator').default;
  const ast = babel.parseSync(code, {
    babelrc: false,
    configFile: false,
    filename: 'output.js',
    parserOpts: { plugins: ['jsx', ['flow', { all: true }]] },
  });
  return generate(ast.program, { compact: true, comments: false }).code;
}

function normalizeMetadata(rules) {
  return (rules || []).map(([key, value, priority]) => [
    key,
    { ...value, rtl: value.rtl ?? null },
    priority,
  ]);
}

function quantile(sorted, q) {
  return sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * q))];
}

function measure(run, files, iterations) {
  const warmup = Math.min(5, iterations);
  for (let i = 0; i < warmup; i += 1) for (const input of files) run(input);
  const samples = [];
  for (let i = 0; i < iterations; i += 1) {
    const start = process.hrtime.bigint();
    for (const input of files) run(input);
    samples.push(Number(process.hrtime.bigint() - start) / 1e6);
  }
  samples.sort((a, b) => a - b);
  return {
    median: quantile(samples, 0.5),
    p90: quantile(samples, 0.9),
    samples: samples.length,
  };
}

// Cold time: a fresh process loads the engine and performs the first transform.
function cold(engine, entry) {
  const script = `
    const start = process.hrtime.bigint();
    const run = require(${JSON.stringify(__filename)}).loadEngine(${JSON.stringify(engine)});
    const entry = require(${JSON.stringify(__filename)}).cases().find((item) => item.name === ${JSON.stringify(entry.name)});
    for (const input of entry.files) run(input);
    process.stdout.write(String(Number(process.hrtime.bigint() - start) / 1e6));
  `;
  const samples = [];
  for (let i = 0; i < 3; i += 1) {
    const result = spawnSync(process.execPath, ['-e', script], {
      cwd: packageRoot,
      encoding: 'utf8',
      env: { ...process.env, STYLEX_BENCHMARK_CHILD: '1' },
    });
    if (result.status !== 0) throw new Error(result.stderr);
    samples.push(Number(result.stdout));
  }
  samples.sort((a, b) => a - b);
  return samples[1];
}

function main() {
  const engines = { babel: loadEngine('babel'), rust: loadEngine('rust') };
  const iterations = Number(args.iterations || 30);
  const report = [];
  for (const entry of cases()) {
    const outputs = {};
    const errors = {};
    for (const [name, run] of Object.entries(engines)) {
      try {
        outputs[name] = entry.files.map((input) => {
          const result = run(input);
          return [
            normalizeMetadata(result.metadata),
            normalizeCode(result.code),
          ];
        });
      } catch (error) {
        errors[name] = String(error.message).split('\n')[0];
      }
    }
    const comparable =
      Object.keys(errors).length === 0 &&
      isDeepStrictEqual(outputs.babel, outputs.rust);
    const row = {
      name: entry.name,
      comparable,
      errors,
      rules: (outputs.babel || []).reduce(
        (sum, [rules]) => sum + rules.length,
        0,
      ),
      bytes: entry.files.reduce(
        (sum, input) => sum + Buffer.byteLength(input.source),
        0,
      ),
    };
    for (const [name, run] of Object.entries(engines)) {
      row[name] =
        errors[name] != null
          ? { warm: null, cold: null }
          : {
              warm: measure(run, entry.files, entry.iterations || iterations),
              cold: args.cold === 'false' ? null : cold(name, entry),
            };
    }
    row.warmSpeedup = comparable
      ? row.babel.warm.median / row.rust.warm.median
      : null;
    row.coldSpeedup =
      comparable && row.rust.cold ? row.babel.cold / row.rust.cold : null;
    report.push(row);
  }
  const format = (value) => (value == null ? 'n/a' : value.toFixed(2));
  console.log(
    `node ${process.version} ${process.platform}-${process.arch}; warm = median of ${iterations} runs after warmup; cold = median of 3 fresh processes (load + first transform)`,
  );
  console.log(
    '| fixture | bytes | rules | code + metadata equal | babel warm ms | rust warm ms | warm speedup | babel cold ms | rust cold ms | cold speedup |',
  );
  console.log('|---|---:|---:|:---:|---:|---:|---:|---:|---:|---:|');
  for (const row of report) {
    console.log(
      `| ${row.name} | ${row.bytes} | ${row.rules} | ${row.comparable ? 'yes' : 'NO'} | ${format(row.babel.warm?.median)} | ${format(row.rust.warm?.median)} | ${row.warmSpeedup ? `${format(row.warmSpeedup)}x` : 'n/a'} | ${format(row.babel.cold)} | ${format(row.rust.cold)} | ${row.coldSpeedup ? `${format(row.coldSpeedup)}x` : 'n/a'} |`,
    );
  }
  for (const row of report) {
    for (const [engine, message] of Object.entries(row.errors))
      console.log(`${row.name}: ${engine} failed: ${message}`);
  }
  if (args.json)
    fs.writeFileSync(
      path.resolve(args.json),
      `${JSON.stringify({ node: process.version, platform: `${process.platform}-${process.arch}`, report }, null, 2)}\n`,
    );
  if (report.some((row) => !row.comparable)) process.exitCode = 1;
}

module.exports = { cases, loadEngine };
if (require.main === module && !process.env.STYLEX_BENCHMARK_CHILD) main();
