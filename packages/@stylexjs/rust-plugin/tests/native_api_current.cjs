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
const { appFile } = require('./support/babel_errors.cjs');

const header = "import * as stylex from '@stylexjs/stylex';\n";
const styles =
  "export const s = stylex.create({ a: { color: stylex.env.brand('x') } });";

test('an unconfigured env reports the same unframed style-value error as Babel', () => {
  const { runEngine } = require('./support/babel_errors.cjs');
  const filename = appFile('unconfigured-env.js');
  const capture = (engine, source, options) => {
    try {
      runEngine(engine, source, filename, options);
      assert.fail('the invalid style value must not compile');
    } catch (error) {
      return { kind: error.constructor.name, code: error.code, message: error.message };
    }
  };
  for (const value of ['stylex.env.brand', "stylex.env['brand']"]) {
    const source = `${header}export const s = stylex.create({ a: { color: ${value} } });`;
    for (const options of [{}, { env: undefined }, { env: {} }]) {
      const expected = capture('babel', source, options);
      assert.equal(expected.message, `${filename}: A style value can only contain an array, string or number.`);
      assert.equal(expected.message.includes('\n'), false, 'the full message has no code frame');
      for (const engine of ['direct', 'facade']) {
        assert.deepEqual(capture(engine, source, options), expected, `${engine}: ${value}`);
      }
    }
  }
  const configured = `${header}export const s = stylex.create({ a: { color: stylex.env.brand } });`;
  const expected = runEngine('babel', configured, filename, { env: { brand: 'red' } });
  for (const engine of ['direct', 'facade']) {
    assert.deepEqual(runEngine(engine, configured, filename, { env: { brand: 'red' } }).metadata.stylex, expected.metadata.stylex);
  }
});

test('unframed validation errors use Babel error constructors', () => {
  const { runEngine } = require('./support/babel_errors.cjs');
  const filename = appFile('validation-kinds.js');
  const capture = (engine, source, options) => {
    try {
      runEngine(engine, source, filename, options);
      assert.fail('invalid input must not compile');
    } catch (error) {
      return { kind: error.constructor.name, code: error.code, message: error.message };
    }
  };
  const cases = [
    ['duplicate container', `${header}export const s=stylex.create({a:{boxShadow:{'@container':{'@container':'red'}}}});`, {}],
    ['invalid nested pseudo', `${header}export const s=stylex.create({a:{b:{c:{color:'red'}}}});`, {}],
    ['property validation throw', `${header}export const s=stylex.create({a:{background:'red'}});`, { propertyValidationMode: 'throw' }],
    ['defineVars without resolution', `${header}export const v=stylex.defineVars({color:'red'});`, {}],
  ];
  for (const [label, source, options] of cases) {
    const expected = capture('babel', source, options);
    assert.equal(expected.kind, 'Error', label);
    assert.equal(expected.message.includes('\n'), false, label);
    for (const engine of ['direct', 'facade']) {
      assert.deepEqual(capture(engine, source, options), expected, `${engine}: ${label}`);
    }
  }
  const framed = `${header}export const s=stylex.create(1);`;
  const expected = capture('babel', framed, {});
  assert.equal(expected.kind, 'SyntaxError');
  for (const engine of ['direct', 'facade']) {
    assert.deepEqual(capture(engine, framed, {}), expected, `${engine}: framed diagnostic`);
  }
  // The native parser is SWC, so only the SyntaxError, code and frame are compared.
  const parsed = capture('direct', 'const = ;', {});
  assert.equal(parsed.kind, 'SyntaxError');
  assert.equal(parsed.code, 'BABEL_PARSE_ERROR');
  assert.ok(parsed.message.startsWith(`${filename}: Unexpected token \`=\`. Expected yield, an identifier, [ or { (1:6)`));
  assert.ok(parsed.message.includes('> 1 | const = ;'));
});

test('a named env import reports the same unframed style-value error as Babel when unconfigured', () => {
  const { runEngine } = require('./support/babel_errors.cjs');
  const filename = appFile('unconfigured-named-env.js');
  const capture = (engine, source, options) => {
    try {
      runEngine(engine, source, filename, options);
      assert.fail('the invalid style value must not compile');
    } catch (error) {
      return { kind: error.constructor.name, code: error.code, message: error.message };
    }
  };
  // An unconfigured named `env` reads undefined, refused as Babel does, unframed.
  const named = [
    ["import { create, env } from '@stylexjs/stylex';\n", 'env.brand'],
    ["import { create, env } from '@stylexjs/stylex';\n", "env['brand']"],
    ["import { create, env as runtime } from '@stylexjs/stylex';\n", 'runtime.brand'],
  ];
  for (const [imports, value] of named) {
    const source = `${imports}export const s = create({ a: { color: ${value} } });`;
    for (const options of [{}, { env: undefined }, { env: {} }]) {
      const expected = capture('babel', source, options);
      assert.equal(expected.message, `${filename}: A style value can only contain an array, string or number.`);
      assert.equal(expected.message.includes('\n'), false, 'the full message has no code frame');
      for (const engine of ['direct', 'facade']) {
        assert.deepEqual(capture(engine, source, options), expected, `${engine}: ${value}`);
      }
    }
  }
  // Configured, the named import reads the option as Babel does.
  for (const [imports, value] of named) {
    const source = `${imports}export const s = create({ a: { color: ${value} } });`;
    const expected = runEngine('babel', source, filename, { env: { brand: 'red' } });
    for (const engine of ['direct', 'facade']) {
      assert.deepEqual(
        runEngine(engine, source, filename, { env: { brand: 'red' } }).metadata.stylex,
        expected.metadata.stylex,
        `${engine}: configured ${value}`,
      );
    }
  }
});

