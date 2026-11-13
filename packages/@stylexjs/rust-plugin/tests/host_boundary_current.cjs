/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

// Values, callbacks and compiler functions crossing the native boundary must
// behave exactly like the live values Babel's evaluator sees.

const { test } = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');
const { Worker } = require('node:worker_threads');
const { transformSync } = require('..');

const header = "import * as stylex from '@stylexjs/stylex';\n";
const make = (value) =>
  `${header}export const s = stylex.create({ a: { color: ${value} } });`;
const width = (value) =>
  `${header}export const s = stylex.create({ a: { width: ${value} } });`;

function outcome(run) {
  try {
    return { rules: run().map((rule) => rule[1].ltr) };
  } catch (error) {
    return {
      error: String(error && error.message)
        .split('\n')[0]
        .replace(/^.*?\.js: /, ''),
    };
  }
}
function babel(code, options, filename = '/app/a.js') {
  const core = require('@babel/core');
  const plugin = require('@stylexjs/babel-plugin');
  return core.transformSync(code, {
    filename,
    babelrc: false,
    configFile: false,
    plugins: [[plugin, options]],
  }).metadata.stylex;
}
function native(code, options, filename = '/app/a.js') {
  return transformSync(code, { filename, ...options }).metadata.stylex;
}
// Runs a scenario through Babel and the native compiler, requiring equal results.
function same(name, code, makeOptions) {
  const expected = outcome(() => babel(code, makeOptions()));
  const actual = outcome(() => native(code, makeOptions()));
  assert.deepEqual(actual, expected, name);
  return actual;
}
const color = (result) => result.rules[0].replace(/^.*\{color:(.*)\}$/, '$1');

test('an env function called with a number answers a number', () => {
  same('number argument and result', width('stylex.env.f(2)'), () => ({
    env: { f: (n) => n + 1 },
  }));
});

test('a retained compiler function runs its own arrow during a later transform', () => {
  for (const engine of [babel, native]) {
    let kept;
    const env = {
      keep: (fn) => {
        kept = fn;
        return 'red';
      },
      apply: (fn) => String(fn('own')),
      use: () => String(kept('x')),
    };
    engine(make("stylex.env.keep((v) => 'A-' + v)"), { env }, '/app/a.js');
    const withOwn = outcome(() =>
      engine(
        make("stylex.env.apply((v) => 'B-' + v) + '|' + stylex.env.use()"),
        { env },
        '/app/b.js',
      ),
    );
    assert.equal(color(withOwn), 'B-own|A-x', engine.name);
    const withoutOwn = outcome(() =>
      engine(make('stylex.env.use()'), { env }, '/app/c.js'),
    );
    assert.equal(color(withoutOwn), 'A-x', engine.name);
  }
});

test('a retained compiler function reads its own transform options', () => {
  for (const engine of [babel, native]) {
    let kept;
    const first = {
      keep: (fn) => {
        kept = fn;
        return 'red';
      },
      tag: (v) => `first-${v}`,
    };
    const second = { tag: (v) => `second-${v}`, use: () => kept('x') };
    engine(make('stylex.env.keep((v) => stylex.env.tag(v))'), { env: first });
    const result = outcome(() =>
      engine(make("stylex.env.use() + '|' + stylex.env.tag('y')"), {
        env: second,
      }),
    );
    assert.equal(color(result), 'first-x|second-y', engine.name);
  }
});

test('retained compiler functions read their own handles when ids collide', () => {
  // Both transforms give their first env object the same handle id.
  for (const engine of [babel, native]) {
    let kept;
    const first = {
      cfg: { name: 'A' },
      keep: (fn) => {
        kept = fn;
        return 'red';
      },
    };
    const second = { cfg: { name: 'B' }, use: () => kept('x') };
    engine(make('stylex.env.keep((v) => stylex.env.cfg.name + v)'), {
      env: first,
    });
    const result = outcome(() =>
      engine(make("stylex.env.use() + '|' + stylex.env.cfg.name"), {
        env: second,
      }),
    );
    assert.equal(color(result), 'Ax|B', engine.name);
    assert.equal(kept('z'), 'Az', engine.name);
  }
});

test('a function from a nested transform keeps its owner when the outer one calls it', () => {
  for (const engine of [babel, native]) {
    let innerFn;
    let outerFn;
    const inner = {
      cfg: { name: 'INNER' },
      give: (fn) => {
        innerFn = fn;
        return 'red';
      },
    };
    const outer = { cfg: { name: 'OUTER' } };
    outer.run = (fn) => {
      outerFn = fn;
      engine(make('stylex.env.give((v) => stylex.env.cfg.name + v)'), {
        env: inner,
      });
      return `${innerFn('x')}|${outerFn('y')}`;
    };
    const result = outcome(() =>
      engine(make('stylex.env.run((v) => stylex.env.cfg.name + v)'), {
        env: outer,
      }),
    );
    assert.equal(color(result), 'INNERx|OUTERy', engine.name);
    assert.equal(
      `${innerFn('1')}|${outerFn('2')}`,
      'INNER1|OUTER2',
      engine.name,
    );
  }
});

