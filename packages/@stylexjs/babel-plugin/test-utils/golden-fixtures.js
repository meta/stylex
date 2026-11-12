/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

const fs = require('node:fs');
const path = require('node:path');
const { transformFileSync } = require('@babel/core');

const FIXTURES_ROOT = path.join(__dirname, '../__tests__/__fixtures__/golden');

const DEFAULT_PARSER_OPTS = {
  flow: 'all',
};

const DEFAULT_PROCESS_OPTIONS = {
  useLayers: false,
  enableLTRRTLComments: false,
};

function readJSON(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'));
}

function loadStylexPlugin() {
  try {
    const sourceModule = require('../src/index');
    return sourceModule.default ?? sourceModule;
  } catch (sourceError) {
    try {
      const builtModule = require('../lib/index.js');
      return builtModule.default ?? builtModule;
    } catch (builtError) {
      builtError.message +=
        '\nFailed to load the StyleX Babel plugin from src/ or lib/. ' +
        'Run the package build before generating golden fixtures.';
      throw builtError;
    }
  }
}

function getFixtureNames() {
  return fs
    .readdirSync(FIXTURES_ROOT)
    .filter((entry) =>
      fs.existsSync(path.join(FIXTURES_ROOT, entry, 'manifest.json')),
    )
    .sort();
}

function loadFixtureManifest(fixtureName) {
  const fixtureDir = path.join(FIXTURES_ROOT, fixtureName);
  const manifest = readJSON(path.join(fixtureDir, 'manifest.json'));
  return {
    fixtureName,
    fixtureDir,
    entry: manifest.entry ?? 'input.js',
    parserOpts: manifest.parserOpts ?? DEFAULT_PARSER_OPTS,
    pluginOptions: manifest.pluginOptions ?? {},
    processOptions: manifest.processOptions ?? DEFAULT_PROCESS_OPTIONS,
    ...manifest,
  };
}

function formatConsoleArgs(args) {
  return args
    .map((value) =>
      typeof value === 'string' ? value : JSON.stringify(value, null, 2),
    )
    .join(' ');
}

function normalizeString(value, fixtureDir) {
  return value
    .replaceAll('\r\n', '\n')
    .replaceAll(path.resolve(fixtureDir), '<FIXTURE_ROOT>')
    .replaceAll(path.resolve(process.cwd()), '<REPO_ROOT>');
}

function normalizeValue(value, fixtureDir) {
  if (Array.isArray(value)) {
    return value.map((item) => normalizeValue(item, fixtureDir));
  }
  if (value != null && typeof value === 'object') {
    return Object.fromEntries(
      Object.keys(value)
        .sort()
        .map((key) => [key, normalizeValue(value[key], fixtureDir)]),
    );
  }
  return typeof value === 'string' ? normalizeString(value, fixtureDir) : value;
}

function renderGoldenFixture(fixtureName) {
  const manifest = loadFixtureManifest(fixtureName);
  const stylexPlugin = loadStylexPlugin();
  const entryPath = path.join(manifest.fixtureDir, manifest.entry);

  const warnings = [];
  const errors = [];
  const originalWarn = console.warn;
  const originalError = console.error;

  console.warn = (...args) => {
    warnings.push(formatConsoleArgs(args));
  };
  console.error = (...args) => {
    errors.push(formatConsoleArgs(args));
  };

  try {
    const result = transformFileSync(entryPath, {
      babelrc: false,
      filename: entryPath,
      parserOpts: manifest.parserOpts,
      plugins: [[stylexPlugin, manifest.pluginOptions]],
    });

    if (result == null) {
      throw new Error(
        `No Babel transform result for fixture "${fixtureName}".`,
      );
    }

    const metadataStylex = result.metadata?.stylex ?? [];
    const finalCss = stylexPlugin.processStylexRules(
      metadataStylex,
      manifest.processOptions,
    );

    return normalizeValue(
      {
        code: result.code,
        entry: manifest.entry,
        errors,
        finalCss,
        fixtureName,
        metadataStylex,
        pluginOptions: manifest.pluginOptions,
        processOptions: manifest.processOptions,
        status: 'ok',
        warnings,
      },
      manifest.fixtureDir,
    );
  } catch (error) {
    return normalizeValue(
      {
        entry: manifest.entry,
        errors,
        error: {
          message: error instanceof Error ? error.message : String(error),
          name: error instanceof Error ? error.name : 'Error',
        },
        fixtureName,
        pluginOptions: manifest.pluginOptions,
        processOptions: manifest.processOptions,
        status: 'error',
        warnings,
      },
      manifest.fixtureDir,
    );
  } finally {
    console.warn = originalWarn;
    console.error = originalError;
  }
}

function getExpectedOutputPath(fixtureName) {
  return path.join(FIXTURES_ROOT, fixtureName, 'expected.json');
}

function readExpectedOutput(fixtureName) {
  return readJSON(getExpectedOutputPath(fixtureName));
}

module.exports = {
  FIXTURES_ROOT,
  getFixtureNames,
  loadFixtureManifest,
  readExpectedOutput,
  renderGoldenFixture,
};
