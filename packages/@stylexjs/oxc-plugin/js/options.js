/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
'use strict';

// Plugin options: the Babel plugin's validation, defaults and fall-back-and-warn
// behavior (@stylexjs/babel-plugin src/utils/state-manager.js `setOptions`).
//
// normalizeOptions(options) -> {
//   native,   // plain JSON object for the Rust side, every option validated and defaulted
//   errors,   // Babel's console.error lines for invalid options, in Babel's order
//   fns,      // JS functions Rust may call back, by id: [{ fn, self }]
//   aliases,  // normalized aliases
// }
//
// Unknown options are ignored, as Babel ignores them.

const z = require('./validate.js');

const DEFAULT_INJECT_PATH = '@stylexjs/stylex/lib/stylex-inject';

// The plugin's `defaultOptions` (src/shared/utils/default-options.js) on main.
const DEFAULTS = {
  classNamePrefix: 'x',
  dev: false,
  debug: false,
  propertyValidationMode: 'silent',
  enableDevClassNames: false,
  enableDebugDataProp: true,
  enableFontSizePxToRem: false,
  enableInlinedConditionalMerge: true,
  enableMediaQueryOrder: true,
  enableLegacyValueFlipping: false,
  enableLogicalStylesPolyfill: false,
  enableLTRRTLComments: false,
  enableMinifiedKeys: true,
  styleResolution: 'application-order',
  importSources: [],
  treeshakeCompensation: false,
  test: false,
};

const checkModuleResolution = z.unionOf(
  z.object({
    type: z.literal('commonJS'),
    rootDir: z.unionOf(z.nullish(), z.string()),
    themeFileExtension: z.unionOf(z.nullish(), z.string()),
  }),
  z.object({
    type: z.literal('haste'),
    themeFileExtension: z.unionOf(z.nullish(), z.string()),
  }),
  z.object({
    type: z.literal('custom'),
    themeFileExtension: z.unionOf(z.nullish(), z.string()),
    filePathResolver: z.func(),
    getCanonicalFilePath: z.func(),
  }),
  z.object({
    type: z.literal('experimental_crossFileParsing'),
    rootDir: z.string(),
    themeFileExtension: z.unionOf(z.nullish(), z.string()),
  }),
);

const checkImportSources = z.array(z.unionOf(z.string(), z.object({ from: z.string(), as: z.string() })));

const checkRuntimeInjection = z.unionOf(z.boolean(), z.string(), z.object({ from: z.string(), as: z.string() }));

const checkEnv = (value, name = 'options.env') => {
  if (typeof value !== 'object' || value == null || Array.isArray(value)) {
    return new Error(`Expected (${name}) to be an object, but got \`${String(JSON.stringify(value))}\`.`);
  }
  return value;
};

const checkAliases = z.unionOf(z.nullish(), z.objectOf(z.unionOf(z.string(), z.array(z.string()))));

// Bridge JSON (INTERFACES.md). A function becomes {"__oxsFn": id}, an index into `fns`
// ({fn, self}: `self` is the object holding it, Babel's `this` for the call). Values JSON
// cannot hold become {"__oxs": "undefined" | "NaN" | "Infinity" | "-Infinity" | "-0"}.
function encodeValue(value, fns, self = undefined, seen = new Set()) {
  switch (typeof value) {
    case 'function':
      fns.push({ fn: value, self });
      return { __oxsFn: fns.length - 1 };
    case 'undefined':
      return { __oxs: 'undefined' };
    case 'number':
      if (Number.isNaN(value)) return { __oxs: 'NaN' };
      if (value === Infinity) return { __oxs: 'Infinity' };
      if (value === -Infinity) return { __oxs: '-Infinity' };
      if (value === 0 && 1 / value < 0) return { __oxs: '-0' };
      return value;
    case 'bigint':
      return Number(value);
    case 'symbol':
      return { __oxs: 'undefined' };
    case 'object':
      break;
    default:
      return value;
  }
  if (value === null) return null;
  if (seen.has(value)) return null;
  seen.add(value);
  let out;
  if (Array.isArray(value)) out = value.map((v) => encodeValue(v, fns, value, seen));
  else {
    out = {};
    for (const k of Object.keys(value)) out[k] = encodeValue(value[k], fns, value, seen);
  }
  seen.delete(value);
  return out;
}