test('a getter that throws inside a retained function reaches the caller', () => {
  for (const engine of [babel, native]) {
    let kept;
    const failure = new Error('getter failure');
    const first = {
      cfg: {
        get bad() {
          throw failure;
        },
      },
      keep: (fn) => {
        kept = fn;
        return 'red';
      },
    };
    const second = {
      probe: () => {
        try {
          kept();
          return 'not thrown';
        } catch (error) {
          return error === failure ? 'caught' : `other ${error.message}`;
        }
      },
    };
    engine(make('stylex.env.keep(() => stylex.env.cfg.bad)'), { env: first });
    const result = outcome(() =>
      engine(make('stylex.env.probe()'), { env: second }),
    );
    assert.equal(color(result), 'caught', engine.name);
  }
});

test('an outer compiler function called from a nested transform runs in the outer one', () => {
  for (const engine of [babel, native]) {
    const options = { env: {} };
    options.env.outer = (fn) => {
      options.env.inner = (innerFn) => `${fn('x')}|${innerFn('y')}`;
      const inner = outcome(() =>
        engine(
          make("stylex.env.inner((v) => 'INNER-' + v)"),
          options,
          '/app/inner.js',
        ),
      );
      return color(inner);
    };
    const result = outcome(() =>
      engine(make("stylex.env.outer((v) => 'OUTER-' + v)"), options),
    );
    assert.equal(color(result), 'OUTER-x|INNER-y', engine.name);
  }
});

test('transforms nested in a callback behave exactly as when run alone', () => {
  // A nested transform's failures, warnings and retained functions stay its own.
  const shared = {
    runtimeInjection: true,
    unstable_moduleResolution: { type: 'haste' },
    propertyValidationMode: 'warn',
  };
  const settle = (run) => {
    try {
      const out = run();
      return {
        code: out.code,
        metadata: out.metadata,
        warnings: out.warnings,
      };
    } catch (error) {
      return { error: String(error.message).split('\n')[0] };
    }
  };
  let kept;
  const inner = {
    failing: [
      `${header}export const s = stylex.create({ a: { color: missing } });`,
      { filename: '/app/failing.js', ...shared },
    ],
    warning: [
      `${header}export const s = stylex.create({ a: { border: '1px solid red' } });`,
      { filename: '/app/warning.js', ...shared },
    ],
    retaining: [
      `${header}export const s = stylex.create({ a: { color: stylex.env.keep((v) => stylex.env.tag + v) } });`,
      {
        filename: '/app/retaining.js',
        ...shared,
        env: {
          tag: 'inner-',
          keep: (fn) => {
            kept = fn;
            return 'red';
          },
        },
      },
    ],
  };
  const outer = `${header}export const a = stylex.create({ x: { color: stylex.env.hook('red'), border: '2px solid blue' } });
export const k = stylex.keyframes({ from: { opacity: 0 } });
export const b = stylex.create({ y: { marginInlineStart: 4 }, d: (p) => ({ width: p }) });
`;
  const runOuter = (hook) =>
    settle(() =>
      transformSync(outer, {
        filename: '/app/outer.js',
        ...shared,
        env: { hook },
      }),
    );
  const later = () =>
    settle(() =>
      transformSync(
        `${header}export const s = stylex.create({ a: { color: 'green' } });`,
        {
          filename: '/app/later.js',
          ...shared,
        },
      ),
    );

  const alone = {};
  for (const [name, [source, options]] of Object.entries(inner)) {
    alone[name] = settle(() => transformSync(source, options));
  }
  const aloneKept = kept('x');
  const aloneOuter = runOuter((value) => value);
  const aloneLater = later();

  kept = undefined;
  const nested = {};
  const nestedOuter = runOuter((value) => {
    for (const [name, [source, options]] of Object.entries(inner)) {
      nested[name] = settle(() => transformSync(source, options));
    }
    return value;
  });
  assert.deepEqual(nested, alone);
  assert.deepEqual(nestedOuter, aloneOuter);
  assert.equal(kept('x'), aloneKept);
  assert.equal(aloneKept, 'inner-x');
  assert.deepEqual(later(), aloneLater);
  assert.ok(alone.failing.error, 'the failing transform fails');
  assert.ok(alone.warning.warnings.length > 0, 'the warning transform warns');
  assert.ok(aloneOuter.warnings.length > 0, 'the outer transform warns');
});

test('a compiler function stays callable after its transform finished', () => {
  for (const engine of [babel, native]) {
    let kept;
    const env = {
      keep: (fn) => {
        kept = fn;
        return 'red';
      },
      tag: (v) => `t-${v}`,
    };
    engine(make("stylex.env.keep((v) => stylex.env.tag(v) + '!')"), { env });
    assert.equal(kept('z'), 't-z!', engine.name);
    assert.equal(kept('w'), 't-w!', engine.name);
  }
});

