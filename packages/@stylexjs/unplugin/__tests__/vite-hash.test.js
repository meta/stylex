/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const packageDir = path.resolve(__dirname, '..');

// Vite 8 only ships ESM, which Jest cannot load here, so every build runs in a
// child Node process. That process loads the plugin from `src` through
// `@babel/register` and prints the bundle as JSON.
const BUILD_SCRIPT = `
import path from 'node:path';
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';

const [packageDir, root] = process.argv.slice(2);
const require = createRequire(path.join(packageDir, 'package.json'));
require('@babel/register')({
  cwd: packageDir,
  only: [path.join(packageDir, 'src')],
  cache: false,
});
const stylex = require('./src/vite.js').default;
const { build } = await import(pathToFileURL(require.resolve('vite')).href);

const result = await build({
  root,
  configFile: false,
  logLevel: 'silent',
  plugins: [stylex()],
  build: {
    write: false,
    rolldownOptions: { external: ['@stylexjs/stylex'] },
  },
});
const output = (Array.isArray(result) ? result : [result]).flatMap(
  (r) => r.output,
);
process.stdout.write(
  JSON.stringify(
    output.map((file) => ({
      type: file.type,
      name: file.name,
      fileName: file.fileName,
      content: file.type === 'chunk' ? file.code : String(file.source),
    })),
  ),
);
`;

function writeFile(root, file, content) {
  const target = path.join(root, file);
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.writeFileSync(target, content, 'utf8');
}

describe('Vite build file names', () => {
  let root;

  beforeAll(() => {
    root = fs.mkdtempSync(path.join(os.tmpdir(), 'stylex-unplugin-hash-'));
    writeFile(root, 'package.json', '{ "private": true, "type": "module" }');
    writeFile(root, 'build.mjs', BUILD_SCRIPT);
    writeFile(
      root,
      'index.html',
      '<!doctype html><html><body><script type="module" src="/src/main.js"></script></body></html>',
    );
    writeFile(root, 'src/app.css', 'body { margin: 0; }\n');
    writeFile(root, 'src/lazy.js', 'export const answer = 42;\n');
    writeFile(
      root,
      'src/main.js',
      `
import * as stylex from '@stylexjs/stylex';
import cssUrl from './app.css?url';
import { tokens } from './tokens.stylex.js';

const styles = stylex.create({ root: { color: tokens.accent } });

const link = document.createElement('link');
link.rel = 'stylesheet';
link.href = cssUrl;
document.head.append(link);
document.body.className = stylex.props(styles.root).className ?? '';
import('./lazy.js').then(({ answer }) => {
  document.body.dataset.answer = String(answer);
});
`,
    );
  });

  afterAll(() => {
    fs.rmSync(root, { recursive: true, force: true });
  });

  function buildWithAccent(accent) {
    // Only the token value changes, so the compiled JS stays the same and only
    // the StyleX CSS differs between builds.
    writeFile(
      root,
      'src/tokens.stylex.js',
      `
import * as stylex from '@stylexjs/stylex';

export const tokens = stylex.defineVars({ accent: '${accent}' });
`,
    );
    const env = { ...process.env, NODE_ENV: 'production', BABEL_ENV: 'cjs' };
    const result = spawnSync(
      process.execPath,
      ['build.mjs', packageDir, root],
      { cwd: root, env, encoding: 'utf8', timeout: 60000 },
    );
    if (result.error != null || result.status !== 0) {
      throw new Error(
        `vite build failed:\n${result.error ?? ''}${result.stderr}${result.stdout}`,
      );
    }
    return JSON.parse(result.stdout);
  }

  const chunksByName = (files) =>
    new Map(files.filter((f) => f.type === 'chunk').map((f) => [f.name, f]));
  const cssFile = (files) => files.find((f) => f.fileName.endsWith('.css'));

  test('renames every chunk whose content changes with the StyleX CSS', () => {
    const first = buildWithAccent('#123456');
    const second = buildWithAccent('#654321');

    expect(cssFile(first).content).toContain('#123456');
    expect(cssFile(second).content).toContain('#654321');
    expect(cssFile(second).fileName).not.toBe(cssFile(first).fileName);

    const firstChunks = chunksByName(first);
    const secondChunks = chunksByName(second);
    expect([...secondChunks.keys()].sort()).toEqual(
      [...firstChunks.keys()].sort(),
    );

    // A chunk that keeps its hashed name must keep its exact bytes, or caches
    // that treat hashed files as immutable serve stale code.
    const staleChunks = [...firstChunks.values()]
      .filter((chunk) => {
        const next = secondChunks.get(chunk.name);
        return (
          next.fileName === chunk.fileName && next.content !== chunk.content
        );
      })
      .map((chunk) => chunk.fileName);
    expect(staleChunks).toEqual([]);

    // The entry embeds the CSS file name, so it has to be renamed.
    const entry = secondChunks.get('index');
    expect(entry.content).toContain(path.basename(cssFile(second).fileName));
    expect(entry.fileName).not.toBe(firstChunks.get('index').fileName);

    // A chunk that does not link to any CSS keeps its name.
    expect(secondChunks.get('lazy').fileName).toBe(
      firstChunks.get('lazy').fileName,
    );

    // The same input gives the same file names.
    const firstAgain = buildWithAccent('#123456');
    expect(firstAgain.map((f) => f.fileName).sort()).toEqual(
      first.map((f) => f.fileName).sort(),
    );
  }, 120000);
});
