/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @jest-environment jsdom
 */

'use strict';

const {
  DEV_RUNTIME_SCRIPT,
  VIRTUAL_STYLEX_RUNTIME_SCRIPT,
  VIRTUAL_STYLEX_CSS_ONLY_SCRIPT,
} = require('../src/consts');

// Execute the generated browser module with a controllable HMR connection.
function startRuntime(script, css) {
  const handlers = {};
  const disposers = [];
  const hot = {
    on: (event, handler) => {
      handlers[event] = handler;
    },
    off: jest.fn((event, handler) => {
      if (handlers[event] === handler) delete handlers[event];
    }),
    dispose: (callback) => disposers.push(callback),
  };
  const fetch = jest.fn(async () => ({ text: async () => css }));
  const source = script
    .replaceAll('import.meta.hot', 'hot')
    .replace('export {};', '');
  // The input is our generated module, not user-provided code.
  // eslint-disable-next-line no-new-func
  new Function('hot', 'fetch', source)(hot, fetch);
  return {
    handlers,
    fetch,
    off: hot.off,
    dispose: () => disposers.forEach((fn) => fn()),
  };
}

async function flushUpdate() {
  // fetchCSS and update each await a promise.
  for (let i = 0; i < 5; i++) await Promise.resolve();
}

describe.each([
  ['virtual runtime', VIRTUAL_STYLEX_RUNTIME_SCRIPT],
  ['middleware runtime', DEV_RUNTIME_SCRIPT],
])('%s stylesheet handoff', (_name, script) => {
  beforeEach(() => {
    document.head.innerHTML = '';
  });

  function createLink(href) {
    const link = document.createElement('link');
    link.rel = 'stylesheet';
    link.href = href;
    document.head.appendChild(link);
    return link;
  }

  test('disables the SSR stylesheet without changing hydration attributes, including HMR', async () => {
    const link = createLink('/virtual:stylex.css');
    const other = createLink('/other.css');
    const sheet = { disabled: false };
    const otherSheet = { disabled: false };
    Object.defineProperty(link, 'sheet', { value: sheet });
    Object.defineProperty(other, 'sheet', { value: otherSheet });
    const markup = link.outerHTML;
    const { handlers, fetch } = startRuntime(
      script,
      '.example { color: red; }',
    );
    await flushUpdate();
    expect(sheet.disabled).toBe(true);
    expect(link.outerHTML).toBe(markup);
    expect(link.hasAttribute('disabled')).toBe(false);
    expect(otherSheet.disabled).toBe(false);
    expect(document.getElementById('__stylex_virtual__').textContent).toContain(
      'red',
    );

    fetch.mockResolvedValue({ text: async () => '.example { color: blue; }' });
    await handlers['stylex:css-update']();
    expect(document.getElementById('__stylex_virtual__').textContent).toContain(
      'blue',
    );
    expect(link.outerHTML).toBe(markup);
    expect(sheet.disabled).toBe(true);
  });

  test('disables a stylesheet that loads after the runtime without changing the link', async () => {
    const link = createLink('/virtual:stylex.css');
    let sheet = null;
    Object.defineProperty(link, 'sheet', { get: () => sheet });
    const markup = link.outerHTML;
    startRuntime(script, '.example { color: red; }');
    await flushUpdate();
    expect(link.outerHTML).toBe(markup);
    sheet = { disabled: false };
    link.dispatchEvent(new Event('load'));
    expect(sheet.disabled).toBe(true);
    expect(link.outerHTML).toBe(markup);
  });
});

