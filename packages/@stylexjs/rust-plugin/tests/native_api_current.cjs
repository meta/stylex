/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const { test } = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');
const { Worker } = require('node:worker_threads');
const { transformSync, processStylexRules } = require('..');

const header = "import * as stylex from '@stylexjs/stylex';\n";
const styles =
  "export const s = stylex.create({ a: { color: stylex.env.brand('x') } });";

test('exceptions thrown by user callbacks propagate unchanged', () => {
  const failure = new Error('boom');
  failure.extra = 42;
  assert.throws(
    () =>
      transformSync(header + styles, {
        filename: '/app/a.js',
        env: {
          brand: () => {
            throw failure;
          },
        },
      }),
    (error) => error === failure,
  );
  assert.throws(
    () =>
      transformSync(
        header +
          "import { t } from './t.stylex';\n" +
          'export const s = stylex.create({ a: { color: t.c } });',
        {
          filename: '/app/a.js',
          unstable_moduleResolution: {
            type: 'custom',
            filePathResolver: () => {
              // A non-Error value must also reach the caller untouched.
              // eslint-disable-next-line no-throw-literal
              throw 'resolver boom';
            },
            getCanonicalFilePath: (file) => file,
          },
        },
      ),
    (error) => error === 'resolver boom',
  );
});

test('callbacks receive their owning object as `this`', () => {
  let receiver;
  const env = {
    brand() {
      receiver = this;
      return 'red';
    },
  };
  transformSync(header + styles, { filename: '/app/a.js', env });
  assert.equal(receiver, env);
});

test('option data cannot spell host callback tags', () => {
  for (const key of [
    '__stylexCallback',
    '__stylexUndefined',
    '__stylexNumber',
  ]) {
    assert.throws(
      () =>
        transformSync(header + styles, {
          filename: '/app/a.js',
          env: { brand: { [key]: '@debugFile' } },
        }),
      new RegExp(`cannot contain the key ${key}`),
    );
  }
});

test('env callback results behave like the JavaScript values Babel sees', () => {
  const babel = require('@babel/core');
  const babelPlugin = require('@stylexjs/babel-plugin');
  const make = (value) =>
    `${header}export const s = stylex.create({ a: { color: ${value} } });`;
  const cyclic = () => {
    const object = { color: 'red' };
    object.self = object;
    return object;
  };
  const cases = [
    [
      'Date as a style value',
      make("stylex.env.f('x')"),
      { f: () => new Date(0) },
    ],
    [
      'Map as a style value',
      make("stylex.env.f('x')"),
      { f: () => new Map([['a', 1]]) },
    ],
    [
      'Map getter',
      make("stylex.env.f('x').size"),
      { f: () => new Map([['a', 1]]) },
    ],
    [
      'cyclic object projection',
      make("stylex.env.f('x').self.color"),
      { f: cyclic },
    ],
    [
      'compiler function argument',
      make('stylex.env.f((v) => v + "px")'),
      { f: (fn) => fn(3) },
    ],
    [
      'nested env function',
      make('stylex.env.a.b(3)'),
      { a: { b: (x) => `c${x}` } },
    ],
    [
      'class instance method',
      make("stylex.env.f().name('y')"),
      {
        f: () =>
          new (class {
            name(x) {
              return `n-${x}`;
            }
          })(),
      },
    ],
  ];
  for (const [name, code, env] of cases) {
    const run = (transform) => {
      try {
        return { rules: transform().map((rule) => rule[1].ltr) };
      } catch (error) {
        return {
          error: String(error.message)
            .split('\n')[0]
            .replace(/^.*?: /, ''),
        };
      }
    };
    const expected = run(
      () =>
        babel.transformSync(code, {
          filename: '/app/a.js',
          babelrc: false,
          configFile: false,
          plugins: [[babelPlugin, { env }]],
        }).metadata.stylex,
    );
    const actual = run(
      () => transformSync(code, { filename: '/app/a.js', env }).metadata.stylex,
    );
    assert.deepEqual(actual, expected, name);
  }
});

test('deep nesting compiles or fails catchably, never crashing the process', () => {
  const nested = (depth) =>
    `${header}export function f(a) { ${'if (a) '.repeat(depth)} return 1; }\n` +
    'export const s = stylex.create({ a: { color: "red" } });';
  // Babel compiles this depth too.
  const result = transformSync(nested(1000), { filename: '/app/deep.js' });
  assert.equal(result.metadata.stylex.length, 1);
  // Babel throws a RangeError here; the native compiler reports its limit.
  assert.throws(
    () => transformSync(nested(20000), { filename: '/app/deeper.js' }),
    /nesting exceeds/,
  );
});

test('nested input also compiles inside a worker with the default stack', async () => {
  const script = `
    const { parentPort } = require('node:worker_threads');
    const { transformSync } = require(${JSON.stringify(path.resolve(__dirname, '..'))});
    const code = "import * as stylex from '@stylexjs/stylex';\\n" +
      'export const x = ' + '('.repeat(1000) + '1' + ')'.repeat(1000) + ';\\n' +
      'export const s = stylex.create({ a: { color: "red" } });';
    parentPort.postMessage(transformSync(code, { filename: '/app/w.js' }).metadata.stylex.length);
  `;
  const count = await new Promise((resolve, reject) => {
    const worker = new Worker(script, { eval: true });
    worker.once('message', resolve);
    worker.once('error', reject);
    worker.once(
      'exit',
      (code) => code !== 0 && reject(new Error(`worker exited ${code}`)),
    );
  });
  assert.equal(count, 1);
});