test('exceptions inside a retained compiler function reach its caller', () => {
  for (const engine of [babel, native]) {
    let kept;
    const failure = new Error('retained failure');
    const env = {
      keep: (fn) => {
        kept = fn;
        return 'red';
      },
      boom: () => {
        throw failure;
      },
      probe: () => {
        try {
          kept();
          return 'not thrown';
        } catch (error) {
          return error === failure ? 'caught' : `other ${error.message}`;
        }
      },
    };
    engine(make('stylex.env.keep(() => stylex.env.boom())'), { env });
    const result = outcome(() => engine(make('stylex.env.probe()'), { env }));
    assert.equal(color(result), 'caught', engine.name);
    assert.throws(
      () => kept(),
      (error) => error === failure,
      engine.name,
    );
  }
});

test('a getter planted by one file runs that file’s arrow when another file reads it', () => {
  class Instance {}
  for (const engine of [babel, native]) {
    const env = { inst: new Instance(), apply: (fn, v) => fn(v) };
    engine(
      make(
        "stylex.env.inst.__defineGetter__('x', () => 'A-planted') === undefined ? 'red' : 'blue'",
      ),
      { env },
      '/app/a.js',
    );
    const result = outcome(() =>
      engine(
        make(
          "stylex.env.apply((v) => 'B-private-' + v, 1) + '|' + stylex.env.inst.x",
        ),
        { env },
        '/app/b.js',
      ),
    );
    assert.equal(color(result), 'B-private-1|A-planted', engine.name);
  }
});

test('callback results keep their identity, receiver and laziness', () => {
  const cases = [
    [
      'method mutates its receiver',
      "stylex.env.obj().inc() + '-' + stylex.env.obj().inc()",
      () => {
        const o = {
          n: 0,
          inc() {
            return String(++this.n);
          },
        };
        return { obj: () => o };
      },
    ],
    [
      'method receiver is the original object',
      "stylex.env.obj().isSelf() ? 'red' : 'blue'",
      () => {
        const o = {
          isSelf() {
            return this === o;
          },
        };
        return { obj: () => o };
      },
    ],
    [
      'returned object identity',
      "stylex.env.get() === stylex.env.get() ? 'red' : 'blue'",
      () => {
        const o = { a: 1 };
        return { get: () => o };
      },
    ],
    [
      'object passed back keeps identity',
      "stylex.env.check(stylex.env.get()) ? 'red' : 'blue'",
      () => {
        const o = { a: 1 };
        return { get: () => o, check: (x) => x === o };
      },
    ],
    [
      'array passed back keeps identity',
      "stylex.env.check(stylex.env.get()) ? 'red' : 'blue'",
      () => {
        const o = ['a'];
        return { get: () => o, check: (x) => x === o };
      },
    ],
    [
      'unused throwing getter is never read',
      'stylex.env.get().used',
      () => ({
        get: () => ({
          used: 'red',
          get unused() {
            throw new Error('unused getter');
          },
        }),
      }),
    ],
    [
      'getters run only when read',
      'stylex.env.get().a + stylex.env.count()',
      () => {
        let count = 0;
        const o = {
          get a() {
            count++;
            return 'x';
          },
          get b() {
            count++;
            return 'y';
          },
        };
        return { get: () => o, count: () => String(count) };
      },
    ],
    [
      'array with its own map',
      'stylex.env.get()[0]',
      () => {
        const a = ['red'];
        a.map = () => 'replaced';
        return { get: () => a };
      },
    ],
    [
      'array subclass',
      'stylex.env.get().hello() + stylex.env.get()[0]',
      () => {
        class List extends Array {
          hello() {
            return 'h-';
          }
          map() {
            return 'replaced';
          }
        }
        return { get: () => List.from(['red']) };
      },
    ],
    [
      'array extra property',
      'stylex.env.get().extra',
      () => {
        const a = ['x'];
        a.extra = 'red';
        return { get: () => a };
      },
    ],
    [
      'sparse array hole',
      "typeof stylex.env.get()[0] === 'undefined' ? 'red' : 'blue'",
      () => ({ get: () => [, 'x'] }),
    ],
    [
      'sparse array length',
      "stylex.env.get().length + 'px'",
      () => ({ get: () => [, , 'x'] }),
    ],
    [
      'proxy get trap',
      'stylex.env.get().color',
      () => ({
        get: () =>
          new Proxy(
            {},
            { get: (_, key) => (key === 'color' ? 'red' : undefined) },
          ),
      }),
    ],
    [
      'proxy with throwing ownKeys',
      'stylex.env.get().color',
      () => ({
        get: () =>
          new Proxy(
            { color: 'red' },
            {
              ownKeys() {
                throw new Error('ownKeys');
              },
            },
          ),
      }),
    ],
    [
      'proxy with throwing getPrototypeOf',
      'stylex.env.get().color',
      () => ({
        get: () =>
          new Proxy(
            { color: 'red' },
            {
              getPrototypeOf() {
                throw new Error('proto');
              },
            },
          ),
      }),
    ],
    [
      'deeply nested result',
      "stylex.env.deep() ? 'red' : 'blue'",
      () => ({
        deep: () => {
          let value = 'red';
          for (let i = 0; i < 5000; i++) value = [{ k: value }];
          return value;
        },
      }),
    ],
    [
      'function result name',
      'stylex.env.get().name',
      () => ({ get: () => function red() {} }),
    ],
    [
      'function length',
      "stylex.env.fn.length + 'px'",
      () => ({ fn: (a, b) => a + b }),
    ],
    [
      'function call through call()',
      "stylex.env.fn.call({ p: 'r' }, 'ed')",
      () => ({
        fn(value) {
          return this.p + value;
        },
      }),
    ],
    [
      'spread of a live object',
      '({ ...stylex.env.get(), b: 1 }).color',
      () => ({
        get: () => ({ color: 'red' }),
      }),
    ],
    [
      'Object.keys of a live object without reading values',
      "Object.keys(stylex.env.get()).join('-')",
      () => ({
        get: () => ({
          a: 1,
          get b() {
            throw new Error('read');
          },
        }),
      }),
    ],
    [
      'Array.isArray of a live array',
      "Array.isArray(stylex.env.get()) ? 'red' : 'blue'",
      () => ({ get: () => [1] }),
    ],
    [
      'wide result',
      'stylex.env.wide().k2999',
      () => ({
        wide: () =>
          Object.fromEntries(
            Array.from({ length: 3000 }, (_, i) => [`k${i}`, 'red']),
          ),
      }),
    ],
  ];
  for (const [name, value, env] of cases) {
    same(name, make(value), () => ({ env: env() }));
  }
});

