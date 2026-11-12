/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { babel } from '@rollup/plugin-babel';
import commonjs from '@rollup/plugin-commonjs';
import json from '@rollup/plugin-json';
import { nodeResolve } from '@rollup/plugin-node-resolve';
import replace from '@rollup/plugin-replace';
import stylex from '@stylexjs/unplugin';
import browserslist from 'browserslist';
import { browserslistToTargets, Features } from 'lightningcss';

const rootDir = path.dirname(fileURLToPath(import.meta.url));
const distDir = path.join(rootDir, 'dist');
const sharedDir = path.join(distDir, '.shared');
const appOutDir = path.join(sharedDir, 'app');
const runtimeOutDir = path.join(sharedDir, 'runtime');
const extensions = ['.js', '.jsx'];
const isWatch = Boolean(process.env.ROLLUP_WATCH);

function cleanDirectory(directory) {
  return {
    name: `clean-${path.basename(directory)}`,
    async buildStart() {
      await fs.rm(directory, { recursive: true, force: true });
    },
  };
}

function copyStaticFiles() {
  const files = [
    ['devtools.html', 'devtools.html'],
    ['panel.html', 'panel.html'],
    ['src/panel/index.css', 'assets/reset.css'],
    ['icons/stylex-icon.svg', 'assets/stylex-icon.svg'],
    ['icons/stylex-icon-16.png', 'assets/stylex-icon-16.png'],
    ['icons/stylex-icon-32.png', 'assets/stylex-icon-32.png'],
    ['icons/stylex-icon-48.png', 'assets/stylex-icon-48.png'],
    ['icons/stylex-icon-128.png', 'assets/stylex-icon-128.png'],
    ['icons/stylex-icon-256.png', 'assets/stylex-icon-256.png'],
    ['icons/stylex-icon-512.png', 'assets/stylex-icon-512.png'],
  ];
  return {
    name: 'copy-static-files',
    buildStart() {
      for (const [source] of files) {
        this.addWatchFile(path.join(rootDir, source));
      }
    },
    writeBundle: {
      sequential: true,
      async handler() {
        for (const [source, destination] of files) {
          const output = path.join(appOutDir, destination);
          await fs.mkdir(path.dirname(output), { recursive: true });
          await fs.copyFile(path.join(rootDir, source), output);
        }
      },
    },
  };
}

async function readManifest(name) {
  return JSON.parse(
    await fs.readFile(path.join(rootDir, 'manifests', `${name}.json`), 'utf8'),
  );
}

let assemblyQueue = Promise.resolve();
async function assembleBrowserOutputs() {
  const baseManifest = await readManifest('base');
  for (const browserName of ['chrome', 'firefox', 'safari']) {
    const output = path.join(distDir, browserName);
    const overlay = await readManifest(browserName);
    await fs.rm(output, { recursive: true, force: true });
    await fs.mkdir(output, { recursive: true });
    const sources = [appOutDir, runtimeOutDir];
    for (const source of sources) {
      try {
        await fs.cp(source, output, { recursive: true });
      } catch (error) {
        if (error?.code !== 'ENOENT') throw error;
      }
    }
    await fs.writeFile(
      path.join(output, 'manifest.json'),
      `${JSON.stringify({ ...baseManifest, ...overlay }, null, 2)}\n`,
    );
  }
}

function assembleBrowsers() {
  return {
    name: 'assemble-browser-outputs',
    writeBundle: {
      order: 'post',
      sequential: true,
      async handler() {
        assemblyQueue = assemblyQueue
          .catch(() => {})
          .then(assembleBrowserOutputs);
        await assemblyQueue;
      },
    },
  };
}

function babelPlugin() {
  return babel({
    babelHelpers: 'bundled',
    extensions,
    babelrc: true,
    configFile: path.join(rootDir, '.babelrc.js'),
    include: [
      path.join(rootDir, 'src/**/*'),
      path.join(rootDir, 'flow-types/**/*'),
    ],
    exclude: ['**/node_modules/**'],
  });
}

const resolvePlugin = () =>
  nodeResolve({
    browser: true,
    extensions,
    preferBuiltins: false,
  });

const runtimeConfig = {
  input: path.join(rootDir, 'src/inspected/runtimeEntry.js'),
  output: {
    dir: runtimeOutDir,
    format: 'iife',
    name: 'StylexInspectedRuntime',
    entryFileNames: 'assets/inspected-runtime.js',
    inlineDynamicImports: true,
    sourcemap: false,
  },
  plugins: [
    cleanDirectory(runtimeOutDir),
    babelPlugin(),
    resolvePlugin(),
    json(),
    commonjs({ include: /node_modules/ }),
    assembleBrowsers(),
  ],
};

const appConfig = {
  input: {
    devtools: path.join(rootDir, 'src/devtools/main.js'),
    panel: path.join(rootDir, 'src/panel/main.js'),
  },
  output: {
    dir: appOutDir,
    format: 'es',
    sourcemap: false,
    entryFileNames: 'assets/[name].js',
    chunkFileNames: 'assets/chunk-[hash].js',
    assetFileNames: 'assets/[name][extname]',
  },
  plugins: [
    cleanDirectory(appOutDir),
    replace({
      preventAssignment: true,
      values: {
        'process.env.NODE_ENV': JSON.stringify(
          isWatch ? 'development' : 'production',
        ),
      },
    }),
    stylex.rollup({
      devMode: 'off',
      useCSSLayers: true,
      lightningcssOptions: {
        targets: browserslistToTargets(browserslist('>= 2%')),
        exclude: Features.LightDark,
      },
    }),
    babelPlugin(),
    resolvePlugin(),
    json(),
    commonjs({ include: /node_modules/ }),
    copyStaticFiles(),
    assembleBrowsers(),
  ],
};

async function getConfigs() {
  await fs.rm(distDir, { recursive: true, force: true });
  return [runtimeConfig, appConfig];
}

export default getConfigs();
