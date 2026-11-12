/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @jest-environment jsdom
 */

import { createInspectedPageRuntime } from '../src/inspected/runtime';
import { collectStylexDebugData } from '../src/inspected/collector';
import { RULE_TARGET_ATTRIBUTE } from '../src/inspected/ruleOverrideSheet';

const baseCommand = (data, values = {}) => ({
  selectionId: data.selectionId,
  contextKey: 'color-context',
  property: 'color',
  value: 'red',
  important: false,
  conditions: [],
  replaceOverrideIds: [],
  ...values,
});

test('identifies selections without collecting stylesheet data', () => {
  const runtime = createInspectedPageRuntime();
  const element = document.createElement('div');

  expect(runtime.identify(null)).toBe('none');
  expect(runtime.identify(document.createTextNode('text'))).toBe('non-element');
  const selectionId = runtime.identify(element);
  expect(selectionId).toMatch(/^selection-/);
  expect(runtime.identify(element)).toBe(selectionId);
  runtime.identify(null);
});

describe('override transactions', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
    Object.defineProperty(document, 'styleSheets', {
      configurable: true,
      value: [],
    });
    Object.defineProperty(document, 'adoptedStyleSheets', {
      configurable: true,
      value: [],
      writable: true,
    });
    jest.spyOn(window, 'getComputedStyle').mockReturnValue({
      getPropertyValue: () => '',
    });
  });

  afterEach(() => {
    jest.restoreAllMocks();
    delete document.styleSheets;
    delete document.adoptedStyleSheets;
    delete global.$0;
  });

  test('restores an existing inline value and priority exactly', () => {
    const element = document.createElement('div');
    element.style.setProperty('color', 'blue', 'important');
    document.body.appendChild(element);
    const runtime = createInspectedPageRuntime();
    const data = runtime.collect(element);

    const setResult = runtime.mutate(
      { ...baseCommand(data), type: 'set-inline' },
      element,
    );
    expect(setResult.ok).toBe(true);
    expect(element.style.getPropertyValue('color')).toBe('red');

    const override = setResult.ok ? setResult.data.overrides[0] : null;
    const removeResult = runtime.mutate(
      {
        type: 'remove',
        selectionId: data.selectionId,
        overrideId: override?.id ?? '',
      },
      element,
    );

    expect(removeResult.ok).toBe(true);
    expect(element.style.getPropertyValue('color')).toBe('blue');
    expect(element.style.getPropertyPriority('color')).toBe('important');
  });

  test('preserves pre-existing replacement classes on removal', () => {
    const element = document.createElement('div');
    element.className = 'xold xnew';
    document.body.appendChild(element);
    const runtime = createInspectedPageRuntime();
    const data = runtime.collect(element);

    const swapResult = runtime.mutate(
      {
        ...baseCommand(data),
        type: 'swap-class',
        fromClassName: 'xold',
        toClassName: 'xnew',
      },
      element,
    );
    expect(swapResult.ok).toBe(true);
    const override = swapResult.ok ? swapResult.data.overrides[0] : null;

    runtime.mutate(
      {
        type: 'remove',
        selectionId: data.selectionId,
        overrideId: override?.id ?? '',
      },
      element,
    );

    expect(element.classList.contains('xold')).toBe(true);
    expect(element.classList.contains('xnew')).toBe(true);
  });

  test('repeated class edits restore the first class', () => {
    const element = document.createElement('div');
    element.className = 'xold';
    document.body.appendChild(element);
    const runtime = createInspectedPageRuntime();
    const data = runtime.collect(element);

    const first = runtime.mutate(
      {
        ...baseCommand(data),
        type: 'swap-class',
        fromClassName: 'xold',
        toClassName: 'xred',
      },
      element,
    );
    expect(first.ok).toBe(true);
    const firstOverride = first.ok ? first.data.overrides[0] : null;
    const second = runtime.mutate(
      {
        ...baseCommand(data, {
          replaceOverrideIds: [firstOverride?.id ?? ''],
        }),
        type: 'swap-class',
        fromClassName: 'xred',
        toClassName: 'xblue',
      },
      element,
    );
    expect(second.ok).toBe(true);
    const secondOverride = second.ok ? second.data.overrides[0] : null;

    runtime.mutate(
      {
        type: 'remove',
        selectionId: data.selectionId,
        overrideId: secondOverride?.id ?? '',
      },
      element,
    );

    expect(element.className).toBe('xold');
  });

  test('rolls back DOM changes when a mutation throws', () => {
    const element = document.createElement('div');
    element.className = 'xold';
    document.body.appendChild(element);
    const runtime = createInspectedPageRuntime();
    const data = runtime.collect(element);
    jest.spyOn(element.classList, 'add').mockImplementation(() => {
      throw new Error('Mutation blocked');
    });

    const result = runtime.mutate(
      {
        ...baseCommand(data),
        type: 'swap-class',
        fromClassName: 'xold',
        toClassName: 'xnew',
      },
      element,
    );

    expect(result).toMatchObject({ ok: false, code: 'mutation-failed' });
    expect(element.className).toBe('xold');
  });

  test('rejects CSS values the browser does not apply', () => {
    const element = document.createElement('div');
    document.body.appendChild(element);
    const runtime = createInspectedPageRuntime();
    const data = runtime.collect(element);

    const result = runtime.mutate(
      {
        ...baseCommand(data, { value: 'not-a-color' }),
        type: 'set-inline',
      },
      element,
    );

    expect(result).toMatchObject({ ok: false, code: 'mutation-failed' });
    expect(element.hasAttribute('style')).toBe(false);
    expect(runtime.collect(element).overrides).toEqual([]);
  });

  test('rolls back DOM and stored state when refreshed collection fails', () => {
    const element = document.createElement('div');
    element.style.setProperty('color', 'blue', 'important');
    document.body.appendChild(element);
    let collectionCount = 0;
    const runtime = createInspectedPageRuntime((target) => {
      collectionCount += 1;
      if (collectionCount === 2) {
        throw new Error('Collection blocked');
      }
      return collectStylexDebugData(target);
    });
    const data = runtime.collect(element);

    const result = runtime.mutate(
      { ...baseCommand(data), type: 'set-inline' },
      element,
    );

    expect(result).toMatchObject({
      ok: false,
      code: 'mutation-failed',
      message: 'Collection blocked',
    });
    expect(element.style.getPropertyValue('color')).toBe('blue');
    expect(element.style.getPropertyPriority('color')).toBe('important');
    expect(runtime.collect(element).overrides).toEqual([]);
  });

  test('rejects stale selections', () => {
    const first = document.createElement('div');
    const second = document.createElement('div');
    document.body.append(first, second);
    const runtime = createInspectedPageRuntime();
    const data = runtime.collect(first);

    expect(
      runtime.mutate({ ...baseCommand(data), type: 'set-inline' }, second),
    ).toMatchObject({ ok: false, code: 'stale-selection' });
  });

  test('creates and removes a scoped pseudo-element rule', () => {
    const element = document.createElement('dialog');
    element.setAttribute(RULE_TARGET_ATTRIBUTE, 'page-value');
    document.body.appendChild(element);
    const runtime = createInspectedPageRuntime();
    const data = runtime.collect(element);

    const setResult = runtime.mutate(
      {
        ...baseCommand(data, {
          property: 'background-color',
          pseudoElement: '::backdrop',
        }),
        type: 'set-rule',
      },
      element,
    );

    expect(setResult.ok).toBe(true);
    expect(element.getAttribute(RULE_TARGET_ATTRIBUTE)).toBe(data.selectionId);
    expect(document.adoptedStyleSheets).toHaveLength(1);
    expect(document.adoptedStyleSheets[0].cssRules[0].cssText).toContain(
      `[${RULE_TARGET_ATTRIBUTE}="${data.selectionId}"]`,
    );
    expect(document.adoptedStyleSheets[0].cssRules[0].cssText).toContain(
      '::backdrop',
    );
    const override = setResult.ok ? setResult.data.overrides[0] : null;
    expect(override).toMatchObject({
      kind: 'rule',
      property: 'background-color',
      pseudoElement: '::backdrop',
      value: 'red',
    });

    const removeResult = runtime.mutate(
      {
        type: 'remove',
        selectionId: data.selectionId,
        overrideId: override?.id ?? '',
      },
      element,
    );

    expect(removeResult.ok).toBe(true);
    expect(document.adoptedStyleSheets).toEqual([]);
    expect(element.getAttribute(RULE_TARGET_ATTRIBUTE)).toBe('page-value');
  });

  test('replaces pseudo-element rules without leaking stylesheets', () => {
    const element = document.createElement('div');
    document.body.appendChild(element);
    const runtime = createInspectedPageRuntime();
    const data = runtime.collect(element);

    const first = runtime.mutate(
      {
        ...baseCommand(data, { pseudoElement: '::before' }),
        type: 'set-rule',
      },
      element,
    );
    expect(first.ok).toBe(true);
    const firstOverride = first.ok ? first.data.overrides[0] : null;
    const firstSheet = document.adoptedStyleSheets[0];

    const second = runtime.mutate(
      {
        ...baseCommand(data, {
          pseudoElement: '::before',
          replaceOverrideIds: [firstOverride?.id ?? ''],
          value: 'blue',
        }),
        type: 'set-rule',
      },
      element,
    );

    expect(second.ok).toBe(true);
    expect(document.adoptedStyleSheets).toHaveLength(1);
    expect(document.adoptedStyleSheets).not.toContain(firstSheet);
    expect(document.adoptedStyleSheets[0].cssRules[0].cssText).toContain(
      'blue',
    );
  });

  test('adopts pseudo-element rules into the selected shadow root', () => {
    const host = document.createElement('div');
    const shadowRoot = host.attachShadow({ mode: 'open' });
    Object.defineProperty(shadowRoot, 'adoptedStyleSheets', {
      configurable: true,
      value: [],
      writable: true,
    });
    const element = document.createElement('div');
    shadowRoot.appendChild(element);
    document.body.appendChild(host);
    const runtime = createInspectedPageRuntime();
    const data = runtime.collect(element);

    const result = runtime.mutate(
      {
        ...baseCommand(data, { pseudoElement: '::before' }),
        type: 'set-rule',
      },
      element,
    );

    expect(result.ok).toBe(true);
    expect(shadowRoot.adoptedStyleSheets).toHaveLength(1);
    expect(document.adoptedStyleSheets).toEqual([]);
  });

  test('preserves selector and at-rule context in pseudo-element rules', () => {
    const parent = document.createElement('div');
    parent.className = 'parent';
    const element = document.createElement('div');
    parent.appendChild(element);
    document.body.appendChild(parent);
    const runtime = createInspectedPageRuntime();
    const data = runtime.collect(element);

    const result = runtime.mutate(
      {
        ...baseCommand(data, {
          conditions: [
            {
              kind: 'at-rule',
              text: '@media (min-width: 0px)',
              active: true,
            },
            {
              kind: 'selector',
              text: '.parent &:hover',
              active: false,
            },
          ],
          pseudoElement: '::before',
        }),
        type: 'set-rule',
      },
      element,
    );

    expect(result.ok).toBe(true);
    const cssText = document.adoptedStyleSheets[0].cssRules[0].cssText;
    expect(cssText).toContain('@media (min-width: 0px)');
    expect(cssText).toContain('.parent [data-stylex-devtools-target=');
    expect(cssText).toContain(':hover::before');
  });

  test('rolls back pseudo-element rules when refreshed collection fails', () => {
    const element = document.createElement('div');
    document.body.appendChild(element);
    let collectionCount = 0;
    const runtime = createInspectedPageRuntime((target) => {
      collectionCount += 1;
      if (collectionCount === 2) throw new Error('Collection blocked');
      return collectStylexDebugData(target);
    });
    const data = runtime.collect(element);

    const result = runtime.mutate(
      {
        ...baseCommand(data, { pseudoElement: '::before' }),
        type: 'set-rule',
      },
      element,
    );

    expect(result).toMatchObject({
      ok: false,
      code: 'mutation-failed',
      message: 'Collection blocked',
    });
    expect(document.adoptedStyleSheets).toEqual([]);
    expect(element.hasAttribute(RULE_TARGET_ATTRIBUTE)).toBe(false);
    expect(runtime.collect(element).overrides).toEqual([]);
  });

  test('restores a replaced pseudo-element rule when collection fails', () => {
    const element = document.createElement('div');
    document.body.appendChild(element);
    const runtime = createInspectedPageRuntime();
    const data = runtime.collect(element);
    const first = runtime.mutate(
      {
        ...baseCommand(data, { pseudoElement: '::before' }),
        type: 'set-rule',
      },
      element,
    );
    expect(first.ok).toBe(true);
    const firstOverride = first.ok ? first.data.overrides[0] : null;
    const firstSheet = document.adoptedStyleSheets[0];
    let collectionCount = 0;
    const failingRuntime = createInspectedPageRuntime((target) => {
      collectionCount += 1;
      if (collectionCount === 2) throw new Error('Collection blocked');
      return collectStylexDebugData(target);
    });
    const nextData = failingRuntime.collect(element);

    const result = failingRuntime.mutate(
      {
        ...baseCommand(nextData, {
          pseudoElement: '::before',
          replaceOverrideIds: [firstOverride?.id ?? ''],
          value: 'blue',
        }),
        type: 'set-rule',
      },
      element,
    );

    expect(result).toMatchObject({ ok: false, code: 'mutation-failed' });
    expect(document.adoptedStyleSheets).toEqual([firstSheet]);
    expect(runtime.collect(element).overrides[0]).toMatchObject({
      kind: 'rule',
      value: 'red',
    });
  });

  test('stores overrides independently per selected element', () => {
    const first = document.createElement('div');
    const second = document.createElement('div');
    document.body.append(first, second);
    const runtime = createInspectedPageRuntime();

    const firstData = runtime.collect(first);
    runtime.mutate({ ...baseCommand(firstData), type: 'set-inline' }, first);

    expect(runtime.collect(second).overrides).toEqual([]);
    expect(first.style.color).toBe('red');
    expect(second.style.color).toBe('');
  });

  test('keeps case-sensitive custom property overrides independent', () => {
    const element = document.createElement('div');
    document.body.appendChild(element);
    const runtime = createInspectedPageRuntime();
    const data = runtime.collect(element);

    expect(
      runtime.mutate(
        {
          ...baseCommand(data, {
            contextKey: 'upper-custom-property',
            property: '--Brand',
            value: 'red',
          }),
          type: 'set-inline',
        },
        element,
      ).ok,
    ).toBe(true);
    const result = runtime.mutate(
      {
        ...baseCommand(data, {
          contextKey: 'lower-custom-property',
          property: '--brand',
          value: 'blue',
        }),
        type: 'set-inline',
      },
      element,
    );

    expect(result.ok).toBe(true);
    expect(result.ok ? result.data.overrides : []).toHaveLength(2);
    expect(element.style.getPropertyValue('--Brand')).toBe('red');
    expect(element.style.getPropertyValue('--brand')).toBe('blue');
  });
});
