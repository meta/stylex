/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 *
 */

import { createUnplugin } from 'unplugin';
import { transformAsync } from '@babel/core';
import stylexBabelPlugin from '@stylexjs/babel-plugin';
import flowSyntaxPlugin from '@babel/plugin-syntax-flow';
import jsxSyntaxPlugin from '@babel/plugin-syntax-jsx';
import typescriptSyntaxPlugin from '@babel/plugin-syntax-typescript';
import path from 'node:path';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import { createRequire } from 'node:module';
import { transform as lightningTransform } from 'lightningcss';
import browserslist from 'browserslist';
import { browserslistToTargets } from 'lightningcss';

// The babel plugin always recognises these on top of any configured source.
const BUILT_IN_IMPORT_SOURCES = ['stylex', '@stylexjs/stylex'];

// Vite and Rollup hash asset file names by default, so `index.css` reaches the
// output directory as e.g. `assets/index-B5Jdbbfd.css`. Both patterns therefore
// allow an optional content hash.
//
// Rollup's default `hashCharacters` is `base64`, which is base64url, so a hash
// may contain `-` and `_` alongside letters and digits (about one in five
// eight character hashes does, e.g. `assets/index-BVm_Qe95.css`). The character
// class has to cover the whole base64url alphabet or the miss this is meant to
// fix simply comes back for those builds.
export const INDEX_CSS_RE = /(^|\/)index(-[\w-]{8,})?\.css$/i;
export const STYLE_CSS_RE = /(^|\/)style(-[\w-]{8,})?\.css$/i;

/**
 * Try to pick a stable CSS asset to inject into.
 * - Prefer files named like `style.css` or `index.css`, hashed or not
 * - Otherwise, first .css asset encountered
 */
export function pickCssAssetFromRollupBundle(bundle, choose) {
  const assets = Object.values(bundle).filter(
    (a) =>
      a &&
      a.type === 'asset' &&
      typeof a.fileName === 'string' &&
      a.fileName.endsWith('.css'),
  );
  if (assets.length === 0) return null;
  if (typeof choose === 'function') {
    const chosen = assets.find((a) => choose(a.fileName));
    if (chosen) return chosen;
  }
  const best =
    assets.find((a) => INDEX_CSS_RE.test(a.fileName)) ||
    assets.find((a) => STYLE_CSS_RE.test(a.fileName));
  return best || assets[0];
}

function processCollectedRulesToCSS(rules, options) {
  if (!rules || rules.length === 0) return '';
  const collectedCSS = stylexBabelPlugin.processStylexRules(rules, {
    useLayers: options.useCSSLayers ?? false,
    enableLTRRTLComments: options?.enableLTRRTLComments,
  });
  const { code } = lightningTransform({
    targets: browserslistToTargets(browserslist()),
    ...options.lightningcssOptions,
    filename: 'stylex.css',
    code: Buffer.from(collectedCSS),
  });
  return code.toString();
}

function getAssetBaseName(asset) {
  if (asset?.name && typeof asset.name === 'string') return asset.name;
  const fallback = asset?.fileName
    ? path.basename(asset.fileName)
    : 'stylex.css';
  const match = /^(.*?)(-[a-z0-9]{8,})?\.css$/i.exec(fallback);
  if (match) return `${match[1]}.css`;
  return fallback || 'stylex.css';
}

function replaceBundleReferences(bundle, oldFileName, newFileName) {
  for (const item of Object.values(bundle)) {
    if (!item) continue;
    if (item.type === 'chunk') {
      if (typeof item.code === 'string' && item.code.includes(oldFileName)) {
        item.code = item.code.split(oldFileName).join(newFileName);
      }
      const importedCss = item.viteMetadata?.importedCss;
      if (importedCss instanceof Set && importedCss.has(oldFileName)) {
        importedCss.delete(oldFileName);
        importedCss.add(newFileName);
      } else if (Array.isArray(importedCss)) {
        const next = importedCss.map((name) =>
          name === oldFileName ? newFileName : name,
        );
        item.viteMetadata.importedCss = next;
      }
    } else if (item.type === 'asset' && typeof item.source === 'string') {
      if (item.source.includes(oldFileName)) {
        item.source = item.source.split(oldFileName).join(newFileName);
      }
    }
  }
}

