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
  ['wrapped length', 'const lit = [1, 2];\n', 'String([...lit, , 3].length)'],
  ['index', 'const lit = [1, 2];\n', '[...lit, , 3][1]'],
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

// Approved spread support may reach an operand hole Babel never evaluates.
// These cells keep refusing with SyntaxError; their exact baseline/hole message
// alternatives are checked by the independent per-cell matrix.
test('spread-operand holes remain refused', () => {
  const prelude = "const holey = ['x', , 'y'];\n";
  for (const value of ['[...holey].length', "[...holey].join('-')", '[...holey][1]']) {
    for (const engine of ['direct', 'facade']) {
      const actual = failure(engine, prelude, value);
      assert.equal(actual.kind, 'SyntaxError');
      assert.equal(actual.code, 'BABEL_TRANSFORM_ERROR');
    }
  }
});

for (const [label, prelude, value] of [
  ['leading join', '', "[, 'a'].join('-')"],
  ['middle join', '', "[1, , 2].join('-')"],
  ['leading spread join', '', "[, ...['a']].join('')"],
  ['map length', '', '[, 1].map(x => x * 2).length'],
  ['concat length', '', '[, 1].concat([2]).length'],
  ['bound join', "const d = [, 'a'];\n", "d.join('-')"],
  ['map index', '', '[1, , 2].map(x => x)[0]'],
]) {
  for (const engine of ['direct', 'facade']) {
    test(`call-wrapped source hole ${label} matches Babel (${engine})`, () => {
      const expected = failure('babel', prelude, value);
      assert.equal(expected.kind, 'SyntaxError');
      assert.equal(expected.code, 'BABEL_TRANSFORM_ERROR');
      assert.ok(expected.message.startsWith(`${filename}: Unsupported expression: CallExpression\n`));
      assert.deepEqual(failure(engine, prelude, value), expected);
    });
  }
}

test('spread-operand hole diagnostics preserve their complete base frames', () => {
  const holeText = `${filename}: Unexpected error:\nCould not resolve the code being evaluated.\n`;
  for (const [prelude, value, frame] of [
    ["const holey = ['x', , 'y'];\n", '[...holey].length',
      "  1 | import * as stylex from '@stylexjs/stylex';\n" +
      "  2 | const holey = ['x', , 'y'];\n" +
      '> 3 | export const s = stylex.create({ x: { width: [...holey].length } });\n' +
      '    |                                                  ^^^^^'],
    ['', '[...[, 1]].length',
      "  1 | import * as stylex from '@stylexjs/stylex';\n" +
      '> 2 | export const s = stylex.create({ x: { width: [...[, 1]].length } });\n' +
      '    |                                                  ^^^^^'],
    ["const holey = ['x', , 'y'];\n", '((a,b,c)=>a+b+c)(...holey)',
      "  1 | import * as stylex from '@stylexjs/stylex';\n" +
      "> 2 | const holey = ['x', , 'y'];\n" +
      '    |               ^^^^^^^^^^^^\n' +
      '  3 | export const s = stylex.create({ x: { width: ((a,b,c)=>a+b+c)(...holey) } });'],
    ['', 'Math.max(...[1, , 3])',
      "  1 | import * as stylex from '@stylexjs/stylex';\n" +
      '> 2 | export const s = stylex.create({ x: { width: Math.max(...[1, , 3]) } });\n' +
      '    |                                                          ^^^^^^^^'],
    ['', 'Math.min(...[, 2])',
      "  1 | import * as stylex from '@stylexjs/stylex';\n" +
      '> 2 | export const s = stylex.create({ x: { width: Math.min(...[, 2]) } });\n' +
      '    |                                                          ^^^^^'],
  ]) {
    for (const engine of ['direct', 'facade']) {
      assert.deepEqual(failure(engine, prelude, value), {
        kind: 'SyntaxError',
        code: 'BABEL_TRANSFORM_ERROR',
        message: holeText + frame,
      }, `${engine}: ${value}`);
    }
  }
});

test('a hole in the argument is not relabeled as a receiver-call refusal', () => {
  const value = "['a'].join([, 1])";
  const expected = failure('babel', '', value);
  assert.equal(expected.message, `${filename}: Unexpected error:\nCould not resolve the code being evaluated.\n`);
  for (const engine of ['direct', 'facade']) {
    assert.deepEqual(failure(engine, '', value), expected);
  }
});

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
