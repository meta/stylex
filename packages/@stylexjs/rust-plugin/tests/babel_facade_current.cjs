/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
'use strict';
// Raw current-contract regression tests through the actual native Babel facade.
// No original StyleX compiler, mock transform, snapshot normalization or skip.
const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');
const babel = require('@babel/core');
const plugin = require('../babel');
const filename = path.resolve(__dirname, 'Fixture.js');
function transform(
  source,
  options = {},
  extraPlugins = [],
  parserPlugins = [],
) {
  return babel.transformSync(source, {
    filename,
    babelrc: false,
    configFile: false,
    parserOpts: { plugins: parserPlugins },
    plugins: [...extraPlugins, [plugin, options]],
  });
}

test('native compilation precedes preset-env, React and CommonJS lowering', () => {
  const source =
    "import * as stylex from '@stylexjs/stylex';const base={color:'red'};const n=4;export const s=stylex.create({a:{...base,width:`${n}px`}});export const C=()=> <div {...stylex.props(s.a)}/>;";
  for (const before of [
    [],
    [require.resolve('@babel/plugin-transform-modules-commonjs')],
    [require.resolve('@babel/plugin-transform-template-literals')],
  ]) {
    const result = babel.transformSync(source, {
      filename,
      babelrc: false,
      configFile: false,
      plugins: [...before, [plugin, {}]],
      presets: [
        [require.resolve('@babel/preset-env'), { targets: 'ie 11' }],
        require.resolve('@babel/preset-react'),
      ],
    });
    assert.equal(result.metadata.stylex.length, 2);
    assert.ok(!result.code.includes('stylex.create('));
    assert.ok(!result.code.includes('.props('));
    assert.ok(result.code.includes('React.createElement'));
    assert.ok(result.code.includes('x1e2nbdu'));
  }
});

const { createRequire } = require('node:module');
const { TraceMap, originalPositionFor } = createRequire(
  require.resolve('@babel/core'),
)('@jridgewell/trace-mapping');
function mappedIdentifier(result, needle) {
  const lines = result.code.split('\n');
  const index = lines.findIndex((line) => line.includes(needle));
  assert.ok(index >= 0);
  return originalPositionFor(new TraceMap(result.map), {
    line: index + 1,
    column: lines[index].indexOf(needle),
  });
}
test('adapter source maps retain original locations after removed StyleX declarations', () => {
  const source = `import * as stylex from '@stylexjs/stylex';
const styles = stylex.create({
  root: {
    color: 'red',
    padding: 4,
  },
});
// a comment
export function markerFunction(p) {
  const valueAfter = computeSomething(p);
  return <div {...stylex.props(styles.root)}>{valueAfter}</div>;
}
`;
  const result = babel.transformSync(source, {
    filename,
    babelrc: false,
    configFile: false,
    sourceMaps: true,
    parserOpts: { plugins: ['jsx'] },
    plugins: [[plugin, {}]],
  });
  const marker = mappedIdentifier(result, 'markerFunction');
  assert.equal(marker.line, 9);
  assert.equal(marker.column, 16);
  const compute = mappedIdentifier(result, 'computeSomething');
  assert.equal(compute.line, 10);
  assert.equal(compute.column, 21);
});

test('adapter maps use only Babel source names and preserve sourcesContent', () => {
  const source = `import * as stylex from '@stylexjs/stylex';
const styles = stylex.create({a: {color: 'red'}});
export function markerFunction(){return stylex.props(styles.a);}
`;
  for (const extra of [
    {},
    { sourceFileName: 'src/custom.js' },
    { sourceFileName: 'custom.js', sourceRoot: '/src' },
  ]) {
    const options = {
      filename: '/repo/src/A.js',
      babelrc: false,
      configFile: false,
      sourceMaps: true,
      ...extra,
    };
    const baseline = babel.transformSync(source, options);
    const result = babel.transformSync(source, {
      ...options,
      plugins: [[plugin, {}]],
    });
    assert.deepEqual(result.map.sources, baseline.map.sources);
    assert.deepEqual(result.map.sourcesContent, baseline.map.sourcesContent);
    assert.equal(result.map.sourceRoot, baseline.map.sourceRoot);
    const marker = mappedIdentifier(result, 'markerFunction');
    assert.equal(marker.line, 3);
    assert.equal(marker.column, 16);
  }
});

