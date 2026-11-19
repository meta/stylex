/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
'use strict';

const assert = require('node:assert/strict');
const test = require('node:test');
const babel = require('@babel/core');
const stylex = require('@stylexjs/babel-plugin');
const native = require('../index');

const filename = '/tmp/compiled-consumer.js';
const syntaxOptions = {
  filename,
  babelrc: false,
  configFile: false,
  highlightCode: false,
};
function compare(source, options) {
  const expected = babel.transformSync(source, {
    ...syntaxOptions,
    plugins: [[stylex, options]],
  });
  // Through the Node API, so regressions in host-backed object carriers show up.
  const actual = native.transformSync(source, { ...options, filename });
  const printed = babel.transformSync(actual.code, syntaxOptions).code;
  assert.equal(printed, expected.code, source);
  assert.deepEqual(
    actual.metadata.stylex,
    expected.metadata.stylex,
    'ordered metadata',
  );
  return actual;
}
const conditionalPrefix =
  "import stylex from 'stylex';const styles=stylex.create({red:{color:'red'},blue:{color:null}});";

test('host-backed legacy conditional keeps its required null cancellation', () => {
  compare(`${conditionalPrefix}stylex(styles.red,isActive&&styles.blue);`, {
    styleResolution: 'application-order',
    enableInlinedConditionalMerge: false,
    runtimeInjection: true,
  });
});
test('host-backed props array conditional keeps its required null cancellation', () => {
  compare(
    `${conditionalPrefix}stylex.props([styles.red,isActive&&styles.blue]);`,
    {
      unstable_moduleResolution: { rootDir: '/', type: 'commonJS' },
      enableInlinedConditionalMerge: false,
      runtimeInjection: true,
    },
  );
});
test('host-backed shorthand prefix keeps only required null cancellations', () => {
  compare(
    "import stylex from 'stylex';const styles=stylex.create({foo:{padding:5},bar:{paddingBlock:10},baz:{paddingTop:7}});stylex(styles.foo);stylex(styles.foo,styles.bar);stylex(styles.bar,styles.foo);stylex(styles.foo,styles.bar,styles.baz);stylex(styles.baz,styles.foo,somethingElse);",
    {
      styleResolution: 'application-order',
      runtimeInjection: true,
    },
  );
});
for (const callee of ['stylex', 'stylex.props']) {
  test(`host-backed unknown compiled namespace retains ${callee} runtime read`, () => {
    compare(
      `import * as stylex from '@stylexjs/stylex';const styles=stylex.create({tileHeading:{marginRight:12}});${callee}(styles.unknown);`,
      {
        styleResolution: 'application-order',
        runtimeInjection: true,
        dev: true,
        enableDevClassNames: false,
      },
    );
  });
}
test('host-backed ordinary missing properties remain JavaScript undefined', () => {
  compare(
    "import * as stylex from '@stylexjs/stylex';const data={};const absent=data.missing;const styles=stylex.create({root:{width:absent==null?1:2,color:null}});use(styles);",
    {},
  );
});