test('array spread of a live sparse array follows JavaScript iteration', () => {
  // Babel's evaluator rejects array spread, so expect plain JS (holes as undefined).
  const result = outcome(() =>
    native(
      make(
        "[...stylex.env.get()].length + '-' + typeof [...stylex.env.get()][0]",
      ),
      {
        env: { get: () => [, , 'x'] },
      },
    ),
  );
  assert.equal(color(result), '3-undefined');
});

test('env options are live values, not copies', () => {
  const cases = [
    [
      'env object identity',
      "stylex.env.check(stylex.env.cfg) ? 'red' : 'blue'",
      () => {
        const cfg = { a: 1 };
        return { cfg, check: (x) => x === cfg };
      },
    ],
    [
      'env array identity',
      "stylex.env.check(stylex.env.list) ? 'red' : 'blue'",
      () => {
        const list = ['a'];
        return { list, check: (x) => x === list };
      },
    ],
    [
      'env mutated by a callback',
      'stylex.env.mutate() + stylex.env.cfg.v',
      () => {
        const env = { cfg: { v: 'before' } };
        env.mutate = () => {
          env.cfg.v = 'after';
          return '';
        };
        return env;
      },
    ],
    [
      'env non-enumerable property',
      'stylex.env.cfg.hidden',
      () => {
        const cfg = {};
        Object.defineProperty(cfg, 'hidden', {
          value: 'red',
          enumerable: false,
        });
        return { cfg };
      },
    ],
    [
      'env unused throwing getter',
      'stylex.env.used',
      () => ({
        used: 'red',
        get unused() {
          throw new Error('unused env getter');
        },
      }),
    ],
    [
      'env getter count',
      'stylex.env.g + stylex.env.count()',
      () => {
        let count = 0;
        return {
          get g() {
            count++;
            return 'x';
          },
          get h() {
            count++;
            return 'y';
          },
          count: () => String(count),
        };
      },
    ],
    [
      'env top-level getter read once per transform',
      'stylex.env.g + stylex.env.g',
      () => {
        let count = 0;
        return {
          get g() {
            return String(++count);
          },
        };
      },
    ],
    [
      'env deeply nested object',
      `stylex.env.d${'.k'.repeat(90)}`,
      () => {
        let value = 'red';
        for (let i = 0; i < 90; i++) value = { k: value };
        return { d: value };
      },
    ],
    [
      'env data spelling internal tags',
      'stylex.env.cfg.x',
      () => ({
        cfg: JSON.parse(
          '{"__stylexHandle":"0","__stylexCallback":"0","x":"red"}',
        ),
      }),
    ],
  ];
  for (const [name, value, env] of cases) {
    same(name, make(value), () => ({ env: env() }));
  }
});

