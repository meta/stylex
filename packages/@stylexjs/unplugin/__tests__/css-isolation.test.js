/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { unplugin } = require('../src');

const FIRST_STYLE = `
  import * as stylex from '@stylexjs/stylex';
  export const styles = stylex.create({ root: { zIndex: 271828 } });
`;
const SECOND_STYLE = `
  import * as stylex from '@stylexjs/stylex';
  export const styles = stylex.create({ root: { zIndex: 314159 } });
`;

describe('production CSS isolation', () => {
  beforeEach(() => {
    delete globalThis.__stylex_unplugin_store;
  });

  afterEach(() => {
    delete globalThis.__stylex_unplugin_store;
  });

  test.each(['vite', 'rollup'])(
    '%s does not collect rules from a previous build',
    async (framework) => {
      const first = unplugin[framework]({ dev: false });
      first.buildStart();
      await first.transform(FIRST_STYLE, '/virtual/first.js');
      expect(first.__stylexCollectCss()).toContain('271828');

      const second = unplugin[framework]({ dev: false });
      second.buildStart();
      await second.transform(SECOND_STYLE, '/virtual/second.js');

      const css = second.__stylexCollectCss();
      expect(css).toContain('314159');
      expect(css).not.toContain('271828');
    },
  );

  test.each(['vite', 'rollup'])(
    '%s clears rules when the same plugin starts another build',
    async (framework) => {
      const plugin = unplugin[framework]({ dev: false });
      plugin.buildStart();
      await plugin.transform(FIRST_STYLE, '/virtual/first.js');
      expect(plugin.__stylexCollectCss()).toContain('271828');

      plugin.buildStart();
      await plugin.transform(SECOND_STYLE, '/virtual/second.js');

      const css = plugin.__stylexCollectCss();
      expect(css).toContain('314159');
      expect(css).not.toContain('271828');
    },
  );

  test('Vite development client and SSR instances share their CSS', async () => {
    const client = unplugin.vite({ dev: true });
    client.configResolved({ command: 'serve' });
    client.buildStart();
    await client.transform(FIRST_STYLE, '/virtual/client.js');

    const ssr = unplugin.vite({ dev: true });
    ssr.configResolved({ command: 'serve' });
    ssr.buildStart();
    await ssr.transform(SECOND_STYLE, '/virtual/ssr.js');

    for (const plugin of [client, ssr]) {
      const css = plugin.__stylexCollectCss();
      expect(css).toContain('271828');
      expect(css).toContain('314159');
    }
  });

  test('Vite retains server rules across environments in one production build', async () => {
    const plugin = unplugin.vite({ dev: false });
    plugin.configResolved({ command: 'build' });
    plugin.buildStart();
    await plugin.transform(FIRST_STYLE, '/virtual/server.js');

    plugin.buildStart();
    await plugin.transform(SECOND_STYLE, '/virtual/client.js');

    const css = plugin.__stylexCollectCss();
    expect(css).toContain('271828');
    expect(css).toContain('314159');
  });

  test('Vite clears production rules when resolving a new build', async () => {
    const plugin = unplugin.vite({ dev: false });
    plugin.configResolved({ command: 'build' });
    plugin.buildStart();
    await plugin.transform(FIRST_STYLE, '/virtual/first.js');

    plugin.configResolved({ command: 'build' });
    plugin.buildStart();
    await plugin.transform(SECOND_STYLE, '/virtual/second.js');

    const css = plugin.__stylexCollectCss();
    expect(css).toContain('314159');
    expect(css).not.toContain('271828');
  });

  test('Vite writes fallback CSS after an environment with a CSS asset', async () => {
    const plugin = unplugin.vite({ dev: false });
    plugin.configResolved({ command: 'build' });
    plugin.buildStart();
    await plugin.transform(FIRST_STYLE, '/virtual/server.js');
    const context = {
      emitFile: () => 'css',
      getFileName: () => 'assets/index.css',
    };
    plugin.generateBundle.call(
      context,
      {},
      {
        'assets/index.css': {
          type: 'asset',
          fileName: 'assets/index.css',
          source: '',
        },
      },
    );

    plugin.buildStart();
    await plugin.transform(SECOND_STYLE, '/virtual/client.js');
    plugin.generateBundle.call(context, {}, {});
    const output = fs.mkdtempSync(path.join(os.tmpdir(), 'stylex-isolation-'));
    try {
      await plugin.writeBundle({ dir: output }, {});
      const css = fs.readFileSync(
        path.join(output, 'assets/stylex.css'),
        'utf8',
      );
      expect(css).toContain('271828');
      expect(css).toContain('314159');
    } finally {
      fs.rmSync(output, { recursive: true, force: true });
    }
  });

  test('a production build neither reads nor modifies active development rules', async () => {
    const development = unplugin.vite({ dev: true });
    development.configResolved({ command: 'serve' });
    development.buildStart();
    await development.transform(FIRST_STYLE, '/virtual/development.js');
    expect(development.__stylexCollectCss()).toContain('271828');

    const production = unplugin.vite({ dev: true });
    production.configResolved({ command: 'build' });
    production.buildStart();
    await production.transform(SECOND_STYLE, '/virtual/production.js');

    const productionCss = production.__stylexCollectCss();
    expect(productionCss).toContain('314159');
    expect(productionCss).not.toContain('271828');

    const developmentCss = development.__stylexCollectCss();
    expect(developmentCss).toContain('271828');
    expect(developmentCss).not.toContain('314159');
  });

  test('a Vite instance stops sharing when reused for a production build', async () => {
    const plugin = unplugin.vite({ dev: true });
    plugin.configResolved({ command: 'serve' });
    plugin.buildStart();
    await plugin.transform(FIRST_STYLE, '/virtual/development.js');

    plugin.configResolved({ command: 'build' });
    plugin.buildStart();
    await plugin.transform(SECOND_STYLE, '/virtual/production.js');

    const css = plugin.__stylexCollectCss();
    expect(css).toContain('314159');
    expect(css).not.toContain('271828');
  });
});
