'use strict';

const { test } = require('node:test');
const assert = require('node:assert/strict');
const { runEngine } = require('./support/babel_errors.cjs');

const filename = '/app/Probe.js';
const header = "import * as stylex from '@stylexjs/stylex';\n";
const options = {
  classNamePrefix: 'x',
  dev: false,
  test: false,
  runtimeInjection: false,
  unstable_moduleResolution: { type: 'commonJS', rootDir: '/app' },
};

function failure(engine, prelude, value) {
  const source = `${header}${prelude}export const s = stylex.create({ x: { width: ${value} } });`;
  let caught;
  try {
    runEngine(engine, source, filename, { ...options });
  } catch (error) {
    caught = error;
  }
  assert.ok(caught, `${engine} must keep refusing ${value}`);
  return {
    kind: caught.constructor.name,
    code: caught.code,
    message: caught.message,
  };
}

// Executed unchanged Babel: buildCodeFrameError receives an absent array
// element, so it throws SyntaxError but has no node location to underline.
for (const [label, prelude, value] of [
  ['literal', '', '[, ...[1, 2]].length'],
  ['binding', 'const lit = [1, 2];\n', '[, ...lit].length'],
  ['wrapped', 'const lit = [1, 2];\n', 'String([, ...lit].length)'],
]) {
  test(`hole-first ${label} length matches Babel's unlocated SyntaxError`, () => {
    const expected = failure('babel', prelude, value);
    assert.deepEqual(expected, {
      kind: 'SyntaxError',
      code: 'BABEL_TRANSFORM_ERROR',
      message: `${filename}: Unexpected error:\nCould not resolve the code being evaluated.\n`,
    });
    for (const engine of ['direct', 'facade']) {
      assert.deepEqual(failure(engine, prelude, value), expected, engine);
    }
  });
}

// Pin the complete native messages captured from the exact 2d78c64 addon.
// This follow-up does not change the pre-existing spread-first frame extent.
for (const [label, prelude, value, lines] of [
  [
    'literal', '', '[...[1, 2], , 3].length',
    "  1 | import * as stylex from '@stylexjs/stylex';\n" +
      '> 2 | export const s = stylex.create({ x: { width: [...[1, 2], , 3].length } });\n' +
      '    |                                              ^^^^^^^^^^^^^^^^^^^^^^^',
  ],
  [
    'binding', 'const lit = [1, 2];\n', '[...lit, , 3].length',
    "  1 | import * as stylex from '@stylexjs/stylex';\n" +
      '  2 | const lit = [1, 2];\n' +
      '> 3 | export const s = stylex.create({ x: { width: [...lit, , 3].length } });\n' +
      '    |                                              ^^^^^^^^^^^^^^^^^^^^',
  ],
]) {
  test(`spread-first ${label} length keeps its existing full frame`, () => {
    const expected = {
      kind: 'SyntaxError',
      code: 'BABEL_TRANSFORM_ERROR',
      message: `${filename}: Unsupported expression: SpreadElement\n\n\n${lines}`,
    };
    const babel = failure('babel', prelude, value);
    assert.equal(babel.kind, expected.kind);
    assert.equal(babel.code, expected.code);
    assert.equal(babel.message.split('\n')[0], expected.message.split('\n')[0]);
    for (const engine of ['direct', 'facade']) {
      assert.deepEqual(failure(engine, prelude, value), expected, engine);
    }
  });
}