test('native maps compose with earlier Program.enter edits', () => {
  const source = `import * as stylex from '@stylexjs/stylex';
const styles = stylex.create({red: {color: 'red'}});


export function markerFunction(){return stylex.props(styles.red);}
`;
  const before = () => ({
    visitor: {
      Program(program) {
        program.traverse({
          StringLiteral(p) {
            if (p.node.value === 'red') {
              p.node.value = 'blue';
              delete p.node.extra;
            }
          },
        });
      },
    },
  });
  const result = babel.transformSync(source, {
    filename,
    babelrc: false,
    configFile: false,
    sourceMaps: true,
    plugins: [before, [plugin, {}]],
  });
  const marker = mappedIdentifier(result, 'markerFunction');
  assert.equal(marker.line, 5);
  assert.equal(marker.column, 16);
  assert.equal(result.metadata.stylex[0][1].ltr, '.xju2f9n{color:blue}');
});

test('unrelated TypeScript files retain their input AST and source maps', () => {
  const source = `import type { A } from './a';
interface Props {
  a: string;
  b: number;
}
export function withTypes(p: Props): string {
  const marker = p.a + String(p.b);
  return marker;
}
`;
  const options = {
    filename: '/x/a.ts',
    babelrc: false,
    configFile: false,
    sourceMaps: true,
    presets: [require.resolve('@babel/preset-typescript')],
  };
  const baseline = babel.transformSync(source, options);
  const result = babel.transformSync(source, {
    ...options,
    plugins: [[plugin, {}]],
  });
  assert.equal(result.code, baseline.code);
  assert.deepEqual(result.map, baseline.map);
  const marker = mappedIdentifier(result, 'marker');
  assert.equal(marker.line, 7);
  assert.equal(marker.column, 8);
});

test('unchanged relevant imports preserve original AST locations', () => {
  const source =
    "import * as stylex from '@stylexjs/stylex';\n\n\nexport function markerFunction(){return 1;}";
  const options = {
    filename,
    babelrc: false,
    configFile: false,
    sourceMaps: true,
  };
  const baseline = babel.transformSync(source, options);
  const result = babel.transformSync(source, {
    ...options,
    plugins: [[plugin, {}]],
  });
  assert.deepEqual(result.map, baseline.map);
  assert.equal(result.code, baseline.code);
});

test('adapter forwards sloppy script grammar rather than forcing module syntax', () => {
  const source =
    "const stylex=require('@stylexjs/stylex');const legacy=010;const styles=stylex.create({a:{color:'red'}});const p=stylex.props(styles.a);";
  const result = babel.transformSync(source, {
    filename,
    babelrc: false,
    configFile: false,
    sourceType: 'script',
    plugins: [[plugin, {}]],
  });
  assert.equal(result.metadata.stylex.length, 1);
  assert.ok(!result.code.includes('stylex.create('));
  assert.ok(!result.code.includes('stylex.props('));
  assert.ok(result.code.includes('legacy = 010'));
});

test('keyframes arity errors highlight the variable declarator', () => {
  const source =
    "import * as stylex from '@stylexjs/stylex';\nconst k = stylex.keyframes();";
  assert.throws(
    () => transform(source),
    (error) =>
      error.name === 'SyntaxError' &&
      /\n> 2 \| const k = stylex\.keyframes\(\);\n {4}\| {7}\^/.test(
        error.message,
      ),
  );
});

test('diagnostic columns count Unicode line separators', () => {
  for (const separator of ['\u2028', '\u2029']) {
    const source =
      "import * as stylex from '@stylexjs/stylex';" +
      separator +
      '  stylex.create({});';
    assert.throws(
      () => transform(source),
      (error) =>
        error.name === 'SyntaxError' &&
        /\n> 2 \| {3}stylex\.create\(\{\}\);\n {4}\| {3}\^/.test(error.message),
    );
  }
});

