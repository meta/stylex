/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

// Jest setup that records every call of the real Babel plugin as one JSON line
// ({ code, filename, options, parserPlugins }) in $OXC_RECORD_FILE. Calls whose options hold
// functions or regexes are skipped, since they cannot be replayed from JSON.

const fs = require('node:fs');
const path = require('node:path');

require('./unfocus-jest-setup.cjs');

const babelPluginRoot = path.resolve(__dirname, '../../babel-plugin');
const file = process.env.OXC_RECORD_FILE;
if (!file) throw new Error('OXC_RECORD_FILE is not set');

function plain(value) {
  if (typeof value === 'function' || value instanceof RegExp) return false;
  if (value && typeof value === 'object') return Object.values(value).every(plain);
  return true;
}

let cached = null;
function recorder() {
  if (cached) return cached;
  const realModule = jest.requireActual(path.join(babelPluginRoot, 'src/index.js'));
  const real = realModule.default ?? realModule;
  function plugin(api, options, dirname) {
    const inner = real(api, options, dirname);
    return {
      ...inner,
      pre(f) {
        if (plain(options ?? {})) {
          const parserOpts = f.opts.parserOpts || {};
          fs.appendFileSync(
            file,
            JSON.stringify({
              code: f.code,
              filename: f.opts.filename ?? null,
              options: options ?? {},
              parserPlugins: parserOpts.plugins || [],
            }) + '\n',
          );
        }
        if (inner.pre) return inner.pre.call(this, f);
      },
    };
  }
  Object.assign(plugin, real);
  plugin.default = plugin;
  cached = plugin;
  return plugin;
}

jest.doMock(path.join(babelPluginRoot, 'src/index.js'), recorder);
jest.doMock(path.join(babelPluginRoot, 'lib/index.js'), recorder);
jest.doMock('@stylexjs/babel-plugin', recorder);