describe('css-only stylesheet handoff', () => {
  let runtime;
  let readyState;

  beforeEach(() => {
    document.head.innerHTML = '';
    readyState = jest.spyOn(document, 'readyState', 'get');
    readyState.mockReturnValue('complete');
    jest.spyOn(Date, 'now').mockReturnValue(12345);
  });

  afterEach(() => {
    runtime?.dispose();
    runtime = null;
    jest.restoreAllMocks();
  });

  function createLink(href) {
    const link = document.createElement('link');
    link.rel = 'stylesheet';
    link.href = href;
    document.head.appendChild(link);
    return link;
  }

  function start() {
    runtime = startRuntime(VIRTUAL_STYLEX_CSS_ONLY_SCRIPT, '');
    return runtime;
  }

  test('keeps the active sheet until load and preserves attributes and cascade order', () => {
    const before = createLink('/before.css');
    const link = createLink('/virtual:stylex.css?existing=1');
    link.media = 'screen';
    link.crossOrigin = 'anonymous';
    const after = createLink('/after.css');
    const markup = link.outerHTML;
    const { handlers } = start();
    const replacement = link.nextElementSibling;

    expect(link.outerHTML).toBe(markup);
    expect(replacement).not.toBe(after);
    expect(replacement.media).toBe('screen');
    expect(replacement.crossOrigin).toBe('anonymous');
    expect(replacement.getAttribute('href')).toBe(
      '/virtual:stylex.css?existing=1&t=12345',
    );
    expect([...document.head.children]).toEqual([
      before,
      link,
      replacement,
      after,
    ]);

    replacement.dispatchEvent(new Event('load'));
    expect([...document.head.children]).toEqual([before, replacement, after]);
    handlers['stylex:css-update']();
    const next = replacement.nextElementSibling;
    expect(replacement.isConnected).toBe(true);
    next.dispatchEvent(new Event('load'));
    expect([...document.head.children]).toEqual([before, next, after]);
  });

  test('retains the active sheet on error and allows the next update to succeed', () => {
    const link = createLink('/virtual:stylex.css');
    const { handlers } = start();
    const failed = link.nextElementSibling;
    failed.dispatchEvent(new Event('error'));
    expect([...document.head.children]).toEqual([link]);

    handlers['stylex:css-update']();
    const replacement = link.nextElementSibling;
    expect(replacement.href).not.toBe(failed.href);
    failed.dispatchEvent(new Event('load'));
    expect(link.isConnected).toBe(true);
    replacement.dispatchEvent(new Event('load'));
    expect([...document.head.children]).toEqual([replacement]);
  });

  test('cancels stale requests, including repeated updates in the same millisecond', () => {
    const link = createLink('/virtual:stylex.css');
    const { handlers } = start();
    const first = link.nextElementSibling;
    handlers['stylex:css-update']();
    const second = link.nextElementSibling;
    handlers['stylex:css-update']();
    const latest = link.nextElementSibling;

    expect(first.isConnected).toBe(false);
    expect(second.isConnected).toBe(false);
    expect(new Set([first.href, second.href, latest.href]).size).toBe(3);
    expect([...document.head.children]).toEqual([link, latest]);
    first.dispatchEvent(new Event('load'));
    second.dispatchEvent(new Event('error'));
    expect([...document.head.children]).toEqual([link, latest]);
    latest.dispatchEvent(new Event('load'));
    second.dispatchEvent(new Event('load'));
    expect([...document.head.children]).toEqual([latest]);
  });

  test('handles multiple matching links without replacing pending or unrelated sheets', () => {
    const first = createLink('/virtual:stylex.css');
    const other = createLink('/other.css?next=/virtual:stylex.css');
    const second = createLink('/base/virtual:stylex.css');
    const remote = createLink('https://example.com/virtual:stylex.css');
    const { handlers } = start();
    const staleFirst = first.nextElementSibling;
    const staleSecond = second.nextElementSibling;
    handlers['stylex:css-update']();
    const nextFirst = first.nextElementSibling;
    const nextSecond = second.nextElementSibling;

    expect(staleFirst.isConnected).toBe(false);
    expect(staleSecond.isConnected).toBe(false);
    expect([...document.head.children]).toEqual([
      first,
      nextFirst,
      other,
      second,
      nextSecond,
      remote,
    ]);
    nextSecond.dispatchEvent(new Event('load'));
    nextFirst.dispatchEvent(new Event('error'));
    expect([...document.head.children]).toEqual([
      first,
      other,
      nextSecond,
      remote,
    ]);
    handlers['stylex:css-update']();
    expect(document.head.children.length).toBe(6);
    expect(other.isConnected).toBe(true);
    expect(remote.isConnected).toBe(true);
  });

  test('skips disabled links and sheets while refreshing enabled sheets', () => {
    const disabled = createLink('/virtual:stylex.css?disabled');
    disabled.disabled = true;
    const sheetDisabled = createLink('/virtual:stylex.css?sheet-disabled');
    const sheet = { disabled: true };
    Object.defineProperty(sheetDisabled, 'sheet', { value: sheet });
    const active = createLink('/virtual:stylex.css?active');
    const { handlers } = start();
    expect([...document.head.children]).toEqual([
      disabled,
      sheetDisabled,
      active,
      active.nextElementSibling,
    ]);
    active.nextElementSibling.dispatchEvent(new Event('load'));
    expect(disabled.isConnected).toBe(true);
    expect(sheet.disabled).toBe(true);

    disabled.disabled = false;
    sheet.disabled = false;
    handlers['stylex:css-update']();
    expect(disabled.nextElementSibling).not.toBe(sheetDisabled);
    expect(sheetDisabled.nextElementSibling.href).toContain('sheet-disabled');
  });

  test.each(['href', 'media', 'disabled', 'sheet-disabled', 'reinserted'])(
    'keeps host changes when the active link is %s while loading',
    (change) => {
      const link = createLink('/virtual:stylex.css');
      const { handlers } = start();
      const replacement = link.nextElementSibling;
      if (change === 'href') link.href = '/host.css';
      if (change === 'media') link.media = 'print';
      if (change === 'disabled') link.disabled = true;
      if (change === 'sheet-disabled') {
        Object.defineProperty(link, 'sheet', { value: { disabled: true } });
      }
      if (change === 'reinserted') {
        link.remove();
        document.head.appendChild(link);
      }
      const markup = link.outerHTML;
      replacement.dispatchEvent(new Event('load'));
      expect([...document.head.children]).toEqual([link]);
      expect(link.outerHTML).toBe(markup);
      if (
        change === 'href' ||
        change === 'disabled' ||
        change === 'sheet-disabled'
      ) {
        handlers['stylex:css-update']();
        expect([...document.head.children]).toEqual([link]);
      }
    },
  );

  test('waits for DOMContentLoaded and cleans pending requests on disposal', () => {
    readyState.mockReturnValue('loading');
    const link = createLink('/virtual:stylex.css');
    const { handlers, dispose, off } = start();
    const staleHandler = handlers['stylex:css-update'];
    expect([...document.head.children]).toEqual([link]);
    document.dispatchEvent(new Event('DOMContentLoaded'));
    const replacement = link.nextElementSibling;
    expect(document.head.children.length).toBe(2);

    dispose();
    expect(off).toHaveBeenCalledWith('stylex:css-update', staleHandler);
    expect(handlers['stylex:css-update']).toBeUndefined();
    replacement.dispatchEvent(new Event('load'));
    staleHandler();
    document.dispatchEvent(new Event('DOMContentLoaded'));
    expect([...document.head.children]).toEqual([link]);
  });

  test('removes a deferred initialization listener when disposed before DOMContentLoaded', () => {
    readyState.mockReturnValue('loading');
    const link = createLink('/virtual:stylex.css');
    const { dispose } = start();
    dispose();
    document.dispatchEvent(new Event('DOMContentLoaded'));
    expect([...document.head.children]).toEqual([link]);
  });

  test('does not resurrect a sheet whose active link was removed while loading', () => {
    const link = createLink('/virtual:stylex.css');
    const other = createLink('/other.css');
    start();
    const replacement = link.nextElementSibling;
    link.remove();
    replacement.dispatchEvent(new Event('load'));
    expect([...document.head.children]).toEqual([other]);
  });
});