function decodeValue(value) {
  if (value == null || typeof value !== 'object') return value;
  if (Array.isArray(value)) return value.map(decodeValue);
  const keys = Object.keys(value);
  if (keys.length === 1 && keys[0] === '__oxs') {
    switch (value.__oxs) {
      case 'undefined':
        return undefined;
      case 'NaN':
        return NaN;
      case 'Infinity':
        return Infinity;
      case '-Infinity':
        return -Infinity;
      case '-0':
        return -0;
    }
  }
  const out = {};
  for (const k of keys) out[k] = decodeValue(value[k]);
  return out;
}

function normalizeOptions(input) {
  const options = input ?? {};
  const errors = [];
  const check = (c, value, def, name) => z.logAndDefault(c, value, def, name, errors);

  const dev = check(z.boolean(), options.dev ?? DEFAULTS.dev, false, 'options.dev');
  const debug = check(z.boolean(), options.debug ?? dev, false, 'options.debug');
  const enableDebugDataProp = check(z.boolean(), options.enableDebugDataProp ?? debug, false, 'options.enableDebugDataProp');
  const enableDevClassNames = check(z.boolean(), options.enableDevClassNames ?? dev, dev, 'options.enableDevClassNames');
  const enableFontSizePxToRem = check(
    z.boolean(),
    options.enableFontSizePxToRem ?? DEFAULTS.enableFontSizePxToRem,
    false,
    'options.enableFontSizePxToRem',
  );
  const enableInlinedConditionalMerge = check(
    z.boolean(),
    options.enableInlinedConditionalMerge ?? DEFAULTS.enableInlinedConditionalMerge,
    true,
    'options.enableInlinedConditionalMerge',
  );
  const enableMinifiedKeys = check(
    z.boolean(),
    options.enableMinifiedKeys ?? DEFAULTS.enableMinifiedKeys,
    true,
    'options.enableMinifiedKeys',
  );
  const enableMediaQueryOrder = check(
    z.boolean(),
    options.enableMediaQueryOrder ?? DEFAULTS.enableMediaQueryOrder,
    true,
    'options.enableMediaQueryOrder',
  );
  const enableLegacyValueFlipping = check(
    z.boolean(),
    options.enableLegacyValueFlipping ?? DEFAULTS.enableLegacyValueFlipping,
    false,
    'options.enableLegacyValueFlipping',
  );
  const enableLogicalStylesPolyfill = check(
    z.boolean(),
    options.enableLogicalStylesPolyfill ?? DEFAULTS.enableLogicalStylesPolyfill,
    false,
    'options.enableLogicalStylesPolyfill',
  );
  const enableLTRRTLComments = check(
    z.boolean(),
    options.enableLTRRTLComments ?? DEFAULTS.enableLTRRTLComments,
    false,
    'options.enableLTRRTLComments',
  );
  const sxPropName = check(z.unionOf(z.string(), z.literal(false)), options.sxPropName ?? 'sx', 'sx', 'options.sxPropName');
  const test = check(z.boolean(), options.test ?? DEFAULTS.test, false, 'options.test');
  const configRuntimeInjection = check(checkRuntimeInjection, options.runtimeInjection ?? false, false, 'options.runtimeInjection');
  const runtimeInjection =
    configRuntimeInjection === true
      ? DEFAULT_INJECT_PATH
      : configRuntimeInjection === false
        ? undefined
        : configRuntimeInjection;
  const classNamePrefix = check(z.string(), options.classNamePrefix ?? DEFAULTS.classNamePrefix, 'x', 'options.classNamePrefix');
  const importSources = check(checkImportSources, options.importSources ?? DEFAULTS.importSources, [], 'options.importSources');
  const styleResolution = check(
    z.unionOf(z.literal('application-order'), z.literal('property-specificity'), z.literal('legacy-expand-shorthands')),
    options.styleResolution ?? DEFAULTS.styleResolution,
    'application-order',
    'options.styleResolution',
  );
  const propertyValidationMode = check(
    z.unionOf(z.literal('throw'), z.literal('warn'), z.literal('silent')),
    options.propertyValidationMode ?? DEFAULTS.propertyValidationMode,
    'silent',
    'options.propertyValidationMode',
  );
  const moduleResolution = check(
    z.unionOf(z.nullish(), checkModuleResolution),
    options.unstable_moduleResolution,
    null,
    'options.unstable_moduleResolution',
  );
  const treeshakeCompensation = check(
    z.boolean(),
    options.treeshakeCompensation ?? DEFAULTS.treeshakeCompensation,
    false,
    'options.treeshakeCompensation',
  );
  const env = check(checkEnv, options.env ?? {}, {}, 'options.env');
  const aliasesOption = check(checkAliases, options.aliases, null, 'options.aliases');
  const aliases =
    aliasesOption == null
      ? aliasesOption
      : Object.fromEntries(Object.entries(aliasesOption).map(([k, v]) => [k, typeof v === 'string' ? [v] : v]));
  const debugFilePath = check(z.optional(z.func()), options.debugFilePath, undefined, 'options.debugFilePath');
  const rewriteAliases = typeof options.rewriteAliases === 'boolean' ? options.rewriteAliases : false;

  const fns = [];
  const nativeEnv = encodeValue(env, fns);
  const fnMarker = (fn) => {
    fns.push({ fn, self: undefined });
    return { __oxsFn: fns.length - 1 };
  };

  let nativeResolution = null;
  if (moduleResolution != null) {
    nativeResolution = { type: moduleResolution.type };
    if (moduleResolution.rootDir != null) nativeResolution.rootDir = moduleResolution.rootDir;
    if (moduleResolution.themeFileExtension != null)
      nativeResolution.themeFileExtension = moduleResolution.themeFileExtension;
    if (moduleResolution.type === 'custom') {
      const { filePathResolver, getCanonicalFilePath } = moduleResolution;
      // Babel calls filePathResolver(importPath, sourceFilePath, aliases).
      nativeResolution.filePathResolver = fnMarker((importPath, sourceFilePath) =>
        filePathResolver(importPath, sourceFilePath, aliases),
      );
      nativeResolution.getCanonicalFilePath = fnMarker((p) => getCanonicalFilePath(p));
    }
  }

  const native = {
    aliases,
    classNamePrefix,
    debug,
    dev,
    enableDebugDataProp,
    enableDevClassNames,
    enableFontSizePxToRem,
    enableInlinedConditionalMerge,
    enableLegacyValueFlipping,
    enableLogicalStylesPolyfill,
    enableLTRRTLComments,
    enableMediaQueryOrder,
    enableMinifiedKeys,
    env: nativeEnv,
    importSources,
    propertyValidationMode,
    rewriteAliases,
    runtimeInjection: runtimeInjection ?? null,
    styleResolution,
    sxPropName,
    test,
    treeshakeCompensation,
    unstable_moduleResolution: nativeResolution,
    debugFilePath: debugFilePath == null ? null : fnMarker(debugFilePath),
    // oxs only: when to parse as Flow ('detect' by default, 'all' or true, false). See parse.rs.
    flow: options.flow ?? null,
    // oxs only: hermes-parser options for Flow lowering (reactRuntimeTarget, transformOptions).
    flowParserOptions: options.flowParserOptions ?? null,
  };
  // oxs only, set by babel.js: mark folded JSX props for the adapter (Opts::jsx_fold_marker).
  if (options.__oxsJsxFoldMarker === true) native.jsxFoldMarker = true;
  // Babel's own `sourceMaps` (true, 'inline' or 'both') asks for a map (emit). Off by default.
  if (options.sourceMaps === true || options.sourceMaps === 'inline' || options.sourceMaps === 'both') native.sourceMaps = true;
  return { native, errors, fns, aliases };
}

module.exports = { normalizeOptions, encodeValue, decodeValue, DEFAULTS };
