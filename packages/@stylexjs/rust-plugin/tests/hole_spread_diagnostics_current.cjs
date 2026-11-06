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
  for (const engine of ['direct', 'facade']) {
    test(`hole-first ${label} length matches Babel's unlocated SyntaxError (${engine})`, () => {
      const expected = failure('babel', prelude, value);
      assert.deepEqual(expected, {
        kind: 'SyntaxError',
        code: 'BABEL_TRANSFORM_ERROR',
        message: `${filename}: Unexpected error:\nCould not resolve the code being evaluated.\n`,
      });
      assert.deepEqual(failure(engine, prelude, value), expected, engine);
    });
  }
}

// Source holes refuse before a property read, even when there is no spread.
for (const [label, prelude, value] of [
  ['empty-slot length', '', '[,].length'],
  ['leading-slot length', '', '[, 1].length'],
  ['middle-slot length', '', '[1, , 2].length'],
  ['parenthesized length', '', '([, 1]).length'],
  ['bound array length', 'const a = [, ...[1, 2]];\n', 'a.length'],
  ['index', '', '[, ...[1, 2]][1]'],
]) {
  for (const engine of ['direct', 'facade']) {
    test(`source hole ${label} matches Babel (${engine})`, () => {
      const expected = failure('babel', prelude, value);
      assert.deepEqual(expected, {
        kind: 'SyntaxError',
        code: 'BABEL_TRANSFORM_ERROR',
        message: `${filename}: Unexpected error:\nCould not resolve the code being evaluated.\n`,
      });
      assert.deepEqual(failure(engine, prelude, value), expected);
    });
  }
}

// A spread reached first in a hole-containing source literal keeps Babel's
// SpreadElement refusal. The general rule now reports the actual spread span.
for (const [label, prelude, value] of [
  ['literal', '', '[...[1, 2], , 3].length'],
  ['binding', 'const lit = [1, 2];\n', '[...lit, , 3].length'],
  ['index', '', '[...[1, 2], , 3][1]'],
]) {
  for (const engine of ['direct', 'facade']) {
    test(`spread-first ${label} source hole matches Babel (${engine})`, () => {
      const expected = failure('babel', prelude, value);
      assert.equal(expected.kind, 'SyntaxError');
      assert.equal(expected.code, 'BABEL_TRANSFORM_ERROR');
      assert.ok(expected.message.startsWith(`${filename}: Unsupported expression: SpreadElement\n`));
      assert.deepEqual(failure(engine, prelude, value), expected);
    });
  }
}

test('host-provided sparse arrays keep their existing values and slot counts', () => {
  for (const value of ['stylex.env.list.length', 'Object.keys(stylex.env.list).length']) {
    const source = `${header}export const s=stylex.create({x:{width:${value}}});`;
    const sparse = [2, , 4];
    const expected = runEngine('babel', source, filename, { ...options, env: { list: sparse } });
    for (const engine of ['direct', 'facade']) {
      assert.deepEqual(
        runEngine(engine, source, filename, { ...options, env: { list: sparse } }).metadata.stylex,
        expected.metadata.stylex,
      );
    }
  }
});
