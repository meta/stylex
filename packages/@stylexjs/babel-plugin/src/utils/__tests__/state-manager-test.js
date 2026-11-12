/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @flow
 */

import StateManager from '../state-manager';

const defaultConfig = {
  file: { metadata: {} } as any,
  key: 'key',
  opts: {},
  cwd: '/home/test/',
  filename: '/home/test/main.js',
  get(_key: mixed): any {},
  set(_key: mixed, _value: mixed): void {},
};

const makeState = (opts?: { ... } = {}) =>
  new StateManager({ ...defaultConfig, opts });

describe('StateManager config parsing', () => {
  let warnings = [];
  beforeEach(() => {
    warnings = [];
    jest.spyOn(console, 'error').mockImplementation((...args) => {
      warnings.push(args);
    });
  });

  describe('"classNamePrefix" option', () => {
    test('logs errors if invalid', () => {
      const stateManager = makeState({ classNamePrefix: 1 });
      expect(stateManager.options.classNamePrefix).toBe('x');
      expect(warnings).toMatchInlineSnapshot(`
        [
          [
            "[@stylexjs/babel-plugin]",
            "Expected (options.classNamePrefix) to be a string, but got \`1\`.",
          ],
        ]
      `);
    });

    test('default value', () => {
      const stateManager = makeState();
      expect(stateManager.options.classNamePrefix).toBe('x');
      expect(warnings).toEqual([]);
    });

    // This is unsafe if hashed classNames start with a number
    test('empty string value', () => {
      const stateManager = makeState({ classNamePrefix: '' });
      expect(stateManager.options.classNamePrefix).toBe('');
      expect(warnings).toEqual([]);
    });

    test('provided value', () => {
      const stateManager = makeState({ classNamePrefix: 'prefix' });
      expect(stateManager.options.classNamePrefix).toBe('prefix');
      expect(warnings).toEqual([]);
    });
  });

  describe('"debug" option (boolean)', () => {
    test('logs errors if invalid', () => {
      const stateManager = makeState({ debug: 'true' });
      expect(stateManager.options.debug).toBe(false);
      expect(warnings).toEqual([
        [
          '[@stylexjs/babel-plugin]',
          'Expected (options.debug) to be a boolean, but got `"true"`.',
        ],
      ]);
    });

    test('default value', () => {
      const stateManager = makeState();
      expect(stateManager.options.debug).toBe(false);
      expect(warnings).toEqual([]);
    });

    test('false value', () => {
      const stateManager = makeState({ debug: false });
      expect(stateManager.options.debug).toBe(false);
      expect(warnings).toEqual([]);
    });

    test('true value', () => {
      const stateManager = makeState({ debug: true });
      expect(stateManager.options.debug).toBe(true);
      expect(stateManager.options.enableDebugDataProp).toBe(true);
      expect(warnings).toEqual([]);
    });
  });

  describe('"dev" option (boolean)', () => {
    test('logs errors if invalid', () => {
      const stateManager = makeState({ dev: 'true' });
      expect(stateManager.options.dev).toBe(false);
      expect(warnings).toEqual([
        [
          '[@stylexjs/babel-plugin]',
          'Expected (options.dev) to be a boolean, but got `"true"`.',
        ],
      ]);
    });

    test('default value', () => {
      const stateManager = makeState();
      expect(stateManager.options.dev).toBe(false);
      expect(warnings).toEqual([]);
    });

    test('false value', () => {
      const stateManager = makeState({ dev: false });
      expect(stateManager.options.dev).toBe(false);
      expect(warnings).toEqual([]);
    });

    test('true value', () => {
      const stateManager = makeState({ dev: true });
      expect(stateManager.options.dev).toBe(true);
      // debug is enabled by default in 'dev'
      expect(stateManager.options.debug).toBe(true);
      // enableDevClassNames is disabled by default in 'dev'
      expect(stateManager.options.enableDevClassNames).toBe(true);
      expect(warnings).toEqual([]);
    });
  });

  describe('"enableDebugDataProp" option (boolean)', () => {
    test('logs errors if invalid', () => {
      const stateManager = makeState({
        debug: true,
      });
      expect(stateManager.options.enableDebugDataProp).toBe(true);
    });

    test('default value', () => {
      const stateManager = makeState();
      expect(stateManager.options.enableDebugDataProp).toBe(false);
      expect(warnings).toEqual([]);
    });

    test('false value', () => {
      const stateManager = makeState({
        debug: true,
        enableDebugDataProp: false,
      });
      expect(stateManager.options.enableDebugDataProp).toBe(false);
      expect(warnings).toEqual([]);
    });

    test('true value', () => {
      const stateManager = makeState({
        debug: true,
        enableDebugDataProp: true,
      });
      expect(stateManager.options.enableDebugDataProp).toBe(true);
      expect(warnings).toEqual([]);
    });
  });

  describe('"enableDevClassNames" option (boolean)', () => {
    test('logs errors if invalid', () => {
      const stateManager = makeState({ enableDevClassNames: 'true' });
      expect(stateManager.options.enableDevClassNames).toBe(false);
      expect(warnings).toEqual([
        [
          '[@stylexjs/babel-plugin]',
          'Expected (options.enableDevClassNames) to be a boolean, but got `"true"`.',
        ],
      ]);
    });

    test('default value', () => {
      const stateManager = makeState();
      expect(stateManager.options.enableDevClassNames).toBe(false);
      expect(warnings).toEqual([]);
    });

    test('false value', () => {
      const stateManager = makeState({
        debug: true,
        enableDevClassNames: false,
      });
      expect(stateManager.options.enableDevClassNames).toBe(false);
      expect(warnings).toEqual([]);
    });

    test('true value', () => {
      const stateManager = makeState({
        debug: true,
        enableDevClassNames: true,
      });
      expect(stateManager.options.enableDevClassNames).toBe(true);
      expect(warnings).toEqual([]);
    });
  });

  describe('"enableInlinedConditionalMerge" option (boolean)', () => {
    test('logs errors if invalid', () => {
      const stateManager = makeState({
        enableInlinedConditionalMerge: 'false',
      });
      expect(stateManager.options.enableInlinedConditionalMerge).toBe(true);
      expect(warnings).toEqual([
        [
          '[@stylexjs/babel-plugin]',
          'Expected (options.enableInlinedConditionalMerge) to be a boolean, but got `"false"`.',
        ],
      ]);
    });

    test('default value', () => {
      const stateManager = makeState();
      expect(stateManager.options.enableInlinedConditionalMerge).toBe(true);
      expect(warnings).toEqual([]);
    });

    test('false value', () => {
      const stateManager = makeState({ enableInlinedConditionalMerge: false });
      expect(stateManager.options.enableInlinedConditionalMerge).toBe(false);
      expect(warnings).toEqual([]);
    });

    test('true value', () => {
      const stateManager = makeState({ enableInlinedConditionalMerge: true });
      expect(stateManager.options.enableInlinedConditionalMerge).toBe(true);
      expect(warnings).toEqual([]);
    });
  });

  describe('"enableMinifiedKeys" option (boolean)', () => {
    test('logs errors if invalid', () => {
      const stateManager = makeState({ enableMinifiedKeys: 'false' });
      expect(stateManager.options.enableMinifiedKeys).toBe(true);
      expect(warnings).toEqual([
        [
          '[@stylexjs/babel-plugin]',
          'Expected (options.enableMinifiedKeys) to be a boolean, but got `"false"`.',
        ],
      ]);
    });

    test('default value', () => {
      const stateManager = makeState();
      expect(stateManager.options.enableMinifiedKeys).toBe(true);
      expect(warnings).toEqual([]);
    });

    test('false value', () => {
      const stateManager = makeState({
        enableMinifiedKeys: false,
      });
      expect(stateManager.options.enableMinifiedKeys).toBe(false);
      expect(warnings).toEqual([]);
    });

    test('true value', () => {
      const stateManager = makeState({
        enableMinifiedKeys: true,
      });
      expect(stateManager.options.enableDebugDataProp).toBe(false);
      expect(warnings).toEqual([]);
    });
  });

  describe('"importSources" option', () => {
    test('logs errors if invalid', () => {
      const stateManager = makeState({ importSources: 1 });
      expect(stateManager.options.importSources).toEqual([
        '@stylexjs/stylex',
        'stylex',
      ]);
      expect(warnings).toMatchInlineSnapshot(`
        [
          [
            "[@stylexjs/babel-plugin]",
            "Expected (options.importSources) to be an array, but got \`1\`.",
          ],
        ]
      `);
    });

    test('default value', () => {
      const stateManager = makeState();
      expect(stateManager.options.importSources).toEqual([
        '@stylexjs/stylex',
        'stylex',
      ]);
      expect(warnings).toEqual([]);
    });

    test('empty value', () => {
      const stateManager = makeState({ importSources: [] });
      expect(stateManager.options.importSources).toEqual([
        '@stylexjs/stylex',
        'stylex',
      ]);
      expect(warnings).toEqual([]);
    });

    test('provided value', () => {
      const stateManager = makeState({
        importSources: [
          'built-with-stylex',
          { from: 'react-strict-dom', as: 'css' },
        ],
      });
      expect(stateManager.options.importSources).toMatchInlineSnapshot(`
        [
          "@stylexjs/stylex",
          "stylex",
          "built-with-stylex",
          {
            "as": "css",
            "from": "react-strict-dom",
          },
        ]
      `);
      expect(warnings).toEqual([]);
    });
  });

  describe('"runtimeInjection" option', () => {
    test('logs errors if invalid', () => {
      const stateManager = makeState({ runtimeInjection: 1 });
      expect(stateManager.options.runtimeInjection).toBeUndefined();
      expect(warnings).toMatchInlineSnapshot(`
        [
          [
            "[@stylexjs/babel-plugin]",
            "Expected (options.runtimeInjection) to be one of
        	- a boolean
        	- a string
        	- an object where:
        But got: 1",
          ],
        ]
      `);
    });

    test('default value', () => {
      const stateManager = makeState();
      expect(stateManager.options.runtimeInjection).toBeUndefined();
      expect(warnings).toEqual([]);
    });

    test('false value', () => {
      const stateManager = makeState({ runtimeInjection: false });
      expect(stateManager.options.runtimeInjection).toBeUndefined();
      expect(warnings).toEqual([]);
    });

    test('true value', () => {
      const stateManager = makeState({ runtimeInjection: true });
      expect(stateManager.options.runtimeInjection).toBe(
        '@stylexjs/stylex/lib/stylex-inject',
      );
      expect(warnings).toEqual([]);
    });

    test('"true" value', () => {
      const stateManager = makeState({ runtimeInjection: 'true' });
      expect(stateManager.options.runtimeInjection).toBe('true');
      expect(warnings).toEqual([]);
    });
  });

  describe('"styleResolution" option', () => {
    test('logs errors if invalid', () => {
      const stateManager = makeState({ styleResolution: 'something-else' });
      expect(stateManager.options.styleResolution).toBe('property-specificity');
      expect(warnings).toMatchInlineSnapshot(`
        [
          [
            "[@stylexjs/babel-plugin]",
            "Expected (options.styleResolution) to be one of
        	- the literal "application-order"
        	- the literal "property-specificity"
        	- the literal "legacy-expand-shorthands"
        But got: "something-else"",
          ],
        ]
      `);
    });

    test('default value', () => {
      const stateManager = makeState();
      expect(stateManager.options.styleResolution).toBe('property-specificity');
      expect(warnings).toEqual([]);
    });

    test('valid values', () => {
      let stateManager = makeState({ styleResolution: 'application-order' });
      expect(stateManager.options.styleResolution).toBe('application-order');

      stateManager = makeState({ styleResolution: 'property-specificity' });
      expect(stateManager.options.styleResolution).toBe('property-specificity');

      stateManager = makeState({ styleResolution: 'legacy-expand-shorthands' });
      expect(stateManager.options.styleResolution).toBe(
        'legacy-expand-shorthands',
      );

      expect(warnings).toEqual([]);
    });
  });

  describe('"test" option (boolean)', () => {
    test('logs errors if invalid', () => {
      const stateManager = makeState({ test: 'true' });
      expect(stateManager.options.test).toBe(false);
      expect(warnings).toEqual([
        [
          '[@stylexjs/babel-plugin]',
          'Expected (options.test) to be a boolean, but got `"true"`.',
        ],
      ]);
    });

    test('default value', () => {
      const stateManager = makeState();
      expect(stateManager.options.test).toBe(false);
      expect(warnings).toEqual([]);
    });

    test('false value', () => {
      const stateManager = makeState({ test: false });
      expect(stateManager.options.test).toBe(false);
      expect(warnings).toEqual([]);
    });

    test('true value', () => {
      const stateManager = makeState({ test: true });
      expect(stateManager.options.test).toBe(true);
      expect(warnings).toEqual([]);
    });
  });

  describe('"treeshakeCompensation" option (boolean)', () => {
    test('logs errors if invalid', () => {
      const stateManager = makeState({ treeshakeCompensation: 'true' });
      expect(stateManager.options.treeshakeCompensation).toBe(false);
      expect(warnings).toEqual([
        [
          '[@stylexjs/babel-plugin]',
          'Expected (options.treeshakeCompensation) to be a boolean, but got `"true"`.',
        ],
      ]);
    });

    test('default value', () => {
      const stateManager = makeState();
      expect(stateManager.options.treeshakeCompensation).toBe(false);
      expect(warnings).toEqual([]);
    });

    test('false value', () => {
      const stateManager = makeState({ treeshakeCompensation: false });
      expect(stateManager.options.treeshakeCompensation).toBe(false);
      expect(warnings).toEqual([]);
    });

    test('true value', () => {
      const stateManager = makeState({ treeshakeCompensation: true });
      expect(stateManager.options.treeshakeCompensation).toBe(true);
      expect(warnings).toEqual([]);
    });
  });

  describe('"unstable_moduleResolution" option', () => {
    test('default value', () => {
      const stateManager = makeState();
      expect(stateManager.options.unstable_moduleResolution).toBeUndefined();
      expect(warnings).toEqual([]);
    });

    test('"commonJS" type', () => {
      const stateManager = makeState({
        unstable_moduleResolution: { type: 'commonJS' },
      });
      expect(stateManager.options.unstable_moduleResolution).toEqual({
        type: 'commonJS',
      });
      expect(warnings).toEqual([]);
    });

    test('"haste" type', () => {
      const stateManager = makeState({
        unstable_moduleResolution: { type: 'haste' },
      });
      expect(stateManager.options.unstable_moduleResolution).toEqual({
        type: 'haste',
      });
      expect(warnings).toEqual([]);
    });

    test('"custom" type', () => {
      const stateManager = makeState({
        unstable_moduleResolution: {
          type: 'custom',
          filePathResolver() {},
          getCanonicalFilePath() {},
        },
      });
      expect(stateManager.options.unstable_moduleResolution).toEqual({
        type: 'custom',
        filePathResolver: expect.any(Function),
        getCanonicalFilePath: expect.any(Function),
      });
      expect(warnings).toEqual([]);
    });

    test('"rootDir" option', () => {
      const stateManager = makeState({
        unstable_moduleResolution: { type: 'commonJS', rootDir: '/test/' },
      });
      expect(stateManager.options.unstable_moduleResolution).toEqual({
        type: 'commonJS',
        rootDir: '/test/',
      });
      expect(warnings).toEqual([]);
    });
  });

  describe('"enableFontSizePxToRem" option (boolean)', () => {
    test('logs errors if invalid', () => {
      const stateManager = makeState({ enableFontSizePxToRem: 'true' });
      expect(stateManager.options.enableFontSizePxToRem).toBe(false);
      expect(warnings).toEqual([
        [
          '[@stylexjs/babel-plugin]',
          'Expected (options.enableFontSizePxToRem) to be a boolean, but got `"true"`.',
        ],
      ]);
    });

    test('default value', () => {
      const stateManager = makeState();
      expect(stateManager.options.enableFontSizePxToRem).toBe(false);
      expect(warnings).toEqual([]);
    });

    test('false value', () => {
      const stateManager = makeState({ enableFontSizePxToRem: false });
      expect(stateManager.options.enableFontSizePxToRem).toBe(false);
      expect(warnings).toEqual([]);
    });

    test('true value', () => {
      const stateManager = makeState({ enableFontSizePxToRem: true });
      expect(stateManager.options.enableFontSizePxToRem).toBe(true);
      expect(warnings).toEqual([]);
    });
  });

  describe('"propertyValidationMode" option', () => {
    test('logs errors if invalid', () => {
      const stateManager = makeState({
        propertyValidationMode: 'something-else',
      });
      expect(stateManager.options.propertyValidationMode).toBe('silent');
      expect(warnings).toMatchInlineSnapshot(`
        [
          [
            "[@stylexjs/babel-plugin]",
            "Expected (options.propertyValidationMode) to be one of
        	- the literal "throw"
        	- the literal "warn"
        	- the literal "silent"
        But got: "something-else"",
          ],
        ]
      `);
    });

    test('default value', () => {
      const stateManager = makeState();
      expect(stateManager.options.propertyValidationMode).toBe('silent');
      expect(warnings).toEqual([]);
    });

    test('valid values', () => {
      let stateManager = makeState({ propertyValidationMode: 'throw' });
      expect(stateManager.options.propertyValidationMode).toBe('throw');

      stateManager = makeState({ propertyValidationMode: 'warn' });
      expect(stateManager.options.propertyValidationMode).toBe('warn');

      stateManager = makeState({ propertyValidationMode: 'silent' });
      expect(stateManager.options.propertyValidationMode).toBe('silent');

      expect(warnings).toEqual([]);
    });
  });
});