test('one function shared by several env objects gets each as its receiver', () => {
  const env = () => {
    function f() {
      return this.t;
    }
    return { o1: { t: 'a', f }, o2: { t: 'b', f } };
  };
  for (const value of [
    'stylex.env.o1.f() + stylex.env.o2.f()',
    'stylex.env.o2.f() + stylex.env.o1.f()',
    "stylex.env.o1.f === stylex.env.o2.f ? 'red' : 'blue'",
    "stylex.env.check(stylex.env.o1.f, stylex.env.o2.f) ? 'red' : 'blue'",
  ]) {
    same(value, make(value), () => {
      const values = env();
      return {
        env: { ...values, check: (a, b) => a === b && a === values.o1.f },
      };
    });
  }
});

test('env objects still compile as style and theme values', () => {
  same(
    'env namespace object',
    `${header}export const s = stylex.create({ a: stylex.env.style });`,
    () => ({ env: { style: { color: 'red', padding: 4 } } }),
  );
  same(
    'env fallback array',
    `${header}export const s = stylex.create({ a: { color: stylex.env.colors } });`,
    () => ({ env: { colors: ['red', 'blue'] } }),
  );
});

test('getters and methods run once, in source order', () => {
  const run = (engine) => {
    const log = [];
    const env = {
      obj: () => ({
        get g() {
          log.push('get g');
          return 'G';
        },
        m() {
          log.push('m');
          return 'M';
        },
      }),
    };
    const result = outcome(() =>
      engine(make('stylex.env.obj().g + stylex.env.obj().m()'), { env }),
    );
    return { log, result };
  };
  assert.deepEqual(run(native), run(babel));
});

test('strings with lone UTF-16 surrogates cross the host boundary intact', () => {
  for (const expression of [
    "stylex.env.unit('\\ud83d\\ude00'[0])",
    "stylex.env.unit('\\ud83d')",
    'stylex.env.value.charCodeAt(0)',
    "stylex.env.echo('\\udc00x').charCodeAt(0)",
  ]) {
    same(expression, width(expression), () => ({
      env: {
        unit: (value) => value.charCodeAt(0),
        echo: (value) => value,
        value: '\ud83d',
      },
    }));
  }
});

test('a wide callback result is decoded with bounded memory', () => {
  // A child with a deadline guards against copying the receiver per member.
  const { spawnSync } = require('node:child_process');
  const script = `
    const { transformSync } = require(${JSON.stringify(path.join(__dirname, '..'))});
    const header = "import * as stylex from '@stylexjs/stylex';\\n";
    // Peak resident set size in bytes; maxRSS is kilobytes on every platform.
    const peak = () => process.resourceUsage().maxRSS * 1024;
    const make = (size) => {
      const object = {};
      for (let i = 0; i < size; i++) { object['m' + i] = () => 'red'; object['p' + i] = 'x'.repeat(64); }
      return object;
    };
    transformSync(header + "export const s = stylex.create({ a: { color: 'red' } });", { filename: '/app/w.js' });
    const object = make(4000);
    const before = peak();
    const out = transformSync(header + 'export const s = stylex.create({ a: { color: stylex.env.get().m3999() } });', { filename: '/app/w.js', env: { get: () => object } });
    process.stdout.write(JSON.stringify({ growth: peak() - before, rule: out.metadata.stylex[0][1].ltr }));
  `;
  const child = spawnSync(
    process.execPath,
    ['--max-old-space-size=512', '-e', script],
    {
      encoding: 'utf8',
      timeout: 60000,
    },
  );
  assert.equal(child.status, 0, child.stderr || String(child.signal));
  const { growth, rule } = JSON.parse(child.stdout);
  assert.match(rule, /color:red/);
  assert.ok(growth < 128 * 1024 * 1024, `peak RSS grew by ${growth} bytes`);
});

function inWorker(source, resourceLimits) {
  return new Promise((resolve, reject) => {
    const worker = new Worker(source, {
      eval: true,
      workerData: { plugin: path.join(__dirname, '..') },
      resourceLimits,
    });
    worker.once('message', resolve);
    worker.once('error', reject);
    worker.once('exit', (code) => {
      if (code !== 0) reject(new Error(`worker exited with ${code}`));
    });
  });
}

test('nested transforms in a worker with a small stack fail catchably', async () => {
  const source = `
    const { parentPort, workerData } = require('node:worker_threads');
    const { transformSync } = require(workerData.plugin);
    const header = "import * as stylex from '@stylexjs/stylex';\\n";
    const make = (n) => header + 'export const s = stylex.create({ a: { color: stylex.env.nest(' + n + ') } });';
    const results = [];
    for (const levels of [1, 8, 32, 64]) {
      const options = { env: {} };
      options.env.nest = (n) => {
        if (n >= levels) return 'red';
        transformSync(make(n + 1), { filename: '/app/n.js', ...options });
        return 'red';
      };
      try {
        transformSync(make(1), { filename: '/app/n.js', ...options });
        results.push('ok');
      } catch (error) {
        results.push(error instanceof RangeError ? 'range' : 'other: ' + error.message);
      }
    }
    parentPort.postMessage(results);
  `;
  for (const stackSizeMb of [0.5, 0.6, 1]) {
    const results = await inWorker(source, { stackSizeMb });
    assert.equal(results[0], 'ok', `${stackSizeMb}MB ordinary input`);
    for (const result of results) {
      assert.ok(
        result === 'ok' || result === 'range',
        `${stackSizeMb}MB: ${result}`,
      );
    }
  }
});

