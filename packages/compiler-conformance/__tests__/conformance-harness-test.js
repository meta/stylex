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

const IMPLEMENTATION_TAG = '@stylexjs/babel-plugin';
const FIXTURE = {
  dir: '/repo/packages/compiler-conformance/fixtures/demo',
  entry: 'input.js',
  repoRoot: '/repo',
};
const CONTEXT = {
  entry: FIXTURE.entry,
  fixtureDir: FIXTURE.dir,
  implementationTag: IMPLEMENTATION_TAG,
  repoRoot: FIXTURE.repoRoot,
};
const WINDOWS_CONTEXT = {
  entry: 'input.js',
  fixtureDir: 'C:\\repo\\packages\\compiler-conformance\\fixtures\\demo',
  implementationTag: IMPLEMENTATION_TAG,
  repoRoot: 'C:\\repo',
};

function validSuccess(overrides = {}) {
  return {
    css: '',
    errors: [],
    js: '',
    metadata: [],
    status: 'ok',
    warnings: [],
    ...overrides,
  };
}

describe('normalizePaths', () => {
  test('replaces the fixture directory before the repository root', () => {
    expect(
      normalizePaths(`${FIXTURE.dir}/input.js imports /repo/other.js`, CONTEXT),
    ).toBe('<FIXTURE_ROOT>/input.js imports <REPO_ROOT>/other.js');
  });

  test('only replaces roots at a path boundary', () => {
    expect(normalizePaths('/repo-copy/result.js', CONTEXT)).toBe(
      '/repo-copy/result.js',
    );
  });

  test('normalizes Windows roots and separators', () => {
    expect(
      normalizePaths(
        `${WINDOWS_CONTEXT.fixtureDir}\\input.js imports C:/repo/other.js`,
        WINDOWS_CONTEXT,
      ),
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

  test('drops a Windows location prefix with line and column coordinates', () => {
    expect(
      normalizeDiagnostic(
        `${WINDOWS_CONTEXT.fixtureDir}\\input.js:12:3: bad token`,
        WINDOWS_CONTEXT,
      ),
    ).toBe('bad token');
  });

  test('collapses the caller-provided implementation tag', () => {
    expect(
      normalizeDiagnostic('[@stylexjs/babel-plugin] Expected a boolean.', {
        ...CONTEXT,
      }),
    ).toBe('[stylex] Expected a boolean.');
  });

  test('preserves a semantic bracketed prefix', () => {
    expect(normalizeDiagnostic('[E100] Expected a boolean.', CONTEXT)).toBe(
      '[E100] Expected a boolean.',
    );
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

  test('preserves pipe-prefixed alternatives in a multiline message', () => {
    expect(
      normalizeDiagnostic('Expected one of:\n  | string\n  | number', CONTEXT),
    ).toBe('Expected one of:\n  | string\n  | number');
  });

  // Babel's code frame colorizes whenever the environment looks color-capable,
  // which on GitHub Actions it does — so this is the shape the very same
  // diagnostic takes on CI, and the escapes must not hide the excerpt.
  test('drops a colorized source excerpt', () => {
    const esc = '\u001B';
    const message = [
      `${FIXTURE.dir}/input.js: create() can only accept an object.`,
      `${esc}[0m ${esc}[90m 1 |${esc}[39m ${esc}[36mimport${esc}[39m stylex${esc}[33m;${esc}[39m`,
      `${esc}[31m${esc}[1m>${esc}[22m${esc}[39m${esc}[90m 2 |${esc}[39m stylex${esc}[33m.${esc}[39mcreate(${esc}[35m1${esc}[39m)${esc}[33m;${esc}[39m`,
      `${esc}[90m   |${esc}[39m       ${esc}[31m${esc}[1m^${esc}[22m${esc}[39m${esc}[0m`,
    ].join('\n');

    expect(normalizeDiagnostic(message, CONTEXT)).toBe(
      'create() can only accept an object.',
    );
  });

  test('removes color from a message that has no source excerpt', () => {
    expect(
      normalizeDiagnostic(
        '\u001B[1m\u001B[31m[@stylexjs/babel-plugin]\u001B[39m Expected a boolean.\u001B[22m',
        CONTEXT,
      ),
    ).toBe('[stylex] Expected a boolean.');
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
    expect(isJsEquivalent('({1n: x});', '({"1": x});', ['flow'])).toBe(true);
  });

  test('ignores property shorthand', () => {
    expect(isJsEquivalent('({a});', '({a: a});', ['flow'])).toBe(true);
    expect(
      isJsEquivalent('const {a} = x;', 'const {a: a} = x;', ['flow']),
    ).toBe(true);
  });

  test('preserves the semantic distinction for __proto__ shorthand', () => {
    expect(
      isJsEquivalent(
        'const __proto__ = null; ({__proto__});',
        'const __proto__ = null; ({__proto__: __proto__});',
        ['flow'],
      ),
    ).toBe(false);
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

  test('fails loudly when identical output is unparseable', () => {
    expect(() => isJsEquivalent('const = ;', 'const = ;', ['flow'])).toThrow(
      /Failed to parse the expected output as JavaScript/,
    );
  });
});

describe('normalizeResult', () => {
  test('normalizes a successful transform', () => {
    const result = normalizeResult(
      FIXTURE,
      validSuccess({
        css: '.a{color:red}  ',
        js: '  export const a = 1;  ',
        metadata: [['a', { rtl: null, ltr: '.a{color:red}' }, 3000]],
        warnings: ['[@stylexjs/babel-plugin] heads up'],
      }),
      IMPLEMENTATION_TAG,
    );

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
      errors: [],
      status: 'error',
      warnings: [],
    });

    expect(result).toEqual({
      error: { message: 'create() can only accept an object.' },
      errors: [],
      status: 'error',
      warnings: [],
    });
    expect(jsPart(result)).toBeNull();
  });

  test.each([
    ['a missing status', { errors: [], warnings: [] }, /result.status/],
    [
      'an unknown status',
      { errors: [], status: 'failed', warnings: [] },
      /result.status/,
    ],
    [
      'missing diagnostics',
      { css: '', js: '', metadata: [], status: 'ok' },
      /result.warnings/,
    ],
    [
      'a non-string diagnostic',
      validSuccess({ warnings: [{}] }),
      /result.warnings/,
    ],
    [
      'missing JavaScript',
      { css: '', errors: [], metadata: [], status: 'ok', warnings: [] },
      /result.js/,
    ],
    [
      'missing metadata',
      { css: '', errors: [], js: '', status: 'ok', warnings: [] },
      /result.metadata/,
    ],
    [
      'missing CSS',
      { errors: [], js: '', metadata: [], status: 'ok', warnings: [] },
      /result.css/,
    ],
    [
      'a missing fatal message',
      { error: {}, errors: [], status: 'error', warnings: [] },
      /result.error.message/,
    ],
  ])('rejects %s', (_description, result, expectedError) => {
    expect(() => normalizeResult(FIXTURE, result)).toThrow(expectedError);
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

  test('default arrays and objects are isolated between loads', () => {
    const first = loadFixture('create-basic');
    const second = loadFixture('create-basic');

    expect(first.processOptions).not.toBe(second.processOptions);
    expect(first.syntax).not.toBe(second.syntax);

    first.processOptions.useLayers = true;
    first.syntax.push('jsx');

    expect(second.processOptions.useLayers).toBe(false);
    expect(second.syntax).toEqual(['flow']);
  });
});