test('a binding that shadows the named env import is never the configured env (native policy)', () => {
  const { runEngine } = require('./support/babel_errors.cjs');
  const filename = appFile('shadowed-named-env.js');
  const rules = (engine, source, options) =>
    runEngine(engine, source, filename, options).metadata.stylex.map((rule) => rule[1].ltr);
  // Deliberately unlike the reference, which matches `env` by spelling, not binding.
  for (const [imports, param] of [
    ["import { create, env } from '@stylexjs/stylex';\n", 'env'],
    ["import { create, env as runtime } from '@stylexjs/stylex';\n", 'runtime'],
  ]) {
    const source = `${imports}export const s = create({ a: (${param}) => ({ color: ${param}.brand }) });`;
    for (const options of [{ env: { brand: 'red' } }, {}]) {
      for (const engine of ['direct', 'facade']) {
        const emitted = rules(engine, source, options);
        assert.equal(emitted.some((rule) => rule.includes('color:red')), false, `${engine}: ${param} folded the env`);
        assert.equal(emitted.some((rule) => rule.includes('var(')), true, `${engine}: ${param} is dynamic`);
      }
    }
  }
});

test('a write through the named env import refuses rather than folding a stale value (native policy)', () => {
  const { runEngine } = require('./support/babel_errors.cjs');
  const filename = appFile('mutated-named-env.js');
  const source =
    "import { create, env } from '@stylexjs/stylex';\nenv.brand = 'blue';\nexport const s = create({ a: { color: env.brand } });";
  for (const engine of ['direct', 'facade']) {
    assert.throws(
      () => runEngine(engine, source, filename, { env: { brand: 'red' } }),
      /not a constant/,
      `${engine}: a mutated env import must refuse`,
    );
  }
});

test('exceptions thrown by user callbacks propagate as in Babel', () => {
  const { outcome, cases } = require('./support/babel_errors.cjs');
  for (const [label, scenario, filename] of cases()) {
    const expected = outcome('babel', scenario, filename);
    assert.equal(expected.completed, undefined, `Babel threw: ${label}`);
    assert.deepEqual(outcome('direct', scenario, filename), expected, label);
  }
  // Pin the reference itself for the common case.
  const { scenarios } = require('./support/babel_errors.cjs');
  const pinned = outcome(
    'babel',
    scenarios['an Error with extra fields'],
    appFile('a.js'),
  );
  assert.deepEqual(pinned.caught, {
    kind: 'Error',
    message: `${appFile('a.js')}: boom`,
    code: 'BABEL_TRANSFORM_ERROR',
    extra: 42,
  });
  assert.equal(pinned.same, true);
});

test('errors name the file exactly as Babel names it', () => {
  const { runEngine, filenames } = require('./support/babel_errors.cjs');
  const sources = {
    parse: 'const = ;',
    transform: `${header}export const s = stylex.create(1);`,
  };
  // The label Babel puts before the compiler's error message.
  const label = (engine, source, filename) => {
    try {
      runEngine(engine, source, filename, {});
      return { completed: true };
    } catch (error) {
      return {
        kind: error.constructor.name,
        code: error.code,
        label: error.message.slice(0, error.message.indexOf(': ')),
      };
    }
  };
  for (const [kind, source] of Object.entries(sources)) {
    for (const filename of filenames) {
      const expected = label('babel', source, filename);
      assert.equal(expected.completed, undefined, kind);
      assert.deepEqual(
        label('direct', source, filename),
        expected,
        `${kind}, filename ${String(filename)}`,
      );
    }
  }
  assert.equal(label('babel', sources.parse, undefined).label, 'unknown');
  assert.equal(
    label('babel', sources.transform, undefined).label,
    'unknown file',
  );
});

test('compiler functions kept past their transform throw as in Babel', () => {
  const { runEngine, describe } = require('./support/babel_errors.cjs');
  const owner = `${header}export const s = stylex.create({ a: { color: stylex.env.keep((v) => stylex.env.inner(v)) } });`;
  const later = `${header}export const s = stylex.create({ a: { color: stylex.env.use('x') } });`;
  // A kept function called with no transform, from a later transform, then again.
  const sequence = (engine) => {
    let thrown;
    let kept;
    runEngine(engine, owner, appFile('owner.js'), {
      env: {
        keep: (fn) => {
          kept = fn;
          return 'red';
        },
        inner: () => {
          throw thrown;
        },
      },
    });
    const steps = [];
    for (const phase of ['outside', 'later', 'outside']) {
      thrown = new Error(`kept ${phase}`);
      try {
        if (phase === 'outside') kept('x');
        else
          runEngine(engine, later, appFile('later.js'), { env: { use: kept } });
        steps.push({ phase, completed: true });
      } catch (error) {
        steps.push({ phase, same: error === thrown, caught: describe(error) });
      }
    }
    return steps;
  };
  const expected = sequence('babel');
  assert.deepEqual(
    expected.map((step) => step.caught?.message),
    ['kept outside', `${appFile('later.js')}: kept later`, 'kept outside'],
  );
  assert.deepEqual(sequence('direct'), expected);
});

test('a transform nested in a callback wraps its exception as nested Babel does', () => {
  const { outcome, runEngine } = require('./support/babel_errors.cjs');
  const scenario = (engine) => () => {
    const thrown = new Error('inner boom');
    return {
      code: `${header}export const s = stylex.create({ a: { color: stylex.env.nest('x') } });`,
      options: {
        env: {
          nest: () =>
            runEngine(engine, header + styles, appFile('inner.js'), {
              env: {
                brand: () => {
                  throw thrown;
                },
              },
            }),
        },
      },
      thrown,
    };
  };
  const expected = outcome('babel', scenario('babel'), appFile('outer.js'));
  assert.equal(
    expected.caught.message,
    `${appFile('outer.js')}: ${appFile('inner.js')}: inner boom`,
  );
  assert.deepEqual(
    outcome('direct', scenario('direct'), appFile('outer.js')),
    expected,
  );
});

