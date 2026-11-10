/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
'use strict';
// Current-contract regression tests run through the native Babel facade.
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
    // Code frames are compared as text; keep them uncoloured on CI terminals.
    highlightCode: false,
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

test('producer error frames match the original plugin', () => {
  // The original visitors frame these at the declarator, call or argument.
  const original = require('@stylexjs/babel-plugin');
  const cases = [
    'const k = stylex.keyframes();',
    'const k = stylex.keyframes({}, {});',
    'const k = stylex.keyframes(1);',
    "const k = stylex.keyframes({from: 'red'});",
    'const k = stylex.keyframes({from: {color: x}});',
    'const p = stylex.positionTry();',
    'const v = stylex.viewTransitionClass();',
    "const p = stylex.positionTry({color: 'red'});",
    'const p = stylex.positionTry({top: x});',
    "const c = 'x';\nconst p = stylex.positionTry(c);",
    "const v = stylex.viewTransitionClass({foo: 'x'});",
    'const v = stylex.viewTransitionClass({group: {color: x}});',
    "const c = 'x';\nconst v = stylex.viewTransitionClass(c);",
    'const s = stylex.create(1);',
    'const s = stylex.create({...a});',
    'export const c = stylex.defineConsts(1);',
    'export const c = stylex.defineConsts({a: x});',
    'const d = stylex.defaultMarker(1);',
  ];
  const report = (stylex, code) => {
    try {
      babel.transformSync(
        `import * as stylex from '@stylexjs/stylex';\n${code}`,
        {
          filename,
          babelrc: false,
          configFile: false,
          highlightCode: false,
          plugins: [[stylex, {}]],
        },
      );
      return { completed: true };
    } catch (error) {
      return { name: error.name, code: error.code, message: error.message };
    }
  };
  for (const code of cases) {
    assert.deepEqual(report(plugin, code), report(original, code), code);
  }
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

test('generated JSX backslashes match Babel in direct and adapter output', () => {
  const native = require('../index');
  const source = `import stylex from "stylex";
const styles = stylex.create({ red: { color: "red" } });
function Foo() { return <div css={styles.red}>Hello</div>; }`;
  for (const debugPath of [
    'package:dist/compo\\nents/Foo.js',
    'npm-package:js\\node_modules\\npm-package\\dist\\components\\Foo.js',
  ]) {
    const options = {
      dev: true,
      enableDevClassNames: false,
      sxPropName: 'css',
      debugFilePath: () => debugPath,
    };
    const expected = `import stylex from "stylex";
function Foo() {
  return <div className="x1e2nbdu" data-style-src=${JSON.stringify(debugPath + ':2')}>Hello</div>;
}`;
    const direct = native.transformSync(source, { ...options, filename });
    const printed = babel.transformSync(direct.code, {
      filename,
      babelrc: false,
      configFile: false,
      parserOpts: { plugins: ['jsx'] },
    });
    assert.equal(printed.code, expected);
    const adapted = babel.transformSync(source, {
      filename,
      babelrc: false,
      configFile: false,
      parserOpts: { plugins: ['jsx'] },
      plugins: [[plugin, options]],
    });
    assert.equal(adapted.code, expected);
    assert.ok(adapted.code.includes('\\\\'));
  }
});

test('generated JSX double quotes retain valid native single-quote spelling', () => {
  const native = require('../index');
  const source = `import stylex from "stylex";
const styles=stylex.create({red:{color:"red"}});
const view=<div css={styles.red}/>;`;
  const options = {
    dev: true,
    enableDevClassNames: false,
    sxPropName: 'css',
    debugFilePath: () => 'package:dist/compo"nents/Foo.js',
  };
  const direct = native.transformSync(source, { ...options, filename });
  assert.doesNotThrow(() =>
    babel.parseSync(direct.code, {
      filename,
      babelrc: false,
      configFile: false,
      parserOpts: { plugins: ['jsx'] },
    }),
  );
  const adapted = babel.transformSync(source, {
    filename,
    babelrc: false,
    configFile: false,
    parserOpts: { plugins: ['jsx'] },
    plugins: [[plugin, options]],
  });
  assert.equal(
    adapted.code,
    `import stylex from "stylex";
const view = <div className="x1e2nbdu" data-style-src='package:dist/compo"nents/Foo.js:2' />;`,
  );
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

// The original plugin, configured like `transform`, as the reference output.
function transformWithBabel(source, options = {}, parserPlugins = [], file = filename) {
  return babel.transformSync(source, {
    filename: file,
    babelrc: false,
    configFile: false,
    highlightCode: false,
    parserOpts: { plugins: parserPlugins },
    plugins: [[require('@stylexjs/babel-plugin'), options]],
  });
}

test('sx reuses a top-level CommonJS require of stylex, as Babel does', () => {
  const native = require('../index');
  for (const source of [
    "const stylex=require('@stylexjs/stylex');module.exports=(s)=><div sx={s}/>;",
    "var stylex=require('@stylexjs/stylex');module.exports=(s)=><div sx={s}/>;",
    "const stylex=require('@stylexjs/stylex');module.exports=(stylex,s)=><div sx={s}/>;",
  ]) {
    const expected = transformWithBabel(source, {}, ['jsx']).code;
    assert.equal(transform(source, {}, [], ['jsx']).code, expected, source);
    const direct = native.transformSync(source, { filename }).code;
    assert.equal(direct.includes('import'), expected.includes('import'), source);
  }
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

// Integration with other Babel plugins: unchanged nodes keep their identity.

const refreshSource = `import * as stylex from '@stylexjs/stylex';
import {useState} from 'react';
const styles = stylex.create({root: {color: 'red'}});
export default function App() {
  const [count] = useState(0);
  return <div {...stylex.props(styles.root)}>{count}</div>;
}
`;

test('React Refresh hook signatures survive when Refresh runs before StyleX', () => {
  const refresh = [
    require.resolve('react-refresh/babel'),
    { skipEnvCheck: true },
  ];
  const run = (plugins) =>
    babel.transformSync(refreshSource, {
      filename,
      babelrc: false,
      configFile: false,
      parserOpts: { plugins: ['jsx'] },
      plugins,
    }).code;
  const refreshFirst = run([refresh, [plugin, {}]]);
  assert.equal(refreshFirst, run([[plugin, {}], refresh]));
  assert.ok(refreshFirst.includes('var _s = $RefreshSig$();'));
  assert.ok(refreshFirst.includes('_s(App, "'));
  assert.ok(refreshFirst.includes('className="x1e2nbdu"'));
});

test('unrelated statements and enclosing functions keep their AST node identity', () => {
  let recorded;
  const recorder = () => ({
    visitor: {
      Program: {
        enter(program) {
          const body = program.node.body;
          recorded = { react: body[1], app: body[3], fn: body[3].declaration };
        },
        exit(program) {
          recorded.after = program.node.body;
        },
      },
    },
  });
  const result = babel.transformSync(refreshSource, {
    filename,
    babelrc: false,
    configFile: false,
    parserOpts: { plugins: ['jsx'] },
    plugins: [recorder, [plugin, {}]],
  });
  assert.ok(result.code.includes('className="x1e2nbdu"'));
  assert.ok(recorded.after.includes(recorded.react));
  assert.ok(recorded.after.includes(recorded.app));
  assert.equal(recorded.app.declaration, recorded.fn);
});

// An earlier plugin's per-node visitor creates StyleX calls during traversal.
function macroPlugin() {
  const t = babel.types;
  return {
    visitor: {
      CallExpression(path) {
        const callee = path.node.callee;
        const rename = { makeStyles: 'create', cx: 'props' }[callee.name];
        if (callee.type === 'Identifier' && rename != null) {
          path.node.callee = t.memberExpression(
            t.identifier('stylex'),
            t.identifier(rename),
          );
        }
      },
    },
  };
}

test('StyleX calls created by an earlier CallExpression visitor compile before later lowering', () => {
  const body = (create, props) => `import * as stylex from '@stylexjs/stylex';
const base = {margin: 0};
const n = 4;
const styles = ${create}({root: {...base, width: \`\${n}px\`, color: 'red'}});
export const s = ${props}(styles.root);
`;
  const options = (plugins) => ({
    filename,
    babelrc: false,
    configFile: false,
    plugins: [...plugins, [plugin, {}]],
    presets: [[require.resolve('@babel/preset-env'), { targets: 'ie 11' }]],
  });
  const direct = babel.transformSync(
    body('stylex.create', 'stylex.props'),
    options([]),
  );
  const macro = babel.transformSync(
    body('makeStyles', 'cx'),
    options([macroPlugin]),
  );
  assert.ok(!macro.code.includes('stylex.create('));
  assert.ok(!macro.code.includes('.props('));
  assert.equal(macro.code, direct.code);
  assert.deepEqual(macro.metadata.stylex, direct.metadata.stylex);
});

test('StyleX calls appended by an earlier Program.exit compile like source calls', () => {
  const head = `import * as stylex from '@stylexjs/stylex';
const styles = stylex.create({root: {color: 'red'}, other: {padding: 4}});
export const s = stylex.props(styles.root);
`;
  const late = 'export const late = stylex.props(styles.other);';
  const appender = () => ({
    visitor: {
      Program: {
        exit(program) {
          program.pushContainer('body', babel.template.statement.ast(late));
        },
      },
    },
  });
  const direct = transform(`${head}${late}\n`);
  const appended = transform(head, {}, [appender]);
  assert.ok(!appended.code.includes('.props('));
  assert.equal(appended.code, direct.code);
  assert.deepEqual(appended.metadata.stylex, direct.metadata.stylex);
});

// Hermes supplies the AST; the facade must not reparse with Babel's Flow parser.
const hermesSource = `import * as stylex from '@stylexjs/stylex';
const styles = stylex.create({root: {color: 'red'}, pad: {padding: 4}});
hook useTheme(dark: boolean): string { return dark ? 'a' : 'b'; }
function isString(x: mixed): x is string { return typeof x === 'string'; }
type Cond<T> = T extends string ? 'yes' : 'no';
type Mapped<O> = {[K in keyof O]: O[K]};
export component Button(label: string, ...rest: {}) {
  const kind = match (label) { 'a' => styles.pad, _ => null };
  return <div {...stylex.props(styles.root, kind)}>{(label as string)}</div>;
}
`;
const hermes = ['babel-plugin-syntax-hermes-parser', { flow: 'detect' }];

test('Hermes ASTs with modern Flow syntax compile without a Babel Flow reparse', () => {
  const result = transform(hermesSource, {}, [hermes]);
  assert.equal(
    result.code,
    `import * as stylex from '@stylexjs/stylex';
const styles = {
  root: {
    kMwMTN: "x1e2nbdu",
    $$css: true
  },
  pad: {
    kmVPX3: "xfawy5m",
    $$css: true
  }
};
function useTheme(dark: boolean): string {
  return dark ? 'a' : 'b';
}
function isString(x: mixed): any {
  return typeof x === 'string';
}
type Cond<T> = any;
type Mapped<O> = any;
export function Button({
  label,
  ...rest
}: $ReadOnly<{
  ...{},
  label: string,
}>): React.Node {
  const kind = label === 'a' ? styles.pad : null;
  return <div {...stylex.props(styles.root, kind)}>{(label: string)}</div>;
}`,
  );
  assert.deepEqual(result.metadata.stylex, [
    ['x1e2nbdu', { ltr: '.x1e2nbdu{color:red}', rtl: null }, 3000],
    ['xfawy5m', { ltr: '.xfawy5m{padding:4px}', rtl: null }, 1000],
  ]);
});

test('Hermes `as` casts keep original source maps', () => {
  const source = `import * as stylex from '@stylexjs/stylex';
const styles = stylex.create({root: {color: 'red'}});
export function markerFunction(x: mixed) { return [(x as string), stylex.props(styles.root)]; }
`;
  const result = babel.transformSync(source, {
    filename,
    babelrc: false,
    configFile: false,
    sourceMaps: true,
    plugins: [hermes, [plugin, {}]],
  });
  assert.ok(result.code.includes('(x: string)'));
  assert.ok(result.code.includes('className: "x1e2nbdu"'));
  const marker = mappedIdentifier(result, 'markerFunction');
  assert.equal(marker.line, 3);
  assert.equal(marker.column, 16);
});

test('Hermes syntax the native parser cannot read keeps original maps and code frames', () => {
  const source = `import * as stylex from '@stylexjs/stylex';
hook useStyles() { return 1; }
const styles = stylex.create({root: {color: 'red'}});
export function markerFunction() { return stylex.props(styles.root); }
`;
  const result = babel.transformSync(source, {
    filename,
    babelrc: false,
    configFile: false,
    sourceMaps: true,
    plugins: [hermes, [plugin, {}]],
  });
  assert.ok(result.code.includes('function useStyles()'));
  assert.ok(result.code.includes('className: "x1e2nbdu"'));
  const marker = mappedIdentifier(result, 'markerFunction');
  assert.equal(marker.line, 4);
  assert.equal(marker.column, 16);
  let error;
  try {
    transform(
      `import * as stylex from '@stylexjs/stylex';
hook useStyles() { return 1; }
stylex.create({red: {color: 'red'}});
`,
      {},
      [hermes],
    );
  } catch (cause) {
    error = cause;
  }
  assert.equal(error?.name, 'SyntaxError');
  assert.equal(
    error.message,
    `${filename}: create() calls must be bound to a bare variable.
  1 | import * as stylex from '@stylexjs/stylex';
  2 | hook useStyles() { return 1; }
> 3 | stylex.create({red: {color: 'red'}});
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  4 |`,
  );
});

test('definitions created by an earlier visitor register rules in Babel traversal order', () => {
  // `fade` becomes a keyframes definition only once the macro visits it.
  const source = `import * as stylex from '@stylexjs/stylex';
const fade = makeKeyframes({from: {opacity: 0}});
const a = stylex.create({x: {color: 'red', animationName: fade}});
const m = makeStyles({y: {color: 'blue'}});
const b = stylex.create({w: {color: 'green'}});
export const s = cx(a.x, m.y, b.w);
`;
  const keyframes = () => {
    const t = babel.types;
    return {
      visitor: {
        CallExpression(path) {
          if (path.node.callee.name === 'makeKeyframes')
            path.node.callee = t.memberExpression(
              t.identifier('stylex'),
              t.identifier('keyframes'),
            );
        },
      },
    };
  };
  const result = transform(source, {}, [keyframes, macroPlugin]);
  assert.equal(
    result.code,
    `import * as stylex from '@stylexjs/stylex';
const fade = "xw2c1lr-B";
export const s = {
  className: "x1j4p4jw x1prwzq3"
};`,
  );
  assert.deepEqual(result.metadata.stylex, [
    [
      'xw2c1lr-B',
      { ltr: '@keyframes xw2c1lr-B{from{opacity:0;}}', rtl: null },
      0,
    ],
    ['x1e2nbdu', { ltr: '.x1e2nbdu{color:red}', rtl: null }, 3000],
    [
      'x1j4p4jw',
      { ltr: '.x1j4p4jw{animation-name:xw2c1lr-B}', rtl: null },
      3000,
    ],
    ['xju2f9n', { ltr: '.xju2f9n{color:blue}', rtl: null }, 3000],
    ['x1prwzq3', { ltr: '.x1prwzq3{color:green}', rtl: null }, 3000],
  ]);
});

test('errors in StyleX the traversal did not change keep their original code frames', () => {
  const recolor = () => ({
    visitor: {
      Program(program) {
        program.traverse({
          StringLiteral(p) {
            if (p.node.value === 'unrelated') {
              p.node.value = 'changed';
              delete p.node.extra;
            }
          },
        });
      },
    },
  });
  let error;
  try {
    transform(
      "import * as stylex from '@stylexjs/stylex';\nconst label = 'unrelated';\nstylex.create({red: {color: 'red'}});\n",
      {},
      [recolor],
    );
  } catch (cause) {
    error = cause;
  }
  assert.equal(error?.name, 'SyntaxError');
  assert.equal(
    error.message,
    `${filename}: create() calls must be bound to a bare variable.
  1 | import * as stylex from '@stylexjs/stylex';
  2 | const label = 'unrelated';
> 3 | stylex.create({red: {color: 'red'}});
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  4 |`,
  );
});

test('sx elements created by an earlier visitor are rewritten before later JSX lowering', () => {
  const t = babel.types;
  const toSx = () => ({
    visitor: {
      JSXOpeningElement(path) {
        for (const attribute of path.node.attributes)
          if (
            attribute.type === 'JSXAttribute' &&
            attribute.name.name === 'css'
          )
            attribute.name = t.jsxIdentifier('sx');
      },
    },
  });
  const result = babel.transformSync(
    `import * as stylex from '@stylexjs/stylex';
const styles = stylex.create({root: {color: 'red'}, b: {padding: 4}});
export const A = () => <div css={[styles.root, styles.b]} id="x" />;
`,
    {
      filename,
      babelrc: false,
      configFile: false,
      parserOpts: { plugins: ['jsx'] },
      presets: [require.resolve('@babel/preset-react')],
      plugins: [toSx, [plugin, {}]],
    },
  );
  assert.equal(
    result.code,
    `function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
import * as stylex from '@stylexjs/stylex';
export const A = () => /*#__PURE__*/React.createElement("div", _extends({}, {
  className: "x1e2nbdu xfawy5m"
}, {
  id: "x"
}));`,
  );
  assert.deepEqual(result.metadata.stylex, [
    ['x1e2nbdu', { ltr: '.x1e2nbdu{color:red}', rtl: null }, 3000],
    ['xfawy5m', { ltr: '.xfawy5m{padding:4px}', rtl: null }, 1000],
  ]);
});

test('parser overrides run once on user text; compiles are counted per pipeline', () => {
  const compiler = require('../index');
  const transformSync = compiler.transformSync;
  let compiles = 0;
  compiler.transformSync = (...args) => {
    compiles++;
    return transformSync(...args);
  };
  const parsed = [];
  // A user override that only accepts the user's own text.
  const counting = () => {
    const hermesParser = require('hermes-parser');
    return {
      parserOverride(code) {
        parsed.push(code);
        return hermesParser.parse(code, { babel: true, flow: 'detect' });
      },
    };
  };
  const count = (run) => {
    parsed.length = 0;
    compiles = 0;
    const result = run();
    return { result, compiles, parsed: [...parsed] };
  };
  try {
    const plain = count(() =>
      transform(
        "import * as stylex from '@stylexjs/stylex';\nconst s = stylex.create({a: {color: 'red'}});\nexport const p = stylex.props(s.a);\n",
      ),
    );
    assert.equal(plain.compiles, 1);
    const source = `import * as stylex from '@stylexjs/stylex';
hook useValue() { return 1; }
const s = stylex.create({a: {color: 'red'}});
const b = makeStyles({x: {padding: 4}});
export component App() { return <div {...stylex.props(s.a, b.x)} />; }
`;
    const directSource = source.replace('makeStyles', 'stylex.create');
    const direct = count(() => transform(directSource, {}, [counting]));
    assert.deepEqual([direct.parsed, direct.compiles], [[directSource], 1]);
    const created = count(() => transform(source, {}, [counting, macroPlugin]));
    assert.deepEqual([created.parsed, created.compiles], [[source], 2]);
    assert.equal(created.result.code, direct.result.code);
    assert.deepEqual(
      created.result.metadata.stylex,
      direct.result.metadata.stylex,
    );
    // Needs a macro-created definition: entry compile fails, exit compile succeeds.
    const keyframes = () => ({
      visitor: {
        CallExpression(path) {
          if (path.node.callee.name === 'makeKeyframes')
            path.node.callee = babel.types.memberExpression(
              babel.types.identifier('stylex'),
              babel.types.identifier('keyframes'),
            );
        },
      },
    });
    const dependent = count(() =>
      transform(
        `import * as stylex from '@stylexjs/stylex';
const fade = makeKeyframes({from: {opacity: 0}});
const s = stylex.create({a: {animationName: fade}});
export const p = stylex.props(s.a);
`,
        {},
        [keyframes],
      ),
    );
    assert.equal(dependent.compiles, 2);
    assert.ok(dependent.result.code.includes('className: "x1j4p4jw"'));
  } finally {
    compiler.transformSync = transformSync;
  }
});

test('the exit recompile keeps what later plugins moved, replaced or removed', () => {
  const t = babel.types;
  const createdBlue = () => ({
    visitor: {
      CallExpression(path) {
        if (path.node.callee.name === 'mk')
          path.replaceWith(
            babel.template.expression.ast(
              "stylex.create({x: {color: 'blue'}})",
            ),
          );
      },
    },
  });
  const lateProps = () => ({
    visitor: {
      Program: {
        exit(program) {
          program.pushContainer(
            'body',
            babel.template.statement.ast(
              'export const late = stylex.props(s.a);',
            ),
          );
        },
      },
    },
  });
  const topLevelInit = (path) =>
    path.parentPath.isProgram() &&
    path.node.expression.type === 'CallExpression' &&
    path.node.expression.callee.name === 'init';
  // One-shot plugins after StyleX: their edits must not be undone.
  const mover = () => ({
    visitor: {
      ExpressionStatement(path) {
        if (!topLevelInit(path)) return;
        const setup = path.parentPath
          .get('body')
          .find((statement) =>
            statement.get('declaration').isFunctionDeclaration(),
          );
        setup
          .get('declaration.body')
          .pushContainer('body', t.cloneNode(path.node));
        path.remove();
      },
    },
  });
  const replacer = () => ({
    visitor: {
      ExpressionStatement(path) {
        if (topLevelInit(path))
          path.replaceWith(
            t.expressionStatement(
              t.callExpression(t.identifier('replaced'), []),
            ),
          );
      },
    },
  });
  const source =
    "import * as stylex from '@stylexjs/stylex'; init(); const s=stylex.create({a:{color:'red'}}); export const p=stylex.props(s.a); const b=mk(); export const q=stylex.props(b.x); export function setup(){}";
  const run = (trigger, later) =>
    babel.transformSync(source, {
      filename,
      babelrc: false,
      configFile: false,
      plugins: [trigger, [plugin, {}], later],
    }).code;
  const compiled = `export const p = {
  className: "x1e2nbdu"
};
export const q = {
  className: "xju2f9n"
};`;
  assert.equal(
    run(createdBlue, mover),
    `import * as stylex from '@stylexjs/stylex';
${compiled}
export function setup() {
  init();
}`,
  );
  assert.equal(
    run(createdBlue, replacer),
    `import * as stylex from '@stylexjs/stylex';
replaced();
${compiled}
export function setup() {}`,
  );
  for (const later of [mover, replacer]) {
    const code = run(lateProps, later);
    assert.ok(
      code.includes('export const late = {\n  className: "x1e2nbdu"\n};'),
    );
    assert.equal(code.match(/\b(init|replaced)\(\)/g).length, 1);
  }
});

test('debug data reports input lines after earlier Program.enter edits', () => {
  const recolor = () => ({
    visitor: {
      Program(program) {
        program.traverse({
          StringLiteral(p) {
            if (p.node.value === 'unrelated') {
              p.node.value = 'changed';
              delete p.node.extra;
            }
          },
        });
      },
    },
  });
  const result = babel.transformSync(
    "import * as stylex from '@stylexjs/stylex';\nconst label = 'unrelated';\n\n\n\nconst styles = stylex.create({\n\n  root: {color: 'red'},\n});\nexport const s = stylex.props(styles.root);\n",
    {
      filename: path.resolve('/repo/src/Fixture.js'),
      babelrc: false,
      configFile: false,
      plugins: [
        recolor,
        [
          plugin,
          {
            debug: true,
            unstable_moduleResolution: {
              type: 'commonJS',
              rootDir: path.resolve('/repo'),
            },
          },
        ],
      ],
    },
  );
  // The label is the path relative to rootDir, with this platform's separator.
  const label = JSON.stringify(`${path.join('src', 'Fixture.js')}:8`);
  assert.ok(result.code.includes(`"data-style-src": ${label}`));
});

// The dead-branch plugin (inline __DEV__) runs both before and after StyleX.
test('definitions detached by dead-branch elimination leave no output or rules', () => {
  const t = babel.types;
  const strip = (value) => () => ({
    visitor: {
      IfStatement(path) {
        if (
          path.get('test').isIdentifier({ name: '__DEV__' }) &&
          !path.scope.hasBinding('__DEV__')
        ) {
          const kept = value ? path.node.consequent : path.node.alternate;
          if (kept) path.replaceWith(kept);
          else path.remove();
        }
      },
      ConditionalExpression(path) {
        if (
          path.get('test').isIdentifier({ name: '__DEV__' }) &&
          !path.scope.hasBinding('__DEV__')
        )
          path.replaceWith(value ? path.node.consequent : path.node.alternate);
      },
    },
  });
  const created = () => ({
    visitor: {
      CallExpression(path) {
        if (path.node.callee.name === 'makeStyles')
          path.node.callee = t.memberExpression(
            t.identifier('stylex'),
            t.identifier('create'),
          );
      },
    },
  });
  const rule = (value) =>
    value
      ? ['x1prwzq3', { ltr: '.x1prwzq3{color:green}', rtl: null }, 3000]
      : ['xju2f9n', { ltr: '.xju2f9n{color:blue}', rtl: null }, 3000];
  const object = (className, indent = '') =>
    `{\n${indent}  a: {\n${indent}    kMwMTN: "${className}",\n${indent}    $$css: true\n${indent}  }\n${indent}}`;
  const sources = {
    ifElse:
      "import * as stylex from '@stylexjs/stylex';\nlet s;\nif (__DEV__) {\n  s = stylex.create({a: {color: 'green'}});\n} else {\n  s = makeStyles({a: {color: 'blue'}});\n}\nexport const p = stylex.props(s.a);\n",
    ternary:
      "import * as stylex from '@stylexjs/stylex';\nexport const s = __DEV__ ? stylex.create({a: {color: 'green'}}) : stylex.create({a: {color: 'blue'}});\n",
    shadowed:
      "import * as stylex from '@stylexjs/stylex';\nexport function f(__DEV__) {\n  if (__DEV__) {\n    return stylex.create({a: {color: 'green'}});\n  }\n  return stylex.create({a: {color: 'blue'}});\n}\n",
  };
  for (const value of [true, false]) {
    const className = rule(value)[0];
    const expected = {
      ifElse: [
        `import * as stylex from '@stylexjs/stylex';\nlet s;\nconst _styles = ${object(className)};\n{\n  s = _styles;\n}\nexport const p = stylex.props(s.a);`,
        [rule(value)],
      ],
      ternary: [
        `import * as stylex from '@stylexjs/stylex';\nexport const s = ${object(className)};`,
        [rule(value)],
      ],
      shadowed: [
        `import * as stylex from '@stylexjs/stylex';\nconst _styles = ${object('x1prwzq3')};\nconst _styles2 = ${object('xju2f9n')};\nexport function f(__DEV__) {\n  if (__DEV__) {\n    return _styles;\n  }\n  return _styles2;\n}`,
        [rule(true), rule(false)],
      ],
    };
    for (const [name, source] of Object.entries(sources)) {
      for (const order of ['before', 'after']) {
        const plugins =
          order === 'before'
            ? [created, strip(value), [plugin, {}]]
            : [created, [plugin, {}], strip(value)];
        const result = babel.transformSync(source, {
          filename,
          babelrc: false,
          configFile: false,
          plugins,
        });
        const [code, metadata] = expected[name];
        assert.equal(result.code, code, `${name} ${value} ${order}`);
        assert.deepEqual(result.metadata.stylex, metadata);
      }
    }
  }
});

test('an unused definition another plugin removes registers no rules', () => {
  const removeDead = () => ({
    visitor: {
      VariableDeclaration(path) {
        if (path.node.declarations[0].id.name === 'dead') path.remove();
      },
    },
  });
  const result = transform(
    "import * as stylex from '@stylexjs/stylex';\nconst dead = stylex.create({a: {color: 'green', margin: 1}});\nexport const keep = stylex.create({k: {padding: 2}});\n",
    {},
    [],
  );
  assert.equal(result.metadata.stylex.length, 3);
  const removed = babel.transformSync(
    "import * as stylex from '@stylexjs/stylex';\nconst dead = stylex.create({a: {color: 'green', margin: 1}});\nexport const keep = stylex.create({k: {padding: 2}});\n",
    {
      filename,
      babelrc: false,
      configFile: false,
      plugins: [[plugin, {}], removeDead],
    },
  );
  assert.equal(
    removed.code,
    `import * as stylex from '@stylexjs/stylex';
export const keep = {
  k: {
    kmVPX3: "x1i3ajwb",
    $$css: true
  }
};`,
  );
  assert.deepEqual(removed.metadata.stylex, [
    ['x1i3ajwb', { ltr: '.x1i3ajwb{padding:2px}', rtl: null }, 1000],
  ]);
});

test('coverage instrumentation counts unused definitions in either plugin order', () => {
  // Outside tests/ (excluded by istanbul) and resolved, for istanbul's Windows check.
  const istanbul = [
    require.resolve('babel-plugin-istanbul'),
    { cwd: path.resolve('/repo') },
  ];
  const source = `import * as stylex from '@stylexjs/stylex';
const styles = stylex.create({root: {color: 'red'}});
export function f(x) { if (x) { return stylex.props(styles.root); } return null; }
`;
  const run = (plugins) =>
    babel.transformSync(source, {
      filename: path.resolve('/repo/src/Fixture.js'),
      cwd: path.resolve('/repo'),
      babelrc: false,
      configFile: false,
      plugins,
    }).code;
  const stylexFirst = run([[plugin, {}], istanbul]);
  assert.equal(stylexFirst, run([istanbul, [plugin, {}]]));
  assert.equal(stylexFirst.match(/\.s\[\d+\]\+\+/g).length, 4);
});

test('module lowering before StyleX does not revive unused definitions', () => {
  for (const source of [
    "import stylex from '@stylexjs/stylex';\nconst s = stylex.create({a: {color: 'red'}});\nexport const f = (x) => stylex.props(x && s.a);\n",
    "import {create, props} from '@stylexjs/stylex';\nconst s = create({a: {color: 'red'}});\nexport const f = (x) => props(x && s.a);\n",
  ]) {
    const result = transform(source, {}, [
      require.resolve('@babel/plugin-transform-modules-commonjs'),
    ]);
    assert.ok(!/\bcreate\(/.test(result.code), result.code);
    assert.ok(!/\bstylex\.|\bprops\(/.test(result.code), result.code);
    assert.deepEqual(result.metadata.stylex, [
      ['x1e2nbdu', { ltr: '.x1e2nbdu{color:red}', rtl: null }, 3000],
    ]);
  }
});

test('coverage after StyleX spans the source of compiled and hoisted definitions', () => {
  // Resolved, for istanbul's Windows exclusion check (see above).
  const istanbul = [
    require.resolve('babel-plugin-istanbul'),
    { cwd: path.resolve('/repo') },
  ];
  const strip = () => ({
    visitor: {
      IfStatement(path) {
        if (path.get('test').isIdentifier({ name: '__DEV__' }))
          path.replaceWith(path.node.alternate);
      },
    },
  });
  // The literal coverage data istanbul embeds, read without evaluating code.
  const literal = (node) => {
    switch (node.type) {
      case 'ObjectExpression':
        return Object.fromEntries(
          node.properties.map((property) => [
            property.key.name ?? property.key.value,
            literal(property.value),
          ]),
        );
      case 'ArrayExpression':
        return node.elements.map(literal);
      case 'NumericLiteral':
      case 'StringLiteral':
      case 'BooleanLiteral':
        return node.value;
      default:
        return undefined;
    }
  };
  const ranges = (code) => {
    let data;
    babel.traverse(babel.parseSync(code, { configFile: false }), {
      VariableDeclarator(declarator) {
        if (declarator.node.id.name === 'coverageData')
          data = literal(declarator.node.init);
      },
    });
    const range = (loc) => [
      loc.start.line ?? null,
      loc.start.column ?? null,
      loc.end.line ?? null,
      loc.end.column ?? null,
    ];
    return {
      s: Object.values(data.statementMap).map(range),
      b: Object.values(data.branchMap).map((branch) => [
        range(branch.loc),
        ...branch.locations.map(range),
      ]),
      f: Object.values(data.fnMap).map((fn) => range(fn.loc)),
    };
  };
  const cases = [
    [
      "import * as stylex from '@stylexjs/stylex';\nexport const s=stylex.create({a:{color:'red'}});\nexport function g(y){ if(y) return 1; return 2; }\n",
      [],
      {
        s: [
          [2, 15, 2, 47],
          [3, 22, 3, 37],
          [3, 28, 3, 37],
          [3, 38, 3, 47],
        ],
        b: [
          [
            [3, 22, 3, 37],
            [3, 22, 3, 37],
            [null, null, null, null],
          ],
        ],
        f: [[3, 20, 3, 49]],
      },
    ],
    [
      "import * as stylex from '@stylexjs/stylex';\nconst s=stylex.create({a:{color:'red'}});\nexport const f=(x)=>x?stylex.props(s.a):null;\n",
      [],
      {
        s: [
          [2, 8, 2, 40],
          [3, 15, 3, 44],
          [3, 20, 3, 44],
        ],
        b: [
          [
            [3, 20, 3, 44],
            [3, 22, 3, 39],
            [3, 40, 3, 44],
          ],
        ],
        f: [[3, 20, 3, 44]],
      },
    ],
    [
      "import * as stylex from '@stylexjs/stylex';\nlet s;\nif (__DEV__) { s = stylex.create({a:{color:'green'}}); } else { s = stylex.create({a:{color:'blue'}}); }\nexport const p=()=>stylex.props(s.a);\n",
      [strip],
      {
        s: [
          [3, 0, 3, 104],
          [3, 15, 3, 54],
          [3, 64, 3, 102],
          [4, 15, 4, 36],
          [4, 19, 4, 36],
        ],
        b: [
          [
            [3, 0, 3, 104],
            [3, 0, 3, 104],
            [3, 62, 3, 104],
          ],
        ],
        f: [[4, 19, 4, 36]],
      },
    ],
  ];
  for (const [source, before, expected] of cases) {
    const result = babel.transformSync(source, {
      filename: path.resolve('/repo/src/Fixture.js'),
      cwd: path.resolve('/repo'),
      babelrc: false,
      configFile: false,
      plugins: [...before, [plugin, {}], istanbul],
    });
    assert.deepEqual(ranges(result.code), expected);
  }
});

// TypeScript elides imports without value references when it enters the program.
test('theme modules used only by definitions stay in a TypeScript module graph', async () => {
  const fs = require('node:fs');
  const os = require('node:os');
  const { rollup } = require('rollup');
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'stylex-graph-'));
  try {
    fs.writeFileSync(
      path.join(directory, 'tokens.stylex.ts'),
      "import * as stylex from '@stylexjs/stylex';\nexport const colors = stylex.defineVars({primary: 'red'});\nexport type Unused = string;\n",
    );
    fs.writeFileSync(
      path.join(directory, 'entry.ts'),
      "import * as stylex from '@stylexjs/stylex';\nimport {colors, type Unused} from './tokens.stylex';\nimport {helper} from './helper';\nexport const s = stylex.create({a: {color: colors.primary}});\nexport type T = Unused;\n",
    );
    // Already unused before StyleX: TypeScript still elides it.
    fs.writeFileSync(
      path.join(directory, 'helper.ts'),
      'export const helper = 1;\n',
    );
    for (const treeshakeCompensation of [false, true]) {
      const visited = [];
      const rules = [];
      const outputs = {};
      const bundle = await rollup({
        input: path.join(directory, 'entry.ts'),
        onwarn() {},
        plugins: [
          {
            name: 'stylex',
            resolveId(source, importer) {
              if (source.startsWith('@stylexjs/'))
                return { id: source, external: true };
              if (source.startsWith('.'))
                return `${path.resolve(path.dirname(importer), source)}.ts`;
              return null;
            },
            async transform(code, id) {
              visited.push(path.basename(id));
              const result = await babel.transformAsync(code, {
                filename: id,
                babelrc: false,
                configFile: false,
                presets: [require.resolve('@babel/preset-typescript')],
                plugins: [
                  [
                    plugin,
                    {
                      treeshakeCompensation,
                      unstable_moduleResolution: {
                        type: 'commonJS',
                        rootDir: directory,
                      },
                    },
                  ],
                ],
              });
              rules.push(...result.metadata.stylex);
              outputs[path.basename(id)] = result.code;
              return { code: result.code, map: null };
            },
          },
        ],
      });
      await bundle.close();
      assert.deepEqual(visited, ['entry.ts', 'tokens.stylex.ts']);
      assert.ok(!outputs['entry.ts'].includes('Unused'));
      const css = plugin.processStylexRules(rules);
      assert.match(css, /--x[a-z0-9]+:red;/);
      assert.match(css, /color:var\(--x[a-z0-9]+\)/);
    }
  } finally {
    fs.rmSync(directory, { recursive: true, force: true });
  }
});

test('exceptions thrown by user callbacks reach Babel once, as from the original plugin', () => {
  const { outcome, cases } = require('./support/babel_errors.cjs');
  for (const [label, scenario, filename] of cases()) {
    const expected = outcome('babel', scenario, filename);
    assert.equal(expected.completed, undefined, `Babel threw: ${label}`);
    assert.deepEqual(outcome('facade', scenario, filename), expected, label);
  }
});

test('a direct transform nested in a facade callback wraps as nested Babel does', () => {
  const {
    appFile,
    outcome,
    runEngine,
    header,
  } = require('./support/babel_errors.cjs');
  const inner = `${header}export const s = stylex.create({ a: { color: stylex.env.brand('x') } });`;
  const scenario = (outer, nested) => () => {
    const thrown = new Error('inner boom');
    return {
      code: `${header}export const s = stylex.create({ a: { color: stylex.env.nest('x') } });`,
      options: {
        env: {
          nest: () =>
            runEngine(nested, inner, appFile('inner.js'), {
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
  const expected = outcome(
    'babel',
    scenario('babel', 'babel'),
    appFile('outer.js'),
  );
  assert.equal(
    expected.caught.message,
    `${appFile('outer.js')}: ${appFile('inner.js')}: inner boom`,
  );
  for (const nested of ['facade', 'direct']) {
    assert.deepEqual(
      outcome('facade', scenario('facade', nested), appFile('outer.js')),
      expected,
      nested,
    );
  }
});

// Babel's insertion leaves new references uncounted for unused-binding removal.
test('hoists inserted at a definition visit stay referenced for later plugins', () => {
  const removeUnused = () => ({
    visitor: {
      VariableDeclarator(path) {
        if (path.node.id.type !== 'Identifier') return;
        const binding = path.scope.getBinding(path.node.id.name);
        if (
          binding != null &&
          !binding.referenced &&
          !binding.path.parentPath.parentPath.isExportNamedDeclaration() &&
          path.get('init').isPure()
        )
          path.remove();
      },
    },
  });
  const object = (indent) =>
    `{\n${indent}  a: {\n${indent}    kMwMTN: "x1e2nbdu",\n${indent}    $$css: true\n${indent}  }\n${indent}}`;
  const head = "import * as stylex from '@stylexjs/stylex';\n";
  const cases = [
    [
      "export function f(){ const local = [stylex.create({a:{color:'red'}})]; return local; }\n",
      `const _styles = ${object('')};\nexport function f() {\n  const local = [_styles];\n  return local;\n}`,
    ],
    [
      "export function g(){ return register(stylex.create({a:{color:'red'}})); }\n",
      `const _styles = ${object('')};\nexport function g() {\n  return register(_styles);\n}`,
    ],
    [
      "export function h(){ return stylex.create({a:{color:'red'}}); }\n",
      `const _styles = ${object('')};\nexport function h() {\n  return _styles;\n}`,
    ],
    [
      "export const r = register(stylex.create({a:{color:'red'}}));\n",
      `export const r = register(${object('')});`,
    ],
    [
      "export const arr = [stylex.create({a:{color:'red'}})];\n",
      `export const arr = [${object('')}];`,
    ],
  ];
  for (const [source, expected] of cases) {
    const result = transform(head + source, {}, [], ['jsx']);
    const removed = babel.transformSync(head + source, {
      filename,
      babelrc: false,
      configFile: false,
      plugins: [[plugin, {}], removeUnused],
    });
    assert.equal(removed.code, `${head}${expected}`);
    assert.equal(removed.code, result.code);
  }
});

test('a hoist inserted at a definition visit is a referenced program binding', () => {
  let binding;
  const probe = () => ({
    visitor: {
      Program: {
        exit(program) {
          binding = program.scope.getBinding('_styles');
        },
      },
    },
  });
  babel.transformSync(
    "import * as stylex from '@stylexjs/stylex';\nexport function f(){ const local = [stylex.create({a:{color:'red'}})]; return local; }\n",
    {
      filename,
      babelrc: false,
      configFile: false,
      plugins: [[plugin, {}], probe],
    },
  );
  assert.equal(binding?.kind, 'const');
  assert.equal(binding.references, 1);
  assert.equal(binding.referencePaths[0].parentPath.isArrayExpression(), true);
});

test('hoisted definitions survive unused-binding removers in every order and shape', () => {
  const C = "stylex.create({a:{color:'red'}})";
  const D = "stylex.create({b:{color:'blue'}})";
  const removeUnused = () => ({
    visitor: {
      VariableDeclarator(path) {
        if (path.node.id.type !== 'Identifier') return;
        const binding = path.scope.getBinding(path.node.id.name);
        if (
          binding != null &&
          !binding.referenced &&
          !path.parentPath.parentPath.isExportNamedDeclaration() &&
          path.get('init').isPure()
        )
          path.remove();
      },
      Program: {
        exit(program) {
          program.scope.crawl();
        },
      },
    },
  });
  const removeUnusedOnExit = () => ({
    visitor: {
      Program: {
        exit(program) {
          for (const binding of Object.values(program.scope.bindings))
            if (
              binding.kind !== 'module' &&
              !binding.referenced &&
              binding.path.isVariableDeclarator() &&
              !binding.path.parentPath.parentPath.isExportNamedDeclaration() &&
              binding.path.get('init').isPure()
            )
              binding.path.remove();
        },
      },
    },
  });
  const shapes = [
    `export function f(){ const local = [${C}]; return local; }`,
    `export function g(){ return register(${C}); }`,
    `export function o(){ function i(){ return [${C}]; } return i; }`,
    `export class K { m(){ return register(${C}); } static s = [${D}]; }`,
    `export const a = () => register(${C});`,
    `export const a2 = (x) => { const r = [${C}, ${D}]; return r[x]; };`,
    `export function l(n){ const out=[]; for (let i=0;i<n;i++){ out.push(register(${C})); } return out; }`,
    `export function m(){ return [register(${C}), [${D}], use(${C})]; }`,
    `export const arr = [${C}];`,
  ];
  const orders = [
    (stylex) => [stylex, removeUnused],
    (stylex) => [stylex, removeUnusedOnExit],
    (stylex) => [removeUnused, stylex],
  ];
  for (const shape of shapes) {
    const source = `import * as stylex from '@stylexjs/stylex';\n${shape}\n`;
    const plain = transform(source).code;
    for (const order of orders) {
      const { code } = babel.transformSync(source, {
        filename,
        babelrc: false,
        configFile: false,
        plugins: order([plugin, {}]),
      });
      assert.equal(code, plain, shape);
    }
  }
});

test('definitions an earlier visitor rewrote in place compile as rewritten', () => {
  const t = babel.types;
  const recolor = (at) => () => ({
    visitor: {
      [at](path) {
        const call = at === 'VariableDeclarator' ? path.node.init : path.node;
        if (
          call?.type !== 'CallExpression' ||
          call.callee.type !== 'MemberExpression' ||
          call.recolored
        )
          return;
        call.recolored = true;
        t.traverseFast(call.arguments[0], (node) => {
          if (node.type === 'StringLiteral' && node.value === 'red')
            node.value = 'green';
        });
      },
    },
  });
  const renameEarly = () => ({
    visitor: {
      Program(program) {
        program.traverse({
          StringLiteral(p) {
            if (p.node.value === 'unrelated') {
              p.node.value = 'changed';
              delete p.node.extra;
            }
          },
        });
      },
    },
  });
  const recolorLater = () => ({
    visitor: {
      StringLiteral(p) {
        if (p.node.value === 'red') {
          p.node.value = 'green';
          delete p.node.extra;
        }
      },
    },
  });
  const head = "import * as stylex from '@stylexjs/stylex';\n";
  const styles = (className) =>
    `{\n  a: {\n    kMwMTN: "${className}",\n    $$css: true\n  }\n}`;
  const green = [
    ['x1prwzq3', { ltr: '.x1prwzq3{color:green}', rtl: null }, 3000],
  ];
  const cases = [
    [
      "export const s = stylex.create({a: {color: 'red'}});\n",
      [recolor('VariableDeclarator')],
      `export const s = ${styles('x1prwzq3')};`,
      green,
    ],
    [
      "const s = stylex.create({a: {color: 'red'}});\nexport const p = stylex.props(s.a);\n",
      [recolor('VariableDeclarator')],
      'export const p = {\n  className: "x1prwzq3"\n};',
      green,
    ],
    [
      "export const arr = [stylex.create({a: {color: 'red'}})];\n",
      [recolor('CallExpression')],
      `export const arr = [${styles('x1prwzq3')}];`,
      green,
    ],
    // The compile read Babel's printing (an earlier Program.enter edit).
    [
      "const label = 'unrelated';\nexport const s = stylex.create({a: {color: 'red'}});\n",
      [renameEarly, recolor('CallExpression')],
      `const label = "changed";\nexport const s = ${styles('x1prwzq3')};`,
      green,
    ],
    // A descendant visitor runs after the definition compiled, as in the original.
    [
      "export const s = stylex.create({a: {color: 'red'}});\n",
      [recolorLater],
      `export const s = ${styles('x1e2nbdu')};`,
      [['x1e2nbdu', { ltr: '.x1e2nbdu{color:red}', rtl: null }, 3000]],
    ],
  ];
  for (const [source, before, code, metadata] of cases) {
    const result = transform(head + source, {}, before);
    assert.equal(result.code, head + code);
    assert.deepEqual(result.metadata.stylex, metadata);
  }
  // The same in a preset pass of its own (passPerPreset).
  const [source, before, code, metadata] = cases[0];
  const result = babel.transformSync(head + source, {
    filename,
    babelrc: false,
    configFile: false,
    passPerPreset: true,
    presets: [{ plugins: [...before, [plugin, {}]] }],
  });
  assert.equal(result.code, head + code);
  assert.deepEqual(result.metadata.stylex, metadata);
});

test('stale definitions rebuild their consumers and track what they read', () => {
  const t = babel.types;
  const recolorOnEnter = () => ({
    visitor: {
      Program: {
        enter(program) {
          program.traverse({
            StringLiteral(p) {
              if (p.node.value === 'red') p.node.value = 'green';
            },
          });
        },
      },
    },
  });
  const recolorDeclarator = (name) => () => ({
    visitor: {
      VariableDeclarator(path) {
        if (path.node.id.name !== name || path.node.recolored) return;
        path.node.recolored = true;
        t.traverseFast(path.node.init, (node) => {
          if (node.type === 'StringLiteral' && node.value === 'red')
            node.value = 'green';
        });
      },
    },
  });
  const react = require.resolve('@babel/preset-react');
  const head = "import * as stylex from '@stylexjs/stylex';\n";
  const green = [
    ['x1prwzq3', { ltr: '.x1prwzq3{color:green}', rtl: null }, 3000],
  ];
  const object = '{\n  a: {\n    kMwMTN: "x1prwzq3",\n    $$css: true\n  }\n}';
  const cases = [
    [
      "const s = stylex.create({a: {color: 'red'}});\nexport const C = () => <div {...stylex.props(s.a)} />;\n",
      { after: [recolorOnEnter], presets: [react] },
      'export const C = () => /*#__PURE__*/React.createElement("div", {\n  className: "x1prwzq3"\n});',
    ],
    [
      "const base = {color: 'red'};\nexport const s = stylex.create({a: {...base}});\n",
      { after: [recolorOnEnter] },
      `const base = {\n  color: "green"\n};\nexport const s = ${object};`,
    ],
    [
      "const base = {color: 'red'};\nexport const s = stylex.create({a: {...base}});\n",
      { before: [recolorDeclarator('base')] },
      `const base = {\n  color: "green"\n};\nexport const s = ${object};`,
    ],
    [
      "const C = 'red';\nexport const s = stylex.create({a: {color: C}});\n",
      { before: [recolorDeclarator('C')] },
      `const C = "green";\nexport const s = ${object};`,
    ],
  ];
  for (const [
    source,
    { before = [], after = [], presets = [] },
    code,
  ] of cases) {
    const result = babel.transformSync(head + source, {
      filename,
      babelrc: false,
      configFile: false,
      parserOpts: { plugins: ['jsx'] },
      presets,
      plugins: [...before, [plugin, {}], ...after],
    });
    assert.equal(result.code, head + code);
    assert.deepEqual(result.metadata.stylex, green);
  }
});

test('definitions StyleX applies itself do not make their readers stale', () => {
  const compiler = require('../index');
  const transformSync = compiler.transformSync;
  let compiles = 0;
  compiler.transformSync = (...args) => {
    compiles++;
    return transformSync(...args);
  };
  try {
    const refresh = [
      require.resolve('react-refresh/babel'),
      { skipEnvCheck: true },
    ];
    // `anim` reads the keyframes `fade`, which StyleX replaces at its visit.
    const source = `import * as stylex from '@stylexjs/stylex';
const fade = stylex.keyframes({from: {opacity: 0}});
const anim = stylex.create({a: {animationName: fade}});
export const C = () => <div {...stylex.props(anim.a)} />;
`;
    const alone = transform(source, {}, [], ['jsx']);
    compiles = 0;
    const withRefresh = babel.transformSync(source, {
      filename,
      babelrc: false,
      configFile: false,
      parserOpts: { plugins: ['jsx'] },
      plugins: [[plugin, {}], refresh],
    });
    assert.equal(compiles, 1);
    assert.deepEqual(withRefresh.metadata.stylex, alone.metadata.stylex);
    assert.ok(withRefresh.code.includes('<div className="x1j4p4jw" />'));
  } finally {
    compiler.transformSync = transformSync;
  }
});

// A lowered consumer cannot be recompiled, and stale class names would lack CSS.
test('a definition rewritten after its consumer was lowered fails the build', () => {
  const t = babel.types;
  const recolor = (name) => () => ({
    visitor: {
      VariableDeclarator(path) {
        if (path.node.id.name !== name || path.node.recolored) return;
        path.node.recolored = true;
        t.traverseFast(path.node.init, (node) => {
          if (node.type === 'StringLiteral' && node.value === 'red')
            node.value = 'green';
        });
      },
    },
  });
  const react = require.resolve('@babel/preset-react');
  const run = (source, plugins) =>
    babel.transformSync(source, {
      filename,
      babelrc: false,
      configFile: false,
      highlightCode: false,
      presets: [react],
      plugins: [...plugins, [plugin, {}]],
    });
  const head = "import * as stylex from '@stylexjs/stylex';\n";
  const consumer =
    'export function C(){ return <div {...stylex.props(s.a)} />; }\n';
  const message = (definition) =>
    `${filename}: A plugin rewrote the StyleX definition at ${definition} (or a value it reads) during the traversal, after its consumer at 2:34 was compiled and transformed by other plugins, so the consumer would keep class names without CSS. Move the definition above its consumers, or run that plugin in an earlier pass.
  1 | import * as stylex from '@stylexjs/stylex';
> 2 | export function C(){ return <div {...stylex.props(s.a)} />; }
    |                                  ^^^^^^^^^^^^^^^^^^^^^^`;
  const refresh = [
    require.resolve('react-refresh/babel'),
    { skipEnvCheck: true },
  ];
  assert.throws(
    () =>
      run(
        `${head}export const C = () => <div sx={s.a} />;\nconst s = stylex.create({a: {color: 'red'}});\n`,
        [recolor('s')],
      ),
    (error) =>
      error.code === 'BABEL_TRANSFORM_ERROR' &&
      error.message.includes(
        'rewrote the StyleX definition at 3:11 (or a value it reads) during the traversal, after its consumer at 2:29',
      ),
  );
  for (const [source, plugins, definition] of [
    [
      `${head}${consumer}const s = stylex.create({a: {color: 'red'}});\n`,
      [recolor('s')],
      '3:11',
    ],
    [
      `${head}${consumer}const s = stylex.create({a: {color: 'red'}});\n`,
      [recolor('s'), refresh],
      '3:11',
    ],
    [
      `${head}${consumer}const base = {color: 'red'};\nconst s = stylex.create({a: {...base}});\n`,
      [recolor('base')],
      '4:11',
    ],
  ]) {
    assert.throws(
      () => run(source, plugins),
      (error) =>
        error.code === 'BABEL_TRANSFORM_ERROR' &&
        error.message.startsWith(message(definition)),
    );
  }
});

test('lowered consumers above their definitions compile normally otherwise', () => {
  const t = babel.types;
  const original = require('../../babel-plugin/lib/index.js');
  const recolor = (name) => () => ({
    visitor: {
      VariableDeclarator(path) {
        if (path.node.id.name !== name || path.node.recolored) return;
        path.node.recolored = true;
        t.traverseFast(path.node.init, (node) => {
          if (node.type === 'StringLiteral' && node.value === 'red')
            node.value = 'green';
        });
      },
    },
  });
  const recolorOnEnter = () => ({
    visitor: {
      Program: {
        enter(program) {
          program.traverse({
            StringLiteral(p) {
              if (p.node.value === 'red') p.node.value = 'green';
            },
          });
        },
      },
    },
  });
  const removeComponent = () => ({
    visitor: {
      FunctionDeclaration(path) {
        if (path.node.id.name === 'C') path.parentPath.remove();
      },
    },
  });
  const react = require.resolve('@babel/preset-react');
  const env = [require.resolve('@babel/preset-env'), { targets: 'ie 11' }];
  const refresh = [
    require.resolve('react-refresh/babel'),
    { skipEnvCheck: true },
  ];
  const head = "import * as stylex from '@stylexjs/stylex';\n";
  const bottom = `${head}export function C(){ return <div {...stylex.props(s.a)} />; }\nconst s = stylex.create({a: {color: 'red'}});\n`;
  const cases = [
    [bottom, [react], [], []],
    [bottom, [env, react], [], []],
    [bottom, [react], [refresh], []],
    [bottom, [react], [], [recolorOnEnter]],
    [bottom, [react], [removeComponent, recolor('s')], []],
    [
      `${head}const s = stylex.create({a: {color: 'red'}});\nexport function C(){ return <div {...stylex.props(s.a)} />; }\n`,
      [react],
      [recolor('s')],
      [],
    ],
    [
      `${head}export function C(){ return <div {...stylex.props(u.a)} />; }\nconst s = stylex.create({a: {color: 'red'}});\nconst u = stylex.create({a: {color: 'blue'}});\nexport {s};\n`,
      [react],
      [recolor('s')],
      [],
    ],
    [
      `${head}export function C(){ return <div {...stylex.props(s.a)} />; }\nconst other = {color: 'red'};\nconst s = stylex.create({a: {color: 'blue'}});\nexport {other};\n`,
      [react],
      [recolor('other')],
      [],
    ],
    // Another definition, consumed below it, is rewritten; this one is unchanged.
    [
      `${head}export function C(){ return <div {...stylex.props(s.a)} />; }\nconst s = stylex.create({a: {color: 'blue'}});\nconst u = stylex.create({b: {color: 'red'}});\nexport const U = stylex.props(u.b);\n`,
      [react],
      [recolor('u')],
      [],
    ],
    // Nothing lowers the consumer: it is undone and recompiled.
    [
      `${head}export function C(){ return stylex.props(s.a); }\nconst s = stylex.create({a: {color: 'red'}});\n`,
      [],
      [recolor('s')],
      [],
    ],
    // A dynamic consumer stays a runtime call.
    [
      `${head}export function C(x){ return <div {...stylex.props(x && s.a)} />; }\nconst s = stylex.create({a: {color: 'red'}});\n`,
      [env, react],
      [recolor('s')],
      [],
    ],
  ];
  for (const [source, presets, before, after] of cases) {
    const run = (stylex) =>
      babel.transformSync(source, {
        filename,
        babelrc: false,
        configFile: false,
        presets,
        plugins: [...before, [stylex, {}], ...after],
      });
    const expected = run(original);
    const result = run(plugin);
    assert.equal(result.code, expected.code);
    assert.deepEqual(result.metadata.stylex, expected.metadata.stylex);
  }
});

// Consumers link to definitions by Babel's bindings, not by name.
test('a parameter named like a rewritten definition does not hide it', () => {
  const t = babel.types;
  const recolor = () => ({
    visitor: {
      VariableDeclarator(path) {
        if (path.node.id.name !== 's' || path.node.recolored) return;
        path.node.recolored = true;
        t.traverseFast(path.node.init, (node) => {
          if (node.type === 'StringLiteral' && node.value === 'red')
            node.value = 'green';
        });
      },
    },
  });
  const result = babel.transformSync(
    `import * as stylex from '@stylexjs/stylex';
export function C(s){ return <div {...stylex.props(s.a)} />; }
const s = stylex.create({a: {color: 'red'}});
export const D = stylex.props(s.a);
`,
    {
      filename,
      babelrc: false,
      configFile: false,
      presets: [require.resolve('@babel/preset-react')],
      plugins: [recolor, [plugin, {}]],
    },
  );
  assert.ok(
    result.code.includes('export const D = {\n  className: "x1prwzq3"\n};'),
  );
  assert.deepEqual(result.metadata.stylex, [
    ['x1prwzq3', { ltr: '.x1prwzq3{color:green}', rtl: null }, 3000],
  ]);
});

test('declarations of different bindings are never paired by the graft', () => {
  const t = babel.types;
  const original = require('../../babel-plugin/lib/index.js');
  const isCreate = (node) =>
    node?.type === 'CallExpression' &&
    node.callee.type === 'MemberExpression' &&
    node.callee.property.name === 'create';
  const spy = (log) => () => ({
    visitor: {
      VariableDeclarator(path) {
        if (!isCreate(path.node.init)) return;
        let color = null;
        t.traverseFast(path.node.init, (node) => {
          if (node.type === 'StringLiteral') color ??= node.value;
        });
        log.push(`${path.node.id.name}@${path.node.loc?.start.line}:${color}`);
      },
    },
  });
  const recolor = (name) => () => ({
    visitor: {
      VariableDeclarator(path) {
        if (path.node.id.name !== name || path.node.recolored) return;
        path.node.recolored = true;
        t.traverseFast(path.node.init, (node) => {
          if (node.type === 'StringLiteral') node.value = 'green';
        });
      },
    },
  });
  const wrap = (name) => () => ({
    visitor: {
      VariableDeclarator(path) {
        if (path.node.id.name !== name || path.node.wrapped) return;
        path.node.wrapped = true;
        path.node.init = t.callExpression(t.identifier('freeze'), [
          path.node.init,
        ]);
      },
    },
  });
  const head = "import * as stylex from '@stylexjs/stylex';\n";
  const cases = [
    [
      "const s = stylex.create({a: {color: 'red'}});\nconst u = stylex.create({a: {color: 'blue'}});\nexport const A = u;\n",
      [],
    ],
    [
      "const s = stylex.create({a: {color: 'red'}});\nconst w = stylex.create({a: {color: 'teal'}});\nconst u = stylex.create({a: {color: 'blue'}});\nexport const B = [u];\n",
      [],
    ],
    [
      "const s = stylex.create({a: {color: 'red'}});\nconst u = stylex.create({a: {color: 'blue'}});\nexport const A = u;\n",
      [recolor('u')],
    ],
    [
      "const s = stylex.create({a: {color: 'red'}});\nconst u = stylex.create({a: {color: 'blue'}});\nexport const A = u;\n",
      [wrap('u')],
    ],
  ];
  for (const [source, before] of cases) {
    const run = (stylex) => {
      const log = [];
      const result = babel.transformSync(head + source, {
        filename,
        babelrc: false,
        configFile: false,
        plugins: [spy(log), ...before, [stylex, {}]],
      });
      return { code: result.code, metadata: result.metadata.stylex, log };
    };
    assert.deepEqual(run(plugin), run(original));
  }
});

// Any other origin or earlier plugin hook compiles the program, not the text.
test('only an untouched parse of the file skips the comparison with its text', () => {
  const t = babel.types;
  const parser = require('@babel/parser');
  const original = require('../../babel-plugin/lib/index.js');
  const source = `import * as stylex from '@stylexjs/stylex';
const unused = stylex.create({u: {color: 'gray'}});
const styles = stylex.create({root: {color: 'red'}});
export const C = stylex.props(styles.root);
`;
  const other = `import * as stylex from '@stylexjs/stylex';
const styles = stylex.create({root: {color: 'blue'}});
export const D = stylex.props(styles.root);
`;
  const recolor = (ast) =>
    t.traverseFast(ast, (node) => {
      if (node.type === 'StringLiteral' && node.value === 'red')
        node.value = 'green';
    });
  const recolorInPre = () => ({
    pre(file) {
      recolor(file.ast);
    },
  });
  const recolorOnEnter = () => ({
    visitor: {
      Program: {
        enter(program) {
          recolor(program.node);
        },
      },
    },
  });
  const options = { filename, babelrc: false, configFile: false };
  const parse = (code) => parser.parse(code, { sourceType: 'module' });
  const cases = [
    (plugin) =>
      babel.transformSync(source, {
        ...options,
        plugins: [recolorInPre, [plugin, {}]],
      }),
    (plugin) =>
      babel.transformSync(source, {
        ...options,
        plugins: [[plugin, {}], recolorInPre],
      }),
    (plugin) =>
      babel.transformSync(source, {
        ...options,
        plugins: [recolorOnEnter, [plugin, {}]],
      }),
    (plugin) =>
      babel.transformSync(source, {
        ...options,
        passPerPreset: true,
        presets: [{ plugins: [recolorOnEnter] }, { plugins: [[plugin, {}]] }],
      }),
    // An AST handed to Babel describes the program, not the text beside it.
    (plugin) =>
      babel.transformFromAstSync(parse(other), source, {
        ...options,
        plugins: [[plugin, {}]],
      }),
    // A caller-edited parser AST, cloned or not: Babel prints the AST, not text.
    ...[true, false].map((cloneInputAst) => (plugin) => {
      const ast = parse(source);
      recolor(ast);
      return babel.transformFromAstSync(ast, source, {
        ...options,
        cloneInputAst,
        plugins: [[plugin, {}]],
      });
    }),
    // Unedited, but still not Babel's parse of this run.
    (plugin) =>
      babel.transformFromAstSync(parse(source), source, {
        ...options,
        cloneInputAst: false,
        plugins: [[plugin, {}]],
      }),
  ];
  // The default clone leaves no parser prototype behind, as the facade assumes.
  const prototypeOf = (ast) => Object.getPrototypeOf(ast.program);
  const parsed = prototypeOf(parse(source));
  let seen = null;
  babel.transformFromAstSync(parse(source), source, {
    ...options,
    plugins: [
      () => ({
        visitor: {
          Program(path) {
            seen = Object.getPrototypeOf(path.node);
          },
        },
      }),
    ],
  });
  assert.notEqual(parsed, Object.prototype);
  assert.equal(seen, Object.prototype);
  // Edited parser ASTs handed over without a clone, text omitted or empty.
  const handedOver =
    (edit, code = source) =>
    (plugin) => {
      const ast = babel.parseSync(source, options);
      edit(ast);
      return babel.transformFromAstSync(ast, code, {
        ...options,
        cloneInputAst: false,
        plugins: [[plugin, {}]],
      });
    };
  const statement = (text) => babel.template.statement.ast(text);
  cases.push(
    handedOver((ast) => {
      ast.program.body.unshift(statement("import 'polyfill';"));
      ast.program.body.push(statement('window.ready = true;'));
    }),
    handedOver(recolor),
    handedOver((ast) => {
      ast.program.body.push(
        statement('export const late = stylex.props(styles.root);'),
      );
    }),
    handedOver(() => {}, undefined),
    handedOver(() => {}, ''),
  );
  // Wrappers that edit before StyleX's Program.enter or add StyleX right after it.
  const wrapped = (wrap) => (plugin) =>
    babel.transformSync(source, {
      ...options,
      wrapPluginVisitorMethod: wrap,
      plugins: [[plugin, {}]],
    });
  cases.push(
    wrapped(
      (key, visitorType, fn) =>
        function (path, state) {
          if (path.isProgram() && !path.node.recolored) {
            path.node.recolored = true;
            recolor(path.node);
          }
          return fn.call(this, path, state);
        },
    ),
    wrapped(
      (key, visitorType, fn) =>
        function (path, state) {
          const result = fn.call(this, path, state);
          if (path.isProgram() && visitorType === 'enter' && !path.node.added) {
            path.node.added = true;
            path.pushContainer(
              'body',
              statement('export const late = stylex.props(styles.root);'),
            );
          }
          return result;
        },
    ),
  );
  for (const run of cases) {
    const expected = run(original);
    const result = run(plugin);
    assert.equal(result.code, expected.code);
    assert.deepEqual(result.metadata.stylex, expected.metadata.stylex);
  }
});

test('StyleX-free text is skipped only where it cannot spell a source', () => {
  const t = babel.types;
  const original = require('../../babel-plugin/lib/index.js');
  const options = { filename, babelrc: false, configFile: false };
  const run = (stylex, source, plugins = [], extra = {}) => {
    const result = babel.transformSync(source, {
      ...options,
      ...extra,
      plugins: [
        ...(plugins.before ?? []),
        [stylex, {}],
        ...(plugins.after ?? []),
      ],
    });
    return { code: result.code, metadata: result.metadata.stylex };
  };
  const create = "stylex.create({root: {color: 'red'}})";
  const sources = [
    // Escapes that spell the source: unicode, hex, identity, line continuation.
    `import * as stylex from '\\u0040stylexjs/stylex';\nexport const s = ${create};\n`,
    `import * as stylex from '@stylexjs\\x2fstylex';\nexport const s = ${create};\n`,
    `import * as stylex from '@\\stylexjs/stylex';\nexport const s = ${create};\n`,
    `import * as stylex from '@stylexjs/sty\\\nlex';\nexport const s = ${create};\n`,
    `const stylex = require('\\u0040stylexjs/stylex');\nexport const s = ${create};\n`,
    // Free of StyleX, with escapes that cannot spell a source.
    'export const message = "a\\nb" + `c\\td`;\n',
  ];
  for (const source of sources)
    assert.deepEqual(run(plugin, source), run(original, source));

  // Another plugin adds StyleX at the end of the traversal, before or after ours.
  const addStyleX = () => ({
    visitor: {
      Program: {
        exit(program) {
          program.unshiftContainer(
            'body',
            babel.template.statement.ast(
              "import * as stylex from '@stylexjs/stylex';",
            ),
          );
          program.pushContainer(
            'body',
            babel.template.statement.ast(`export const late = ${create};`),
          );
        },
      },
    },
  });
  const free = 'export const value = 1;\n';
  for (const plugins of [{ after: [addStyleX] }, { before: [addStyleX] }])
    assert.deepEqual(run(plugin, free, plugins), run(original, free, plugins));
  void t;
});
