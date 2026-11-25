/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const { test, beforeEach } = require('node:test');
const assert = require('node:assert/strict');
const { unpluginFactory } = require('../../unplugin/lib/core.js');

const source = `
  import * as stylex from '@stylexjs/stylex';
  export const styles = stylex.create({
    root: { color: 'red', paddingInline: 4 },
    hover: { color: { default: null, ':hover': 'blue' } },
  });
`;

function plugin(compiler, extra = {}) {
  const result = unpluginFactory(
    {
      compiler,
      dev: false,
      devPersistToDisk: false,
      runtimeInjection: false,
      unstable_moduleResolution: { type: 'haste' },
      ...extra,
    },
    { framework: 'vite' },
  );
  result.buildStart();
  return result;
}

beforeEach(() => {
  if (globalThis.__stylex_unplugin_store)
    globalThis.__stylex_unplugin_store.rulesById.clear();
});

test('Rust opt-in collects exactly the same CSS as the default Babel compiler', async () => {
  const original = plugin('babel');
  await original.transform(source, '/virtual/component.js');
  const expected = original.__stylexCollectCss();
  const rust = plugin('rust');
  const result = await rust.transform(source, '/virtual/component.js');
  assert.ok(!result.code.includes('stylex.create('));
  assert.equal(result.map.version, 3);
  assert.deepEqual(result.map.sourcesContent, [source]);
  assert.equal(rust.__stylexCollectCss(), expected);
});

test('Rust opt-in removes stale cached CSS when a file loses its styles', async () => {
  const rust = plugin('rust');
  await rust.transform(source, '/virtual/component.js');
  assert.ok(rust.__stylexCollectCss().includes('color: red'));
  await rust.transform(
    "import * as stylex from '@stylexjs/stylex'; export const value = 1;",
    '/virtual/component.js',
  );
  assert.equal(rust.__stylexCollectCss(), '');
});

test('Rust opt-in matches the Babel path when Babel plugins run alongside', async () => {
  const recolor = (path) => {
    if (path.node.value === 'red') path.node.value = 'green';
  };
  // StyleX compiles on Program entry, so per-node visitors of other plugins see
  // compiled output while an earlier plugin's Program.enter edits are compiled.
  const perNode = () => ({ visitor: { StringLiteral: recolor } });
  const onEntry = () => ({
    visitor: {
      Program(program) {
        program.traverse({ StringLiteral: recolor });
      },
    },
  });
  for (const [preplugin, expected] of [
    [perNode, 'color: red'],
    [onEntry, 'color: green'],
  ]) {
    const original = plugin('babel', { babelConfig: { plugins: [preplugin] } });
    await original.transform(source, '/virtual/component.js');
    const expectedCss = original.__stylexCollectCss();
    globalThis.__stylex_unplugin_store?.rulesById.clear();
    const rust = plugin('rust', { babelConfig: { plugins: [preplugin] } });
    await rust.transform(source, '/virtual/component.js');
    assert.ok(expectedCss.includes(expected));
    assert.equal(rust.__stylexCollectCss(), expectedCss);
  }
});

test('Rust opt-in parses non-JS extensions with the TypeScript grammar the Babel path uses', async () => {
  const typed = `
    import * as stylex from '@stylexjs/stylex';
    type Size = number;
    const size: Size = 4;
    export const styles = stylex.create({ root: { padding: size, color: 'red' } });
    export function size2<T>(value: T): T { return value; }
  `;
  // The Babel path parses every non-JS extension as TSX, so JSX in a `.ts` id is accepted.
  for (const [code, id] of [
    [typed, '/virtual/component.mts'],
    [`${typed}\nexport const el = <div />;`, '/virtual/component.ts'],
  ]) {
    const original = plugin('babel');
    await original.transform(code, id);
    const expectedCss = original.__stylexCollectCss();
    globalThis.__stylex_unplugin_store?.rulesById.clear();
    const rust = plugin('rust');
    const result = await rust.transform(code, id);
    assert.ok(!result.code.includes('stylex.create('));
    assert.ok(expectedCss.includes('padding: 4px'));
    assert.equal(rust.__stylexCollectCss(), expectedCss);
  }
});

test('an invalid compiler choice fails rather than silently falling back', () => {
  assert.throws(
    () => plugin('other'),
    /StyleX compiler must be "babel" or "rust"/,
  );
});

test('Rust opt-in applies the project babel.config like the Babel path', async () => {
  const fs = require('node:fs');
  const os = require('node:os');
  const path = require('node:path');
  const project = fs.mkdtempSync(path.join(os.tmpdir(), 'stylex-unplugin-'));
  const previous = process.cwd();
  try {
    fs.writeFileSync(path.join(project, 'package.json'), '{"private":true}');
    fs.mkdirSync(path.join(project, 'node_modules/@stylexjs'), {
      recursive: true,
    });
    fs.symlinkSync(
      path.resolve(__dirname, '..'),
      path.join(project, 'node_modules/@stylexjs/rust-plugin'),
      'junction',
    );
    // A project plugin that recolors a style value before StyleX compiles it
    // and marks the module, so both its edit and its output are observable.
    fs.writeFileSync(
      path.join(project, 'babel.config.js'),
      `module.exports = { plugins: [({ types: t }) => ({ visitor: { Program: {
        enter(p) { p.traverse({ StringLiteral(s) { if (s.node.value === 'red') s.node.value = 'green'; } }); },
        exit(p) { p.pushContainer('body', t.expressionStatement(t.stringLiteral('project config'))); },
      } } })] };`,
    );
    process.chdir(project);
    const id = path.join(project, 'src/component.js');
    const original = plugin('babel');
    const expected = await original.transform(source, id);
    const expectedCss = original.__stylexCollectCss();
    globalThis.__stylex_unplugin_store?.rulesById.clear();
    const rust = plugin('rust');
    const result = await rust.transform(source, id);
    assert.ok(expected.code.includes('"project config"'));
    assert.ok(expectedCss.includes('color: green'));
    assert.ok(result.code.includes('"project config"'));
    assert.equal(rust.__stylexCollectCss(), expectedCss);
  } finally {
    process.chdir(previous);
    fs.rmSync(project, { recursive: true, force: true });
  }
});
