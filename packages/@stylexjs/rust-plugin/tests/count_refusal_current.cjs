"use strict";
const assert = require("node:assert/strict");
const test = require("node:test");
const babel = require("@babel/core");
const original = require("../../babel-plugin/lib");
const facade = require("../babel");
const native = require("..");
const filename = require("node:path").resolve("/app/Count.js");
const options = { unstable_moduleResolution: { type: "haste" } };
const source =
  "import * as stylex from '@stylexjs/stylex';export const s=stylex.create({root:{width:[-1,2].map(each=>'x'.repeat(each*2))}});";
function thrown(run) {
  try {
    run();
  } catch (error) {
    return error;
  }
  assert.fail("the invalid repeat count must throw");
}
function transformed(plugin, code) {
  return babel.transformSync(code, {
    filename,
    babelrc: false,
    configFile: false,
    plugins: [[plugin, options]],
  });
}
function assertExistingRefusal(error) {
  // Parent approved keeping the exact PR #2008 6fc3a507/native refusal.
  // Babel's RangeError is intentionally not a second kind mechanism here.
  assert.equal(error.name, "Error");
  assert.equal(
    error.message,
    `${filename}: Unsupported expression: CallExpression\n\n`,
  );
  assert.equal(error.code, "BABEL_TRANSFORM_ERROR");
}
test("negative repeat callback keeps the direct PR-head thrown refusal", () => {
  const runtimeCountError = thrown(() => "x".repeat(-2));
  assert.equal(runtimeCountError.name, "RangeError");
  const oracle = thrown(() => transformed(original, source));
  assert.equal(oracle.name, runtimeCountError.name);
  // V8's message differs between supported Node versions; match this runtime
  // exactly, including Babel's resolved filename prefix.
  assert.equal(oracle.message, `${filename}: ${runtimeCountError.message}`);
  assertExistingRefusal(
    thrown(() => native.transformSync(source, { filename, ...options })),
  );
});
test("negative repeat callback keeps the facade PR-head thrown refusal", () => {
  assertExistingRefusal(thrown(() => transformed(facade, source)));
});
test("public parser still rejects hard nesting in an otherwise dead operand", () => {
  const deep = "[".repeat(1025) + "'x'" + "]".repeat(1025);
  const code =
    "import * as stylex from '@stylexjs/stylex';export const s=stylex.create({root:{width:['a',false&&" +
    deep +
    "].join('-')}});";
  const error = thrown(() =>
    native.transformSync(code, { filename, ...options }),
  );
  assert.equal(error.name, "SyntaxError");
  assert.equal(error.code, "BABEL_PARSE_ERROR");
  assert.equal(error.stylexDiagnostic?.kind, "parse");
});
