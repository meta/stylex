'use strict';
// Evaluator cases. Each one is compiled by the Babel plugin (record.cjs) and by oxs
// (eval_babel.rs), and the results must match: the same metadata, or the same error
// message at the same place.
//
// mode 'consts': `export const c = stylex.defineConsts({ v: <expr> })`. constVal shows the raw value.
// mode 'create': `export const styles = stylex.create({ a: { <prop>: <expr> } })`. Errors carry
//                Babel's deopt reason and code frame.
// mode 'raw':    `code` is the whole file.
// `pre` goes between the import and the call.

const IMPORT = "import * as stylex from '@stylexjs/stylex';";
const FILENAME = '/project/oxs-eval/src/x.stylex.js';
const BASE_OPTIONS = { unstable_moduleResolution: { type: 'commonJS', rootDir: '/project/oxs-eval' } };

function compose(c) {
  const pre = c.pre ? c.pre + '\n' : '';
  let code;
  if (c.mode === 'raw') code = c.code;
  else if (c.mode === 'create')
    code = `${IMPORT}\n${pre}export const styles = stylex.create({ a: { ${c.prop || 'color'}: ${c.expr} } });\n`;
  else code = `${IMPORT}\n${pre}export const c = stylex.defineConsts({ v: ${c.expr} });\n`;
  const dir = __dirname;
  const sub = (v) => (typeof v === 'string' ? v.split('{{DIR}}').join(dir) : v);
  const options = JSON.parse(JSON.stringify({ ...BASE_OPTIONS, ...(c.options || {}) }), (_k, v) => sub(v));
  for (const [k, v] of Object.entries(c.options || {})) if (typeof v === 'function' || (v && typeof v === 'object' && Object.values(v).some((x) => typeof x === 'function'))) options[k] = v;
  return { code, options, filename: sub(c.filename || FILENAME) };
}

const cases = [];
const add = (id, mode, expr, extra = {}) => cases.push({ id, mode, expr, ...extra });
const consts = (id, expr, extra) => add(id, 'consts', expr, extra);
const create = (id, expr, extra) => add(id, 'create', expr, extra);

// Literals and number formatting.
consts('lit/num-third', '1/3');
consts('lit/num-big', '1e21');
consts('lit/num-small', '1e-7');
consts('lit/num-hex', '0x1f');
consts('lit/num-sep', '1_000');
consts('lit/num-neg-zero-str', 'String(-0)');
consts('lit/num-tofixed', '(1.005).toFixed(2)');
consts('lit/str-escape', "'a\\u0041\\n'");
consts('lit/str-plus-num', "'a' + 1.50");
consts('lit/num-plus-str', "1 + '2'");
consts('lit/str-big', "'' + 123456789012345680000");
consts('lit/str-tiny', "'' + 0.000001");
consts('lit/str-tinier', "'' + 0.0000001");
consts('lit/str-nan', "'' + NaN");
consts('lit/str-inf', "'' + -Infinity");
consts('lit/bool-str', "'' + true");
consts('lit/null-str', "'' + null");
consts('lit/undef-str', "'' + undefined");
consts('lit/bigint', '10n');
consts('lit/regex', '/a/');

// Template literals.
consts('tpl/basic', '`a${1 + 1}b`');
consts('tpl/obj', '`${{}}`');
consts('tpl/arr', '`${[1, [2, 3]]}`');
consts('tpl/null', '`${null}-${undefined}`');
consts('tpl/raw', 'String.raw`a\\nb${1}`');
consts('tpl/undef-ref', '`${nope}`');

// Unary operators.
consts('un/neg-str', "-'3'");
consts('un/plus-str', "+' 12 '");
consts('un/plus-empty', "+''");
consts('un/plus-hex-str', "+'0x10'");
consts('un/plus-bad', "String(+'12px')");
consts('un/plus-arr', '+[5]');
consts('un/plus-arr2', 'String(+[1, 2])');
consts('un/plus-null', '+null');
consts('un/plus-true', '+true');
consts('un/neg-arr', 'String(-[])');
consts('un/not', "!''");
consts('un/tilde', '~5.7');
consts('un/tilde-str', "~'7'");
consts('un/typeof-num', 'typeof 1');
consts('un/typeof-null', 'typeof null');
consts('un/typeof-arr', 'typeof []');
consts('un/typeof-undef', 'typeof undefined');
consts('un/typeof-fn', 'typeof (() => 1)');
consts('un/typeof-unbound', 'typeof nope');
consts('un/void', 'String(void nope)');
create('un/delete', 'delete x.y', { pre: 'const x = {y: 1};' });

