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
} = require('../src/consts');

// Execute the generated browser module with a controllable HMR connection.
function startRuntime(script, css) {
  const handlers = {};
  const hot = {
    on: (event, handler) => {
      handlers[event] = handler;
    },
    dispose: () => {},
  };
  const fetch = jest.fn(async () => ({ text: async () => css }));
  const source = script
    .replaceAll('import.meta.hot', 'hot')
    .replace('export {};', '');
  // The input is our generated module, not user-provided code.
  // eslint-disable-next-line no-new-func
  new Function('hot', 'fetch', source)(hot, fetch);
  return { handlers, fetch };
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
