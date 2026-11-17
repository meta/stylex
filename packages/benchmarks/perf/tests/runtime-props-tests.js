/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const path = require('path');
const vm = require('vm');
const stylex = require('@stylexjs/stylex');
const stylexPlugin = require('@stylexjs/babel-plugin');
const { transformFileSync } = require('@babel/core');
const { createSuite } = require('../helpers');

const fixture = path.resolve(__dirname, '../fixtures/props-runtime.js');

// Compile the fixture and load its exported styles, so the benchmark merges
// the same objects the compiler emits for this style resolution.
function loadStyles(styleResolution) {
  const { code } = transformFileSync(fixture, {
    babelrc: false,
    configFile: false,
    plugins: [
      [
        stylexPlugin,
        {
          runtimeInjection: false,
          styleResolution,
          unstable_moduleResolution: { type: 'haste' },
        },
      ],
      '@babel/plugin-transform-modules-commonjs',
    ],
  });
  const mod = { exports: {} };
  const load = vm.compileFunction(code, ['module', 'exports', 'require']);
  load(mod, mod.exports, (id) =>
    id === '@stylexjs/stylex' ? stylex : require(id),
  );
  return mod.exports;
}

/**
 * Performance of 'stylex.props' at runtime
 */
function runSuite(options) {
  for (const styleResolution of ['application-order', 'property-specificity']) {
    const { styles, dynamic } = loadStyles(styleResolution);
    const { suite, test } = createSuite(
      `stylex.props: ${styleResolution}`,
      options,
    );
    const all = Object.values(styles);
    const combinations = [];
    for (let n = 0; n < 1024; n++) {
      combinations.push([
        all[n % all.length],
        n & 8 && all[(n >> 4) % all.length],
        [n & 16 && all[(n >> 5) % all.length]],
      ]);
    }
    let i = 0;

    test('single style', () => {
      stylex.props(styles.root);
    });

    test('merge 3 styles', () => {
      stylex.props(styles.root, styles.primary, styles.large);
    });

    test('merge 3 styles, first use', () => {
      stylex.props(
        { ...styles.root },
        { ...styles.primary },
        { ...styles.large },
      );
    });

    test('conditional styles', () => {
      i++;
      stylex.props(
        styles.root,
        i & 1 && styles.primary,
        i & 2 && styles.large,
        i & 4 && styles.disabled,
        [styles.text, i & 8 && styles.compact, i & 16 && styles.responsive],
      );
    });

    test('many combinations', () => {
      i++;
      stylex.props(combinations[i & 1023]);
    });

    test('dynamic styles', () => {
      i++;
      stylex.props(
        styles.root,
        dynamic.size(i & 255, 20),
        dynamic.spacing(i & 15),
      );
    });

    suite.run();
  }
}

module.exports = runSuite;