// Binary operators.
consts('bin/sub-str', "'10' - '4'");
consts('bin/mul-null', 'null * 5');
consts('bin/div-zero', 'String(1 / 0)');
consts('bin/neg-div-zero', 'String(-1 / 0)');
consts('bin/mod-neg', '-7 % 3');
consts('bin/exp', '2 ** 10');
consts('bin/exp-frac', '2 ** 0.5');
consts('bin/lt-str', "'10' < '9'");
consts('bin/lt-mixed', "'10' < 9");
consts('bin/le-null', 'null <= 0');
consts('bin/gt-undef', 'String(undefined > 0)');
consts('bin/loose-eq', "String(1 == '1')");
consts('bin/loose-eq-null', 'String(null == undefined)');
consts('bin/loose-ne', "String(1 != '1')");
consts('bin/strict-eq', "String(1 === '1')");
consts('bin/strict-ne-nan', 'String(NaN !== NaN)');
consts('bin/or', '5 | 2');
consts('bin/and', '6 & 3');
consts('bin/xor', '6 ^ 3');
consts('bin/shl', '1 << 31');
consts('bin/shr', '-16 >> 2');
consts('bin/ushr', '-16 >>> 2');
consts('bin/in-obj', "String('a' in {a: 1})");
consts('bin/in-arr', 'String(0 in [5])');
consts('bin/plus-arr', '[1, 2] + [3]');
consts('bin/plus-obj', "({}) + 'x'");
consts('bin/plus-bool', 'true + 1');
consts('bin/plus-undef', 'String(undefined + 1)');
consts('bin/plus-null-str', "null + 'a'");
consts('bin/float', '0.1 + 0.2');
consts('bin/neg-zero', 'String(0 * -1)');
create('bin/in-num', "'a' in 5");
create('bin/instanceof', '1 instanceof Object');

// Logical operators. Babel evaluates both sides.
consts('log/or-left', "'a' || nope");
consts('log/or-right', "'' || 'b'");
consts('log/and-left', "0 && nope");
consts('log/and-right', "1 && 'b'");
consts('log/coalesce-null', "null ?? 'b'");
consts('log/coalesce-str', "'a' ?? 'b'");
create('log/coalesce-zero', "0 ?? 'red'");
create('log/coalesce-empty', "'' ?? 'red'");
create('log/or-left-unknown', "nope || 'red'");
create('log/or-both-falsy-unknown', "'' || nope");
create('log/and-unknown-right', "'red' && nope");

// Conditional and sequence.
consts('cond/true', "1 ? 'a' : nope");
consts('cond/false', "'' ? nope : 'b'");
create('cond/unknown-test', "nope ? 'red' : 'blue'");
consts('seq/last', "(nope, 'b')");

// Objects and arrays.
consts('obj/spread', '{...{a: 1}, b: 2}');
consts('obj/spread-str', "{...'ab'}");
consts('obj/spread-null', '{...null, a: 1}');
consts('obj/spread-arr', '{...[7, 8]}');
consts('obj/num-keys', "{b: 1, 2: 'x', 1: 'y', a: 0}");
consts('obj/num-key-lit', "{1.5: 'a', 1e3: 'b', 0x10: 'c'}");
consts('obj/computed', "{[1 + 1]: 'a', ['x' + 'y']: 'b'}");
consts('obj/dup', '{a: 1, b: 2, a: 3}');
create('obj/method', 'x.m', { pre: 'const x = { m() { return 1; } };' });
create('obj/getter', 'x.g', { pre: 'const x = { get g() { return 1; } };' });
consts('arr/basic', '[1, "a", [2]]');
consts('arr/len', '[1, 2, 3].length');
consts('arr/index', "['a', 'b'][1]");
consts('arr/index-str', "['a', 'b']['1']");
consts('arr/oob', 'String([1][5])');
create('arr/hole', '[1, , 2][0]');
create('arr/spread', '[...[1, 2]][0]');

