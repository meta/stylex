#!/usr/bin/env node
'use strict';
// Runs the repo's compiler-conformance golden fixtures straight through oxs
// (no Babel in the loop) and prints one line per fixture.
//   node tests/golden.cjs [--json out.json]
// Exit code is 0 even when fixtures fail. scripts/test.cjs reads the JSON.
const fs = require('node:fs');
const path = require('node:path');

const REPO = process.env.STYLEX_REPO || path.resolve(__dirname, '../../../..');
const conf = require(path.join(REPO, 'packages/compiler-conformance/src/index.js'));
const oxs = require(path.resolve(__dirname, '../js/index.js'));

function transform(fixture) {
  const code = fs.readFileSync(fixture.entryPath, 'utf8');
  const options = {
    ...fixture.pluginOptions,
    filename: fixture.entryPath,
    __parserPlugins: fixture.syntax,
  };
  try {
    const r = oxs.transformSync(code, options);
    const metadata = (r.metadata && r.metadata.stylex) || [];
    return {
      status: 'ok',
      js: r.code,
      metadata,
      css: oxs.processStylexRules(metadata, fixture.processOptions),
      warnings: r.warnings || [],
      errors: r.errors || [],
    };
  } catch (e) {
    return { status: 'error', error: { message: String(e && e.message) }, warnings: [], errors: [] };
  }
}

const results = [];
for (const name of conf.getFixtureNames()) {
  const fixture = conf.loadFixture(name);
  let ok = false;
  let why = '';
  try {
    const actual = conf.normalizeResult(fixture, transform(fixture), '@stylexjs/babel-plugin');
    const expected = conf.readExpected(name, fixture);
    const a = JSON.stringify(conf.exactPart(actual));
    const b = JSON.stringify(conf.exactPart(expected));
    if (a !== b) why = 'exact part differs (status, metadata, css or diagnostics)';
    else if (expected.status === 'ok' && !conf.isJsEquivalent(conf.jsPart(expected), conf.jsPart(actual), fixture.syntax))
      why = 'js differs';
    else ok = true;
  } catch (e) {
    why = `threw: ${String(e && e.message).split('\n')[0]}`;
  }
  results.push({ id: `golden > ${name}`, ok, why });
  console.log(`${ok ? 'PASS' : 'FAIL'} ${name}${why ? `  (${why})` : ''}`);
}
const passed = results.filter((r) => r.ok).length;
console.log(`golden ${passed}/${results.length}`);
const i = process.argv.indexOf('--json');
if (i > 0) fs.writeFileSync(process.argv[i + 1], JSON.stringify({ passed, total: results.length, results }, null, 2));
