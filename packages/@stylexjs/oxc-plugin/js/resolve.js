/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
'use strict';

// Babel's commonJS file resolution (`filePathResolver` in
// @stylexjs/babel-plugin src/utils/state-manager.js, MIT), for the host resolver
// callback. Used when `moduleResolve` is a Jest mock, which native code cannot see.

const fs = require('node:fs');
const path = require('node:path');
const url = require('node:url');

const EXTENSIONS = ['.js', '.ts', '.tsx', '.jsx', '.mjs', '.cjs'];

function possibleAliasedPaths(importPath, aliases) {
  const result = [importPath];
  if (aliases == null || Object.keys(aliases).length === 0) return result;
  for (const [alias, value] of Object.entries(aliases)) {
    if (alias.includes('*')) {
      const [before, after] = alias.split('*');
      if (importPath.startsWith(before) && importPath.endsWith(after)) {
        const replacement = importPath.slice(before.length, after.length > 0 ? -after.length : undefined);
        for (const v of value) result.push(v.split('*').join(replacement));
      }
    } else if (alias === importPath) {
      for (const v of value) result.push(v);
    }
  }
  return result;
}

function possibleFilePaths(filePath) {
  const extension = path.extname(filePath);
  const base = EXTENSIONS.includes(extension) ? filePath.slice(0, -extension.length) : filePath;
  return [filePath, ...EXTENSIONS.map((ext) => base + ext)];
}

function commonJSResolver(mod, rootDir) {
  const resolve = (specifier, from) => url.fileURLToPath(mod.moduleResolve(specifier, url.pathToFileURL(from)));
  return (relativeFilePath, sourceFilePath, aliases) => {
    for (const importPathStr of possibleFilePaths(relativeFilePath)) {
      if (importPathStr.startsWith('.')) {
        try {
          return resolve(importPathStr, sourceFilePath);
        } catch {
          continue;
        }
      }
      for (const possiblePath of possibleAliasedPaths(importPathStr, aliases)) {
        if (possiblePath.startsWith('/ROOT/') && rootDir != null) {
          const realPath = path.join(rootDir, possiblePath.slice('/ROOT/'.length));
          for (const candidate of possibleFilePaths(realPath)) if (fs.existsSync(candidate)) return candidate;
          continue;
        }
        if (path.isAbsolute(possiblePath)) {
          for (const candidate of possibleFilePaths(possiblePath)) if (fs.existsSync(candidate)) return candidate;
          continue;
        }
        try {
          return resolve(possiblePath, sourceFilePath);
        } catch {
          continue;
        }
      }
    }
    return null;
  };
}

module.exports = { commonJSResolver, EXTENSIONS };