test('ordered conditionals match the raw Babel snapshot', () => {
  const result = transform(
    "import * as stylex from '@stylexjs/stylex';const styles=stylex.create({red:{color:'red'},blue:{color:'blue'}});const p=stylex.props(flag&&styles.red,styles.blue);",
  );
  assert.equal(
    result.code,
    `import * as stylex from '@stylexjs/stylex';
const p = {
  0: {
    className: "xju2f9n"
  },
  1: {
    className: "xju2f9n"
  }
}[!!flag << 0];`,
  );
  assert.deepEqual(result.metadata.stylex, [
    ['x1e2nbdu', { ltr: '.x1e2nbdu{color:red}', rtl: null }, 3000],
    ['xju2f9n', { ltr: '.xju2f9n{color:blue}', rtl: null }, 3000],
  ]);
});

test('conditional table literal insertion follows current Babel permutations', () => {
  const result = transform(
    "import * as stylex from '@stylexjs/stylex';const styles=stylex.create({red:{color:'red'},blue:{color:'blue'}});const p=stylex.props(a&&styles.red,b&&styles.blue);",
  );
  assert.equal(
    result.code,
    `import * as stylex from '@stylexjs/stylex';
const p = {
  0: {},
  2: {
    className: "x1e2nbdu"
  },
  1: {
    className: "xju2f9n"
  },
  3: {
    className: "xju2f9n"
  }
}[!!a << 1 | !!b << 0];`,
  );
});

test('flattened JSX props does not retain a removed expression edit boundary', () => {
  const result = transform(
    "import * as stylex from '@stylexjs/stylex';const styles=stylex.create({red:{color:'red'}});const view=<div {...stylex.props(styles.red)}/>;",
    {},
    [],
    ['jsx'],
  );
  assert.equal(
    result.code,
    `import * as stylex from '@stylexjs/stylex';
const view = <div className="x1e2nbdu" />;`,
  );
});

test('generated strings use Babel escapes while original literals stay raw', () => {
  const source =
    "import * as stylex from '@stylexjs/stylex';const original='é';const value=stylex.defaultMarker();use(original,value);";
  const result = transform(source, { classNamePrefix: 'é😀' });
  assert.equal(
    result.code,
    `import * as stylex from '@stylexjs/stylex';
const original = 'é';
const value = {
  "\\xE9\\uD83D\\uDE00-default-marker": "\\xE9\\uD83D\\uDE00-default-marker",
  $$css: true
};
use(original, value);`,
  );
});

test('named runtime injection aliases use the configured export name', () => {
  const source =
    "import * as stylex from '@stylexjs/stylex';const styles=stylex.create({a:{color:'red'}});stylex.props(styles.a);";
  const result = transform(source, {
    runtimeInjection: { from: 'custom', as: 'inj' },
  });
  assert.equal(
    result.code,
    `import { inj as _inject } from "custom";
var _inj = _inject;
import * as stylex from '@stylexjs/stylex';
_inj({
  ltr: ".x1e2nbdu{color:red}",
  priority: 3000
});
({
  className: "x1e2nbdu"
});`,
  );
});

test('plain runtime injection preserves alias and raw ordering', () => {
  const result = transform(
    "import * as stylex from '@stylexjs/stylex';const styles=stylex.create({red:{color:'red'}});const p=stylex.props(styles.red);",
    { runtimeInjection: true },
  );
  assert.equal(
    result.code,
    `import _inject from "@stylexjs/stylex/lib/stylex-inject";
var _inject2 = _inject;
import * as stylex from '@stylexjs/stylex';
_inject2({
  ltr: ".x1e2nbdu{color:red}",
  priority: 3000
});
const p = {
  className: "x1e2nbdu"
};`,
  );
});

test('static atoms bailout hoists the reusable object, without snapshot normalization', () => {
  const result = transform(
    "import stylex from 'stylex';import css from '@stylexjs/atoms';stylex.props(css.display.flex,external);",
    { runtimeInjection: true },
  );
  assert.equal(
    result.code,
    `import _inject from "@stylexjs/stylex/lib/stylex-inject";
var _inject2 = _inject;
import stylex from 'stylex';
import css from '@stylexjs/atoms';
_inject2({
  ltr: ".x78zum5{display:flex}",
  priority: 3000
});
const _temp = {
  k1xSpc: "x78zum5",
  $$css: true
};
stylex.props(_temp, external);`,
  );
});