// Member access.
consts('mem/str-len', "'hello'.length");
consts('mem/str-idx', "'hello'[1]");
consts('mem/obj-missing', 'String({a: 1}.b)');
consts('mem/nested', '{a: {b: {c: 3}}}.a.b.c');
consts('mem/computed-num', "{1: 'one'}[1]");
consts('mem/computed-float', "{'1.5': 'x'}[1.5]");
create('mem/proto', "({}).constructor");
create('mem/proto-computed', "({})['__proto__']");
create('mem/of-undefined', 'x.y.z', { pre: 'const x = {};' });
create('mem/optional', 'x?.y', { pre: "const x = {y: 'red'};" });

// Identifiers and bindings.
create('id/undefined', 'nope');
consts('id/const', 'c1', { pre: "const c1 = 'red';" });
consts('id/let-const', 'c1', { pre: "let c1 = 'red';" });
consts('id/var-const', 'c1', { pre: "var c1 = 'red';" });
create('id/let-reassigned', 'c1', { pre: "let c1 = 'red';\nc1 = 'blue';" });
create('id/var-redeclared', 'c1', { pre: "var c1 = 'red';\nvar c1 = 'blue';" });
create('id/update', 'c1', { pre: 'let c1 = 1;\nc1++;' });
create('id/mutated-prop', 'c1.a', { pre: "const c1 = {a: 'red'};\nc1.a = 'blue';" });
create('id/mutated-push', 'c1[0]', { pre: "const c1 = ['red'];\nc1.push('blue');" });
create('id/mutated-delete', 'c1.a', { pre: "const c1 = {a: 'red'};\ndelete c1.a;" });
create('id/mutated-assign', 'c1.a', { pre: "const c1 = {a: 'red'};\nObject.assign(c1, {a: 'blue'});" });
consts('id/nested-mutation-ok', 'c1.a.b', { pre: "const c1 = {a: {b: 'red'}};\nc1.a.b = 'blue';" });
create('id/used-before', 'later', { mode: 'raw', code: `${IMPORT}\nexport const styles = stylex.create({ a: { color: later } });\nconst later = 'red';\n` });
create('id/self-ref', '', { mode: 'raw', code: `${IMPORT}\nconst s = {a: {color: 'red'}, b: {color: s}};\nexport const styles = stylex.create(s);\n` });
create('id/function-decl', 'f', { pre: 'function f() {}' });
create('id/class-decl', 'K', { pre: 'class K {}' });
create('id/destructured', 'c1', { pre: "const {c1} = {c1: 'red'};" });
create('id/uninit-let', 'c1', { pre: 'let c1;' });
consts('id/infinity', "String(Infinity)");
consts('id/nan', "String(NaN)");
create('id/shadow-undefined', 'undefined', { pre: 'const undefined = 1;' });
create('id/global', 'window');
create('id/this', 'this');
create('id/import-default', 'd', { pre: "import d from './other';" });
create('id/import-named', 'n', { pre: "import { n } from './other';" });
create('id/import-ns', 'ns.x', { pre: "import * as ns from './other';" });
create('id/chain', 'c2', { pre: "const c1 = 'red';\nconst c2 = c1;" });
create('id/param-shadow', '', {
  mode: 'raw',
  code: `${IMPORT}\nconst color = 'red';\nfunction f(color) {\n  return stylex.create({ a: { color: color } });\n}\n`,
});
create('id/inner-const', '', {
  mode: 'raw',
  code: `${IMPORT}\nconst color = 'blue';\nfunction f() {\n  const color = 'red';\n  return stylex.create({ a: { color } });\n}\nexport const s = f;\n`,
});

