/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
'use strict';

// Babel visitor validation contracts, checked against the native facade.
const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');
const babel = require('@babel/core');
const { NodePath } = require('@babel/traverse');
const reference = require('@stylexjs/babel-plugin');
const native = require('../babel');
const filename = path.resolve(__dirname, 'Validation.stylex.js');
const prefix = "import * as stylex from '@stylexjs/stylex';\n";

function transform(plugin, source) {
  return babel.transformSync(source, {
    filename,
    babelrc: false,
    configFile: false,
    highlightCode: false,
    plugins: [
      [plugin, { debug: false, dev: false, treeshakeCompensation: false }],
    ],
  });
}

function failure(plugin, source) {
  const originalBuildFrame = NodePath.prototype.buildCodeFrameError;
  let frame = null;
  // Record Babel's real deopt frames; restored after each synchronous transform.
  NodePath.prototype.buildCodeFrameError = function (message, ...rest) {
    frame = { start: this.node.start, end: this.node.end };
    return originalBuildFrame.call(this, message, ...rest);
  };
  try {
    transform(plugin, source);
    assert.fail(`Expected a validation error for ${source}`);
  } catch (error) {
    assert.notEqual(error.code, 'ERR_ASSERTION');
    // Native offsets are UTF-8 bytes; Babel's positions are UTF-16 code units.
    const range =
      error.stylexDiagnostic != null
        ? (error.stylexDiagnostic.range ?? null)
        : frame == null
          ? null
          : {
              start: Buffer.byteLength(source.slice(0, frame.start), 'utf8'),
              end: Buffer.byteLength(source.slice(0, frame.end), 'utf8'),
            };
    return { name: error.name, message: error.message, range };
  } finally {
    NodePath.prototype.buildCodeFrameError = originalBuildFrame;
  }
}

function compareFailure(body, reason) {
  const source = prefix + body;
  const expected = failure(reference, source);
  assert.ok(
    expected.message.includes(reason),
    'current source contract changed',
  );
  assert.ok(expected.range != null, 'Babel must report the actual deopt range');
  assert.deepEqual(failure(native, source), expected, source);
}

// Babel drops only the key's evaluator reason (parse-stylex-create-arg.js:65-66).
test('create namespace-key deopt uses the visitor fallback and exact key frame', () => {
  compareFailure(
    "export const styles = stylex.create({[missing]: {color: 'red'}});",
    'Only static values are allowed inside of a create() call.',
  );
});

// Values keep their evaluator reason (parse-stylex-create-arg.js:71-75).
test('create property and value deopts retain their evaluator reasons and frames', () => {
  for (const body of [
    "export const styles = stylex.create({root: {[missing]: 'red'}});",
    'export const styles = stylex.create({root: {color: missing}});',
    'export const styles = stylex.create({root: {color: makeColor()}});',
  ]) {
    compareFailure(body, 'Referenced constant is not defined.');
  }
});

// Babel reports object-method faults at the method and binding faults at their
// declaration (evaluate-path.js:879-886).
for (const [name, body, reason] of [
  [
    'JSON.stringify call',
    'export const s=stylex.create({a:{color:JSON.stringify({a:1})}});',
    'Unsupported expression: CallExpression',
  ],
  [
    'object destructuring present value',
    "const {red,blue:b='x'}={red:'red'};export const s=stylex.create({a:{color:red}});",
    'Unsupported expression: VariableDeclarator',
  ],
  [
    'object destructuring default value',
    "const {red,blue:b='x'}={red:'red'};export const s=stylex.create({a:{color:b}});",
    'Unsupported expression: VariableDeclarator',
  ],
  [
    'array destructuring',
    "const [red]=['red'];export const s=stylex.create({a:{color:red}});",
    'Unsupported expression: VariableDeclarator',
  ],
  [
    'nested destructuring',
    "const {nested:{red}}={nested:{red:'red'}};export const s=stylex.create({a:{color:red}});",
    'Unsupported expression: VariableDeclarator',
  ],
  [
    'global parseInt call',
    "export const s=stylex.create({a:{color:parseInt('12px')}});",
    'Referenced constant is not defined.',
  ],
  [
    'global parseFloat call',
    "export const s=stylex.create({a:{color:parseFloat('1')}});",
    'Referenced constant is not defined.',
  ],
  [
    'getter property read',
    "const o={get c(){return 'red';}};export const s=stylex.create({a:{color:o.c}});",
    'Unsupported object method.',
  ],
  [
    'this-mutating method',
    "const o={color:'red',m(){this.color='blue';}};o.m();export const s=stylex.create({a:{color:o.color}});",
    'Unsupported object method.',
  ],
  [
    'thenable method',
    "const o={color:'red',then(r){r();}};export const s=stylex.create({a:{color:o.color}});",
    'Unsupported object method.',
  ],
  [
    'namespace theme read',
    "import * as T from './lib/tokens.stylex';export const s=stylex.create({a:{color:T.colors.primary}});",
    'Referenced constant is not defined.',
  ],
  [
    'closure with mutating block body',
    'const o={w:1};const f=()=>{o.w=5;return 0;};export const s=stylex.create({a:{color:f()}});',
    'Unsupported expression: ArrowFunctionExpression',
  ],
  [
    'enclosing arrow parameter',
    'export const f=(x)=>{const s=stylex.create({a:{color:x}});return s;};',
    'Unsupported expression: Identifier',
  ],
  [
    'enclosing function parameter',
    'export function f(x){const s=stylex.create({a:{color:x}});return s;}',
    'Unsupported expression: Identifier',
  ],
  [
    'function-expression parameter',
    'const f=function(p){return stylex.create({a:{color:p}});};',
    'Unsupported expression: Identifier',
  ],
]) {
  test(`PR-head deopt diagnostic: ${name}`, () => {
    compareFailure(body, reason);
  });
}