test('sx injects a value namespace and preserves unknown runtime values', () => {
  const result = transform(
    'function Foo(x){return <svg sx={x}/>}',
    {},
    [],
    ['jsx'],
  );
  assert.equal(
    result.code,
    `import * as stylex from "@stylexjs/stylex";
function Foo(x) {
  return <svg {...stylex.props(x)} />;
}`,
  );
});

test('earlier Program.enter changes are not overwritten by original file.code', () => {
  const preplugin = () => ({
    visitor: {
      Program(program) {
        program.traverse({
          StringLiteral(p) {
            if (p.node.value === 'red') {
              p.node.value = 'blue';
              delete p.node.extra;
            }
          },
        });
      },
    },
  });
  const result = transform(
    "import * as stylex from '@stylexjs/stylex';const styles=stylex.create({red:{color:'red'}});const p=stylex.props(styles.red);",
    {},
    [preplugin],
  );
  assert.equal(result.metadata.stylex[0][1].ltr, '.xju2f9n{color:blue}');
  assert.ok(!result.code.includes('x1e2nbdu'));
});

test('metadata remains registered on facade entry even without styles', () => {
  assert.deepEqual(transform('const n = 1;').metadata, { stylex: [] });
});

test('Flow type imports and raw quote lexemes survive the native facade', () => {
  const result = transform(
    "import type {Foo} from 'types';const s: string='text';",
    {},
    [],
    [['flow', { all: true }]],
  );
  assert.equal(
    result.code,
    "import type { Foo } from 'types';\nconst s: string = 'text';",
  );
});

test('nested bindings do not mask the visible outer runtime namespace', () => {
  const result = transform(
    "import * as stylex from '@stylexjs/stylex';function F(){function G(stylex){return stylex}return <div sx={value}/>}",
    {},
    [],
    ['jsx'],
  );
  assert.equal(
    result.code,
    `import * as stylex from '@stylexjs/stylex';
function F() {
  function G(stylex) {
    return stylex;
  }
  return <div {...stylex.props(value)} />;
}`,
  );
});

test('catch bindings cannot capture the implicit sx runtime namespace', () => {
  const result = transform(
    "import * as stylex from '@stylexjs/stylex';function F(){try{fail()}catch(stylex){return <div sx={value}/>}}",
    {},
    [],
    ['jsx'],
  );
  assert.equal(
    result.code,
    `import * as _stylex from "@stylexjs/stylex";
import * as stylex from '@stylexjs/stylex';
function F() {
  try {
    fail();
  } catch (stylex) {
    return <div {..._stylex.props(value)} />;
  }
}`,
  );
});

test('Hermes parser overrides retain JSX grammar without rerunning visitors', () => {
  const result = transform('const x=<div/>;', {}, [
    ['babel-plugin-syntax-hermes-parser', { flow: 'detect' }],
  ]);
  assert.equal(result.code, 'const x = <div />;');
});

test('binding validation uses native diagnostic ranges and standard Babel code frames', () => {
  const source =
    "\n  import * as stylex from '@stylexjs/stylex';\n  const vars = stylex.defineVars({});\n";
  let error;
  try {
    transform(source, { unstable_moduleResolution: { type: 'haste' } });
  } catch (cause) {
    error = cause;
  }
  assert.equal(error?.name, 'SyntaxError');
  assert.equal(
    error.message,
    `${filename}: The return value of defineVars() must be bound to a named export.
  1 |
  2 |   import * as stylex from '@stylexjs/stylex';
> 3 |   const vars = stylex.defineVars({});
    |                ^^^^^^^^^^^^^^^^^^^^^
  4 |`,
  );
});

test('multiline diagnostics highlight only the start, as Babel does', () => {
  const source =
    "\n  import * as stylex from '@stylexjs/stylex';\n  stylex.create({\n    red: {color: 'red'},\n  });\n";
  let error;
  try {
    transform(source);
  } catch (cause) {
    error = cause;
  }
  assert.equal(error?.name, 'SyntaxError');
  assert.equal(
    error.message,
    `${filename}: create() calls must be bound to a bare variable.
  1 |
  2 |   import * as stylex from '@stylexjs/stylex';
> 3 |   stylex.create({
    |   ^
  4 |     red: {color: 'red'},
  5 |   });
  6 |`,
  );
});