// Calls.
consts('call/math', 'Math.max(1, 5, 3)');
consts('call/math-round', 'Math.round(2.5)');
consts('call/math-round-neg', 'Math.round(-2.5)');
consts('call/math-floor', 'Math.floor(-1.5)');
consts('call/math-pow', 'Math.pow(2, 8)');
consts('call/math-min-empty', 'String(Math.min())');
consts('call/math-hypot', 'Math.hypot(3, 4)');
consts('call/math-sqrt', 'Math.sqrt(2)');
consts('call/math-sign', 'Math.sign(-3)');
consts('call/math-trunc', 'Math.trunc(-4.7)');
consts('call/math-cbrt', 'Math.cbrt(27)');
consts('call/math-clz32', 'Math.clz32(1)');
consts('call/math-imul', 'Math.imul(3, 4)');
create('call/math-random', 'Math.random()');
create('call/math-pi', 'Math.PI');
consts('call/string', 'String(12.50)');
consts('call/string-fromcharcode', 'String.fromCharCode(65, 66)');
consts('call/number', "Number('42')");
consts('call/number-empty', "Number('')");
consts('call/number-parsefloat', "Number.parseFloat('3.5px')");
consts('call/number-parseint', "Number.parseInt('ff', 16)");
consts('call/number-isinteger', 'String(Number.isInteger(5))');
consts('call/object-keys', "Object.keys({a: 1, b: 2}).join(',')");
consts('call/object-values', 'Object.values({a: 1, b: 2})');
consts('call/object-entries', 'Object.entries({a: 1})');
consts('call/object-fromentries', "Object.fromEntries([['a', 1], ['b', 2]])");
create('call/object-assign', 'Object.assign({}, {a: 1}).a');
create('call/object-create', 'Object.create(null)');
consts('call/array-isarray', 'String(Array.isArray([]))');
consts('call/array-of', 'Array.of(1, 2)');
consts('call/array-from', "Array.from('ab')");
consts('call/str-method', "'abc'.toUpperCase()");
consts('call/str-slice', "'abcdef'.slice(1, -1)");
consts('call/str-replace', "'a-b-c'.replace('-', '+')");
consts('call/str-replaceall', "'a-b-c'.replaceAll('-', '+')");
consts('call/str-split', "'a,b'.split(',')");
consts('call/str-padstart', "'5'.padStart(3, '0')");
consts('call/str-trim', "'  x '.trim()");
consts('call/str-repeat', "'ab'.repeat(2)");
consts('call/str-includes', "String('abc'.includes('b'))");
consts('call/str-charcodeat', "'abc'.charCodeAt(1)");
consts('call/str-concat', "'a'.concat(1, 'b')");
consts('call/str-indexof', "'abcb'.lastIndexOf('b')");
consts('call/str-at', "'abc'.at(-1)");
consts('call/num-tofixed', '(3.14159).toFixed(2)');
consts('call/num-tostring', '(255).toString(16)');
create('call/num-lit-method', '1.5.toFixed(1)');
consts('call/arr-join', "[1, 2, 3].join('-')");
consts('call/arr-map', '[1, 2].map((x) => x * 2)');
consts('call/arr-filter', '[1, 2, 3].filter((x) => x > 1)');
consts('call/arr-includes', 'String([1, 2].includes(2))');
consts('call/arr-indexof', '[1, 2].indexOf(2)');
consts('call/arr-slice', '[1, 2, 3].slice(1)');
consts('call/arr-concat', '[1].concat([2], 3)');
consts('call/arr-reduce', '[1, 2, 3].reduce((a, b) => a + b, 0)');
consts('call/arr-tostring', '[1, [2, 3]].toString()');
consts('call/obj-tostring', "({}).toString()");
consts('call/local-arrow', 'double(4)', { pre: 'const double = (x) => x * 2;' });
consts('call/local-arrow-obj', 'fns.add(1, 2)', { pre: 'const fns = { add: (a, b) => a + b };' });
consts('call/local-arrow-missing-arg', 'String(f())', { pre: 'const f = (x) => x;' });
consts('call/arrow-closure-scope', 'f(1)', { pre: 'const k = 10;\nconst f = (x) => x + k;' });
create('call/arrow-throws', 'f(1)', { pre: 'const f = (x) => x + nope;' });
create('call/arrow-block', 'f(1)', { pre: 'const f = (x) => { return x; };' });
create('call/function-expr', 'f(1)', { pre: 'const f = function (x) { return x; };' });
create('call/unknown', 'generateBg()');
create('call/unknown-member', 'utils.bg()');
create('call/unknown-method-on-const', "x.nope()", { pre: 'const x = {};' });
create('call/not-callable', 'x()', { pre: 'const x = 5;' });
create('call/spread-arg', 'Math.max(...[1, 2])');
create('call/new', 'new String("a")');
create('call/eval', "eval('1')");
create('call/function-ctor', "Function('return 1')()");
create('call/constructor-chain', "''.constructor('x')");
create('call/computed-callee', "x['f'](1)", { pre: 'const x = { f: (a) => a };' });
create('call/shadowed-math', 'Math.max(1, 2)', { pre: 'const Math = { max: () => 0 };' });
create('call/shadowed-string', "String(1)", { pre: 'const String = (x) => "s";' });