// validateStyleXCreateTheme checks the call's arity before any evaluation.
test('createTheme keeps two-argument arity diagnostics', () => {
  for (const argumentsText of ['', '{}', '{}, {}, {}']) {
    compareFailure(
      `const theme = stylex.createTheme(${argumentsText});`,
      'createTheme() should have 2 arguments.',
    );
  }
});

// Both errors belong to the call (stylex-create-theme.js:146-164).
test('createTheme evaluates call-shaped overrides before reporting a deopt', () => {
  compareFailure(
    "const theme = stylex.createTheme({__varGroupHash__: 'xbase'}, makeOverrides());",
    'Only static values are allowed inside of a createTheme() call.',
  );
});

test('createTheme checks the evaluated override shape at the original call', () => {
  compareFailure(
    "const makeOverrides = () => null; const theme = stylex.createTheme({__varGroupHash__: 'xbase'}, makeOverrides());",
    'createTheme() can only accept an object.',
  );
});

test('createTheme preserves first-argument evaluation and diagnostic precedence', () => {
  compareFailure(
    'const theme = stylex.createTheme(makeBase(), makeOverrides());',
    'Only static values are allowed inside of a createTheme() call.',
  );
});

test('createTheme checks override shape before validating the theme group', () => {
  for (const argumentsText of ['1, null', '1, 1', '{}, null']) {
    compareFailure(
      `const theme = stylex.createTheme(${argumentsText});`,
      'createTheme() can only accept an object.',
    );
  }
});

test('createTheme frames primitive themes and identifier deopts at the call', () => {
  for (const [body, reason] of [
    [
      'const t = stylex.createTheme(1, {});',
      'Can only override variables theme created with defineVars().',
    ],
    [
      "const t = stylex.createTheme({__varGroupHash__: 'x'}, {a: y});",
      'Only static values are allowed inside of a createTheme() call.',
    ],
    [
      'const t = stylex.createTheme(x, {});',
      'Only static values are allowed inside of a createTheme() call.',
    ],
    [
      "const t = stylex.createTheme({__varGroupHash__: 'x'}, 1);",
      'createTheme() can only accept an object.',
    ],
  ]) {
    compareFailure(body, reason);
  }
});

function clean(value) {
  if (Array.isArray(value)) return value.map(clean);
  if (value != null && typeof value === 'object') {
    return Object.fromEntries(
      Object.entries(value)
        .filter(
          ([key]) =>
            ![
              'start',
              'end',
              'loc',
              'extra',
              'leadingComments',
              'trailingComments',
              'innerComments',
              'comments',
              'tokens',
              'errors',
            ].includes(key),
        )
        .map(([key, item]) => [key, clean(item)]),
    );
  }
  return value;
}

test('createTheme accepts a statically evaluated call-shaped override object', () => {
  const source =
    prefix +
    "const makeOverrides = () => ({color: 'blue'}); export const theme = stylex.createTheme({__varGroupHash__: 'xbase', color: 'var(--color)'}, makeOverrides());";
  const expected = transform(reference, source);
  const actual = transform(native, source);
  assert.deepEqual(actual.metadata.stylex, expected.metadata.stylex);
  const parse = (code) =>
    clean(babel.parseSync(code, { babelrc: false, configFile: false }).program);
  assert.deepEqual(parse(actual.code), parse(expected.code));
});
