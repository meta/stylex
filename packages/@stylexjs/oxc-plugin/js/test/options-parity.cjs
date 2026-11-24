#!/usr/bin/env node
'use strict';
// Options parity: Babel's StateManager.setOptions against js/options.js normalizeOptions.
// Compares the console.error lines and the normalized values for many option inputs.
//   node js/test/options-parity.cjs
const path = require('node:path');

const pluginRoot = path.resolve(__dirname, '../../../../../packages/@stylexjs/babel-plugin');
require(require.resolve('@babel/register', { paths: [pluginRoot] }))({
  cwd: pluginRoot,
  only: [(f) => f.startsWith(path.join(pluginRoot, 'src'))],
});
const StateManager = require(path.join(pluginRoot, 'src/utils/state-manager.js')).default;
const { normalizeOptions } = require('../options.js');

const f = () => 'x';
const cases = [
  {},
  { dev: true },
  { dev: 'yes' },
  { debug: true },
  { dev: true, debug: false },
  { debug: 1 },
  { enableDebugDataProp: false },
  { enableDebugDataProp: 'no' },
  { enableDevClassNames: 3 },
  { dev: true, enableDevClassNames: 3 },
  { enableFontSizePxToRem: 'true' },
  { enableInlinedConditionalMerge: null },
  { enableMinifiedKeys: 0 },
  { enableMediaQueryOrder: 'x' },
  { enableLegacyValueFlipping: [] },
  { enableLogicalStylesPolyfill: {} },
  { enableLTRRTLComments: 1 },
  { sxPropName: false },
  { sxPropName: 'css' },
  { sxPropName: true },
  { sxPropName: 1 },
  { test: 'x' },
  { runtimeInjection: true },
  { runtimeInjection: 'inject' },
  { runtimeInjection: { from: 'a', as: 'b' } },
  { runtimeInjection: { from: 'a' } },
  { runtimeInjection: 3 },
  { classNamePrefix: '' },
  { classNamePrefix: 4 },
  { importSources: ['a', { from: 'b', as: 'c' }] },
  { importSources: 'a' },
  { importSources: ['a', 3, { from: 'b' }] },
  { styleResolution: 'property-specificity' },
  { styleResolution: 'legacy-expand-shorthands' },
  { styleResolution: 'nope' },
  { propertyValidationMode: 'warn' },
  { propertyValidationMode: 'loud' },
  { unstable_moduleResolution: { type: 'commonJS', rootDir: '/a' } },
  { unstable_moduleResolution: { type: 'commonJS' } },
  { unstable_moduleResolution: { type: 'haste' } },
  { unstable_moduleResolution: { type: 'haste', themeFileExtension: 3 } },
  { unstable_moduleResolution: { type: 'custom', filePathResolver: f, getCanonicalFilePath: f } },
  { unstable_moduleResolution: { type: 'custom', filePathResolver: f } },
  { unstable_moduleResolution: { type: 'experimental_crossFileParsing', rootDir: '/r' } },
  { unstable_moduleResolution: { type: 'experimental_crossFileParsing' } },
  { unstable_moduleResolution: { type: 'nope' } },
  { unstable_moduleResolution: 'commonJS' },
  { treeshakeCompensation: 'yes' },
  { env: { a: 1, b: { c: 'd' } } },
  { env: [] },
  { env: 'x' },
  { env: null },
  { aliases: { a: 'b', c: ['d', 'e'] } },
  { aliases: { a: 1 } },
  { aliases: 'x' },
  { debugFilePath: f },
  { debugFilePath: 'x' },
  { rewriteAliases: true },
  { rewriteAliases: 'yes' },
  { unknownOption: 1 },
];

const KEYS = [
  'aliases',
  'classNamePrefix',
  'debug',
  'dev',
  'enableDebugDataProp',
  'enableDevClassNames',
  'enableFontSizePxToRem',
  'enableInlinedConditionalMerge',
  'enableLegacyValueFlipping',
  'enableLogicalStylesPolyfill',
  'enableLTRRTLComments',
  'enableMediaQueryOrder',
  'enableMinifiedKeys',
  'propertyValidationMode',
  'rewriteAliases',
  'runtimeInjection',
  'styleResolution',
  'sxPropName',
  'test',
  'treeshakeCompensation',
];

let bad = 0;
for (const input of cases) {
  const logged = [];
  const oe = console.error;
  console.error = (...a) => logged.push(a.join(' '));
  let babel;
  try {
    babel = new StateManager({ file: { metadata: {} }, opts: input, filename: undefined }).options;
  } finally {
    console.error = oe;
  }
  const ours = normalizeOptions(input);
  const diffs = [];
  if (JSON.stringify(logged) !== JSON.stringify(ours.errors)) diffs.push({ errors: { babel: logged, oxs: ours.errors } });
  for (const k of KEYS) {
    const a = babel[k] ?? null;
    const b = ours.native[k] ?? null;
    if (JSON.stringify(a) !== JSON.stringify(b)) diffs.push({ [k]: { babel: a, oxs: b } });
  }
  // Babel prepends its two default import sources.
  const sources = JSON.stringify(babel.importSources.slice(2));
  if (sources !== JSON.stringify(ours.native.importSources)) diffs.push({ importSources: { babel: sources, oxs: ours.native.importSources } });
  if (JSON.stringify(babel.env) !== JSON.stringify(ours.native.env)) diffs.push({ env: { babel: babel.env, oxs: ours.native.env } });
  const mrA = babel.unstable_moduleResolution;
  const mrB = ours.native.unstable_moduleResolution;
  const typeA = mrA == null ? null : mrA.type;
  const typeB = mrB == null ? null : mrB.type;
  if (typeA !== typeB) diffs.push({ unstable_moduleResolution: { babel: typeA, oxs: typeB } });
  if ((babel.debugFilePath == null) !== (ours.native.debugFilePath == null)) diffs.push({ debugFilePath: true });
  if (diffs.length) {
    bad++;
    console.log(JSON.stringify(input), JSON.stringify(diffs));
  }
}
console.log(`options parity: ${cases.length - bad}/${cases.length} cases match`);
process.exitCode = bad ? 1 : 0;