// TypeScript wrappers.
consts('ts/as', "'red' as string", { ts: true });
consts('ts/satisfies', "'red' satisfies string", { ts: true });
create('ts/non-null', "c1!", { ts: true, pre: "const c1 = 'red';" });

// stylex.env with plain values.
consts('env/plain', 'stylex.env.brand', { options: { env: { brand: 'red' } } });
consts('env/nested', 'stylex.env.colors.primary', { options: { env: { colors: { primary: '#00f' } } } });
consts('env/missing', 'String(stylex.env.nope)', { options: { env: {} } });
create('env/named', 'env.brand', {
  mode: 'raw',
  code: "import { create, env } from '@stylexjs/stylex';\nexport const styles = create({ a: { color: env.brand } });\n",
  options: { env: { brand: 'red' } },
});

// stylex.env with function values (needs the host bridge).
create('env/fn', "stylex.env.mix('red', 'blue')", {
  options: { env: { mix: (a, b) => `color-mix(in srgb, ${a}, ${b})` } },
});
create('env/fn-num', 'stylex.env.px(4)', { prop: 'width', options: { env: { px: (n) => n * 4 } } });
create('env/fn-throws', 'stylex.env.bad()', {
  options: {
    env: {
      bad: () => {
        throw new Error('env exploded');
      },
    },
  },
});
create('env/fn-template', "`${stylex.env.mix('red', 'blue')} !important`", {
  options: { env: { mix: (a, b) => `color-mix(in srgb, ${a}, ${b})` } },
});

// Scopes and closures.
create('scope/block-const', '', {
  mode: 'raw',
  code: `${IMPORT}\nconst c = 'blue';\nif (true) {\n  const c = 'red';\n}\nexport const styles = stylex.create({ a: { color: c } });\n`,
});
create('scope/var-hoisted-later', '', {
  mode: 'raw',
  code: `${IMPORT}\nexport const styles = stylex.create({ a: { color: v } });\nvar v = 'red';\n`,
});
create('scope/function-param-default', 'f(1)', { pre: 'const f = (x = 2) => x;' });
create('scope/rest-param', 'f(1)', { pre: 'const f = (...xs) => xs[0];' });
create('scope/curried', 'f(1)(2)', { pre: 'const f = (x) => (y) => x + y;' });
consts('scope/curried-stored', 'g(2)', { pre: 'const f = (x) => (y) => x + y;\nconst g = f(1);' });
consts('scope/recursion', 'f(3)', { pre: 'const f = (n) => n <= 0 ? 0 : n + f(n - 1);' });
consts('scope/closure-dynamic-scope', 'f(5)', { pre: 'const y = (x) => x + 1;\nconst f = (x) => y(x) * 2;' });
create('scope/catch-param', '', {
  mode: 'raw',
  code: `${IMPORT}\ntry {} catch (e) {\n  stylex.create({ a: { color: e } });\n}\n`,
});
create('scope/for-of-reassigned', 'c', { pre: "let c = 'red';\nfor (c of []) {}" });
create('scope/destructure-assign', 'c', { pre: "let c = 'red';\n[c] = ['blue'];" });
create('scope/object-define-property', 'c.a', { pre: "const c = {a: 'red'};\nObject.defineProperty(c, 'a', {value: 'blue'});" });
consts('scope/computed-push-ok', 'k[0]', { pre: "const k = ['red'];\nk['push']('blue');" });
create('scope/enum', 'E.A', { ts: true, pre: "enum E { A = 'red' }" });

