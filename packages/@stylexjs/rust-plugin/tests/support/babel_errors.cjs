/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

// How an exception thrown by user code reaches the caller, compared between
// the original Babel plugin run by @babel/core, the native compiler's direct
// API and its Babel facade. Every engine gets fresh options and a fresh
// thrown value, since Babel mutates the value it rethrows.

const path = require('node:path');
const babel = require('@babel/core');

const root = path.resolve(__dirname, '..', '..');
const header = "import * as stylex from '@stylexjs/stylex';\n";
const envStyles =
  "export const s = stylex.create({ a: { color: stylex.env.brand('x') } });";

function runEngine(engine, code, filename, options) {
  const file = filename === undefined ? {} : { filename };
  if (engine === 'direct') {
    return require(root).transformSync(code, { ...options, ...file });
  }
  const plugin =
    engine === 'babel'
      ? require('@stylexjs/babel-plugin')
      : require(path.join(root, 'babel'));
  return babel.transformSync(code, {
    ...file,
    babelrc: false,
    configFile: false,
    highlightCode: false,
    plugins: [[plugin, options]],
  });
}

function isObject(value) {
  return (
    value !== null && (typeof value === 'object' || typeof value === 'function')
  );
}

// Plain data describing a caught value, read only after the engine is done.
function describe(value) {
  if (!isObject(value)) return { kind: typeof value, value: String(value) };
  const message = Object.getOwnPropertyDescriptor(value, 'message');
  return {
    kind: value.constructor?.name ?? 'Object',
    message:
      message != null && 'value' in message
        ? String(message.value).split('\n')[0]
        : undefined,
    code: Object.getOwnPropertyDescriptor(value, 'code')?.value,
    extra: Object.getOwnPropertyDescriptor(value, 'extra')?.value,
  };
}

// The scenario factory returns `{ code, options, thrown, reads }` built fresh;
// `reads` counts reads of an accessor message, when the scenario has one.
function outcome(engine, scenario, filename) {
  const { code, options, thrown, reads } = scenario();
  try {
    runEngine(engine, code, filename, options);
    return { completed: true };
  } catch (error) {
    return {
      reads: reads?.(),
      same: error === thrown,
      caught: describe(error),
      thrownAfter: describe(thrown),
    };
  }
}

function envThrow(make) {
  return () => {
    const state = make();
    return {
      code: header + envStyles,
      options: {
        env: {
          brand: () => {
            throw state.thrown;
          },
        },
      },
      thrown: state.thrown,
      reads: state.reads,
    };
  };
}

const scenarios = {
  'an Error with extra fields': envThrow(() => {
    const thrown = new Error('boom');
    thrown.extra = 42;
    return { thrown };
  }),
  'an Error with its own code': envThrow(() => {
    const thrown = new Error('coded');
    thrown.code = 'E_OWN';
    return { thrown };
  }),
  'a string': envThrow(() => ({ thrown: 'string boom' })),
  null: envThrow(() => ({ thrown: null })),
  undefined: envThrow(() => ({ thrown: undefined })),
  'a frozen Error': envThrow(() => ({
    thrown: Object.freeze(new Error('frozen')),
  })),
  'a plain object': envThrow(() => ({ thrown: { message: 'plain' } })),
  'an object with an accessor message': envThrow(() => {
    let count = 0;
    let message = 'accessor';
    const thrown = {
      get message() {
        count++;
        return message;
      },
      set message(value) {
        message = value;
      },
    };
    return { thrown, reads: () => count };
  }),
  // Without a filename Babel's plugin never calls the resolver: the import
  // fails to resolve instead.
  'a module resolver throwing a string': () => {
    const thrown = 'resolver boom';
    return {
      code:
        header +
        "import { t } from './t.stylex';\n" +
        'export const s = stylex.create({ a: { color: t.c } });',
      options: {
        unstable_moduleResolution: {
          type: 'custom',
          filePathResolver: () => {
            throw thrown;
          },
          getCanonicalFilePath: (file) => file,
        },
      },
      thrown,
    };
  },
};

// An absolute filename on this platform (a drive-rooted path on Windows),
// as Babel resolves it.
const appFile = (name) => path.resolve('/app', name);

const filenames = [appFile('a.js'), undefined, 'relative/a.js'];

// The [label, scenario, filename] cases each engine is compared on.
function cases() {
  const result = [];
  for (const [name, scenario] of Object.entries(scenarios)) {
    for (const filename of filenames) {
      result.push([
        `${name}, filename ${String(filename)}`,
        scenario,
        filename,
      ]);
    }
  }
  return result;
}

module.exports = {
  appFile,
  header,
  runEngine,
  describe,
  outcome,
  scenarios,
  cases,
  filenames,
};
