/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import Module, { createRequire } from 'node:module';
import { readdirSync } from 'node:fs';
import { describe, it } from 'node:test';
import { fileURLToPath } from 'node:url';
import { RuleTester as OxlintRuleTester } from 'oxlint/plugins-dev';

const require = createRequire(import.meta.url);
const packageDir = fileURLToPath(new URL('..', import.meta.url));
const testsDir = fileURLToPath(new URL('../__tests__/', import.meta.url));

require('@babel/register')({ cwd: packageDir });

OxlintRuleTester.describe = describe;
OxlintRuleTester.it = it;
OxlintRuleTester.itOnly = it.only;

class RuleTester extends OxlintRuleTester {
  constructor(config = {}) {
    super({
      eslintCompat: true,
      languageOptions: {
        sourceType: config.parserOptions?.sourceType,
        parserOptions: { lang: 'tsx' },
      },
    });
  }
}

const originalLoad = Module._load;
Module._load = function (request, ...args) {
  const loaded = originalLoad.call(this, request, ...args);
  return request === 'eslint' ? { ...loaded, RuleTester } : loaded;
};

globalThis.jest = { disableAutomock() {} };

for (const file of readdirSync(testsDir)) {
  if (file.endsWith('-test.js')) {
    require(testsDir + file);
  }
}
