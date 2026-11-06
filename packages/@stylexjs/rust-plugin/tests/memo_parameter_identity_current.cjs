"use strict";
const assert = require("node:assert/strict");
const test = require("node:test");
const babel = require("@babel/core");
const original = require("../../babel-plugin/lib");
const native = require("..");
const filename = require("node:path").resolve("/app/MemoReduce.js");
const options = { unstable_moduleResolution: { type: "haste" } };
function check(expression, count) {
  const source =
    "import * as stylex from '@stylexjs/stylex';export const s=stylex.create({root:{width:" +
    expression +
    "}});";
  const oracle = babel.transformSync(source, {
    filename,
    configFile: false,
    babelrc: false,
    plugins: [[original, options]],
  });
  assert.ok(
    JSON.stringify(oracle.metadata.stylex).includes("width:" + count + "px"),
  );
  for (let iteration = 0; iteration < 1000; iteration++) {
    const result = native.transformSync(source, { filename, ...options });
    assert.deepEqual(
      result.metadata.stylex,
      oracle.metadata.stylex,
      "wrong CSS for " + expression + " at iteration " + iteration,
    );
  }
}
test("retained callback parameter memo contexts always compile reduce width as six", () =>
  check("[1,2,3].reduce((total,x)=>total+x,0)", 6));
test("multiple reduce steps and nested helpers never truncate the accumulator", () =>
  check("((a)=>a.reduce((total,x)=>total+x,0))([5,7,1,8])", 21));
test("nested reduce invocation frames do not share parameter-dependent folds", () =>
  check("[1,2,3].reduce((total,x)=>total+[x,x+1].reduce((a,b)=>a+b,0),0)", 15));