// Members, proxies and imports.
consts('mem/string-key-num', "({'01': 'a'})['01']");
consts('mem/arr-neg', "String([1, 2][-1])");
consts('mem/num-prop', 'String((5).x)');
create('mem/private', 'x.y', { pre: 'class K { #p = 1; static f(o) { return o.#p; } }\nconst x = {y: 1};' });
// Hashes depend on the file path relative to rootDir, so root it here.
const VARS = { unstable_moduleResolution: { type: 'commonJS', rootDir: '{{DIR}}' } };
create('vars/proxy', 'tokens.color', {
  filename: '{{DIR}}/fixtures/app.js',
  pre: "import { tokens } from './vars.stylex';",
  options: VARS,
});
create('vars/proxy-dashdash', "tokens['--brand']", {
  filename: '{{DIR}}/fixtures/app.js',
  pre: "import { tokens } from './vars.stylex';",
  options: VARS,
});
create('vars/proxy-nested', 'tokens.button.primary.bg', {
  filename: '{{DIR}}/fixtures/app.js',
  pre: "import { tokens } from './vars.stylex';",
  options: VARS,
});
create('vars/proxy-template', '`${tokens}`', {
  filename: '{{DIR}}/fixtures/app.js',
  pre: "import { tokens } from './vars.stylex';",
  options: VARS,
});
create('vars/proxy-alias', 't.color', {
  filename: '{{DIR}}/fixtures/app.js',
  pre: "import { tokens } from './vars.stylex';\nconst t = tokens;",
  options: VARS,
});
create('vars/unresolved', 'tokens.color', {
  filename: '{{DIR}}/fixtures/app.js',
  pre: "import { tokens } from './missing.stylex';",
  options: VARS,
});
const CROSS = { unstable_moduleResolution: { type: 'experimental_crossFileParsing', rootDir: '{{DIR}}' } };
create('cross/const', 'brand', { filename: '{{DIR}}/fixtures/app.js', pre: "import { brand } from './consts.stylex';", options: CROSS });
create('cross/member', 'sizes.large', { prop: 'width', filename: '{{DIR}}/fixtures/app.js', pre: "import { sizes } from './consts.stylex';", options: CROSS });
create('cross/derived', 'derived', { prop: 'fontFamily', filename: '{{DIR}}/fixtures/app.js', pre: "import { derived } from './consts.stylex';", options: CROSS });
create('cross/missing-export', 'String(nope)', { prop: 'fontFamily', filename: '{{DIR}}/fixtures/app.js', pre: "import { nope } from './consts.stylex';", options: CROSS });
create('cross/not-const-export', 'local', { filename: '{{DIR}}/fixtures/app.js', pre: "import { local } from './consts.stylex';", options: CROSS });
create('cross/bad', 'bad', { filename: '{{DIR}}/fixtures/app.js', pre: "import { bad } from './consts.stylex';", options: CROSS });
create('cross/broken', 'x', { full: true, filename: '{{DIR}}/fixtures/app.js', pre: "import { x } from './broken.stylex';", options: CROSS });
create('cross/broken-expected-comma', 'b', { full: true, filename: '{{DIR}}/fixtures/app.js', pre: "import { b } from './broken2.stylex';", options: CROSS });
create('cross/broken-string', 's', { full: true, filename: '{{DIR}}/fixtures/app.js', pre: "import { s } from './broken3.stylex';", options: CROSS });
create('cross/broken-commonjs', 'x', { filename: '{{DIR}}/fixtures/app.js', pre: "import { x } from './broken.stylex';", options: VARS });
create('cross/broken-no-resolution', 'x', { filename: '{{DIR}}/fixtures/app.js', pre: "import { x } from './broken.stylex';", options: { unstable_moduleResolution: undefined } });
create('cross/unresolved', 'x', { filename: '{{DIR}}/fixtures/app.js', pre: "import { x } from './nothere.stylex';", options: CROSS });

