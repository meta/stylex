/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const fs = require('node:fs');

const {
  exactPart,
  getExpectedPath,
  getFixtureNames,
  isJsEquivalent,
  jsPart,
  loadFixture,
  normalizeCss,
  normalizeData,
  normalizeDiagnostic,
  normalizePaths,
  normalizeResult,
} = require('../src/index');

const FIXTURE = {
  dir: '/repo/packages/compiler-conformance/fixtures/demo',
  entry: 'input.js',
  repoRoot: '/repo',
};
const CONTEXT = {
  entry: FIXTURE.entry,
  fixtureDir: FIXTURE.dir,
  repoRoot: FIXTURE.repoRoot,
};

describe('normalizePaths', () => {
  test('replaces the fixture directory before the repository root', () => {
    expect(
      normalizePaths(`${FIXTURE.dir}/input.js imports /repo/other.js`, CONTEXT),
    ).toBe('<FIXTURE_ROOT>/input.js imports <REPO_ROOT>/other.js');
  });

  test('normalizes line endings', () => {
    expect(normalizePaths('a\r\nb', CONTEXT)).toBe('a\nb');
  });
});

describe('normalizeDiagnostic', () => {
  test('drops the location prefix and the source excerpt', () => {
    const message = [
      `${FIXTURE.dir}/input.js: create() can only accept an object.`,
      '  1 | import * as stylex from "@stylexjs/stylex";',
      '> 2 | export const styles = stylex.create(1);',
      '    |                       ^^^^^^^^^^^^^^^^',
    ].join('\n');

    expect(normalizeDiagnostic(message, CONTEXT)).toBe(
      'create() can only accept an object.',
    );
  });

  test('collapses the implementation tag', () => {
    expect(
      normalizeDiagnostic('[@stylexjs/babel-plugin] Expected a boolean.', {
        ...CONTEXT,
      }),
    ).toBe('[stylex] Expected a boolean.');
  });

  test('leaves a message that merely opens with a bracket alone', () => {
    expect(normalizeDiagnostic('[1, 2] is not a valid value.', CONTEXT)).toBe(
      '[1, 2] is not a valid value.',
    );
  });

  test('leaves a pipe that is not a source excerpt alone', () => {
    expect(
      normalizeDiagnostic('bad value\n|start| is reserved.', CONTEXT),
    ).toBe('bad value\n|start| is reserved.');
  });
});

describe('normalizeCss', () => {
  test('strips trailing whitespace from every line and trims', () => {
    expect(
      normalizeCss('\n.a{color:red}   \n.b{color:blue}\t\n\n', CONTEXT),
    ).toBe('.a{color:red}\n.b{color:blue}');
  });
});

describe('normalizeData', () => {
  test('sorts object keys', () => {
    expect(JSON.stringify(normalizeData({ b: 1, a: 2 }, CONTEXT))).toBe(
      '{"a":2,"b":1}',
    );
  });

  test('preserves array order, because rule order is part of the contract', () => {
    expect(normalizeData(['z', 'a'], CONTEXT)).toEqual(['z', 'a']);
  });
});

describe('isJsEquivalent', () => {
  test('ignores formatting, quote style and comments', () => {
    expect(
      isJsEquivalent(
        'export const a = {x: 1};',
        '// a comment\nexport const a = {\n  "x": 1,\n}\n',
        ['flow'],
      ),
    ).toBe(true);
  });

  test('ignores how a property key is spelled', () => {
    expect(isJsEquivalent('({a: 1});', '({"a": 1});', ['flow'])).toBe(true);
    expect(isJsEquivalent('({1: x});', '({"1": x});', ['flow'])).toBe(true);
  });

  test('ignores property shorthand', () => {
    expect(isJsEquivalent('({a});', '({a: a});', ['flow'])).toBe(true);
  });

  test('reports a genuine difference', () => {
    expect(
      isJsEquivalent('export const a = 1;', 'export const a = 2;', ['flow']),
    ).toBe(false);
  });

  test('still reports a differently named property', () => {
    expect(isJsEquivalent('({a: 1});', '({b: 1});', ['flow'])).toBe(false);
  });

  test('still reports a computed key', () => {
    expect(isJsEquivalent('({a: 1});', '({[a]: 1});', ['flow'])).toBe(false);
  });

  test('honors the requested syntax', () => {
    expect(
      isJsEquivalent('const a = <div />;', 'const a = <div/>;', [
        'flow',
        'jsx',
      ]),
    ).toBe(true);
  });

  test('reports a difference when only one side produced output', () => {
    expect(isJsEquivalent(null, 'export const a = 1;', ['flow'])).toBe(false);
  });

  test('fails loudly on unparseable output', () => {
    expect(() =>
      isJsEquivalent('export const a = 1;', 'const = ;', ['flow']),
    ).toThrow(/Failed to parse the actual output as JavaScript/);
  });
});

describe('normalizeResult', () => {
  test('normalizes a successful transform', () => {
    const result = normalizeResult(FIXTURE, {
      css: '.a{color:red}  ',
      js: '  export const a = 1;  ',
      metadata: [['a', { rtl: null, ltr: '.a{color:red}' }, 3000]],
      status: 'ok',
      warnings: ['[@stylexjs/babel-plugin] heads up'],
    });

    expect(result).toEqual({
      css: '.a{color:red}',
      errors: [],
      js: 'export const a = 1;',
      metadata: [['a', { ltr: '.a{color:red}', rtl: null }, 3000]],
      status: 'ok',
      warnings: ['[stylex] heads up'],
    });
    expect(exactPart(result)).not.toHaveProperty('js');
    expect(jsPart(result)).toBe('export const a = 1;');
  });

  test('normalizes a failed transform', () => {
    const result = normalizeResult(FIXTURE, {
      error: {
        message: `${FIXTURE.dir}/input.js: create() can only accept an object.\n> 1 | x\n    | ^`,
      },
      status: 'error',
    });

    expect(result).toEqual({
      error: { message: 'create() can only accept an object.' },
      errors: [],
      status: 'error',
      warnings: [],
    });
    expect(jsPart(result)).toBeNull();
  });
});

describe('fixtures', () => {
  test('every fixture ships an entry file and a recorded result', () => {
    const names = getFixtureNames();
    expect(names.length).toBeGreaterThan(0);

    for (const name of names) {
      const fixture = loadFixture(name);
      expect(fs.existsSync(fixture.entryPath)).toBe(true);
      expect(fs.existsSync(getExpectedPath(name))).toBe(true);
    }
  });
});