test('debugFilePath names debug labels and aborts exactly as in Babel', () => {
  const { runEngine, describe } = require('./support/babel_errors.cjs');
  const code = `${header}export const s = stylex.create({
  a: { color: 'red' },
  b: { color: 'blue' },
});`;
  const filename = appFile('pkg/a.js');
  // Each namer is made fresh per engine and records the files it was given.
  const namers = {
    stateful: () => {
      let n = 0;
      return () => `label${++n}`;
    },
    named: () => () => 'pkg:a.js',
    empty: () => () => '',
    throwing: () => {
      const thrown = new Error('namer failed');
      const namer = () => {
        throw thrown;
      };
      namer.thrown = thrown;
      return namer;
    },
  };
  const run = (engine, kind) => {
    const namer = namers[kind]();
    const calls = [];
    try {
      const { code: output } = runEngine(engine, code, filename, {
        debug: true,
        enableDebugDataProp: true,
        debugFilePath: (file) => {
          calls.push(file);
          return namer(file);
        },
      });
      // Quote style is the printer's; the label text is the contract.
      const labels = [
        ...output.matchAll(/\$\$css: ?("[^"]*"|'[^']*'|true)/g),
      ].map((match) => match[1].replace(/^'(.*)'$/, '"$1"'));
      return { calls, labels };
    } catch (error) {
      return {
        calls,
        same: error === namer.thrown,
        caught: describe(error),
      };
    }
  };
  const expected = Object.fromEntries(
    Object.keys(namers).map((kind) => [kind, run('babel', kind)]),
  );
  assert.deepEqual(expected.stateful, {
    calls: [filename, filename],
    labels: ['"label1:3"', '"label2:4"'],
  });
  assert.deepEqual(expected.empty.labels, ['true', 'true']);
  // A throwing namer aborts the transform at its first call.
  assert.deepEqual(expected.throwing.calls, [filename]);
  assert.equal(expected.throwing.same, true);
  assert.equal(expected.throwing.caught.message, `${filename}: namer failed`);
  for (const engine of ['direct', 'facade']) {
    for (const kind of Object.keys(namers)) {
      assert.deepEqual(run(engine, kind), expected[kind], `${engine} ${kind}`);
    }
  }
});

test('failures inside nested transforms print nothing to stderr', () => {
  const { spawnSync } = require('node:child_process');
  // One process per case, so stderr is exactly the compiler's.
  const script = `
const { transformSync } = require(${JSON.stringify(path.resolve(__dirname, '..'))});
const header = ${JSON.stringify(header)};
const code = header + "export const s = stylex.create({ a: { color: stylex.env.f('x') } });";
const boom = () => { throw new Error('boom'); };
const inner = (f) => () => transformSync(code, { filename: ${JSON.stringify(appFile('inner.js'))}, env: { f } });
const cases = {
  'user exception after a nested success': { f: () => { inner(() => 'red')(); throw new Error('outer'); } },
  'user exception from a nested transform': { f: inner(boom) },
  'StyleX error after a nested success': { f: () => { inner(() => 'red')(); return new Map(); } },
};
const outcome = {};
for (const [name, env] of Object.entries(cases)) {
  try { transformSync(code, { filename: ${JSON.stringify(appFile('outer.js'))}, env }); outcome[name] = 'completed'; }
  catch (error) { outcome[name] = String(error.message).split('\\n')[0]; }
}
process.stdout.write(JSON.stringify(outcome));
`;
  const child = spawnSync(process.execPath, ['-e', script], {
    encoding: 'utf8',
  });
  assert.equal(child.status, 0, child.stderr);
  const outcome = JSON.parse(child.stdout);
  assert.equal(
    outcome['user exception after a nested success'],
    `${appFile('outer.js')}: outer`,
  );
  assert.equal(
    outcome['user exception from a nested transform'],
    `${appFile('outer.js')}: ${appFile('inner.js')}: boom`,
  );
  assert.ok(
    outcome['StyleX error after a nested success'].startsWith(
      `${appFile('outer.js')}: `,
    ),
  );
  assert.equal(child.stderr, '');
});

test('callbacks receive their owning object (the transform env) as `this`', () => {
  let receiver;
  const env = {
    brand() {
      receiver = this;
      return 'red';
    },
  };
  transformSync(header + styles, { filename: '/app/a.js', env });
  // As in Babel, `env` is a frozen shallow copy made for the transform.
  assert.notEqual(receiver, env);
  assert.ok(Object.isFrozen(receiver));
  assert.equal(receiver.brand, env.brand);
});

test('option data cannot spell host callback tags', () => {
  for (const key of [
    '__stylexCallback',
    '__stylexUndefined',
    '__stylexNumber',
    '__stylexHandle',
  ]) {
    assert.throws(
      () =>
        transformSync(header + styles, {
          filename: '/app/a.js',
          aliases: { brand: { [key]: '@debugFile' } },
        }),
      new RegExp(`cannot contain the key ${key}`),
    );
    // `env` is read as live JavaScript, never decoded: such keys are data.
    assert.equal(
      transformSync(
        `${header}export const s = stylex.create({ a: { color: stylex.env.brand['${key}'] } });`,
        { filename: '/app/a.js', env: { brand: { [key]: 'red' } } },
      ).metadata.stylex[0][1].ltr.includes('color:red'),
      true,
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

test('an input source map composes the output map back to the original file', () => {
  const babel = require('@babel/core');
  const original = `${header}\n\nexport const s = stylex.create({ a: { color: 'red' } });\n`;
  // A Babel-produced map (its `file` and `sourceRoot` are undefined).
  const first = babel.transformSync(original, {
    filename: '/app/original.js',
    sourceFileName: 'original.js',
    babelrc: false,
    configFile: false,
    sourceMaps: true,
    retainLines: false,
    compact: false,
  });
  assert.equal(first.map.file, undefined);
  const composed = transformSync(first.code, {
    filename: '/app/compiled.js',
    sourceMaps: true,
    inputSourceMap: first.map,
  });
  assert.deepEqual(composed.map.sources, ['original.js']);
  assert.match(composed.map.mappings, /\S/);
  const plain = transformSync(first.code, {
    filename: '/app/compiled.js',
    sourceMaps: true,
  });
  assert.ok(!plain.map.sources.includes('original.js'));
});

test('an invalid input source map fails catchably and later transforms work', () => {
  for (const inputSourceMap of [
    { version: 3, sources: 'not-a-list', mappings: 42 },
    { version: 3, sources: ['a.js'], names: [], mappings: '!!!!' },
  ]) {
    let outcome;
    try {
      transformSync(header + styles, {
        filename: '/app/a.js',
        sourceMaps: true,
        inputSourceMap,
        env: { brand: () => 'red' },
      });
      outcome = 'ok';
    } catch (error) {
      outcome = error instanceof Error ? 'error' : 'non-error';
    }
    assert.notEqual(outcome, 'non-error');
  }
  const cyclic = { version: 3 };
  cyclic.self = cyclic;
  assert.throws(() =>
    transformSync(header + styles, {
      filename: '/app/a.js',
      sourceMaps: true,
      inputSourceMap: cyclic,
    }),
  );
  assert.ok(
    transformSync(header + styles, {
      filename: '/app/a.js',
      env: { brand: () => 'red' },
    }).metadata.stylex.length > 0,
  );
});

test('compiler errors name the file as Babel resolves the filename', () => {
  const babel = require('@babel/core');
  const babelPlugin = require('@stylexjs/babel-plugin');
  const source = header + 'const s = stylex.create({ a: { color: missing } });';
  const firstLine = (run) => {
    try {
      run();
    } catch (error) {
      return error.message.split('\n')[0];
    }
    return null;
  };
  // Babel resolves an empty filename too; only an absent one is "unknown file".
  for (const options of [{ filename: '' }, {}, { filename: 'rel/a.js' }]) {
    const expected = firstLine(() =>
      babel.transformSync(source, {
        ...options,
        babelrc: false,
        configFile: false,
        plugins: [[babelPlugin, {}]],
      }),
    );
    assert.notEqual(expected, null);
    assert.equal(
      firstLine(() => transformSync(source, options)),
      expected,
      JSON.stringify(options),
    );
  }
  assert.equal(
    firstLine(() => transformSync(source, { filename: '' })),
    `${path.resolve('')}: Referenced constant is not defined.`,
  );
});

// A theme package on disk, compiled and imported as an application would.
function themeFixture() {
  const fs = require('node:fs');
  const os = require('node:os');
  // Real path: import-meta-resolve follows symlinks such as macOS's /var.
  const root = fs.realpathSync(
    fs.mkdtempSync(path.join(os.tmpdir(), 'stylex-resolve-')),
  );
  const write = (file, text) => {
    fs.mkdirSync(path.dirname(path.join(root, file)), { recursive: true });
    fs.writeFileSync(path.join(root, file), text);
  };
  write('package.json', '{"name":"app"}');
  write(
    'theme/tokens.stylex.js',
    `${header}export const colors = stylex.defineVars({ primary: 'red' });\n` +
      "export const sizes = stylex.defineConsts({ sm: '4px' });\n",
  );
  write(
    'theme/more.stylex.const.js',
    `${header}export const gap = stylex.defineConsts({ lg: '20px' });\n`,
  );
  return {
    root,
    file: (name) => path.join(root, name),
    remove: () => fs.rmSync(root, { recursive: true, force: true }),
  };
}

test('rewriteAliases preserves import lines, trailing comments and bare-import spacing', () => {
  const { runEngine } = require('./support/babel_errors.cjs');
  const fixture = themeFixture();
  try {
    const options = {
      rewriteAliases: true,
      aliases: { '~t/*': [fixture.file('theme/*')] },
      unstable_moduleResolution: { type: 'commonJS', rootDir: fixture.root },
    };
    const filename = fixture.file('Comp.js');
    for (const line of [
      "import { colors } from '~t/tokens.stylex'; // keep",
      "import '~t/tokens.stylex'; // bare",
      'import "~t/tokens.stylex"; /* bare block */',
    ]) {
      const source = `${header}${line}\nexport const value = 1;\n`;
      const expected = source.replace(
        /(['"])~t\/tokens\.stylex\1/,
        '"./theme/tokens.stylex"',
      );
      const direct = runEngine('direct', source, filename, options);
      assert.equal(
        direct.code,
        expected,
        `specifier-only replacement: ${line}`,
      );
      const oracle = runEngine('babel', source, filename, options);
      const facade = runEngine('facade', source, filename, options);
      assert.equal(
        facade.code,
        oracle.code,
        `facade preserves comment attachment: ${line}`,
      );
      assert.ok(
        direct.code.includes("stylex';\nimport"),
        'bare import is not reinserted with blank lines',
      );
    }
    const unusual = `${header}import  { colors as theme }  from  '~t/tokens.stylex'  ; // spaced\nexport const value=1;\n`;
    assert.equal(
      runEngine('direct', unusual, filename, options).code,
      unusual.replace("'~t/tokens.stylex'", '"./theme/tokens.stylex"'),
    );
    const unchanged = `${header}import { colors } from './theme/tokens.stylex'; // unchanged\n`;
    assert.equal(
      runEngine('direct', unchanged, filename, options).code,
      unchanged,
    );
    assert.equal(
      runEngine('babel', unchanged, filename, options).code,
      unchanged.trimEnd(),
    );
  } finally {
    fixture.remove();
  }
});

test('rewriteAliases resolves a relative import through a symlinked directory as Babel does', () => {
  const fs = require('node:fs');
  const os = require('node:os');
  const { runEngine } = require('./support/babel_errors.cjs');
  const fixture = themeFixture();
  const link = path.join(
    fs.realpathSync(os.tmpdir()),
    `stylex-resolve-link-${process.pid}-${path.basename(fixture.root)}`,
  );
  fs.symlinkSync(fixture.root, link, 'junction');
  try {
    const options = {
      rewriteAliases: true,
      aliases: { '~t/*': [path.join(link, 'theme/*')] },
      unstable_moduleResolution: { type: 'commonJS', rootDir: link },
    };
    const filename = path.join(link, 'Comp.js');
    const source = `${header}import { colors } from './theme/tokens.stylex'; // linked\n`;
    const importLine = (engine) =>
      runEngine(engine, source, filename, options)
        .code.split('\n')
        .find((line) => line.includes('tokens.stylex'));
    const oracle = importLine('babel');
    assert.equal(importLine('direct'), oracle);
    assert.equal(importLine('facade'), oracle);
  } finally {
    fs.rmSync(link, { recursive: true, force: true });
    fixture.remove();
  }
});

test('option diagnostics are reported for every file, as Babel does', () => {
  const { runEngine } = require('./support/babel_errors.cjs');
  const files = [
    'export const value = 1;\n',
    `${header}export const s = stylex.create({ a: { color: 'red' } });\n`,
    'export const again = 2;\n',
  ];
  const capture = (engine, options) => {
    const calls = [];
    const { error, warn } = console;
    console.error = (...args) => calls.push(['error', ...args]);
    console.warn = (...args) => calls.push(['warn', ...args]);
    try {
      // One options object for every file, as a bundler passes it.
      for (const code of files)
        runEngine(engine, code, appFile('options.js'), options);
    } finally {
      console.error = error;
      console.warn = warn;
    }
    return calls;
  };
  for (const options of [
    {},
    { enableFontSizePxToRem: 'true' },
    { dev: 1, classNamePrefix: 3 },
    { styleResolution: 'bogus', rewriteAliases: 'yes' },
  ]) {
    const expected = capture('babel', options);
    const label = JSON.stringify(options);
    if (Object.keys(options).length > 0)
      assert.equal(expected.length > files.length - 1, true, label);
    assert.deepEqual(capture('facade', options), expected, label);
  }
});

test('rewriteAliases rewrites theme imports exactly as Babel does', () => {
  const { runEngine } = require('./support/babel_errors.cjs');
  const fixture = themeFixture();
  try {
    const theme = fixture.file('theme');
    const sources = [
      "import { colors } from '~t/tokens.stylex';",
      "import { colors } from '~t/tokens.stylex.js';",
      "import { colors } from '@theme/tokens.stylex';",
      "import { colors } from '@tokens.stylex';",
      "import { colors } from '../theme/tokens.stylex';",
      "import { colors } from '../theme/../theme/tokens.stylex';",
      "import { colors } from '~t/missing.stylex';",
      "import { colors } from '/ROOT/theme/tokens.stylex';",
      "import { gap } from '~t/more.stylex.const';",
      "import '~t/tokens.stylex';",
      "export { colors } from '~t/tokens.stylex';",
    ];
    const resolutions = [
      { type: 'commonJS', rootDir: fixture.root },
      { type: 'haste' },
      {
        type: 'custom',
        filePathResolver: (source, from) =>
          source.startsWith('.')
            ? `${path.resolve(path.dirname(from), source)}.js`
            : undefined,
        getCanonicalFilePath: (file) => `custom:${path.basename(file)}`,
      },
    ];
    const specifiers = (code) =>
      [...code.matchAll(/^(?:import|export)\b.*?(["'][^"']*["']);?$/gm)].map(
        (match) => match[1].replace(/^'(.*)'$/, '"$1"'),
      );
    const run = (engine, source, filename, options) => {
      try {
        return specifiers(runEngine(engine, source, filename, options).code);
      } catch (error) {
        return { error: String(error.message).split('\n')[0] };
      }
    };
    let rewritten = 0;
    for (const resolution of resolutions) {
      for (const source of sources) {
        for (const [filename, rewrite] of [
          [fixture.file('src/Comp.js'), true],
          [fixture.file('src/deep/er/Comp.tsx'), true],
          [fixture.file('src/Comp.js'), false],
          [undefined, true],
        ]) {
          const options = {
            unstable_moduleResolution: resolution,
            aliases: {
              '~t/*': [path.join(theme, '*')],
              '@theme/*': path.join(theme, '*'),
              '@tokens.stylex': [path.join(theme, 'tokens.stylex.js')],
              '/ROOT/*': ['/ROOT/*'],
            },
            rewriteAliases: rewrite,
          };
          // With and without StyleX: Babel rewrites theme imports either way.
          for (const prefix of [header, '']) {
            const code = `${prefix}${source}\n`;
            const expected = run('babel', code, filename, options);
            const label = `${resolution.type} ${filename ?? 'unnamed'} ${rewrite} ${prefix === '' ? 'no-stylex ' : ''}${source}`;
            assert.deepEqual(
              run('direct', code, filename, options),
              expected,
              label,
            );
            assert.deepEqual(
              run('facade', code, filename, options),
              expected,
              label,
            );
            if (
              !expected.error &&
              expected.some((s) => /"\.\.?\//.test(s))
            ) {
              rewritten++;
            }
          }
        }
      }
    }
    // The oracle really rewrote: aliases became relative specifiers.
    assert.ok(rewritten > 20, `only ${rewritten} rewrites`);
    const repro = run(
      'babel',
      `${header}import { colors } from '~t/tokens.stylex';\nexport const s = stylex.create({ a: { color: colors.primary } });\n`,
      fixture.file('Comp.js'),
      {
        aliases: { '~t/*': [path.join(theme, '*')] },
        rewriteAliases: true,
        unstable_moduleResolution: { type: 'commonJS', rootDir: fixture.root },
      },
    );
    assert.deepEqual(repro, ['"@stylexjs/stylex"', '"./theme/tokens.stylex"']);
  } finally {
    fixture.remove();
  }
});

test('custom resolution names theme variables with the author canonical paths', () => {
  const { runEngine } = require('./support/babel_errors.cjs');
  const fixture = themeFixture();
  try {
    const calls = [];
    const options = {
      unstable_moduleResolution: {
        type: 'custom',
        filePathResolver: (source, from) =>
          source.startsWith('.')
            ? `${path.resolve(path.dirname(from), source)}.js`
            : undefined,
        getCanonicalFilePath: (file) => {
          calls.push(path.relative(fixture.root, file));
          return `custom:${path.basename(file)}`;
        },
      },
    };
    const fs = require('node:fs');
    const files = [
      [fixture.file('theme/tokens.stylex.js'), null],
      [fixture.file('theme/more.stylex.const.js'), null],
      [
        fixture.file('theme/Comp.js'),
        `${header}import { colors, sizes } from './tokens.stylex';\nimport { gap } from './more.stylex.const';\n` +
          'export const s = stylex.create({ a: { color: colors.primary, padding: sizes.sm, margin: gap.lg } });\n',
      ],
    ];
    const compile = (engine) => {
      calls.length = 0;
      const metadata = files.map(([file, source]) => {
        const code = source ?? fs.readFileSync(file, 'utf8');
        return runEngine(engine, code, file, options).metadata.stylex;
      });
      return { metadata, calls: [...new Set(calls)].sort() };
    };
    const expected = compile('babel');
    const rules = expected.metadata.flat();
    const css = rules.map((rule) => rule[1].ltr).join(' ');
    // Sanity-check the oracle: every variable the consumer reads is defined.
    const defined = [
      ...(css.match(/--x[a-z0-9]+(?=:)/g) ?? []),
      ...rules
        .filter((rule) => rule[1].constKey)
        .map((rule) => `--${rule[1].constKey}`),
    ];
    const used = [...css.matchAll(/var\((--x[a-z0-9]+)\)/g)].map((m) => m[1]);
    assert.equal(used.length, 3);
    assert.deepEqual(
      used.filter((name) => !defined.includes(name)),
      [],
    );
    assert.ok(expected.calls.length > 0);
    assert.deepEqual(compile('direct'), expected);
    assert.deepEqual(compile('facade'), expected);
  } finally {
    fixture.remove();
  }
});

test('a throwing getCanonicalFilePath reaches the caller as in Babel', () => {
  const { runEngine, describe } = require('./support/babel_errors.cjs');
  const fixture = themeFixture();
  try {
    const fs = require('node:fs');
    const file = fixture.file('theme/tokens.stylex.js');
    const run = (engine) => {
      const failure = new Error('canonical failed');
      const options = {
        unstable_moduleResolution: {
          type: 'custom',
          filePathResolver: () => undefined,
          getCanonicalFilePath: () => {
            throw failure;
          },
        },
      };
      try {
        runEngine(engine, fs.readFileSync(file, 'utf8'), file, options);
        return { completed: true };
      } catch (error) {
        return { same: error === failure, caught: describe(error) };
      }
    };
    const expected = run('babel');
    assert.equal(expected.same, true);
    assert.deepEqual(run('direct'), expected);
    assert.deepEqual(run('facade'), expected);
  } finally {
    fixture.remove();
  }
});

test('rewritten theme specifiers take the form Babel writes on every platform', () => {
  const { rewrittenImportPath } = require('../host');
  // Transcribed from the Babel plugin's getRelativePath (state-manager.js).
  const babelForm = (from, resolved, pathImpl) => {
    const relative = pathImpl
      .relative(pathImpl.parse(from).dir, resolved)
      .split(pathImpl.sep)
      .join(path.posix.sep);
    let result = relative.startsWith('.') ? relative : './' + relative;
    const extension = ['.js', '.ts', '.tsx', '.jsx', '.mjs', '.cjs'].find(
      (ext) => result.endsWith(ext),
    );
    if (extension != null) result = result.slice(0, -extension.length);
    return result;
  };
  const cases = [
    [
      path.win32,
      'C:\\app\\src\\a.js',
      'C:\\app\\src\\theme\\tokens.stylex.js',
      './theme/tokens.stylex',
    ],
    [
      path.win32,
      'C:\\app\\src\\deep\\a.tsx',
      'C:\\app\\theme\\tokens.stylex.ts',
      '../../theme/tokens.stylex',
    ],
    [
      path.win32,
      'C:\\app\\src\\a.js',
      'C:\\app\\src\\tokens.stylex.js',
      './tokens.stylex',
    ],
    [
      path.win32,
      'C:\\app\\src\\a.js',
      'C:\\app\\src\\tokens.stylex',
      './tokens.stylex',
    ],
    // Another drive has no relative path: Babel still prefixes `./`.
    [
      path.win32,
      'C:\\app\\src\\a.js',
      'D:\\shared\\tokens.stylex.js',
      './D:/shared/tokens.stylex',
    ],
    [
      path.posix,
      '/app/src/a.js',
      '/app/src/theme/tokens.stylex.js',
      './theme/tokens.stylex',
    ],
    [
      path.posix,
      '/app/src/deep/a.js',
      '/app/theme/tokens.stylex.mjs',
      '../../theme/tokens.stylex',
    ],
    // A leading dot counts as relative for Babel's prefix check.
    [
      path.posix,
      '/app/src/a.js',
      '/app/src/.hidden/tokens.stylex.js',
      '.hidden/tokens.stylex',
    ],
  ];
  for (const [pathImpl, from, resolved, expected] of cases) {
    assert.equal(babelForm(from, resolved, pathImpl), expected, from);
    assert.equal(rewrittenImportPath(from, resolved, pathImpl), expected, from);
  }
});

test('experimental_crossFileParsing imports compile as commonJS ones do', () => {
  const fs = require('node:fs');
  const os = require('node:os');
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'stylex-cross-file-'));
  fs.mkdirSync(path.join(root, 'src'));
  fs.writeFileSync(
    path.join(root, 'src/tokens.stylex.js'),
    header + "export const vars = stylex.defineVars({ color: 'red' });\n",
  );
  const filename = path.join(root, 'src/app.js');
  const source =
    header +
    "import { vars } from './tokens.stylex';\n" +
    'export const s = stylex.create({ a: { color: vars.color } });\n';
  const compile = (type) =>
    transformSync(source, {
      filename,
      unstable_moduleResolution: { type, rootDir: root },
    });
  const crossFile = compile('experimental_crossFileParsing');
  const commonJS = compile('commonJS');
  // Babel's cross-file read rejects every file (an empty `errors` array is truthy).
  assert.deepEqual(crossFile.metadata, commonJS.metadata);
  assert.equal(crossFile.code, commonJS.code);
  assert.match(JSON.stringify(crossFile.metadata), /var\(--x/);
  fs.rmSync(root, { recursive: true, force: true });
});

test('commonJS theme names match Babel for a package without a name', () => {
  const fs = require('node:fs');
  const os = require('node:os');
  const babel = require('@babel/core');
  const babelPlugin = require('@stylexjs/babel-plugin');
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'stylex-nameless-'));
  // A manifest without "name": Babel names its files `undefined:<path>`.
  fs.writeFileSync(path.join(root, 'package.json'), '{}');
  fs.mkdirSync(path.join(root, 'src/theme'), { recursive: true });
  const tokens = path.join(root, 'src/theme/tokens.stylex.js');
  fs.writeFileSync(
    tokens,
    header + "export const vars = stylex.defineVars({ color: 'red' });\n",
  );
  const options = {
    unstable_moduleResolution: { type: 'commonJS' },
    aliases: { '@theme/*': [path.join(root, 'src/theme/*')] },
  };
  const consumer = (specifier) =>
    header +
    `import { vars } from '${specifier}';\n` +
    'export const s = stylex.create({ a: { color: vars.color } });\n';
  const programs = [
    [tokens, fs.readFileSync(tokens, 'utf8')],
    [path.join(root, 'src/app.js'), consumer('./theme/tokens.stylex')],
    [path.join(root, 'src/aliased.js'), consumer('@theme/tokens.stylex')],
  ];
  const css = [];
  for (const [filename, source] of programs) {
    const expected = babel.transformSync(source, {
      filename,
      babelrc: false,
      configFile: false,
      plugins: [[babelPlugin, options]],
    });
    const actual = transformSync(source, { ...options, filename });
    assert.deepEqual(actual.metadata, expected.metadata, filename);
    // Compare Babel's reprint: names, not layout, are what this test is about.
    const reprint = (code) =>
      babel.transformSync(code, { babelrc: false, configFile: false }).code;
    assert.equal(reprint(actual.code), reprint(expected.code), filename);
    css.push(JSON.stringify(actual.metadata));
  }
  // The producer's variable is the one both consumers reference.
  const variable = /--x[a-z0-9]+/.exec(css[0])[0];
  assert.ok(css[1].includes(`var(${variable})`), css[1]);
  assert.ok(css[2].includes(`var(${variable})`), css[2]);
  fs.rmSync(root, { recursive: true, force: true });
});

test('theme files are named as Babel names them on every platform', () => {
  const { canonicalFilePath } = require('../host');
  // Transcribed from the Babel plugin's getCanonicalFilePath (state-manager.js).
  const babelForm = (file, rootDir, pathImpl, files) => {
    const packageOf = (filepath) => {
      const folder = pathImpl.dirname(filepath);
      if (files.exists(pathImpl.join(folder, 'package.json'))) {
        try {
          const { name } = JSON.parse(
            files.read(pathImpl.join(folder, 'package.json')),
          );
          return [name, folder];
        } catch {
          return null;
        }
      }
      if (folder === pathImpl.parse(folder).root || folder === '') return null;
      return packageOf(folder);
    };
    const found = packageOf(file);
    if (found == null) {
      if (rootDir != null) return pathImpl.relative(rootDir, file);
      return `_unknown_path_:${pathImpl.relative(pathImpl.dirname(file), file)}`;
    }
    return `${found[0]}:${pathImpl.relative(found[1], file)}`;
  };
  const filesOf = (manifests) => ({
    exists: (manifest) => Object.hasOwn(manifests, manifest),
    read: (manifest) => manifests[manifest],
  });
  const win = path.win32;
  const posix = path.posix;
  const tokens = 'C:\\app\\src\\theme\\tokens.stylex.js';
  const cases = [
    [
      win,
      tokens,
      null,
      { 'C:\\app\\package.json': '{"name":"app"}' },
      'app:src\\theme\\tokens.stylex.js',
    ],
    [
      win,
      tokens,
      null,
      { 'C:\\app\\src\\package.json': '{"name":"@scope/ui"}' },
      '@scope/ui:theme\\tokens.stylex.js',
    ],
    [
      win,
      tokens,
      null,
      { 'C:\\app\\package.json': '{"private":true}' },
      'undefined:src\\theme\\tokens.stylex.js',
    ],
    [
      win,
      tokens,
      null,
      { 'C:\\package.json': '{"name":"drive"}' },
      'drive:app\\src\\theme\\tokens.stylex.js',
    ],
    [win, tokens, 'C:\\app', {}, 'src\\theme\\tokens.stylex.js'],
    [win, tokens, null, {}, '_unknown_path_:tokens.stylex.js'],
    [
      win,
      tokens,
      'C:\\app',
      { 'C:\\app\\src\\package.json': '{not json' },
      'src\\theme\\tokens.stylex.js',
    ],
    [
      posix,
      '/app/src/theme/tokens.stylex.js',
      null,
      { '/app/package.json': '{"name":"app"}' },
      'app:src/theme/tokens.stylex.js',
    ],
    [
      posix,
      '/app/src/theme/tokens.stylex.js',
      null,
      { '/package.json': '{"name":"root"}' },
      'root:app/src/theme/tokens.stylex.js',
    ],
    [
      posix,
      '/app/src/theme/tokens.stylex.js',
      '/app',
      {},
      'src/theme/tokens.stylex.js',
    ],
    [
      posix,
      '/app/src/theme/tokens.stylex.js',
      null,
      {},
      '_unknown_path_:tokens.stylex.js',
    ],
  ];
  const reported = console.error;
  console.error = () => {};
  try {
    for (const [pathImpl, file, rootDir, manifests, expected] of cases) {
      const files = filesOf(manifests);
      const label = `${file} ${JSON.stringify(manifests)} ${rootDir}`;
      assert.equal(babelForm(file, rootDir, pathImpl, files), expected, label);
      assert.equal(
        canonicalFilePath(file, rootDir, pathImpl, files),
        expected,
        label,
      );
    }
  } finally {
    console.error = reported;
  }
});

// Babel refuses at the call, never reading the receiver the write tracker taints.
// Not asserted: Babel also folds `[3, 1].sort()[0]`, which is the A10 follow-up.
test('a mutating array method on a named array reports Babel\'s call refusal', () => {
  const { runEngine } = require('./support/babel_errors.cjs');
  const filename = appFile('named-array-mutator.js');
  const capture = (engine, source) => {
    try {
      runEngine(engine, source, filename, {});
      assert.fail(`${engine}: a named array mutator must not compile`);
    } catch (error) {
      return {
        kind: error.constructor.name,
        code: error.code,
        message: error.message,
      };
    }
  };
  const cases = [
    'const a = [3, 1];\nexport const s = stylex.create({ x: { color: a.sort()[0] } });',
    'const a = [3, 1];\nexport const s = stylex.create({ x: { color: a.reverse()[0] } });',
    'const a = ["red"];\nexport const s = stylex.create({ x: { color: a.pop() } });',
  ];
  for (const body of cases) {
    const source = `${header}${body}`;
    const expected = capture('babel', source);
    assert.equal(expected.kind, 'SyntaxError', body);
    assert.ok(
      expected.message.includes('Unsupported expression: CallExpression'),
      `babel must label the call, got: ${expected.message}`,
    );
    // `deepEqual` on the whole message pins Babel's code frame too.
    for (const engine of ['direct', 'facade']) {
      assert.deepEqual(capture(engine, source), expected, `${engine}: ${body}`);
    }
  }
});

test('debug labels inside, outside and without a root, and in a package, equal Babel', () => {
  const fs = require('node:fs');
  const os = require('node:os');
  const { runEngine } = require('./support/babel_errors.cjs');
  const base = fs.realpathSync(
    fs.mkdtempSync(path.join(os.tmpdir(), 'stylex-debug-label-')),
  );
  const labels = (code) =>
    [...code.matchAll(/\$\$css: ?("(?:[^"\\]|\\.)*")/g)].map((match) =>
      JSON.parse(match[1]),
    );
  const source = `${header}export const styles = stylex.create({ root: { color: 'red' } });\n`;
  try {
    const rootDir = path.join(base, 'app');
    const cases = [
      ['inside the root', path.join(rootDir, 'src/a.js'), { rootDir }],
      ['outside the root', path.join(base, 'other/b.js'), { rootDir }],
      ['without a root', path.join(base, 'x/y/c.js'), {}],
      [
        'in an installed package',
        path.join(rootDir, 'node_modules/npm-package/dist/Foo.js'),
        { rootDir },
      ],
      ['in a package without a name', path.join(base, 'nameless/lib/Z.js'), { rootDir }],
      ['in a named package', path.join(base, 'named/lib/Z.js'), {}],
    ];
    fs.mkdirSync(path.join(base, 'nameless'), { recursive: true });
    fs.writeFileSync(path.join(base, 'nameless/package.json'), '{"private":true}');
    fs.mkdirSync(path.join(base, 'named'), { recursive: true });
    fs.writeFileSync(path.join(base, 'named/package.json'), '{"name":"named"}');
    for (const [name, filename, resolution] of cases) {
      const options = {
        dev: true,
        unstable_moduleResolution: { type: 'commonJS', ...resolution },
      };
      const expected = labels(runEngine('babel', source, filename, options).code);
      assert.ok(expected.length > 0, `Babel writes a label ${name}`);
      assert.deepEqual(
        labels(runEngine('direct', source, filename, options).code),
        expected,
        `direct label ${name}`,
      );
      assert.deepEqual(
        labels(runEngine('facade', source, filename, options).code),
        expected,
        `facade label ${name}`,
      );
    }
  } finally {
    fs.rmSync(base, { recursive: true, force: true });
  }
});

test('an input source map leaves the debug label as Babel writes it', () => {
  const babel = require('@babel/core');
  const filename = appFile('src/Card.js');
  const source = `${header}export const styles = stylex.create({\n  root: { color: 'red' },\n});\n`;
  // Babel's plugin ignores this map: @babel/core 7 defines no hub.file.codeMap.
  const inputSourceMap = {
    version: 3,
    sources: ['Original.js'],
    names: [],
    mappings: 'AAyCA;AACA;AACA;AACA',
  };
  const options = { dev: true, unstable_moduleResolution: { type: 'commonJS' } };
  const labels = (code) =>
    [...code.matchAll(/\$\$css: ?"([^"]*)"/g)].map((match) => match[1]);
  const viaBabel = (plugin) =>
    babel.transformSync(source, {
      filename,
      babelrc: false,
      configFile: false,
      inputSourceMap,
      plugins: [[plugin, options]],
    }).code;
  const expected = labels(viaBabel(require('@stylexjs/babel-plugin')));
  assert.equal(expected.length, 1);
  assert.ok(expected[0].endsWith(':3'), expected[0]);
  assert.deepEqual(labels(viaBabel(require(path.join(__dirname, '..', 'babel')))), expected);
  assert.deepEqual(
    labels(transformSync(source, { ...options, filename, inputSourceMap }).code),
    expected,
  );
});
