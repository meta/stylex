/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

const path = require('node:path');
const fs = require('node:fs');
const { normalize, resolve } = require('path');
const { globSync } = require('fast-glob');
const isGlob = require('is-glob');
const globParent = require('glob-parent');
const createBundler = require('./bundler');

const NODE_MODULES_CATCH_ALL_EXCLUDE_PATTERNS = new Set([
  'node_modules/**',
  '**/node_modules/**',
]);

// Parses a glob pattern and extracts its base directory and pattern.
// Returns an object with `base` and `glob` properties.
function parseGlob(pattern) {
  // License: MIT
  // Based on:
  // https://github.com/chakra-ui/panda/blob/6ab003795c0b076efe6879a2e6a2a548cb96580e/packages/node/src/parse-glob.ts
  let glob = pattern;
  const base = globParent(pattern);

  if (base !== '.') {
    glob = pattern.substring(base.length);
    if (glob.charAt(0) === '/') {
      glob = glob.substring(1);
    }
  }

  if (glob.substring(0, 2) === './') {
    glob = glob.substring(2);
  }
  if (glob.charAt(0) === '/') {
    glob = glob.substring(1);
  }

  return { base, glob };
}

// Parses a file path or glob pattern into a PostCSS dependency message.
function parseDependency(fileOrGlob, cwd) {
  // License: MIT
  // Based on:
  // https://github.com/chakra-ui/panda/blob/6ab003795c0b076efe6879a2e6a2a548cb96580e/packages/node/src/parse-dependency.ts
  if (fileOrGlob.startsWith('!')) {
    return null;
  }

  let message = null;

  if (isGlob(fileOrGlob)) {
    const { base, glob } = parseGlob(fileOrGlob);
    message = {
      type: 'dir-dependency',
      dir: normalize(resolve(cwd, base)),
      glob,
    };
  } else {
    message = { type: 'dependency', file: normalize(resolve(cwd, fileOrGlob)) };
  }

  return message;
}

function normalizeGlobPattern(pattern) {
  return String(pattern).replace(/\\/g, '/').replace(/^\.\//, '');
}

function isNodeModulesCatchAllExcludePattern(pattern) {
  return NODE_MODULES_CATCH_ALL_EXCLUDE_PATTERNS.has(
    normalizeGlobPattern(pattern),
  );
}

function toCanonicalFilePath(file, cwd) {
  return normalize(resolve(cwd, file));
}

// Creates a builder for transforming files and bundling StyleX CSS.
function createBuilder() {
  let config = null;

  const bundler = createBundler();

  const fileModifiedMap = new Map();

  // Configures the builder with the provided options.
  function configure(options) {
    config = options;
  }

  /// Retrieves the current configuration.
  function getConfig() {
    if (config == null) {
      throw new Error('Builder not configured');
    }
    return config;
  }

  // Finds the `@stylex;` at-rule in the provided PostCSS root.
  function findStyleXAtRule(root) {
    let styleXAtRule = null;
    root.walkAtRules((atRule) => {
      if (atRule.name === 'stylex' && !atRule.params) {
        styleXAtRule = atRule;
      }
    });
    return styleXAtRule;
  }

  // Retrieves all files that match the include and exclude patterns.
  function getFiles() {
    const { cwd, include, exclude } = getConfig();
    const includePatterns = Array.isArray(include) ? include : [];
    if (includePatterns.length === 0) {
      return [];
    }

    const ignoreWithoutNodeModulesCatchAll = (exclude ?? []).filter(
      (pattern) => !isNodeModulesCatchAllExcludePattern(pattern),
    );

    const files = new Set();
    for (const includePattern of includePatterns) {
      const isAbsolutePattern = path.isAbsolute(includePattern);
      const pointsToNodeModules = /(^|[/\\])node_modules([/\\]|$)/.test(
        includePattern,
      );
      const ignore =
        isAbsolutePattern && pointsToNodeModules
          ? ignoreWithoutNodeModulesCatchAll
          : exclude;

      const matchedFiles = globSync(includePattern, {
        onlyFiles: true,
        ignore,
        cwd,
      });
      for (const file of matchedFiles) {
        files.add(toCanonicalFilePath(file, cwd));
      }
    }

    return Array.from(files);
  }

  // Transforms the included files, bundles the CSS, and returns the result.
  async function build({ shouldSkipTransformError }) {
    const {
      cwd,
      babelConfig,
      useCSSLayers,
      enableLTRRTLComments,
      importSources,
      isDev,
    } = getConfig();

    const files = getFiles();
    const filesToTransform = [];

    // Remove deleted files since the last build
    for (const file of fileModifiedMap.keys()) {
      if (!files.includes(file)) {
        fileModifiedMap.delete(file);
        bundler.remove(file);
      }
    }

    for (const file of files) {
      const filePath = path.resolve(cwd, file);
      const mtimeMs = fs.existsSync(filePath)
        ? fs.statSync(filePath).mtimeMs
        : -Infinity;

      // Skip files that have not been modified since the last build
      // On first run, all files will be transformed
      const shouldSkip =
        fileModifiedMap.has(file) && mtimeMs === fileModifiedMap.get(file);

      if (shouldSkip) {
        continue;
      }

      // Record mtime only after transform finishes. Recording earlier lets a
      // concurrent build skip this file and bundle stale CSS.
      filesToTransform.push({ file, mtimeMs });
    }

    await Promise.all(
      filesToTransform.map(async ({ file, mtimeMs }) => {
        const filePath = path.resolve(cwd, file);
        const contents = fs.readFileSync(filePath, 'utf-8');
        if (bundler.shouldTransform(contents, { importSources })) {
          await bundler.transform(filePath, contents, babelConfig, {
            isDev,
            shouldSkipTransformError,
          });
        }
        fileModifiedMap.set(file, mtimeMs);
      }),
    );

    const css = bundler.bundle({
      useCSSLayers,
      enableLTRRTLComments,
    });
    return css;
  }

  // Retrieves the dependencies that PostCSS should watch.
  function getDependencies() {
    const { include, cwd } = getConfig();
    const dependencies = [];

    for (const fileOrGlob of include) {
      const dependency = parseDependency(fileOrGlob, cwd);
      if (dependency != null) {
        dependencies.push(dependency);
      }
    }

    return dependencies;
  }

  return {
    findStyleXAtRule,
    configure,
    build,
    getDependencies,
  };
}

module.exports = createBuilder;
