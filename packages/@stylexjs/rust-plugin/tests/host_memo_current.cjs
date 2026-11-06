/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
'use strict';
const assert = require('node:assert/strict');
const test = require('node:test');
const babel = require('@babel/core');
const plugin = require('@stylexjs/babel-plugin');
const native = require('../index');
const filename = '/app/a.js';
const syntax = {
  filename,
  configFile: false,
  babelrc: false,
  highlightCode: false,
};
function compare(source, makeOptions) {
  const left = makeOptions();
  const expected = babel.transformSync(source, {
    ...syntax,
    plugins: [[plugin, left.options]],
  });
  const right = makeOptions();
  const actual = native.transformSync(source, { ...right.options, filename });
  assert.equal(babel.transformSync(actual.code, syntax).code, expected.code);
  assert.deepEqual(actual.metadata.stylex, expected.metadata.stylex);
  assert.deepEqual(right.trace, left.trace, 'executed host calls and laziness');
  return right.trace;
}
test('host calls run live even when the same callback body is evaluated twice', () => {
  const source =
    "import * as stylex from '@stylexjs/stylex';export const s=stylex.create({root:{color:[0,0].map(()=>stylex.env.next())[1]}});";
  const trace = compare(source, () => {
    const trace = [];
    return {
      trace,
      options: {
        env: {
          next() {
            trace.push('next');
            return trace.length === 1 ? 'red' : 'blue';
          },
        },
      },
    };
  });
  assert.deepEqual(trace, ['next', 'next']);
});
test('skipped host calls stay lazy after memo bypass', () => {
  const source =
    "import * as stylex from '@stylexjs/stylex';export const s=stylex.create({root:{color:true?'red':stylex.env.boom(),width:false?stylex.env.boom():1,opacity:stylex.env.pick(2)}});";
  const trace = compare(source, () => {
    const trace = [];
    return {
      trace,
      options: {
        env: {
          boom() {
            trace.push('boom');
            throw new Error('must stay lazy');
          },
          pick(v) {
            trace.push(['pick', v]);
            return v;
          },
        },
      },
    };
  });
  assert.deepEqual(trace, [['pick', 2]]);
});
test('repeat arguments with host calls are evaluated live before any reuse', () => {
  const source =
    "import * as stylex from '@stylexjs/stylex';export const s=stylex.create({root:{color:[0,0].map(()=> 'red'.repeat(stylex.env.count()))[1]}});";
  const trace = compare(source, () => {
    const trace = [];
    return {
      trace,
      options: {
        env: {
          count() {
            trace.push('count');
            return 1;
          },
        },
      },
    };
  });
  assert.deepEqual(trace, ['count', 'count']);
});

test('profiled forty-rule env calls preserve execution and ordered CSS', () => {
  const source =
    "import * as stylex from '@stylexjs/stylex';export const s=stylex.create({" +
    Array.from(
      { length: 40 },
      (_, i) => `k${i}:{color:stylex.env.f('c${i}')}`,
    ).join(',') +
    '});';
  const trace = compare(source, () => {
    const trace = [];
    return {
      trace,
      options: {
        unstable_moduleResolution: { type: 'haste' },
        env: {
          f(v) {
            trace.push(v);
            return v;
          },
        },
      },
    };
  });
  assert.deepEqual(
    trace,
    Array.from({ length: 40 }, (_, i) => 'c' + i),
  );
});