test('large metadata round-trips through processStylexRules', () => {
  const entries = Array.from(
    { length: 2000 },
    (_, i) => `k${i}: { width: ${i} }`,
  ).join(',');
  const result = transformSync(
    header + `export const s = stylex.create({ ${entries} });`,
    {
      filename: '/app/big.js',
    },
  );
  assert.equal(result.metadata.stylex.length, 2000);
  assert.ok(
    processStylexRules(result.metadata.stylex, {}).includes('width:1999px'),
  );
});

test('a transform started from inside a callback runs while the outer one waits', () => {
  const inner = `${header}export const t = stylex.create({ b: { color: 'blue' } });`;
  let innerRules;
  const result = transformSync(header + styles, {
    filename: '/app/outer.js',
    env: {
      brand() {
        innerRules = transformSync(inner, { filename: '/app/inner.js' })
          .metadata.stylex;
        return 'red';
      },
    },
  });
  assert.equal(innerRules.length, 1);
  assert.ok(result.metadata.stylex[0][1].ltr.includes('color:red'));
  // The shared compiler thread is idle again afterwards.
  assert.equal(
    transformSync(inner, { filename: '/app/again.js' }).metadata.stylex.length,
    1,
  );
});

test('compiler panics or failures leave later transforms working', () => {
  assert.throws(() => transformSync('const = ;', { filename: '/app/bad.js' }));
  const ok = transformSync(
    header + 'export const s = stylex.create({ a: { color: "red" } });',
    {
      filename: '/app/ok.js',
    },
  );
  assert.equal(ok.metadata.stylex.length, 1);
});

test('workers that use the compiler can terminate cleanly', async () => {
  const script = `
    const { parentPort } = require('node:worker_threads');
    const { transformSync } = require(${JSON.stringify(path.resolve(__dirname, '..'))});
    for (let i = 0; i < 20; i++) {
      transformSync("import * as stylex from '@stylexjs/stylex';\\nexport const s = stylex.create({ a: { color: 'red' } });", { filename: '/app/w' + i + '.js' });
    }
    parentPort.postMessage('done');
    setInterval(() => {}, 1000);
  `;
  for (let round = 0; round < 3; round++) {
    const worker = new Worker(script, { eval: true });
    await new Promise((resolve, reject) => {
      worker.once('message', resolve);
      worker.once('error', reject);
    });
    assert.equal(await worker.terminate(), 1);
  }
});

test('deeply nested transforms fail with a catchable RangeError', () => {
  const nested = (level) =>
    transformSync(header + styles, {
      filename: `/app/n${level}.js`,
      env: {
        brand: () => {
          nested(level + 1);
          return 'red';
        },
      },
    });
  assert.throws(() => nested(0), RangeError);
  assert.equal(
    transformSync(header + styles, {
      filename: '/app/after.js',
      env: { brand: () => 'red' },
    }).metadata.stylex.length,
    1,
  );
});

test('user code can catch an exception raised inside a compiler function it called', () => {
  const failure = new Error('inner');
  const code = `${header}export const s = stylex.create({ a: { color: stylex.env.outer((v) => stylex.env.inner(v)) } });`;
  const result = transformSync(code, {
    filename: '/app/catch.js',
    env: {
      inner: () => {
        throw failure;
      },
      outer: (fn) => {
        try {
          return fn('x');
        } catch (error) {
          return error === failure ? 'green' : 'wrong';
        }
      },
    },
  });
  assert.ok(result.metadata.stylex[0][1].ltr.includes('color:green'));
});

test('processStylexRules tolerates the rule shapes the JS compiler accepts', () => {
  const babelPlugin = require('@stylexjs/babel-plugin');
  const shapes = [
    [['x', { ltr: '.x{color:red}' }]],
    [['x', { ltr: '.x{color:red}', rtl: null }, 'NaN']],
    [['x', { ltr: '.x{color:red}', rtl: null }, 3000, 'extra']],
    [['x', { ltr: '.x{color:red}' }, '3000']],
  ];
  for (const rules of shapes) {
    for (const options of [{}, { useLayers: true }, true]) {
      assert.equal(
        processStylexRules(rules, options),
        babelPlugin.processStylexRules(rules, options),
        JSON.stringify([rules, options]),
      );
    }
  }
});

test('errors carry the code fields of Babel errors', () => {
  assert.throws(
    () =>
      transformSync(
        'import * as stylex from "@stylexjs/stylex"; const x = stylex.create(1);',
        {
          filename: '/app/err.js',
        },
      ),
    (error) =>
      error.code === 'BABEL_TRANSFORM_ERROR' && error instanceof SyntaxError,
  );
  assert.throws(
    () => transformSync('const = ;', { filename: '/app/parse.js' }),
    (error) =>
      error instanceof SyntaxError &&
      error.code === 'BABEL_PARSE_ERROR' &&
      error.loc.line === 1,
  );
});