test('workers retain and call their own compiler functions concurrently', async () => {
  const source = `
    const { parentPort, workerData } = require('node:worker_threads');
    const { transformSync } = require(workerData.plugin);
    const header = "import * as stylex from '@stylexjs/stylex';\\n";
    const make = (v) => header + 'export const s = stylex.create({ a: { color: ' + v + ' } });';
    // A letter first: a value starting with a digit would be normalized as a length.
    const id = 'k' + Math.random().toString(36).slice(2);
    let kept;
    const env = { keep: (fn) => { kept = fn; return 'red'; }, use: () => kept('x') };
    const bad = [];
    for (let i = 0; i < 50; i++) {
      transformSync(make("stylex.env.keep((v) => '" + id + "-" + i + "-' + v)"), { filename: '/app/a.js', env });
      const out = transformSync(make('stylex.env.use()'), { filename: '/app/b.js', env });
      const value = out.metadata.stylex[0][1].ltr;
      if (!value.includes(id + '-' + i + '-x')) bad.push(value);
    }
    parentPort.postMessage(bad);
  `;
  const results = await Promise.all(
    Array.from({ length: 8 }, () => inWorker(source)),
  );
  assert.deepEqual(
    results,
    Array.from({ length: 8 }, () => []),
  );
});

test('compiler functions are released once JavaScript drops them', (t) => {
  const plugin = path.join(__dirname, '..');
  const native = require(
    path.join(
      plugin,
      'native',
      `stylex.${process.platform}-${process.arch}.node`,
    ),
  );
  // The count is a test-only diagnostic (STYLEX_NATIVE_TEST_HOOKS=1 builds).
  if (typeof native.compilerFunctionCount !== 'function') {
    assert.notEqual(
      process.env.STYLEX_NATIVE_TEST_HOOKS,
      '1',
      'a test-hooks build must export compilerFunctionCount',
    );
    t.skip('the addon was built without test hooks');
    return;
  }
  // Child process: needs --expose-gc and a fresh compiler thread.
  const { spawnSync } = require('node:child_process');
  const script = `
    const path = require('node:path');
    const plugin = ${JSON.stringify(path.join(__dirname, '..'))};
    const { transformSync } = require(plugin);
    const binding = require(path.join(plugin, 'native', 'stylex.' + process.platform + '-' + process.arch + '.node'));
    const header = "import * as stylex from '@stylexjs/stylex';\\n";
    const make = (v) => header + 'export const s = stylex.create({ a: { color: ' + v + ' } });';
    let kept = [];
    const env = { keep: (fn) => { kept.push(fn); return 'red'; } };
    for (let i = 0; i < 300; i++) transformSync(make("stylex.env.keep((v) => 'A' + v)"), { filename: '/app/a.js', env });
    const held = binding.compilerFunctionCount();
    const stillWorks = kept[299]('z');
    // A compiler function returned by a compiler function is retained too.
    transformSync(make("stylex.env.keep((a) => (b) => a + b)"), { filename: '/app/c.js', env });
    const curried = kept.pop()('x');
    // A function registered for a call that then fails to encode another
    // argument never reaches JavaScript and is collected at the next start.
    let failed = false;
    try {
      transformSync(make('stylex.env.keep((v) => v, stylex.when.ancestor)'), { filename: '/app/d.js', env });
    } catch { failed = true; }
    kept = [];
    (async () => {
      for (let i = 0; i < 10; i++) { global.gc(); await new Promise((r) => setTimeout(r, 10)); }
      const curriedWorks = curried('y');
      transformSync(make("'red'"), { filename: '/app/b.js' });
      const withCurried = binding.compilerFunctionCount();
      process.stdout.write(JSON.stringify({ held, after: withCurried, stillWorks, curriedWorks, failed }));
    })();
  `;
  const child = spawnSync(process.execPath, ['--expose-gc', '-e', script], {
    encoding: 'utf8',
    timeout: 60000,
  });
  assert.equal(child.status, 0, child.stderr || String(child.signal));
  const { held, after, stillWorks, curriedWorks, failed } = JSON.parse(
    child.stdout,
  );
  assert.equal(stillWorks, 'Az');
  assert.equal(curriedWorks, 'xy');
  assert.equal(failed, true);
  assert.equal(held, 300);
  assert.ok(after < 10, `after collection: ${after}`);
});

