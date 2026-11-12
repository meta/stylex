/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @jest-environment jsdom
 */

import { collectStylexDebugData } from '../src/inspected/collector';

function makeRule(selectorText, cssText) {
  const style = document.createElement('div').style;
  style.cssText = cssText;
  return { selectorText, style };
}

function setStylesheets(stylesheets) {
  Object.defineProperty(document, 'styleSheets', {
    configurable: true,
    value: stylesheets,
  });
}

describe('collectStylexDebugData', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
    document.head.innerHTML = '';
    document.documentElement.removeAttribute('class');
    jest.spyOn(window, 'getComputedStyle').mockImplementation(() => ({
      getPropertyValue: (property) =>
        property === 'color' ? 'rgb(255, 0, 0)' : '',
    }));
  });

  afterEach(() => {
    jest.restoreAllMocks();
    delete document.styleSheets;
    delete global.$0;
    document.documentElement.removeAttribute('class');
  });

  test('collects escaped classes and CSS values containing delimiters', () => {
    const element = document.createElement('div');
    element.className =
      'sm:hidden w-[calc(100%-1rem)] shadow-[0_0_#000,0_0_#fff]';
    document.body.appendChild(element);
    Object.defineProperty(element, 'matches', {
      configurable: true,
      value: (selector) =>
        selector === '.sm\\:hidden' ||
        selector === '.w-\\[calc\\(100\\%-1rem\\)\\]' ||
        selector === '.shadow-\\[0_0_\\#000\\,0_0_\\#fff\\]',
    });
    setStylesheets([
      {
        cssRules: [
          makeRule('.sm\\:hidden:hover', 'display: none'),
          makeRule(
            '.w-\\[calc\\(100\\%-1rem\\)\\]',
            'width: calc(100% - 1rem)',
          ),
          makeRule(
            '.shadow-\\[0_0_\\#000\\,0_0_\\#fff\\]',
            'box-shadow: 0 0 #000, 0 0 #fff',
          ),
        ],
      },
    ]);

    const data = collectStylexDebugData(element);

    expect(data.selectionState).toBe('element');
    expect(data.matched.classes.map(({ name }) => name)).toEqual([
      'sm:hidden',
      'w-[calc(100%-1rem)]',
      'shadow-[0_0_#000,0_0_#fff]',
    ]);
    expect(data.matched.classes[0].declarations[0]).toMatchObject({
      property: 'display',
      value: 'none',
      className: 'sm:hidden',
      conditions: [{ kind: 'selector', text: ':hover', active: false }],
    });
    expect(data.matched.classes[2].declarations[0].value).toBe(
      '0 0 #000, 0 0 #fff',
    );
  });

  test('walks nested rules once and preserves ordered conditions', () => {
    const element = document.createElement('div');
    element.className = 'xfoo';
    document.body.appendChild(element);
    setStylesheets([
      {
        cssRules: [
          {
            conditionText: '(min-width: 1px)',
            cssText: '@media (min-width: 1px) {}',
            cssRules: [makeRule('.xfoo:hover', 'color: red')],
          },
        ],
      },
    ]);

    const declaration =
      collectStylexDebugData(element).matched.classes[0].declarations[0];

    expect(declaration.conditions.map(({ text }) => text)).toEqual([
      '@unknown (min-width: 1px)',
      ':hover',
    ]);
  });

  test('deduplicates declarations and suggestions across stylesheets', () => {
    const element = document.createElement('div');
    element.className = 'xfoo';
    document.body.appendChild(element);
    const makeSheet = () => ({
      cssRules: [makeRule('.xfoo', 'color: red')],
    });
    setStylesheets([makeSheet(), makeSheet(), makeSheet()]);

    const data = collectStylexDebugData(element);
    const declaration = data.matched.classes[0].declarations[0];

    expect(data.matched.classes[0].declarations).toHaveLength(1);
    expect(data.suggestions[declaration.contextKey]).toHaveLength(1);
  });

  test('does not duplicate specificity-repeated atomic classes', () => {
    const element = document.createElement('div');
    element.className = 'x1w1tq1y';
    document.body.appendChild(element);
    setStylesheets([
      {
        cssRules: [
          {
            cssText: '@media (forced-colors: active) {}',
            cssRules: [
              makeRule('.x1w1tq1y.x1w1tq1y', 'border-color: canvastext'),
            ],
          },
        ],
      },
    ]);

    const data = collectStylexDebugData(element);
    const declarations = data.matched.classes[0].declarations;

    expect(declarations).toHaveLength(1);
    expect(declarations[0]).toMatchObject({
      className: 'x1w1tq1y',
      conditions: [
        {
          active: null,
          kind: 'at-rule',
          text: '@media (forced-colors: active)',
        },
      ],
      property: 'border-color',
      value: 'canvastext',
    });
  });

  test('hides defineVars values overridden by a theme and keeps variants', () => {
    const element = document.createElement('div');
    element.className = 'xVars xTheme';
    document.body.appendChild(element);
    const mediaRule = (rules) => ({
      conditionText: '(prefers-color-scheme: dark)',
      cssText: '@media (prefers-color-scheme: dark) {}',
      cssRules: rules,
    });
    setStylesheets([
      {
        cssRules: [
          makeRule(':root, .xVars', '--token: vars-default'),
          makeRule(':root, .xVars', '--base-only: base-default'),
          mediaRule([
            makeRule(':root, .xVars', '--token: vars-dark'),
            makeRule(':root, .xVars', '--base-only: base-dark'),
          ]),
          makeRule('.xTheme, .xTheme:root', '--token: theme-default'),
          mediaRule([makeRule('.xTheme, .xTheme:root', '--token: theme-dark')]),
        ],
      },
    ]);

    const data = collectStylexDebugData(element);
    const varsClass = data.matched.classes.find(({ name }) => name === 'xVars');
    const themeClass = data.matched.classes.find(
      ({ name }) => name === 'xTheme',
    );

    expect(varsClass?.declarations).toHaveLength(2);
    expect(varsClass?.declarations).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          property: '--base-only',
          value: 'base-default',
        }),
        expect.objectContaining({
          property: '--base-only',
          value: 'base-dark',
        }),
      ]),
    );
    expect(themeClass?.declarations).toHaveLength(2);
    expect(themeClass?.declarations).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          property: '--token',
          value: 'theme-default',
        }),
        expect.objectContaining({ property: '--token', value: 'theme-dark' }),
      ]),
    );
    expect(
      data.matched.classes.flatMap(({ declarations }) =>
        declarations.map(({ value }) => value),
      ),
    ).not.toEqual(expect.arrayContaining(['vars-default', 'vars-dark']));
    for (const declaration of themeClass?.declarations ?? []) {
      expect(
        data.suggestions[declaration.contextKey].map(
          ({ className }) => className,
        ),
      ).toEqual(['xTheme']);
    }
  });

  test('ignores root-only duplicate contexts for custom properties', () => {
    const element = document.documentElement;
    element.className = 'xTheme';
    setStylesheets([
      {
        cssRules: [
          makeRule('.xTheme, .xTheme:root', '--weight: 400'),
          makeRule(
            '.xTheme, .xTheme:root',
            '--foreground: color-mix(in oklab, white 80%, black)',
          ),
        ],
      },
    ]);

    const declarations =
      collectStylexDebugData(element).matched.classes[0].declarations;

    expect(declarations).toHaveLength(2);
    expect(declarations.map(({ property }) => property)).toEqual([
      '--weight',
      '--foreground',
    ]);
    expect(
      declarations.flatMap(({ conditions }) =>
        conditions.map(({ text }) => text),
      ),
    ).not.toContain(':root');
  });

  test('preserves authored shorthands instead of CSSOM longhands', () => {
    const element = document.createElement('div');
    element.className = 'x-border';
    document.body.appendChild(element);
    const longhands = [
      'border-top-color',
      'border-right-color',
      'border-bottom-color',
      'border-left-color',
    ];
    const style = {
      cssText: 'border-color: canvastext;',
      length: longhands.length,
      item: (index) => longhands[index] ?? '',
      getPropertyPriority: () => '',
      getPropertyValue: () => 'canvastext',
    };
    setStylesheets([{ cssRules: [{ selectorText: '.x-border', style }] }]);

    const data = collectStylexDebugData(element);
    const declarations = data.matched.classes[0].declarations;

    expect(declarations).toHaveLength(1);
    expect(declarations[0]).toMatchObject({
      className: 'x-border',
      property: 'border-color',
      value: 'canvastext',
    });
    expect(Object.keys(data.computed[''])).toEqual(['border-color']);
  });

  test('returns compact computed values for matched properties and pseudos', () => {
    const element = document.createElement('div');
    element.className = 'xfoo';
    document.body.appendChild(element);
    setStylesheets([
      {
        cssRules: [
          makeRule('.xfoo', 'color: red'),
          makeRule('.xfoo::before', 'display: block'),
        ],
      },
    ]);

    const data = collectStylexDebugData(element);

    expect(data.computed['']).toEqual({ color: 'rgb(255, 0, 0)' });
    expect(data.computed['::before']).toEqual({ display: '' });
    expect(Object.keys(data.computed[''])).toEqual(['color']);
  });

  test('resolves only custom properties referenced by matched values', () => {
    const element = document.createElement('div');
    element.className = 'xfoo';
    document.body.appendChild(element);
    setStylesheets([
      {
        cssRules: [
          makeRule(
            '.xfoo',
            'color: var(--foreground); box-shadow: var(--shadow, var(--fallback))',
          ),
        ],
      },
    ]);
    window.getComputedStyle.mockImplementation(() => ({
      getPropertyValue: (property) =>
        ({
          '--fallback': '0 0 2px black',
          '--foreground': 'rgb(10, 20, 30)',
          '--shadow': '0 1px 4px rgb(0 0 0 / 0.2)',
          color: 'rgb(10, 20, 30)',
        })[property] ?? '',
    }));

    const data = collectStylexDebugData(element);

    expect(data.resolvedVariables).toEqual({
      '--fallback': '0 0 2px black',
      '--foreground': 'rgb(10, 20, 30)',
      '--shadow': '0 1px 4px rgb(0 0 0 / 0.2)',
    });
  });

  test('surfaces inaccessible stylesheets without failing collection', () => {
    const element = document.createElement('div');
    document.body.appendChild(element);
    setStylesheets([
      {
        get cssRules() {
          throw new DOMException('Blocked', 'SecurityError');
        },
      },
    ]);

    const data = collectStylexDebugData(element);

    expect(data.warnings).toEqual([
      expect.objectContaining({ code: 'stylesheet-inaccessible', count: 1 }),
    ]);
  });

  test('handles a large atomic-rule fixture without expanding computed data', () => {
    const element = document.createElement('div');
    element.className = 'x250';
    document.body.appendChild(element);
    setStylesheets([
      {
        cssRules: Array.from({ length: 500 }, (_, index) =>
          makeRule(`.x${index}`, `--fixture-${index}: ${index}px`),
        ),
      },
    ]);

    const data = collectStylexDebugData(element);

    expect(data.matched.classes).toHaveLength(1);
    expect(data.matched.classes[0].declarations).toHaveLength(1);
    expect(Object.keys(data.computed[''])).toEqual(['--fixture-250']);
    expect(Object.keys(data.suggestions)).toHaveLength(1);
  });

  test('supports explicit targets and graceful non-element states', () => {
    const element = document.createElement('span');
    element.id = 'target';
    document.body.appendChild(element);
    setStylesheets([]);

    expect(collectStylexDebugData('#target').element.tagName).toBe('span');
    expect(collectStylexDebugData(':::invalid').selectionState).toBe('none');

    expect(
      collectStylexDebugData(document.createTextNode('text')).selectionState,
    ).toBe('non-element');
  });
});