// More operators and coercions.
consts('more/str-cmp-unicode', "String('\\u00e9' < 'z')");
consts('more/add-nested-arr', '[[1, 2], [3]] + ""');
consts('more/num-format-1e21', '"" + 1e21');
consts('more/num-format-neg-exp', '"" + 1.5e-10');
consts('more/num-format-max', '"" + Number.parseFloat("1.7976931348623157e308")');
consts('more/mod-float', '5.5 % 2');
consts('more/mod-zero', 'String(5 % 0)');
consts('more/neg-mod', 'String(-0 % 5)');
consts('more/eq-arr-str', "String([1] == '1')");
consts('more/eq-null-zero', 'String(null == 0)');
consts('more/eq-bool-str', "String(true == '1')");
consts('more/lt-undef', 'String(undefined < 1)');
consts('more/ge-nan', 'String(NaN >= NaN)');
consts('more/shift-big', '1 << 33');
consts('more/ushr-neg', '-1 >>> 0');
consts('more/bitnot-big', '~4294967296');
consts('more/typeof-obj', 'typeof {}');
consts('more/typeof-str', "typeof ''");
consts('more/not-not', '!!"0"');
consts('more/unary-plus-obj', 'String(+{})');
consts('more/template-nested', '`a${`b${1}`}`');
consts('more/template-bool', '`${true}${false}`');
consts('more/template-num', '`${0.1 + 0.2}`');
consts('more/cond-chain', "0 ? 'a' : '' ? 'b' : 'c'");
consts('more/coalesce-undef', "undefined ?? 'b'");
create('more/coalesce-false', "false ?? 'red'");
consts('more/or-chain', "'' || 0 || 'x'");
consts('more/and-chain', "1 && 'a' && 'b'");
consts('more/math-max-str', "Math.max('3', 2)");
consts('more/math-min-nan', 'String(Math.min(1, NaN))');
consts('more/math-round-half', 'Math.round(0.5)');
consts('more/math-abs-neg', 'Math.abs(-7.5)');
consts('more/math-atan2', 'Math.atan2(1, 1)');
consts('more/math-fround', 'Math.fround(5.5)');
consts('more/math-log10', 'Math.log10(1000)');
consts('more/number-isnan-str', "String(Number.isNaN('x'))");
consts('more/number-parseint-float', "Number.parseInt('3.9')");
consts('more/string-fromcodepoint', 'String.fromCodePoint(9731)');
consts('more/object-keys-arr', "Object.keys(['a', 'b']).join('|')");
consts('more/object-hasown', "String(Object.hasOwn({a: 1}, 'a'))");
consts('more/object-is', 'String(Object.is(-0, 0))');
consts('more/array-from-len', 'Array.from({length: 3}, (_, i) => i * 2).join()');
consts('more/str-split-limit', "'a,b,c'.split(',', 2).join('|')");
consts('more/str-split-empty', "'abc'.split('').join('|')");
consts('more/str-substring-swap', "'abcdef'.substring(4, 1)");
consts('more/str-substr', "'abcdef'.substr(-3, 2)");
consts('more/str-codepointat', "'abc'.codePointAt(0)");
consts('more/str-padend', "'5'.padEnd(4, 'ab')");
consts('more/str-endswith', "String('abc'.endsWith('bc'))");
consts('more/str-lower', "'ABC'.toLowerCase()");
consts('more/str-replace-dollar', "'a-b'.replace('-', '$&$&')");
consts('more/num-tofixed-round', '(n).toFixed(1)', { pre: 'const n = 2.45;' });
consts('more/num-tofixed-var', 'n.toFixed(2)', { pre: 'const n = 3.14159;' });
consts('more/num-toprecision', 'n.toPrecision(3)', { pre: 'const n = 123.456;' });
consts('more/num-tostring-radix', 'n.toString(2)', { pre: 'const n = 10;' });
consts('more/arr-find', '[1, 2, 3].find((x) => x > 1)');
consts('more/arr-some', 'String([1, 2].some((x) => x > 1))');
consts('more/arr-every', 'String([1, 2].every((x) => x > 1))');
consts('more/arr-findindex', '[1, 2, 3].findIndex((x) => x === 3)');
consts('more/arr-flat', '[1, [2, [3]]].flat().length');
consts('more/arr-flatmap', '[1, 2].flatMap((x) => [x, x]).join()');
consts('more/arr-reverse', '[1, 2, 3].reverse().join()');
consts('more/arr-sort', '[3, 1, 10].sort().join()');
consts('more/arr-sort-fn', '[3, 1, 10].sort((a, b) => a - b).join()');
consts('more/arr-at', '[1, 2, 3].at(-1)');
consts('more/arr-lastindexof', '[1, 2, 1].lastIndexOf(1)');
consts('more/arr-map-index', "['a', 'b'].map((x, i) => x + i).join()");
consts('more/arr-reduce-noinit', '[1, 2, 3].reduce((a, b) => a * b)');
consts('more/obj-hasownproperty', "String(({a: 1}).hasOwnProperty('a'))");
consts('more/obj-spread-override', "({...{a: 1, b: 2}, a: 3}).a");
consts('more/obj-proto-key', "Object.keys({__proto__: {x: 1}, y: 2}).join()");
consts('more/obj-numeric-order', "Object.keys({b: 1, 10: 1, 2: 1, a: 1}).join()");
consts('more/seq-skip-unknown', "(window, 'x')");