test('the host creates an adopted literal once, as the live value it stands for', () => {
  const { createHost } = require('../host');
  const host = createHost({}, '/app/a.js', {});
  // Primitives cross as themselves; containers as `{ type, value }` records.
  const value = (type, item) => item;
  const record = (type, item) => ({ type, value: item });
  const array = host.invoke('@adopt', [
    'array',
    record('array', [undefined, 2]),
    record('array', [0]),
  ]);
  const object = host.invoke('@adopt', [
    'nullObject',
    record('object', [
      ['__proto__', 1],
      ['x', '\ud800'],
    ]),
  ]);
  const plain = host.invoke('@adopt', [
    'object',
    record('object', [['a', 'b']]),
  ]);
  assert.equal(array.type, 'handle');
  assert.equal(object.type, 'handle');
  // Read the created values back through the host, as the compiler would.
  const get = (reply, key) =>
    host.invoke('@get', [reply, value('string', key)]);
  const call = (name, ...args) =>
    host.invoke('@staticBuiltin', ['Object', value('string', name), ...args]);
  assert.deepEqual(get(array, 'length'), 2);
  assert.deepEqual(host.invoke('@binary', ['in', 0, array]), false);
  assert.deepEqual(host.invoke('@isArray', [array]), true);
  assert.deepEqual(call('hasOwn', object, '__proto__'), true);
  assert.deepEqual(get(object, 'x'), '\ud800');
  assert.deepEqual(host.invoke('@binary', ['in', 'toString', object]), false);
  assert.deepEqual(host.invoke('@binary', ['in', 'toString', plain]), true);
  assert.deepEqual(host.invoke('@sameValue', [plain, plain]), true);
  assert.notDeepEqual(plain, object);
  // Anything but data is refused.
  assert.deepEqual(host.invoke('@adopt', ['function', record('array', [])]), {
    type: 'thrown',
  });
});

// A failing compiler-evaluated arrow throws the plain Error Babel's arrow throws.
function arrowFailure(engine, env, file = '/app/arrow.js') {
  const { runEngine, describe } = require('./support/babel_errors.cjs');
  const source = `import * as stylex from '@stylexjs/stylex';
export const s = stylex.create({ a: { color: stylex.env.apply(() => missing) } });
`;
  try {
    const output = runEngine(engine, source, file, { env });
    return { css: output.metadata.stylex.map((rule) => rule[1].ltr) };
  } catch (error) {
    return { caught: describe(error), user: error === env.thrown };
  }
}

test('a failing compiler arrow reaches the transform only if user code lets it', () => {
  const scenarios = {
    uncaught: () => ({ apply: (fn) => fn() }),
    'caught, another error thrown': () => {
      const thrown = new SyntaxError('user replaced it');
      return {
        thrown,
        apply: (fn) => {
          try {
            return fn();
          } catch {
            throw thrown;
          }
        },
      };
    },
    'caught, rethrown unchanged': () => ({
      apply: (fn) => {
        try {
          return fn();
        } catch (error) {
          throw error;
        }
      },
    }),
    'caught, a value returned': () => ({
      apply: (fn) => {
        try {
          return fn();
        } catch {
          return 'blue';
        }
      },
    }),
  };
  for (const [name, make] of Object.entries(scenarios)) {
    const expected = arrowFailure('babel', make());
    for (const engine of ['direct', 'facade']) {
      assert.deepEqual(
        arrowFailure(engine, make()),
        expected,
        `${engine}: ${name}`,
      );
    }
  }
  // Babel wraps the arrow's Error once, prefixed with its resolved filename.
  assert.deepEqual(arrowFailure('babel', scenarios.uncaught()).caught, {
    kind: 'Error',
    message: `${path.resolve('/app/arrow.js')}: Referenced constant is not defined.`,
    code: 'BABEL_TRANSFORM_ERROR',
    extra: undefined,
  });
});

test('a failing compiler arrow in a nested transform stays in that transform', () => {
  const { runEngine } = require('./support/babel_errors.cjs');
  const outer = `import * as stylex from '@stylexjs/stylex';
export const s = stylex.create({ a: { color: stylex.env.nest('x') } });
`;
  const run = (engine) => {
    let nested;
    const env = {
      nest: () => {
        nested = arrowFailure(engine, { apply: (fn) => fn() }, '/app/inner.js');
        return 'green';
      },
    };
    const output = runEngine(engine, outer, '/app/outer.js', { env });
    return { outer: output.metadata.stylex.map((rule) => rule[1].ltr), nested };
  };
  const expected = run('babel');
  assert.ok(expected.nested.caught);
  assert.deepEqual(run('direct'), expected);
  assert.deepEqual(run('facade'), expected);
});