export function replaceCssAssetWithHashedCopy(ctx, bundle, asset, nextSource) {
  const baseName = getAssetBaseName(asset);
  const referenceId = ctx.emitFile({
    type: 'asset',
    name: baseName,
    source: nextSource,
  });
  const nextFileName = ctx.getFileName(referenceId);
  const oldFileName = asset.fileName;
  if (!nextFileName || !oldFileName || nextFileName === oldFileName) {
    asset.source = nextSource;
    return;
  }
  replaceBundleReferences(bundle, oldFileName, nextFileName);
  delete bundle[oldFileName];
}

function readJSON(file) {
  try {
    const content = fs.readFileSync(file, 'utf8');
    return JSON.parse(content);
  } catch {
    return null;
  }
}

function findNearestPackageJson(startDir) {
  let dir = startDir;
  for (;;) {
    const candidate = path.join(dir, 'package.json');
    if (fs.existsSync(candidate)) return candidate;
    const parent = path.dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  return null;
}

function toPackageName(importSource) {
  const source =
    typeof importSource === 'string' ? importSource : importSource?.from;
  if (!source || source.startsWith('.') || source.startsWith('/')) return null;
  if (source.startsWith('@')) {
    const [scope, name] = source.split('/');
    if (scope && name) return `${scope}/${name}`;
  }
  const [pkg] = source.split('/');
  return pkg || null;
}

function hasStylexDependency(manifest, targetPackages) {
  if (!manifest || typeof manifest !== 'object') return false;
  const depFields = [
    'dependencies',
    'peerDependencies',
    'optionalDependencies',
  ];
  for (const field of depFields) {
    const deps = manifest[field];
    if (!deps || typeof deps !== 'object') continue;
    for (const name of Object.keys(deps)) {
      if (targetPackages.has(name)) return true;
    }
  }
  return false;
}

function discoverStylexPackages({
  importSources,
  explicitPackages,
  rootDir,
  resolver,
}) {
  const targetPackages = new Set(
    importSources
      .map(toPackageName)
      .filter(Boolean)
      .concat(['@stylexjs/stylex']),
  );
  const found = new Set(explicitPackages || []);
  const pkgJsonPath = findNearestPackageJson(rootDir);
  if (!pkgJsonPath) return Array.from(found);
  const pkgDir = path.dirname(pkgJsonPath);
  const pkgJson = readJSON(pkgJsonPath);
  if (!pkgJson) return Array.from(found);
  const depFields = [
    'dependencies',
    'devDependencies',
    'peerDependencies',
    'optionalDependencies',
  ];
  const deps = new Set();
  for (const field of depFields) {
    const entries = pkgJson[field];
    if (!entries || typeof entries !== 'object') continue;
    for (const name of Object.keys(entries)) deps.add(name);
  }
  for (const dep of deps) {
    let manifestPath = null;
    try {
      manifestPath = resolver.resolve(`${dep}/package.json`, {
        paths: [pkgDir],
      });
    } catch {}
    if (!manifestPath) continue;
    const manifest = readJSON(manifestPath);
    if (hasStylexDependency(manifest, targetPackages)) {
      found.add(dep);
    }
  }
  return Array.from(found);
}

const JS_LIKE_RE = /\.[cm]?[jt]sx?(\?|$)/;
const SVELTE_LIKE_RE = /\.svelte(\?|$)/;

export const unpluginFactory = (userOptions = {}, metaOptions) => {
  // framework :: 'rollup' | 'vite' | 'rolldown' | 'farm' | 'unloader'
  const framework = metaOptions?.framework;
  const {
    dev = process.env.NODE_ENV === 'development' ||
      process.env.BABEL_ENV === 'development',
    unstable_moduleResolution = { type: 'commonJS', rootDir: process.cwd() },
    babelConfig: { plugins = [], presets = [] } = {},
    importSources = BUILT_IN_IMPORT_SOURCES,
    useCSSLayers = false,
    lightningcssOptions,
    cssInjectionTarget,
    externalPackages = [],
    // Persist rules to disk in dev to bridge multiple plugin containers/processes.
    // Off by default; enable if your dev setup runs separate Node processes per environment.
    devPersistToDisk = false,
    // Dev integration mode: 'full' (runtime + html), 'css-only' (serve CSS endpoint only), 'off'
    devMode = 'full',
    treeshakeCompensation = ['vite', 'rollup', 'rolldown'].includes(framework),
    ...stylexOptions
  } = userOptions;

  const gateImportSources = [...BUILT_IN_IMPORT_SOURCES, ...importSources];

  // Shared state across a single compilation (used for builds)
  const stylexRulesById = new Map(); // id -> Rule[]
  // Global shared store for Vite dev to aggregate across environments (client/ssr/rsc)
  function getSharedStore() {
    try {
      const g = globalThis;
      if (!g.__stylex_unplugin_store) {
        g.__stylex_unplugin_store = { rulesById: new Map(), version: 0 };
      }
      return g.__stylex_unplugin_store;
    } catch {
      return { rulesById: stylexRulesById, version: 0 };
    }
  }

  const nearestPkgJson = findNearestPackageJson(process.cwd());
  const requireFromCwd = nearestPkgJson
    ? createRequire(nearestPkgJson)
    : createRequire(path.join(process.cwd(), 'package.json'));
  const stylexPackages = discoverStylexPackages({
    importSources,
    explicitPackages: externalPackages,
    rootDir: nearestPkgJson ? path.dirname(nearestPkgJson) : process.cwd(),
    resolver: requireFromCwd,
  });
  // Resolve nearest node_modules and cache under node_modules/.stylex/rules.json
  function findNearestNodeModules(startDir) {
    let dir = startDir;
    // Walk upwards until we find a node_modules directory or hit the FS root
    for (;;) {
      const candidate = path.join(dir, 'node_modules');
      if (fs.existsSync(candidate)) {
        const stat = fs.statSync(candidate);
        if (stat.isDirectory()) return candidate;
      }
      const parent = path.dirname(dir);
      if (parent === dir) break;
      dir = parent;
    }
    return null;
  }
  const NEAREST_NODE_MODULES = findNearestNodeModules(process.cwd());
  const DISK_RULES_DIR = NEAREST_NODE_MODULES
    ? path.join(NEAREST_NODE_MODULES, '.stylex')
    : path.join(process.cwd(), 'node_modules', '.stylex');
  const DISK_RULES_PATH = path.join(DISK_RULES_DIR, 'rules.json');

  async function runBabelTransform(inputCode, filename, callerName) {
    const result = await transformAsync(inputCode, {
      babelrc: false,
      filename,
      presets,
      plugins: [
        ...plugins,
        /\.jsx?/.test(path.extname(filename))
          ? flowSyntaxPlugin
          : [typescriptSyntaxPlugin, { isTSX: true }],
        jsxSyntaxPlugin,
        stylexBabelPlugin.withOptions({
          ...stylexOptions,
          importSources,
          treeshakeCompensation,
          dev,
          unstable_moduleResolution,
        }),
      ],
      caller: {
        name: callerName,
        supportsStaticESM: true,
        supportsDynamicImport: true,
        supportsTopLevelAwait: !inputCode.includes('require('),
        supportsExportNamespaceFrom: true,
      },
    });

    if (!result || result.code == null) {
      return { code: inputCode, map: null, metadata: {} };
    }
    return result;
  }

  function escapeReg(src) {
    return src.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  }

  function containsStylexImport(code, source) {
    const s = escapeReg(typeof source === 'string' ? source : source.from);
    const re = new RegExp(
      // import ... from 'source' | import('source') | require('source') | import 'source'
      `(?:from\\s*['"]${s}['"]|import\\s*\\(\\s*['"]${s}['"]\\s*\\)|require\\s*\\(\\s*['"]${s}['"]\\s*\\)|^\\s*import\\s*['"]${s}['"])`,
      'm',
    );
    return re.test(code);
  }

  function shouldHandle(code) {
    if (!code) return false;
    return gateImportSources.some((src) => containsStylexImport(code, src));
  }

  function resetState() {
    stylexRulesById.clear();
    if (devPersistToDisk) {
      try {
        fs.rmSync(DISK_RULES_PATH, { force: true });
      } catch {}
    }
  }

  function collectCss() {
    const merged = new Map();
    if (devPersistToDisk) {
      try {
        if (fs.existsSync(DISK_RULES_PATH)) {
          const json = JSON.parse(fs.readFileSync(DISK_RULES_PATH, 'utf8'));
          for (const [k, v] of Object.entries(json)) merged.set(k, v);
        }
      } catch {}
    }
    try {
      const shared = getSharedStore().rulesById;
      for (const [k, v] of shared.entries()) merged.set(k, v);
    } catch {}
    for (const [k, v] of stylexRulesById.entries()) merged.set(k, v);
    const allRules = Array.from(merged.values()).flat();
    return processCollectedRulesToCSS(allRules, {
      useCSSLayers,
      lightningcssOptions,
      enableLTRRTLComments: stylexOptions?.enableLTRRTLComments,
    });
  }

  async function persistRulesToDisk(id, rules) {
    if (!devPersistToDisk) return;
    try {
      let current = {};
      try {
        const txt = await fsp.readFile(DISK_RULES_PATH, 'utf8');
        current = JSON.parse(txt);
      } catch {}
      if (rules && Array.isArray(rules) && rules.length > 0) {
        current[id] = rules;
      } else if (current[id]) {
        delete current[id];
      }
      await fsp.writeFile(DISK_RULES_PATH, JSON.stringify(current), 'utf8');
    } catch {}
  }

  // No rollup-style virtual module normalize for webpack/rspack stability

  const plugin = {
    name: '@stylexjs/unplugin',
    apply: (config, env) => {
      try {
        const command =
          env?.command || (typeof config === 'string' ? undefined : undefined);
        if (devMode === 'off' && command === 'serve') return false;
      } catch {}
      return true;
    },
    // Ensure we run before React refresh transforms so HMR stays intact
    enforce: 'pre',

    // Vite/Rollup lifecycle resets
    buildStart() {
      resetState();
    },
    buildEnd() {
      // No-op; bundler-specific hooks handle CSS injection.
    },

    transformInclude(id) {
      return JS_LIKE_RE.test(id) || SVELTE_LIKE_RE.test(id);
    },

    // Core code transform
    async transform(code, id) {
      // Only handle JS-like files; avoid parsing CSS/JSON/etc
      if (!JS_LIKE_RE.test(id) && !SVELTE_LIKE_RE.test(id)) return null;
      if (!shouldHandle(code)) return null;

      // Extract the pure filename by removing everything after '?' (e.g., handling Vite's '?v=' cache busting).
      const dir = path.dirname(id);
      const basename = path.basename(id);
      const file = path.join(dir, basename.split('?')[0] || basename);

      const result = await runBabelTransform(code, file, '@stylexjs/unplugin');
      const { metadata } = result;
      if (!stylexOptions.runtimeInjection) {
        const hasRules =
          metadata &&
          Array.isArray(metadata.stylex) &&
          metadata.stylex.length > 0;
        const shared = getSharedStore();
        if (hasRules) {
          stylexRulesById.set(id, metadata.stylex);
          shared.rulesById.set(id, metadata.stylex);
          shared.version++;
          await persistRulesToDisk(id, metadata.stylex);
        } else {
          stylexRulesById.delete(id);
          if (shared.rulesById.has(id)) {
            shared.rulesById.delete(id);
            shared.version++;
          }
          await persistRulesToDisk(id, []);
        }
      }

      // Rollup/Vite watch-mode support: collect stylex metadata from cached deps
      // Only when running in rollup-like context
      // $FlowExpectedError[incompatible-use]
      const ctx = this;
      if (
        ctx &&
        ctx.meta &&
        ctx.meta.watchMode &&
        typeof ctx.parse === 'function'
      ) {
        try {
          const ast = ctx.parse(result.code);
          for (const stmt of ast.body) {
            if (stmt.type === 'ImportDeclaration') {
              // $FlowExpectedError[incompatible-call]
              const resolved = await ctx.resolve(stmt.source.value, id);
              if (resolved && !resolved.external) {
                // $FlowExpectedError[incompatible-call]
                const loaded = await ctx.load(resolved);
                if (loaded && loaded.meta && 'stylex' in loaded.meta) {
                  stylexRulesById.set(resolved.id, loaded.meta.stylex);
                }
              }
            }
          }
        } catch {}
      }
      return { code: result.code, map: result.map };
    },

    // Rollup: ensure cached modules still provide their metadata
    shouldTransformCachedModule({ id, meta }) {
      if (meta && 'stylex' in meta) {
        stylexRulesById.set(id, meta.stylex);
      }
      return false;
    },
  };

  plugin.__stylexCollectCss = collectCss;
  plugin.__stylexResetState = resetState;
  plugin.__stylexGetSharedStore = getSharedStore;
  plugin.__stylexDevMode = devMode;
  plugin.__stylexCssInjectionTarget = cssInjectionTarget;
  plugin.__stylexPackages = stylexPackages;
  return plugin;
};

const unpluginInstance = createUnplugin(unpluginFactory);

export default unpluginInstance;
export const unplugin = unpluginInstance;