// Each Babel visitor passes its own helpers to evaluate(). Outside create() some are missing.
const raw = (id, code, extra) => cases.push({ id, mode: 'raw', code: `${IMPORT}\n${code}\n`, ...extra });
raw('ctx/vars-first-that-works', "export const v = stylex.defineVars({ c: stylex.firstThatWorks('red', 'blue') });");
raw('ctx/vars-keyframes', "export const v = stylex.defineVars({ a: stylex.keyframes({ from: { opacity: 0 } }) });");
raw('ctx/vars-env', "export const v = stylex.defineVars({ c: stylex.env.brand });", { options: { env: { brand: 'red' } } });
raw('ctx/vars-nested-env', "export const v = stylex.unstable_defineVarsNested({ c: { d: stylex.env.brand } });", { options: { env: { brand: 'red' } } });
raw('ctx/theme-first-that-works', "export const v = stylex.defineVars({ c: 'red' });\nexport const t = stylex.createTheme(v, { c: stylex.firstThatWorks('red', 'blue') });");
raw('ctx/consts-first-that-works', "export const c = stylex.defineConsts({ a: stylex.firstThatWorks('red', 'blue') });");
raw('ctx/consts-env', "export const c = stylex.defineConsts({ a: stylex.env.brand });", { options: { env: { brand: 'red' } } });
raw('ctx/consts-import', "import { tokens } from './vars.stylex';\nexport const c = stylex.defineConsts({ a: tokens.color });", { filename: '{{DIR}}/fixtures/c.stylex.js', options: { unstable_moduleResolution: { type: 'commonJS', rootDir: '{{DIR}}' } } });
raw('ctx/keyframes-position-try', "export const k = stylex.keyframes({ from: { positionTryFallbacks: stylex.positionTry({ top: 0 }) } });");
raw('ctx/keyframes-first-that-works', "export const k = stylex.keyframes({ from: { color: stylex.firstThatWorks('red', 'blue') } });");
raw('ctx/position-try-first-that-works', "export const p = stylex.positionTry({ top: stylex.firstThatWorks('1px', '2px') });");
raw('ctx/position-try-keyframes', "export const p = stylex.positionTry({ top: stylex.keyframes({ from: { top: 0 } }) });");
raw('ctx/create-types', "export const s = stylex.create({ a: { width: stylex.types.length('1px') } });");
raw('ctx/create-default-marker', "export const s = stylex.create({ a: { color: stylex.defaultMarker() } });");
raw('ctx/create-ns-other', "export const s = stylex.create({ a: { color: String(stylex.props) } });");
raw('ctx/vt-position-try', "export const v = stylex.viewTransitionClass({ group: { top: stylex.positionTry({ top: 0 }) } });");

// Babel's path.resolve() follows constant bindings through their initializers without the
// evaluator's checks, and its `seen` map remembers failed nodes.
consts('resolve/skips-used-before', 'String(o1.p)', { pre: "var v2 = v3;\nconst o1 = { p: 1, a: v2 };\nlet v3 = 'k';" });
consts('resolve/skips-mutation', 'a.x', { pre: "const a = b;\nconst b = {x: 'red'};\nb.x = 'blue';" });
create('resolve/stops-at-violation', 'a', { pre: "const a = b;\nlet b = 'red';\nb = 'blue';" });
create('resolve/cycle', 'o1.p', { pre: 'const v1 = v2;\nconst v2 = v1;\nconst o1 = { p: 1, a: v2 };' });
create('resolve/self-in-init', 'o.a', { pre: 'const o = { a: 1, b: o };' });
create('seen/failed-node-again', "o1[k]", {
  pre: "const k = 'a';\nvar v1 = ({a: 1, [(0.5 ? v2 : 0x10)]: 1});\nlet v2 = v1;\nconst o1 = { p: (7 || ({...v1, z: 2})), a: v2 };",
});
consts('seen/array-elements-fresh', "String([v1 || 1, v1 || 2])", { pre: 'const v1 = 0;' });

module.exports = { cases, compose, FILENAME };