test('a compiler arrow kept past its transform fails as Babel’s does, and only for its caller', () => {
  const { runEngine, describe } = require('./support/babel_errors.cjs');
  const source = `import * as stylex from '@stylexjs/stylex';
export const s = stylex.create({ a: { color: stylex.env.keep(() => missing) } });
`;
  const later = `import * as stylex from '@stylexjs/stylex';
export const s = stylex.create({ a: { color: stylex.env.plain('red') } });
`;
  const run = (engine) => {
    let kept;
    runEngine(engine, source, '/app/keep.js', {
      env: {
        keep: (fn) => {
          kept = fn;
          return 'red';
        },
      },
    });
    const steps = [];
    try {
      kept();
      steps.push('returned');
    } catch (error) {
      steps.push(describe(error));
    }
    // A later transform is not affected by the kept arrow's failure.
    const output = runEngine(engine, later, '/app/later.js', {
      env: { plain: (value) => value },
    });
    steps.push(output.metadata.stylex.map((rule) => rule[1].ltr));
    return steps;
  };
  const expected = run('babel');
  // A plain Error with no code: nothing wraps it outside a transform.
  assert.deepEqual(expected[0], {
    kind: 'Error',
    message: 'Referenced constant is not defined.',
    code: undefined,
    extra: undefined,
  });
  assert.deepEqual(run('direct'), expected);
});

test('failing compiler arrows in workers behave as on the main thread', async () => {
  const { Worker } = require('node:worker_threads');
  const support = require.resolve('./support/babel_errors.cjs');
  const source = `
    const { parentPort, workerData } = require('node:worker_threads');
    const { runEngine, describe } = require(workerData.support);
    const code = "import * as stylex from '@stylexjs/stylex';\\nexport const s = stylex.create({ a: { color: stylex.env.apply(() => missing) } });\\n";
    const out = {};
    for (const engine of ['babel', 'direct']) {
      try {
        runEngine(engine, code, '/app/worker.js', { env: { apply: (fn) => fn() } });
        out[engine] = 'completed';
      } catch (error) {
        out[engine] = describe(error);
      }
    }
    parentPort.postMessage(out);
  `;
  const results = await Promise.all(
    Array.from(
      { length: 4 },
      () =>
        new Promise((resolve, reject) => {
          const worker = new Worker(source, {
            eval: true,
            workerData: { support },
          });
          worker.once('message', resolve);
          worker.once('error', reject);
        }),
    ),
  );
  for (const result of results) {
    assert.deepEqual(result.direct, result.babel);
    assert.equal(result.babel.kind, 'Error');
  }
});

// V8 crashes unwinding frames past its stack limit; no JS may run after overflow.
test('nested transforms and compiler functions at the V8 stack limit fail catchably', () => {
  const { spawnSync } = require('node:child_process');
  const script = `
    const { transformSync } = require(${JSON.stringify(path.join(__dirname, '..'))});
    const header = "import * as stylex from '@stylexjs/stylex';\\n";
    const make = (n) => header + 'export const s = stylex.create({ a: { color: stylex.env.nest(' + n + ') } });';
    const outcome = (run) => {
      try {
        run();
        return 'ok';
      } catch (error) {
        return error instanceof RangeError ? 'range' : 'other: ' + String(error && error.message);
      }
    };
    const results = [];
    for (const levels of [1, 8, 32, 64]) {
      const options = { env: {} };
      options.env.nest = (n) => {
        if (n < levels) transformSync(make(n + 1), { filename: '/app/n.js', ...options });
        return 'red';
      };
      results.push(outcome(() => transformSync(make(1), { filename: '/app/n.js', ...options })));
    }
    // A kept compiler function whose host request runs at the limit.
    let kept;
    transformSync(header + 'export const s = stylex.create({ a: { color: stylex.env.keep((v) => stylex.env.inner(v)) } });', {
      filename: '/app/k.js',
      env: { keep: (fn) => { kept = fn; return 'red'; }, inner: (v) => v },
    });
    let max = 0;
    const probe = (d) => { max = d; probe(d + 1); };
    try { probe(0); } catch {}
    const at = (d, run) => (d > 0 ? at(d - 1, run) : run());
    for (let back = 0; back < 400; back += 4) {
      results.push(outcome(() => at(max - back, () => kept('x'))));
    }
    process.stdout.write(JSON.stringify(results));
  `;
  for (const stackSize of [
    120, 136, 152, 168, 184, 200, 216, 232, 248, 264, 280,
  ]) {
    const child = spawnSync(
      process.execPath,
      [`--stack-size=${stackSize}`, '-e', script],
      { encoding: 'utf8', timeout: 60000 },
    );
    assert.equal(
      child.status,
      0,
      `--stack-size=${stackSize}: ${child.signal ?? ''} ${child.stderr.slice(-300)}`,
    );
    const results = JSON.parse(child.stdout);
    assert.equal(results[0], 'ok', `--stack-size=${stackSize} ordinary input`);
    for (const result of results) {
      assert.ok(
        result === 'ok' || result === 'range',
        `--stack-size=${stackSize}: ${result}`,
      );
    }
  }
});
