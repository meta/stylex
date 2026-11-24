'use strict';

// Random but realistic StyleX programs.
//
// gen(seed, caseIndex) returns a *spec*: a plain JSON tree describing one
// program (file kind, syntax, import form, options, items, uses). render(spec)
// turns a spec into source text plus the Babel call inputs. Every list in the
// spec is removable element by element, which is what the minimizer relies on,
// and render() drops references to things that no longer exist.

const path = require('node:path');

// ---------------------------------------------------------------------------
// PRNG

function mulberry32(a) {
  return function () {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function hashSeed(seed, i) {
  let h = 2166136261;
  for (const c of `${seed}:${i}`) h = Math.imul(h ^ c.charCodeAt(0), 16777619);
  return h >>> 0;
}

function makeRng(seed, i) {
  const r = mulberry32(hashSeed(seed, i));
  const rng = {
    f: r,
    int: (a, b) => a + Math.floor(r() * (b - a + 1)),
    chance: (p) => r() < p,
    pick: (arr) => arr[Math.floor(r() * arr.length)],
    weighted: (pairs) => {
      const total = pairs.reduce((s, [w]) => s + w, 0);
      let x = r() * total;
      for (const [w, v] of pairs) {
        if ((x -= w) < 0) return v;
      }
      return pairs[pairs.length - 1][1];
    },
    some: (arr, lo, hi) => {
      const n = rng.int(lo, Math.min(hi, arr.length));
      const copy = arr.slice();
      const out = [];
      for (let k = 0; k < n; k++) out.push(copy.splice(Math.floor(r() * copy.length), 1)[0]);
      return out;
    },
  };
  return rng;
}

// ---------------------------------------------------------------------------
// Vocabulary

const PSEUDO_CLASSES = [
  ':hover', ':focus', ':active', ':focus-visible', ':focus-within', ':disabled',
  ':first-child', ':last-child', ':visited', ':checked', ':nth-child(2n + 1)',
];
const RARE_PSEUDO_CLASSES = [':not(:first-child)', ':is(:hover, :focus)', ':where(.a)', ':has(> img)'];
const AT_RULES = [
  '@media (max-width: 600px)', '@media (min-width: 768px)',
  '@media (min-width: 768px) and (max-width: 1024px)', '@media (prefers-color-scheme: dark)',
  '@media (prefers-reduced-motion: reduce)', '@media print', '@media (hover: hover)',
  '@supports (display: grid)', '@supports not (display: grid)', '@supports (color: oklch(0 0 0))',
];
const RARE_AT_RULES = [
  '@container (min-width: 400px)', '@media screen and (min-width:900px)', '@media (width >= 600px)',
  '@media not print', '@starting-style',
];
const PSEUDO_ELEMENTS = ['::before', '::after', '::placeholder', '::selection', '::-webkit-scrollbar', '::backdrop'];

const COLORS = ['red', 'blue', '#fff', '#333333', 'rgb(0 0 0 / 0.5)', 'rgba(255, 0, 0, 0.5)', 'transparent',
  'currentColor', 'hsl(210, 50%, 40%)', 'oklch(70% 0.1 200)', 'var(--brand)', 'inherit'];
const LENGTHS = [0, 1, 4, 8, 10, 16, 24, 1.5, -4, '0', '1px', '2rem', '0.5em', '100%', '50vh', '10dvw',
  'calc(100% - 10px)', 'min(10px, 2vw)', 'max(1rem, 10px)', 'auto', 'clamp(1rem, 2.5vw, 3rem)'];
const PROPS = {
  color: () => COLORS,
  backgroundColor: () => COLORS,
  borderColor: () => COLORS,
  outlineColor: () => COLORS,
  fill: () => COLORS,
  width: () => LENGTHS,
  height: () => LENGTHS,
  minWidth: () => LENGTHS,
  maxHeight: () => LENGTHS,
  padding: () => [...LENGTHS, '4px 8px', '1px 2px 3px 4px'],
  paddingInline: () => LENGTHS,
  paddingBlockStart: () => LENGTHS,
  paddingLeft: () => LENGTHS,
  margin: () => [...LENGTHS, '0 auto', '4px 8px 12px'],
  marginInlineStart: () => LENGTHS,
  marginTop: () => LENGTHS,
  marginRight: () => LENGTHS,
  inset: () => LENGTHS,
  insetInlineEnd: () => LENGTHS,
  top: () => LENGTHS,
  left: () => LENGTHS,
  gap: () => LENGTHS,
  rowGap: () => LENGTHS,
  borderRadius: () => [...LENGTHS, '50%', '4px 8px'],
  borderWidth: () => [0, 1, 2, '1px', 'thin'],
  borderStyle: () => ['solid', 'dashed', 'none'],
  border: () => ['1px solid red', 'none', '2px dashed currentColor', 0],
  borderInlineStart: () => ['1px solid red', 'none'],
  outline: () => ['none', '2px solid blue'],
  display: () => ['flex', 'grid', 'none', 'block', 'inline-flex', 'contents'],
  position: () => ['relative', 'absolute', 'fixed', 'sticky'],
  flexDirection: () => ['row', 'column', 'row-reverse'],
  flex: () => [1, '1 1 auto', '0 0 100px', 'none'],
  flexGrow: () => [0, 1, 2],
  alignItems: () => ['center', 'flex-start', 'stretch'],
  justifyContent: () => ['center', 'space-between', 'flex-end'],
  gridTemplateColumns: () => ['repeat(3, 1fr)', '1fr 2fr', 'repeat(auto-fill, minmax(100px, 1fr))'],
  gridArea: () => ['1 / 2 / 3 / 4', 'main'],
  fontSize: () => [12, 16, '1rem', '14px', 'clamp(1rem, 2vw, 3rem)', 'larger'],
  fontWeight: () => [400, 700, 'bold', 'normal'],
  fontFamily: () => ['"Inter", sans-serif', 'system-ui', 'monospace'],
  lineHeight: () => [1, 1.5, '20px', 'normal'],
  letterSpacing: () => ['0.1em', 0, '-0.5px'],
  textAlign: () => ['left', 'right', 'start', 'end', 'center'],
  textDecoration: () => ['none', 'underline'],
  textTransform: () => ['uppercase', 'none'],
  whiteSpace: () => ['nowrap', 'pre-wrap'],
  opacity: () => [0, 0.5, 1, '50%'],
  zIndex: () => [1, 10, -1, 9999],
  cursor: () => ['pointer', 'default', 'not-allowed'],
  transform: () => ['translateX(10px)', 'rotate(45deg) scale(1.5)', 'none', 'translate3d(0, 0, 0)'],
  transition: () => ['opacity 0.3s ease', 'all .2s', 'none', 'transform 200ms cubic-bezier(0.4, 0, 0.2, 1)'],
  transitionDuration: () => ['0.3s', '200ms', 500],
  animationDuration: () => ['1s', '500ms'],
  boxShadow: () => ['0 1px 2px rgba(0, 0, 0, 0.1)', 'none', 'inset 0 0 0 1px red'],
  overflow: () => ['hidden', 'auto', 'visible'],
  overflowX: () => ['scroll', 'hidden'],
  content: () => ['""', "''", '"\\201C"', 'none'],
  aspectRatio: () => ['16 / 9', 1, '4/3'],
  objectFit: () => ['cover', 'contain'],
  userSelect: () => ['none', 'auto'],
  pointerEvents: () => ['none', 'auto'],
  float: () => ['left', 'start', 'none'],
  clear: () => ['both', 'inline-start'],
  boxSizing: () => ['border-box'],
  willChange: () => ['transform', 'opacity, transform'],
  backgroundImage: () => ['linear-gradient(to right, red, blue)', 'url("a.png")', 'none'],
  WebkitTapHighlightColor: () => ['transparent'],
  WebkitLineClamp: () => [2, 3],
  scrollbarWidth: () => ['none', 'thin'],
  '--custom-color': () => COLORS,
  '--gap': () => LENGTHS,
};
// Every shorthand Babel's application-order expansion knows (from the plugin's
// own table), with generic values. Values need not be valid CSS for the property.
const SHORTHANDS = ["all", "animation", "animationRange", "background", "backgroundPosition", "border", "borderInline", "borderBlock", "borderTop", "borderInlineEnd", "borderRight", "borderBottom", "borderInlineStart", "borderLeft", "borderInlineWidth", "borderInlineStyle", "borderInlineColor", "borderBlockWidth", "borderBlockStyle", "borderBlockColor", "borderColor", "borderStyle", "borderWidth", "borderInlineStartColor", "borderInlineEndColor", "borderInlineStartStyle", "borderInlineEndStyle", "borderInlineStartWidth", "borderInlineEndWidth", "borderLeftColor", "borderRightColor", "borderLeftStyle", "borderRightStyle", "borderLeftWidth", "borderRightWidth", "borderRadius", "borderStartStartRadius", "borderStartEndRadius", "borderEndStartRadius", "borderEndEndRadius", "borderTopLeftRadius", "borderTopRightRadius", "borderBottomLeftRadius", "borderBottomRightRadius", "cornerShape", "cornerStartStartShape", "cornerStartEndShape", "cornerEndStartShape", "cornerEndEndShape", "cornerTopLeftShape", "cornerTopRightShape", "cornerBottomLeftShape", "cornerBottomRightShape", "borderImage", "columnRule", "columns", "container", "containIntrinsicSize", "flex", "flexFlow", "font", "fontVariant", "gap", "grid", "gridArea", "gridRow", "gridColumn", "gridTemplate", "inset", "insetInline", "insetBlock", "insetInlineStart", "insetInlineEnd", "left", "right", "listStyle", "margin", "marginInline", "marginBlock", "marginInlineStart", "marginInlineEnd", "marginLeft", "marginRight", "mask", "maskBorder", "offset", "outline", "overflow", "padding", "paddingInline", "paddingBlock", "paddingInlineStart", "paddingInlineEnd", "paddingLeft", "paddingRight", "placeContent", "placeItems", "placeSelf", "scrollMargin", "scrollMarginBlock", "scrollMarginInline", "scrollMarginInlineStart", "scrollMarginInlineEnd", "scrollMarginLeft", "scrollMarginRight", "scrollPadding", "scrollPaddingBlock", "scrollPaddingInline", "scrollPaddingInlineStart", "scrollPaddingInlineEnd", "scrollPaddingLeft", "scrollPaddingRight", "scrollSnapType", "scrollTimeline", "textDecoration", "textEmphasis", "transition", "borderHorizontal", "borderVertical", "borderBlockStart", "borderEnd", "borderBlockEnd", "borderStart", "blockSize", "inlineSize", "minBlockSize", "minInlineSize", "maxBlockSize", "maxInlineSize", "borderHorizontalWidth", "borderHorizontalStyle", "borderHorizontalColor", "borderVerticalWidth", "borderVerticalStyle", "borderVerticalColor", "borderBlockStartColor", "borderBlockEndColor", "borderBlockStartStyle", "borderBlockEndStyle", "borderBlockStartWidth", "borderBlockEndWidth", "borderStartColor", "borderEndColor", "borderStartStyle", "borderEndStyle", "borderStartWidth", "borderEndWidth", "borderTopStartRadius", "borderTopEndRadius", "borderBottomStartRadius", "borderBottomEndRadius", "containIntrinsicBlockSize", "containIntrinsicInlineSize", "gridGap", "gridRowGap", "gridColumnGap", "marginBlockStart", "marginBlockEnd", "marginStart", "marginEnd", "marginHorizontal", "marginVertical", "overflowBlock", "overflowInline", "paddingBlockStart", "paddingBlockEnd", "paddingStart", "paddingEnd", "paddingHorizontal", "paddingVertical", "scrollMarginBlockStart", "scrollMarginBlockEnd", "insetBlockStart", "insetBlockEnd", "start", "end"];
const SHORTHAND_VALUES = ['none', 'auto', '0', 0, 4, '1px solid red', '0 auto', '10px 20px', 'center', 'inherit', 'unset', 'initial', 'revert', '1 / 3', 'span 2', 'red', '50%'];
for (const p of SHORTHANDS) if (!PROPS[p]) PROPS[p] = () => SHORTHAND_VALUES;
// Odd but legal values, mixed into any property now and then.
const EDGE_VALUES = ['RED', '#FFF', 'Rgb(0, 0, 0)', 'var(--x, 10px)', 'var(--a,var(--b))', 'calc(100% - (2 * var(--gap)))',
  'url("a b.png")', "url('x.svg')", 'env(safe-area-inset-top)', 'attr(data-x)', '0.5px', '.5em', '-0px', '1e3px', '+2px',
  '"\\201C"', '"✓"', 'inherit !important', '  spaced  value  ', 'min(1px,2px)', 10.25, -0.5, 1e21, 0.000001];
const PROP_NAMES = Object.keys(PROPS);
// Static expressions over GAP, THEME, SCALE (see gen()). All evaluate at compile time in Babel.
const EVAL_EXPRS = [
  '`${GAP}px`', 'GAP * 2', '-GAP', 'GAP / 2', '(GAP + 1) % 3', "GAP + 'px'", "'1px solid ' + THEME.primary",
  'THEME.primary', "THEME['radius']", 'THEME.nested.tone', 'SCALE[1]', 'SCALE.length', "THEME.missing ?? 'red'",
  "THEME.primary || 'red'", "GAP > 5 ? 'big' : 'small'", 'Math.max(GAP, 10)', 'Math.round(GAP / 3)', 'Math.min(...[GAP, 3])',
  "[GAP, GAP].join('px ') + 'px'", '`calc(100% - ${GAP * 2}px)`', 'String(GAP)', "Number('4')", "'red'.toUpperCase()",
  "`${THEME.primary}`", 'THEME?.primary', '!0', 'void 0', 'typeof GAP', "SCALE.map((s) => s + 'px').join(' ')",
  'Object.keys(THEME).length', "['a', 'b'].includes('a') ? 1 : 0", '0.1 + 0.2', '2 ** 3', 'GAP | 1', "'x'.repeat(2)",
];

const TYPE_FNS = {
  color: COLORS.slice(0, 6),
  length: ['10px', '1rem', '0px'],
  angle: ['0deg', '45deg'],
  number: [1, 0.5],
  percentage: ['50%'],
  time: ['1s', '200ms'],
  integer: [1, 2],
  lengthPercentage: ['10px', '50%'],
};

// ---------------------------------------------------------------------------
// Spec generation

const V = {
  lit: (v) => ({ t: 'lit', v }),
  cond: (entries) => ({ t: 'cond', entries }),
};

function genScalar(rng, prop) {
  if (rng.chance(0.03)) return V.lit(rng.pick(EDGE_VALUES));
  const vals = (PROPS[prop] || (() => COLORS))();
  return V.lit(rng.pick(vals));
}

function condKey(rng, ctx, depth) {
  const k = rng.weighted([
    [5, 'pseudo'], [5, 'at'], [1, 'rarePseudo'], [1, 'rareAt'], [ctx.when ? 2 : 0, 'when'],
    [ctx.consts.length ? 2 : 0, 'const'],
    [ctx.tokens ? 1 : 0, 'tokenConst'],
  ]);
  if (k === 'tokenConst') return { computed: 'tokenConst', name: 'sm' };
  if (k === 'pseudo') return rng.pick(PSEUDO_CLASSES);
  if (k === 'rarePseudo') return rng.pick(RARE_PSEUDO_CLASSES);
  if (k === 'at') return rng.pick(AT_RULES);
  if (k === 'rareAt') return rng.pick(RARE_AT_RULES);
  if (k === 'when') {
    const fn = rng.pick(['ancestor', 'descendant', 'siblingBefore', 'siblingAfter', 'anySibling']);
    const marker = ctx.markers.length && rng.chance(0.6) ? rng.pick(ctx.markers) : null;
    return { computed: 'when', fn, pseudo: rng.pick([':hover', ':focus', ':active', ':focus-visible']), marker };
  }
  // computed key from a local constant (media query string)
  const mqConsts = ctx.consts.filter((c) => c.kind === 'mq');
  if (mqConsts.length) return { computed: 'const', name: rng.pick(mqConsts).name };
  return rng.pick(AT_RULES);
}

function genVarValue(rng, prop, depth = 0) {
  if (depth < 1 && rng.chance(0.35)) {
    const entries = [['default', genVarValue(rng, prop, depth + 1)]];
    for (const k of rng.some(AT_RULES.slice(0, 7), 1, 2)) entries.push([k, genVarValue(rng, prop, depth + 1)]);
    return V.cond(entries);
  }
  if (rng.chance(0.04)) return V.lit(null);
  return genScalar(rng, prop);
}

function genValue(rng, ctx, prop, depth, params, used = new Set()) {
  const r = rng.weighted([
    [10, 'scalar'],
    [depth === 0 ? 4 : depth === 1 ? 1 : 0, 'cond'],
    [1, 'null'],
    [1, 'array'],
    [1, 'ftw'],
    [params && params.length ? 6 : 0, 'param'],
    [params && params.length ? 1 : 0, 'expr'],
    [ctx.consts.some((c) => c.kind === 'val') ? 1 : 0, 'constRef'],
    [ctx.evalExprs ? 3 : 0, 'evalExpr'],
    [ctx.tokens ? 2 : 0, 'token'],
    [ctx.env ? 2 : 0, 'env'],
    [ctx.envFns ? 2 : 0, 'envFn'],
    [ctx.tokens ? 1 : 0, 'tokenConst'],
    [ctx.localVars ? 2 : 0, 'localVar'],
    [prop === 'animationName' ? 30 : 0, 'keyframes'],
    [prop === 'positionTryFallbacks' ? 30 : 0, 'positionTry'],
    [prop === 'viewTransitionClass' ? 30 : 0, 'vtc'],
  ]);
  switch (r) {
    case 'scalar':
      return genScalar(rng, prop);
    case 'null':
      return V.lit(null);
    case 'array':
      return { t: 'arr', items: [genScalar(rng, prop), genScalar(rng, prop)] };
    case 'ftw':
      return { t: 'ftw', items: [genScalar(rng, prop), genScalar(rng, prop)] };
    case 'param':
      return { t: 'param', name: rng.pick(params) };
    case 'expr': {
      const p = rng.pick(params);
      return { t: 'expr', params: [p], code: rng.pick([`\`\${${p}}px\``, `${p} * 2`, `${p} ?? 'auto'`]) };
    }
    case 'constRef':
      return { t: 'constRef', name: rng.pick(ctx.consts.filter((c) => c.kind === 'val')).name };
    case 'evalExpr':
      return { t: 'evalExpr', code: rng.pick(EVAL_EXPRS) };
    case 'token':
      return { t: 'token', src: ctx.tokens.src, name: ctx.tokens.name, key: rng.pick(ctx.tokens.keys) };
    case 'env':
      return { t: 'env', key: rng.pick(ctx.env) };
    case 'envFn':
      return { t: 'envFn', call: rng.pick(['space(2)', 'space(0.5)', "mix('red', 'blue', 30)", 'tokens.accent', "join('a', 'b')"]) };
    case 'tokenConst':
      return { t: 'tokenConst', key: rng.pick(['primary', 'gap']) };
    case 'localVar':
      return { t: 'localVar', key: rng.pick(ctx.localVars) };
    case 'keyframes':
      return ctx.keyframes.length ? { t: 'ref', kind: 'keyframes', name: rng.pick(ctx.keyframes) } : genScalar(rng, prop);
    case 'positionTry':
      return ctx.positionTry.length ? { t: 'ref', kind: 'positionTry', name: rng.pick(ctx.positionTry) } : V.lit('flip-block');
    case 'vtc':
      return ctx.vtc.length ? { t: 'ref', kind: 'vtc', name: rng.pick(ctx.vtc) } : V.lit('none');
    case 'cond': {
      const entries = [];
      if (rng.chance(0.85)) entries.push(['default', genValue(rng, ctx, prop, depth + 1, params, used)]);
      const n = rng.int(1, depth ? 2 : 3);
      for (let k = 0; k < n; k++) {
        let ck = condKey(rng, ctx, depth);
        const id = (c) => {
          if (typeof c === 'string') return c;
          if (c.computed === 'const') return (ctx.consts.find((x) => x.name === c.name) || {}).value;
          return JSON.stringify(c);
        };
        const taken = (c) => used.has(id(c)) || entries.some(([ek]) => id(ek) === id(c));
        for (let tries = 0; tries < 5 && taken(ck); tries++) ck = condKey(rng, ctx, depth);
        if (taken(ck)) continue;
        // a rare duplicate on purpose, to exercise the error path
        const inner = new Set(used);
        inner.add(id(ck));
        for (const [ek] of entries) inner.add(id(ek));
        entries.push([ck, genValue(rng, ctx, prop, depth + 1, params, inner)]);
      }
      return V.cond(entries);
    }
  }
}

function genProps(rng, ctx, n, params) {
  const entries = [];
  const used = new Set();
  for (let k = 0; k < n; k++) {
    let prop = rng.pick(PROP_NAMES);
    if (ctx.keyframes.length && rng.chance(0.08)) prop = 'animationName';
    if (ctx.positionTry.length && rng.chance(0.08)) prop = 'positionTryFallbacks';
    if (ctx.vtc.length && rng.chance(0.08)) prop = 'viewTransitionClass';
    if (used.has(prop)) continue;
    used.add(prop);
    entries.push([prop, genValue(rng, ctx, prop, 0, params)]);
  }
  if (!params && rng.chance(0.15)) {
    const pe = rng.pick(PSEUDO_ELEMENTS);
    entries.push([pe, { t: 'obj', entries: genProps(rng, { ...ctx, consts: [] }, rng.int(1, 3), null).filter((e) => !e[0].startsWith(':')) }]);
  }
  return entries;
}

function genNamespaceKey(rng, k) {
  return rng.pick(['root', 'base', 'active', 'container', 'title', 'button', 'icon', 'row', 'col', 'hidden',
    'wrapper', 'label']) + (k ? String(k) : '');
}

function genCreate(rng, ctx, name) {
  const namespaces = [];
  const n = rng.weighted([[4, 1], [4, 2], [2, 3], [1, 5]]);
  for (let k = 0; k < n; k++) {
    const key = genNamespaceKey(rng, k);
    if (rng.chance(0.18)) {
      const params = rng.some(['w', 'h', 'color', 'opacity', 'size'], 1, 2);
      namespaces.push({ key, params, entries: genProps(rng, ctx, rng.int(1, 3), params) });
    } else {
      namespaces.push({ key, entries: genProps(rng, ctx, rng.weighted([[1, 0], [4, 1], [4, 2], [3, 3], [1, 5]]), null), spreadBase: ctx.evalExprs && rng.chance(0.2) });
    }
  }
  return { kind: 'create', name, exported: rng.chance(0.15), namespaces, quotedKeys: rng.chance(0.15), paren: rng.chance(0.05), exportDefault: rng.chance(0.05) };
}

function genArg(rng, nsList, depth) {
  const statics = nsList.filter((ns) => !ns.params);
  const dyns = nsList.filter((ns) => ns.params);
  const kind = rng.weighted([
    [6, 'ns'], [3, 'and'], [2, 'tern'], [depth < 1 ? 1 : 0, 'arr'], [1, 'lit'], [dyns.length ? 3 : 0, 'dyn'],
    [1, 'opaque'], [statics.length ? 0.4 : 0, 'nsComputed'], [statics.length ? 0.4 : 0, 'nsOptional'],
    [statics.length ? 0.3 : 0, 'dynMember'], [depth < 1 && statics.length ? 0.3 : 0, 'spread'],
  ]);
  const anyNs = () => (statics.length ? { t: 'ns', key: rng.pick(statics).key } : { t: 'lit', v: null });
  switch (kind) {
    case 'ns':
      return anyNs();
    case 'and':
      return { t: 'and', cond: rng.pick(['active', 'props.disabled', 'x > 1', '!hidden']), arg: anyNs() };
    case 'tern':
      return { t: 'tern', cond: rng.pick(['active', 'isDark', 'size === "lg"']), a: anyNs(), b: rng.chance(0.3) ? { t: 'lit', v: null } : anyNs() };
    case 'arr':
      return { t: 'arr', items: [genArg(rng, nsList, depth + 1), genArg(rng, nsList, depth + 1)] };
    case 'lit':
      return { t: 'lit', v: rng.pick([false, null, undefined]) };
    case 'dyn': {
      const ns = rng.pick(dyns);
      return { t: 'dyn', key: ns.key, args: ns.params.map(() => rng.pick(['10', "'red'", 'w', 'props.size', '0.5', 'undefined'])) };
    }
    case 'opaque':
      return { t: 'opaque', code: rng.pick(['props.xstyle', 'style']) };
    case 'nsComputed':
      return { t: 'nsComputed', key: rng.pick(statics).key };
    case 'nsOptional':
      return { t: 'nsOptional', key: rng.pick(statics).key };
    case 'dynMember':
      return { t: 'dynMember', code: rng.pick(['size', 'props.variant']) };
    case 'spread':
      return { t: 'spread', items: [anyNs(), anyNs()] };
  }
}

function genUse(rng, ctx, createName, nsList) {
  const form = rng.weighted([[6, 'props'], [2, 'attrs'], [ctx.jsx ? 3 : 0, 'sx'], [ctx.legacyCall ? 1 : 0, 'legacy']]);
  const args = [];
  const n = rng.int(1, 3);
  for (let k = 0; k < n; k++) args.push(genArg(rng, nsList, 0));
  if (ctx.markers.length && rng.chance(0.2)) args.push({ t: 'marker', name: rng.pick(ctx.markers) });
  if (ctx.when && rng.chance(0.15)) args.push({ t: 'defaultMarker' });
  if (ctx.theme && rng.chance(0.5)) args.push({ t: 'theme', name: ctx.theme });
  return { kind: 'use', form, create: createName, args, inJsx: ctx.jsx && form !== 'legacy' && rng.chance(0.7), componentSx: rng.chance(0.1), classNameMerge: rng.chance(0.1), topLevel: rng.chance(0.1) };
}

function genOptions(rng, fileKind, filename) {
  const o = {};
  const pr = (p, k, vals) => {
    if (rng.chance(p)) o[k] = rng.pick(vals);
  };
  pr(0.3, 'dev', [true, false]);
  pr(0.2, 'test', [true, false]);
  pr(0.25, 'debug', [true, false]);
  pr(0.3, 'classNamePrefix', ['x', 's', 'my-', 'css']);
  pr(0.35, 'runtimeInjection', [true, false, '@stylexjs/stylex/lib/stylex-inject', { from: 'my-inject', as: 'inject' }]);
  pr(0.35, 'styleResolution', ['application-order', 'property-specificity', 'legacy-expand-shorthands']);
  pr(0.15, 'enableMinifiedKeys', [true, false]);
  pr(0.15, 'enableInlinedConditionalMerge', [true, false]);
  pr(0.1, 'enableLogicalStylesPolyfill', [true, false]);
  pr(0.1, 'enableLegacyValueFlipping', [true, false]);
  pr(0.1, 'enableMediaQueryOrder', [true, false]);
  pr(0.1, 'enableDevClassNames', [true, false]);
  pr(0.1, 'enableDebugDataProp', [true, false]);
  pr(0.1, 'enableFontSizePxToRem', [true, false]);
  pr(0.05, 'enableLTRRTLComments', [true]);
  pr(0.1, 'sxPropName', ['css', false]);
  pr(0.1, 'treeshakeCompensation', [true]);
  pr(0.05, 'propertyValidationMode', ['silent', 'warn']);
  if (filename) {
    if (rng.chance(0.85)) {
      o.unstable_moduleResolution = rng.weighted([
        [80, { type: 'commonJS', rootDir: '<ROOT>' }],
        [12, { type: 'haste' }],
        [4, { type: 'custom' }], // render() adds filePathResolver and getCanonicalFilePath
        [4, { type: 'experimental_crossFileParsing', rootDir: '<ROOT>' }],
      ]);
    }
  }
  return o;
}

function genProcessConfig(rng) {
  const c = {};
  c.useLayers = rng.weighted([[5, false], [3, true], [1, { before: ['reset'], after: ['utilities'] }], [1, { prefix: 'sx' }]]);
  if (rng.chance(0.1)) c.enableLTRRTLComments = true;
  if (rng.chance(0.1)) c.legacyDisableLayers = true;
  if (rng.chance(0.1)) c.useLegacyClassnamesSort = true;
  return c;
}

const INVALID_MUTATIONS = [
  'nonStaticValue', 'unassignedCreate', 'createNonObject', 'spreadInCreate', 'unknownNamespace',
  'badOptionValue', 'dynamicBlockBody', 'createInsideCall', 'mediaInsidePseudoElementArray', 'numberKey',
];

function genStylexFile(rng, spec) {
  // .stylex.js file: defineVars / defineConsts / defineMarker, optionally types.
  const items = [];
  const nv = rng.int(1, 2);
  for (let k = 0; k < nv; k++) {
    const entries = [];
    const keys = rng.some(['color', 'bg', 'radius', 'space', 'fontSize', 'shadow', 'duration', 'accent'], 1, 5);
    for (const key of keys) {
      const prop = { color: 'color', bg: 'backgroundColor', radius: 'borderRadius', space: 'padding', fontSize: 'fontSize', shadow: 'boxShadow', duration: 'transitionDuration', accent: 'color' }[key];
      if (rng.chance(0.2)) {
        const tf = rng.pick(Object.keys(TYPE_FNS));
        const inner = rng.chance(0.5)
          ? V.lit(rng.pick(TYPE_FNS[tf]))
          : V.cond([['default', V.lit(rng.pick(TYPE_FNS[tf]))], [rng.pick(AT_RULES.slice(0, 5)), V.lit(rng.pick(TYPE_FNS[tf]))]]);
        entries.push([key, { t: 'type', fn: tf, value: inner }]);
      } else {
        entries.push([key, genVarValue(rng, prop)]);
      }
    }
    items.push({ kind: 'defineVars', name: k ? `vars${k}` : 'vars', entries, quotedKeys: rng.chance(0.1) });
  }
  if (rng.chance(0.5)) {
    const entries = [['sm', V.lit('@media (max-width: 600px)')], ['primary', V.lit(rng.pick(COLORS))], ['size', V.lit(rng.pick([4, 8, '1rem']))]];
    // non-string constants: booleans, objects, arrays
    if (rng.chance(0.3)) entries.push(['flag', V.lit(rng.pick([true, false]))]);
    if (rng.chance(0.2)) entries.push(['obj', { t: 'obj', entries: [['a', V.lit(1)], ['b', V.lit('x')]] }]);
    if (rng.chance(0.2)) entries.push(['list', { t: 'arr', items: [V.lit(1), V.lit('two')] }]);
    items.push({ kind: 'defineConsts', name: 'consts', entries });
  }
  if (rng.chance(0.3) && ((spec.options && spec.options.unstable_moduleResolution) || rng.chance(0.1))) items.push({ kind: 'defineMarker', name: 'cardMarker' });
  if (rng.chance(0.3)) {
    // unstable nested APIs
    const leaf = (prop) => (rng.chance(0.4)
      ? { t: 'uc', entries: [['default', genScalar(rng, prop)], [rng.pick(AT_RULES.slice(0, 5)), genScalar(rng, prop)]] }
      : genScalar(rng, prop));
    const groups = rng.some(['surface', 'text', 'accent', 'badge'], 1, 3).map((g) => [g, { t: 'obj', entries: rng.some(['bg', 'fg', 'border', 'hover'], 1, 3).map((k) => [k, leaf(k === 'border' ? 'borderColor' : 'color')]) }]);
    if (rng.chance(0.3)) groups.push(['deep', { t: 'obj', entries: [['info', { t: 'obj', entries: [['bg', leaf('color')], ['text', leaf('color')]] }]] }]);
    items.push({ kind: 'defineVarsNested', name: 'nestedTokens', entries: groups });
    if (rng.chance(0.5)) {
      const ov = groups.filter(() => rng.chance(0.7)).map(([g, o]) => [g, { t: 'obj', entries: o.entries.filter(() => rng.chance(0.6)).map(([k, v]) => [k, v.t === 'obj' ? v : genScalar(rng, 'color')]) }]);
      items.push({ kind: 'createThemeNested', name: 'nestedTheme', entries: ov });
    }
    if (rng.chance(0.4)) items.push({ kind: 'defineConstsNested', name: 'nestedConsts', entries: [['radius', { t: 'obj', entries: [['sm', V.lit('0.375rem')], ['lg', V.lit(rng.pick(['0.75rem', 12]))]] }], ['font', { t: 'obj', entries: [['mono', V.lit('ui-monospace, monospace')]] }]] });
  }
  spec.items = items;
}

// Corpus mode: a real input (e.g. the Babel suite's recorded inputs) with
// random option changes on top of its own options.
function genCorpus(rng, seed, caseIndex, corpus) {
  const rec = corpus[rng.int(0, corpus.length - 1)];
  const o = genOptions(rng, 'component', rec.filename);
  if (rec.options && rec.options.unstable_moduleResolution) delete o.unstable_moduleResolution;
  if (o.unstable_moduleResolution) o.unstable_moduleResolution = { ...o.unstable_moduleResolution };
  return {
    v: 1, kind: 'corpus', seed, caseIndex, corpusId: `${(rec.testPath || '').split('/').pop()} > ${rec.testName || ''}`,
    code: rec.code, filename: rec.filename, parserPlugins: rec.parserPlugins || [], sourceType: rec.sourceType || 'module',
    baseOptions: rec.options || {}, options: o, processConfig: genProcessConfig(rng),
    fileKind: 'corpus', syntax: 'corpus', importForm: 'corpus', items: [], uses: [], consts: [], typeBits: [],
  };
}

function gen(seed, caseIndex, opts = {}) {
  const rng = makeRng(seed, caseIndex);
  if (opts.corpus) return genCorpus(rng, seed, caseIndex, opts.corpus);
  const fileKind = opts.fileKind || rng.weighted([[70, 'component'], [15, 'stylex'], [8, 'theme'], [7, 'tokensUser']]);
  const syntax = rng.weighted([[35, 'js'], [25, 'flow'], [20, 'tsx'], [8, 'ts'], [12, 'plain']]);
  const importForm = rng.weighted([[35, 'ns'], [15, 'default'], [25, 'named'], [8, 'renamed'], [7, 'require'], [10, 'custom']]);
  const named = fileKind === 'stylex' || fileKind === 'theme' || fileKind === 'tokensUser' || rng.chance(0.75);
  const base = fileKind === 'stylex' ? 'tokens.stylex' : rng.pick(['Button', 'Card', 'App', 'Header', 'theme']);
  const filename = named ? `<ROOT>/src/components/${base}<EXT>` : null;
  const spec = {
    v: 1, seed, caseIndex, fileKind, syntax, importForm, filename,
    customSource: importForm === 'custom' ? rng.pick(['@acme/styles', { from: 'react-strict-dom', as: 'css' }]) : null,
    typeBits: genTypeBits(rng, syntax),
    decor: ['lineComments', 'blockComments', 'tabs', 'useStrict', 'unicode', 'longLine', 'trailing', 'crlf', 'shebang', 'bom'].filter(() => rng.chance(0.04)),
    options: genOptions(rng, fileKind, filename),
    processConfig: genProcessConfig(rng),
    consts: [], items: [], uses: [], mutation: null,
    extras: [],
  };
  if (fileKind === 'stylex') {
    genStylexFile(rng, spec);
    if (rng.chance(0.04)) spec.mutation = rng.pick(['nonStaticValue', 'badOptionValue', 'numberKey']);
    return spec;
  }
  if (fileKind === 'theme') {
    // createTheme over a vars object compiled by Babel at render time.
    const sub = { items: [] };
    genStylexFile(rng, sub);
    const vars = sub.items.find((x) => x.kind === 'defineVars');
    spec.themeVars = vars;
    const overrides = vars.entries.filter(() => rng.chance(0.7)).map(([k]) => {
      const prop = k === 'color' || k === 'accent' ? 'color' : k === 'bg' ? 'backgroundColor' : 'padding';
      return [k, genVarValue(rng, prop)];
    });
    spec.items.push({ kind: 'createTheme', name: 'theme', exported: rng.chance(0.8), overrides });
    // optional create using the theme
    if (rng.chance(0.6)) {
      const ctx = { consts: [], markers: [], keyframes: [], positionTry: [], vtc: [], when: false, jsx: syntax !== 'ts' && syntax !== 'plain', localVars: vars.entries.map(([k]) => k), theme: 'theme' };
      const c = genCreate(rng, ctx, 'styles');
      spec.items.push(c);
      spec.uses.push(genUse(rng, ctx, 'styles', c.namespaces));
    }
    if (rng.chance(0.04)) spec.mutation = 'badOptionValue';
    return spec;
  }
  // component (or tokensUser: imports tokens from a .stylex file)
  const ctx = {
    consts: [], markers: [], keyframes: [], positionTry: [], vtc: [],
    when: rng.chance(0.3), jsx: syntax !== 'ts' && syntax !== 'plain', legacyCall: importForm === 'ns' || importForm === 'default',
    tokens: null, theme: null,
  };
  const mr0 = spec.options.unstable_moduleResolution;
  if ((fileKind === 'tokensUser' || (filename && rng.chance(0.15))) && (mr0 || rng.chance(0.05))) {
    const mr = mr0;
    let src = mr && mr.type === 'haste' ? 'tokens.stylex' : rng.pick(['./tokens.stylex', '../theme/tokens.stylex.js', './tokens.stylex.js', '@fuzz/theme/tokens.stylex.js']);
    if (mr && mr.type === 'commonJS' && rng.chance(0.15)) {
      // aliases option
      spec.options.aliases = { '@tokens/*': ['<ROOT>/src/theme/*'] };
      src = '@tokens/tokens.stylex.js';
    }
    ctx.tokens = { src, name: 'tokens', keys: ['color', 'bg', 'radius', 'space'] };
    if (filename && (ctx.when ? rng.chance(0.6) : rng.chance(0.2))) ctx.markers.push('cardMarker');
    if (rng.chance(0.3)) {
      // createTheme over imported vars
      spec.items.push({ kind: 'createTheme', name: 'importedTheme', exported: true, imported: true, overrides: rng.some(ctx.tokens.keys, 1, 3).map((k) => [k, genVarValue(rng, k === 'color' || k === 'bg' ? 'color' : 'padding')]) });
      ctx.theme = 'importedTheme';
    }
  }
  if (rng.chance(0.12)) {
    // stylex.env with plain values from the options, and sometimes functions
    spec.options.env = { brandColor: rng.pick(COLORS.slice(0, 4)), spacing: rng.pick([4, 8, '1rem']), mq: '@media (min-width: 900px)' };
    ctx.env = ['brandColor', 'spacing'];
    if (rng.chance(0.5)) {
      spec.envFns = true; // render() adds env.space(n), env.mix(a, b, p) and env.tokens.accent
      ctx.envFns = true;
    }
  }
  if (filename && rng.chance(0.06)) spec.debugFilePathFn = true;
  if (filename && rng.chance(0.04)) spec.options.rewriteAliases = true;
  spec.tokens = ctx.tokens;
  spec.importedMarkers = ctx.markers.slice();
  if (rng.chance(0.4)) {
    spec.consts.push({ name: 'MOBILE', kind: 'mq', value: rng.pick(AT_RULES.slice(0, 4)) });
    ctx.consts.push(spec.consts[spec.consts.length - 1]);
  }
  if (rng.chance(0.3)) {
    spec.consts.push({ name: 'BRAND', kind: 'val', value: rng.pick(COLORS) });
    ctx.consts.push(spec.consts[spec.consts.length - 1]);
  }
  if (rng.chance(0.15)) {
    spec.consts.push({ name: 'SPACING', kind: 'val', value: rng.pick([4, 8, '12px']) });
    ctx.consts.push(spec.consts[spec.consts.length - 1]);
  }
  if (rng.chance(0.3)) {
    // objects and arrays for the evaluator: member access, arithmetic, templates, calls
    spec.consts.push({ name: 'GAP', kind: 'num', value: rng.pick([4, 6, 8, 10]) });
    spec.consts.push({ name: 'THEME', kind: 'obj', code: `{ primary: ${q(rng.pick(COLORS.slice(0, 5)))}, radius: ${rng.pick([4, 8])}, nested: { tone: 'dark' } }` });
    spec.consts.push({ name: 'SCALE', kind: 'arr', code: `[${rng.pick(['4, 8, 16', "'sm', 'md'", '1.5, 2'])}]` });
    if (rng.chance(0.4)) spec.consts.push({ name: 'BASE', kind: 'style', code: `{ display: 'flex', ${rng.pick(['alignItems', 'gap'])}: ${rng.pick(["'center'", '8'])} }` });
    ctx.evalExprs = true;
  }
  if (rng.chance(0.25)) {
    const frames = [];
    for (const k of rng.some(['from', 'to', '0%', '50%', '100%'], 2, 3)) {
      frames.push([k, { t: 'obj', entries: genProps(rng, { ...ctx, consts: [] }, rng.int(1, 2), null).filter((e) => !e[0].startsWith(':') && e[1].t !== 'cond') }]);
    }
    spec.items.push({ kind: 'keyframes', name: 'fadeIn', frames, exported: false });
    ctx.keyframes.push('fadeIn');
  }
  if (rng.chance(0.1)) {
    spec.items.push({ kind: 'positionTry', name: 'tryTop', entries: [['positionAnchor', V.lit('--anchor')], ['top', V.lit(rng.pick(['0', 'anchor(bottom)']))], ['left', V.lit('0')], ['width', V.lit(rng.pick(['100px', 120]))]] });
    ctx.positionTry.push('tryTop');
  }
  if (rng.chance(0.1)) {
    const entries = rng.some(['group', 'imagePair', 'old', 'new'], 1, 3).map((k) => [k, { t: 'obj', entries: [[rng.pick(['animationDuration', 'transitionProperty', 'borderRadius']), V.lit(rng.pick(['0.5s', 'none', 16]))]] }]);
    spec.items.push({ kind: 'viewTransitionClass', name: 'vt', entries });
    ctx.vtc.push('vt');
  }
  // defineMarker outside a .stylex file is an error in Babel; keep it rare
  if (ctx.when && rng.chance(0.04)) {
    spec.items.push({ kind: 'defineMarker', name: 'localMarker' });
    ctx.markers.push('localMarker');
  }
  const nCreates = rng.weighted([[8, 1], [2, 2]]);
  for (let k = 0; k < nCreates; k++) {
    const name = k ? `styles${k}` : 'styles';
    const c = genCreate(rng, ctx, name);
    if (rng.chance(0.08) && !spec.items.some((x) => x.inFunction)) c.inFunction = true;
    spec.items.push(c);
    const nUses = rng.int(0, 3);
    for (let u = 0; u < nUses; u++) spec.uses.push(genUse(rng, ctx, name, c.namespaces));
  }
  if (rng.chance(0.06)) spec.mutation = rng.pick(INVALID_MUTATIONS);
  // component above the create() it uses (use before define)
  if (rng.chance(0.25)) spec.usesFirst = true;
  return spec;
}

// ---------------------------------------------------------------------------
// Rendering

const IDENT = /^[A-Za-z_$][\w$]*$/;

function q(s) {
  return JSON.stringify(s);
}

function key(k, quoted) {
  return IDENT.test(k) && !quoted ? k : q(k);
}

function apiNamer(spec) {
  const needed = new Map(); // export name -> local
  const form = spec.importForm;
  const ns = form === 'custom' ? (typeof spec.customSource === 'object' ? spec.customSource.as : 'sx') : 'stylex';
  const fn = (name) => {
    if (form === 'named' || form === 'renamed') {
      const local = form === 'renamed' ? `sx${name[0].toUpperCase()}${name.slice(1)}` : name;
      needed.set(name, local);
      return local;
    }
    if (form === 'custom' && typeof spec.customSource === 'object') {
      // `import {css} from 'react-strict-dom'` -> css.create
      return `${ns}.${name}`;
    }
    return `${ns}.${name}`;
  };
  // `when` and `types` are namespaces of functions
  const member = (obj, prop) => {
    if (form === 'named' || form === 'renamed') {
      const local = form === 'renamed' ? `sx${obj[0].toUpperCase()}${obj.slice(1)}` : obj;
      needed.set(obj, local);
      return `${local}.${prop}`;
    }
    return `${ns}.${obj}.${prop}`;
  };
  const legacy = () => ns;
  const header = () => {
    const src = form === 'custom' ? (typeof spec.customSource === 'object' ? spec.customSource.from : spec.customSource) : '@stylexjs/stylex';
    switch (form) {
      case 'ns':
        return `import * as stylex from ${q(src)};`;
      case 'default':
        return `import stylex from ${q(src)};`;
      case 'require':
        return `const stylex = require(${q(src)});`;
      case 'custom':
        return typeof spec.customSource === 'object' ? `import { ${ns} } from ${q(src)};` : `import * as ${ns} from ${q(src)};`;
      default: {
        if (!needed.size) return `import ${form === 'renamed' ? '{ create as sxCreate }' : '{ create }'} from ${q(src)};`;
        const parts = [...needed].map(([n, l]) => (n === l ? n : `${n} as ${l}`));
        return `import { ${parts.join(', ')} } from ${q(src)};`;
      }
    }
  };
  return { fn, member, legacy, header };
}

function renderer(spec) {
  const api = apiNamer(spec);
  const defined = { consts: new Set(spec.consts.map((c) => c.name)), keyframes: new Set(), positionTry: new Set(), vtc: new Set(), markers: new Set(spec.importedMarkers || []), localVars: null };
  for (const it of spec.items) {
    if (it.kind === 'keyframes') defined.keyframes.add(it.name);
    if (it.kind === 'positionTry') defined.positionTry.add(it.name);
    if (it.kind === 'viewTransitionClass') defined.vtc.add(it.name);
    if (it.kind === 'defineMarker') defined.markers.add(it.name);
  }
  let usedTokens = false;
  let usedConsts = false;
  let usedMarkerImport = false;

  function condKeyCode(k) {
    if (typeof k === 'string') return q(k);
    if (k.computed === 'const') return defined.consts.has(k.name) ? `[${k.name}]` : q('@media print');
    if (k.computed === 'tokenConst') {
      if (!spec.tokens) return q('@media print');
      usedConsts = true;
      return `[consts.${k.name}]`;
    }
    if (k.computed === 'when') {
      let marker = '';
      if (k.marker && defined.markers.has(k.marker)) {
        marker = `, ${k.marker}`;
        if ((spec.importedMarkers || []).includes(k.marker)) usedMarkerImport = true;
      }
      return `[${api.member('when', k.fn)}(${q(k.pseudo)}${marker})]`;
    }
    return q(String(k));
  }

  function value(v, params) {
    switch (v.t) {
      case 'lit':
        return v.v === undefined ? 'undefined' : typeof v.v === 'string' ? q(v.v) : String(v.v);
      case 'arr':
        return `[${v.items.map((x) => value(x, params)).join(', ')}]`;
      case 'ftw':
        return `${api.fn('firstThatWorks')}(${v.items.map((x) => value(x, params)).join(', ')})`;
      case 'param':
        return params && params.includes(v.name) ? v.name : q('auto');
      case 'expr':
        return params && v.params.every((p) => params.includes(p)) ? v.code : q('auto');
      case 'constRef':
        return defined.consts.has(v.name) ? v.name : q('red');
      case 'evalExpr':
        return ['GAP', 'THEME', 'SCALE'].every((n) => defined.consts.has(n) || !new RegExp(`\\b${n}\\b`).test(v.code)) ? v.code : q('red');
      case 'tokenConst':
        if (!spec.tokens) return q('red');
        usedConsts = true;
        return `consts.${v.key}`;
      case 'token':
        usedTokens = true;
        return `${v.name}.${v.key}`;
      case 'env':
        return spec.options.env && v.key in spec.options.env ? api.member('env', v.key) : q('red');
      case 'envFn':
        return spec.envFns && spec.options.env ? api.member('env', v.call) : q('red');
      case 'localVar':
        return defined.localVars && defined.localVars.includes(v.key) ? `vars.${v.key}` : q('red');
      case 'ref': {
        const set = defined[v.kind];
        return set.has(v.name) ? v.name : q(v.kind === 'keyframes' ? 'none' : v.kind === 'vtc' ? 'none' : 'flip-block');
      }
      case 'type':
        return `${api.member('types', v.fn)}(${value(v.value, params)})`;
      case 'obj':
        return `{ ${v.entries.map(([k, x]) => `${key(k)}: ${value(x, params)}`).join(', ')} }`;
      case 'uc':
        return `${api.fn('unstable_conditional')}({ ${v.entries.map(([k, x]) => `${k === 'default' ? 'default' : q(k)}: ${value(x, params)}`).join(', ')} })`;
      case 'cond':
        if (!v.entries.length) return 'null';
        return `{ ${v.entries.map(([k, x]) => `${k === 'default' ? 'default' : condKeyCode(k)}: ${value(x, params)}`).join(', ')} }`;
    }
    return 'null';
  }

  function entriesCode(entries, params, quoted) {
    return entries.map(([k, x]) => `${key(k, quoted)}: ${value(x, params)}`).join(',\n    ');
  }

  function arg(a, nsKeys, createName) {
    switch (a.t) {
      case 'ns':
        return nsKeys.static.has(a.key) ? `${createName}.${a.key}` : 'null';
      case 'and':
        return `${a.cond} && ${arg(a.arg, nsKeys, createName)}`;
      case 'tern':
        return `${a.cond} ? ${arg(a.a, nsKeys, createName)} : ${arg(a.b, nsKeys, createName)}`;
      case 'arr':
        return `[${a.items.map((x) => arg(x, nsKeys, createName)).join(', ')}]`;
      case 'lit':
        return a.v === undefined ? 'undefined' : String(a.v);
      case 'dyn': {
        const p = nsKeys.dyn.get(a.key);
        if (!p) return 'null';
        return `${createName}.${a.key}(${p.map((_, i) => a.args[i] ?? '1').join(', ')})`;
      }
      case 'opaque':
        return a.code;
      case 'nsComputed':
        return nsKeys.static.has(a.key) ? `${createName}[${q(a.key)}]` : 'null';
      case 'nsOptional':
        return nsKeys.static.has(a.key) ? `${createName}?.${a.key}` : 'null';
      case 'dynMember':
        return `${createName}[${a.code}]`;
      case 'spread':
        return `...[${a.items.map((x) => arg(x, nsKeys, createName)).join(', ')}]`;
      case 'marker':
        if (!defined.markers.has(a.name)) return 'null';
        if ((spec.importedMarkers || []).includes(a.name)) usedMarkerImport = true;
        return a.name;
      case 'defaultMarker':
        return `${api.fn('defaultMarker')}()`;
      case 'theme':
        return spec.items.some((x) => x.kind === 'createTheme' && x.name === a.name && (!x.imported || spec.tokens)) ? a.name : 'null';
    }
    return 'null';
  }

  return { api, defined, value, entriesCode, arg, condKeyCode, markTokens: () => { usedTokens = true; }, used: () => ({ usedTokens, usedMarkerImport, usedConsts }) };
}

const FLOW_BITS = {
  importType: "import type { StyleXStyles } from '@stylexjs/stylex';",
  importTypeInline: "import { type StyleXStyles as SXS } from '@stylexjs/stylex';",
  importTypeof: "import typeof TStylex from '@stylexjs/stylex';",
  importReact: "import * as React from 'react';",
  propsType: 'type Props = $ReadOnly<{ active?: boolean, isDark?: boolean, size?: string, x: number, hidden?: boolean, w?: number, style?: StyleXStyles<>, xstyle?: StyleXStyles<> }>;',
  cast: 'const two = (2: number);',
  opaque: 'opaque type Token = string;',
  exportType: "export type Size = 'sm' | 'lg';",
  declare: 'declare var __DEV__: boolean;',
  generic: 'function id<T>(x: T): T { return x; }',
  maybe: 'let maybeNum: ?number = null;',
  exact: 'type Exact = {| a: number |};',
  typedExportConst: 'export const typedNum: number = 1;',
  enum: 'enum Status { Active, Off }',
  enumString: "enum Tone of string { Light = 'light', Dark = 'dark' }",
  retAnn: '',
};
const TS_BITS = {
  importType: "import type { StyleXStyles } from '@stylexjs/stylex';",
  importTypeInline: "import { type StyleXStyles as SXS } from '@stylexjs/stylex';",
  importReact: "import * as React from 'react';",
  propsType: 'interface Props { active?: boolean; isDark?: boolean; size?: "sm" | "lg"; x: number; hidden?: boolean; w?: number; style?: StyleXStyles; xstyle?: StyleXStyles<{ color?: string }> }',
  asConst: 'const SIZES = ["sm", "lg"] as const;',
  enum: 'enum Tone { Light = "light", Dark = "dark" }',
  satisfies: 'const cfg = { a: 1 } satisfies Record<string, number>;',
  nonNull: 'const el = globalThis.document!;',
  generic: 'function id<T>(x: T): T { return x; }',
  typeAlias: "export type Size = 'sm' | 'lg';",
  declare: 'declare const __DEV__: boolean;',
  abstract: 'abstract class Base { abstract f(): void; }',
  typedExportConst: 'export const typedNum: number = 1;',
  retAnn: '',
};

function genTypeBits(rng, syntax) {
  const table = syntax === 'flow' ? FLOW_BITS : syntax === 'ts' || syntax === 'tsx' ? TS_BITS : null;
  if (!table) return [];
  const bits = [];
  for (const k of Object.keys(table)) {
    const p = k === 'propsType' || k === 'retAnn' ? 0.7 : k === 'importType' ? 0.5 : 0.15;
    if (rng.chance(p)) bits.push(k);
  }
  return bits;
}

function typeBitsTable(spec) {
  return spec.syntax === 'flow' ? FLOW_BITS : spec.syntax === 'ts' || spec.syntax === 'tsx' ? TS_BITS : {};
}

function typeAnn(spec, kind) {
  const bits = spec.typeBits || [];
  const s = spec.syntax;
  if (s !== 'flow' && s !== 'ts' && s !== 'tsx') return '';
  if (kind === 'props') return bits.includes('propsType') ? ': Props' : '';
  if (kind === 'ret') return bits.includes('retAnn') ? (s === 'flow' ? ': React.Node' : s === 'tsx' ? ': React.ReactNode' : ': unknown') : '';
  return '';
}

function applyMutation(spec, lines, R) {
  switch (spec.mutation) {
    case 'nonStaticValue':
      if (spec.fileKind === 'stylex') return [...lines, `export const badVars = ${R.api.fn('defineVars')}({ c: computeValue() });`];
      return [...lines, `export const bad = ${R.api.fn('create')}({ x: { color: computeValue() } });`];
    case 'unassignedCreate':
      return [...lines, `${R.api.fn('create')}({ x: { color: 'red' } });`];
    case 'createNonObject':
      return [...lines, `export const bad = ${R.api.fn('create')}(42);`];
    case 'spreadInCreate':
      return [...lines, 'const base = { color: "red" };', `export const bad = ${R.api.fn('create')}({ x: { ...base } });`];
    case 'unknownNamespace':
      return [...lines, 'export const u = styles.doesNotExist;'];
    case 'badOptionValue':
      return lines;
    case 'dynamicBlockBody':
      return [...lines, `export const bad = ${R.api.fn('create')}({ x: (w) => { return { width: w }; } });`];
    case 'createInsideCall':
      return [...lines, `foo(${R.api.fn('create')}({ x: { color: 'red' } }));`];
    case 'mediaInsidePseudoElementArray':
      return [...lines, `export const bad = ${R.api.fn('create')}({ x: { color: ['red', { default: 'blue' }] } });`];
    case 'numberKey':
      return [...lines, `export const bad = ${R.api.fn('create')}({ 1: { color: 'red' } });`];
  }
  return lines;
}

function render(spec, env) {
  const root = env.root;
  if (spec.kind === 'corpus') {
    const options = JSON.parse(JSON.stringify({ ...spec.baseOptions, ...spec.options }).replace(/<ROOT>/g, root));
    return { code: spec.code, filename: spec.filename ?? undefined, options, parserPlugins: spec.parserPlugins, sourceType: spec.sourceType, processConfig: spec.processConfig };
  }
  const R = renderer(spec);
  const body = [];
  const ext = { js: '.js', flow: '.js', tsx: '.tsx', ts: '.ts', plain: '.js' }[spec.syntax];
  const fix = (s) => (typeof s === 'string' ? s.replace(/<ROOT>/g, root).replace(/<EXT>/g, ext) : s);
  const filename = fix(spec.filename);

  for (const c of spec.consts) body.push(`const ${c.name} = ${c.code != null ? c.code : q(c.value)};`);

  if (spec.fileKind === 'theme') {
    // Babel compiles the vars file; its output is inlined.
    const varsSpec = { ...spec, decor: [], typeBits: [], fileKind: 'stylex', importForm: 'ns', items: [spec.themeVars], uses: [], consts: [], mutation: null, filename: '<ROOT>/src/components/tokens.stylex<EXT>', syntax: 'plain' };
    const compiled = env.compileVars ? env.compileVars(varsSpec) : null;
    if (compiled == null) return null;
    body.push(compiled.replace(/^import .*?;\n?/gm, '').replace(/^const stylex = require.*?;\n?/gm, ''));
    R.defined.localVars = spec.themeVars.entries.map(([k]) => k);
  }

  for (const it of spec.items) {
    const ex = it.exported ? 'export ' : '';
    switch (it.kind) {
      case 'keyframes':
        body.push(`${ex}const ${it.name} = ${R.api.fn('keyframes')}({\n  ${it.frames.map(([k, v]) => `${q(k)}: ${R.value(v)}`).join(',\n  ')}\n});`);
        break;
      case 'positionTry':
        body.push(`const ${it.name} = ${R.api.fn('positionTry')}({ ${R.entriesCode(it.entries)} });`);
        break;
      case 'viewTransitionClass':
        body.push(`const ${it.name} = ${R.api.fn('viewTransitionClass')}({ ${it.entries.map(([k, v]) => `${k}: ${R.value(v)}`).join(', ')} });`);
        break;
      case 'defineMarker':
        body.push(`export const ${it.name} = ${R.api.fn('defineMarker')}();`);
        break;
      case 'defineVars':
        body.push(`export const ${it.name} = ${R.api.fn('defineVars')}({\n  ${R.entriesCode(it.entries, null, it.quotedKeys)}\n});`);
        break;
      case 'defineVarsNested':
        body.push(`export const ${it.name} = ${R.api.fn('unstable_defineVarsNested')}({\n  ${R.entriesCode(it.entries)}\n});`);
        break;
      case 'createThemeNested':
        if (!spec.items.some((x) => x.kind === 'defineVarsNested')) break;
        body.push(`export const ${it.name} = ${R.api.fn('unstable_createThemeNested')}(nestedTokens, {\n  ${R.entriesCode(it.entries)}\n});`);
        break;
      case 'defineConstsNested':
        body.push(`export const ${it.name} = ${R.api.fn('unstable_defineConstsNested')}({\n  ${R.entriesCode(it.entries)}\n});`);
        break;
      case 'defineConsts':
        body.push(`export const ${it.name} = ${R.api.fn('defineConsts')}({\n  ${R.entriesCode(it.entries)}\n});`);
        break;
      case 'createTheme': {
        if (it.imported) {
          if (!spec.tokens) break;
          R.markTokens();
          body.push(`${ex}const ${it.name} = ${R.api.fn('createTheme')}(${spec.tokens.name}, {\n  ${R.entriesCode(it.overrides)}\n});`);
          break;
        }
        const ov = it.overrides.filter(([k]) => R.defined.localVars && R.defined.localVars.includes(k));
        body.push(`${ex}const ${it.name} = ${R.api.fn('createTheme')}(vars, {\n  ${R.entriesCode(ov)}\n});`);
        break;
      }
      case 'create': {
        const nsCode = it.namespaces.map((ns) => {
          const k = key(ns.key, it.quotedKeys);
          if (ns.params) return `  ${k}: (${ns.params.join(', ')}) => ({\n    ${R.entriesCode(ns.entries, ns.params)}\n  })`;
          const spread = ns.spreadBase && R.defined.consts.has('BASE') ? '...BASE,\n    ' : '';
          return `  ${k}: {\n    ${spread}${R.entriesCode(ns.entries)}\n  }`;
        });
        const call = `${R.api.fn('create')}({\n${nsCode.join(',\n')}\n})`;
        if (it.inFunction) body.push(`function useStyles() {\n  const ${it.name} = ${call};\n  return ${it.name};\n}`);
        else if (it.exportDefault && !spec.uses.some((u) => u.create === it.name)) body.push(`export default ${call};`);
        else body.push(`${ex}const ${it.name} = ${it.paren ? `(${call})` : call};`);
        break;
      }
    }
  }

  // uses
  const topLevelUses = [];
  const jsxLines = [];
  const plainLines = [];
  for (const u of spec.uses) {
    const c = spec.items.find((x) => x.kind === 'create' && x.name === u.create && !x.inFunction);
    if (!c) continue;
    const nsKeys = { static: new Set(c.namespaces.filter((n) => !n.params).map((n) => n.key)), dyn: new Map(c.namespaces.filter((n) => n.params).map((n) => [n.key, n.params])) };
    const args = u.args.map((a) => R.arg(a, nsKeys, c.name));
    if (u.form === 'sx' && u.inJsx) {
      const sxArgs = args.map((x) => (x.startsWith('...') ? `[${x.slice(3)}]` : x));
      const tag = u.componentSx ? 'Box' : 'div';
      jsxLines.push(`<${tag} ${spec.options.sxPropName === 'css' ? 'css' : 'sx'}={${sxArgs.length === 1 ? sxArgs[0] : `[${sxArgs.join(', ')}]`}} />`);
    } else if (u.form === 'legacy') {
      plainLines.push(`${R.api.legacy()}(${args.join(', ')})`);
    } else {
      const fnName = u.form === 'attrs' ? 'attrs' : 'props';
      const call = `${R.api.fn(fnName)}(${args.join(', ')})`;
      // module-level call: only arguments that need no component scope
      if (u.topLevel && !u.inJsx && args.every((x) => !/\b(active|isDark|size|x|hidden|w|props|style)\b/.test(x))) {
        topLevelUses.push(`export const top${topLevelUses.length} = ${call};`);
        continue;
      }
      if (u.inJsx) jsxLines.push(u.classNameMerge ? `<span className="extra" {...${call}} id="x" />` : `<span {...${call}} />`);
      else plainLines.push(call);
    }
  }
  const usesAt = spec.usesFirst ? body.length : -1;
  const bodyBefore = spec.usesFirst ? body.splice(0, body.length) : null;
  if (jsxLines.length || plainLines.length) {
    const params = '{ active, isDark, size, x, hidden, w, style, ...props }';
    const ta = typeAnn(spec, 'props');
    const ret = typeAnn(spec, 'ret');
    const decls = plainLines.map((l, i) => `  const c${i} = ${l};`);
    if (jsxLines.length) {
      body.push(`export function Component(${params}${ta})${ret} {\n${decls.join('\n')}${decls.length ? '\n' : ''}  return (\n    <div>\n      ${jsxLines.join('\n      ')}\n    </div>\n  );\n}`);
    } else {
      body.push(`export function useClasses(${params}${ta})${ret} {\n${decls.join('\n')}\n  return [${plainLines.map((_, i) => `c${i}`).join(', ')}];\n}`);
    }
  }

  body.push(...topLevelUses);
  if (bodyBefore) {
    // consts stay on top, the component goes before the StyleX calls
    const constLines = bodyBefore.filter((l) => /^const [A-Z_]+ = /.test(l));
    const rest = bodyBefore.filter((l) => !/^const [A-Z_]+ = /.test(l));
    body.unshift(...constLines);
    body.push(...rest);
  }
  let lines = body;
  lines = applyMutation(spec, lines, R);

  const head = [];
  const used = R.used();
  const tb = typeBitsTable(spec);
  const bits = (spec.typeBits || []).filter((b) => tb[b] != null);
  for (const b of bits) if (b.startsWith('import') && tb[b]) head.push(tb[b]);
  head.push(R.api.header());
  if ((used.usedTokens || used.usedConsts) && spec.tokens) {
    head.push(`import { ${[used.usedTokens && spec.tokens.name, used.usedConsts && 'consts'].filter(Boolean).join(', ')} } from ${q(spec.tokens.src)};`);
  }
  if (used.usedMarkerImport) head.push(`import { cardMarker } from ${q(spec.tokens ? spec.tokens.src : './tokens.stylex')};`);
  for (const b of bits) if (!b.startsWith('import') && tb[b]) head.push(tb[b]);

  let code = `${head.join('\n')}\n\n${lines.join('\n\n')}\n`;
  for (const d of spec.decor || []) {
    if (d === 'lineComments') code = code.replace(/([,{])\n/g, '$1 // note\n');
    else if (d === 'blockComments') code = code.replace(/\n(\s*)([A-Za-z_$"'][\w$"'-]*): /g, '\n$1/* c */ $2: ');
    else if (d === 'tabs') code = code.replace(/^( {2})+/gm, (m) => '\t'.repeat(m.length / 2));
    else if (d === 'useStrict') code = `'use strict';\n${code}`;
    else if (d === 'unicode') code = `const café = 'ünïcödé ✓';\n${code}`;
    else if (d === 'longLine') code = `const LONG = '${'x'.repeat(3000)}';\n${code}`;
    else if (d === 'trailing') code = `${code}\n// trailing comment without newline`;
  }
  for (const d of spec.decor || []) {
    if (d === 'crlf') code = code.replace(/\n/g, '\r\n');
    else if (d === 'shebang') code = `#!/usr/bin/env node\n${code}`;
  }
  // Babel cannot parse a shebang after a BOM, so a BOM only goes on files without one
  for (const d of spec.decor || []) if (d === 'bom' && !(spec.decor || []).includes('shebang')) code = `\uFEFF${code}`;

  const options = JSON.parse(JSON.stringify(spec.options).replace(/<ROOT>/g, root));
  if (spec.envFns && options.env) {
    options.env.space = (n) => `${n * 4}px`;
    options.env.mix = (a, b, p) => `color-mix(in srgb, ${a} ${p}%, ${b})`;
    options.env.tokens = { accent: '#0055ff' };
    options.env.join = (...xs) => xs.join('-');
  }
  if (spec.debugFilePathFn) options.debugFilePath = (f) => `dbg/${f.split('/').pop()}`;
  if (options.unstable_moduleResolution && options.unstable_moduleResolution.type === 'custom') {
    const pathMod = require('node:path');
    const fsMod = require('node:fs');
    options.unstable_moduleResolution.filePathResolver = (importPath, sourceFilePath) => {
      if (!importPath.startsWith('.')) return undefined;
      const base = pathMod.resolve(pathMod.dirname(sourceFilePath), importPath);
      return [base, `${base}.js`].find((f) => fsMod.existsSync(f) && fsMod.statSync(f).isFile());
    };
    options.unstable_moduleResolution.getCanonicalFilePath = (f) => pathMod.relative(root, f);
  }
  if (spec.importForm === 'custom') options.importSources = [spec.customSource];
  if (spec.mutation === 'badOptionValue') options.styleResolution = 'not-a-mode';
  const parserPlugins =
    spec.syntax === 'flow' ? [['flow', { all: true, enums: true }], 'jsx']
      : spec.syntax === 'tsx' ? ['typescript', 'jsx']
        : spec.syntax === 'ts' ? ['typescript']
          : spec.syntax === 'plain' ? []
            : ['jsx'];
  let extraFiles;
  if (spec.tokens && (used.usedConsts || used.usedTokens)) {
    const src = spec.tokens.src;
    const map = {
      './tokens.stylex': 'src/components/tokens.stylex',
      './tokens.stylex.js': 'src/components/tokens.stylex.js',
      '../theme/tokens.stylex.js': 'src/theme/tokens.stylex.js',
      '@tokens/tokens.stylex.js': 'src/theme/tokens.stylex.js',
      '@fuzz/theme/tokens.stylex.js': 'node_modules/@fuzz/theme/tokens.stylex.js',
    };
    if (map[src]) {
      const f = path.join(root, map[src]);
      extraFiles = [{ code: require('node:fs').readFileSync(f, 'utf8'), filename: f.endsWith('.stylex') ? `${f}.js` : f }];
    }
  }
  return {
    extraFiles,
    code,
    filename: filename ?? undefined,
    options,
    parserPlugins,
    sourceType: 'module',
    processConfig: spec.processConfig,
  };
}

// ---------------------------------------------------------------------------
// Feature tags (for triage and owner assignment)

function features(spec) {
  const tags = new Set();
  if (spec.kind === 'corpus') tags.add(`corpus:${spec.corpusId.split(' > ')[0]}`);
  tags.add(`file:${spec.fileKind}`);
  tags.add(`syntax:${spec.syntax}`);
  tags.add(`import:${spec.importForm}`);
  for (const [k, v] of Object.entries(spec.options)) {
    if (k === 'unstable_moduleResolution') tags.add(`opt:moduleResolution:${v.type}`);
    else tags.add(`opt:${k}=${typeof v === 'object' ? 'obj' : v}`);
  }
  const pc = spec.processConfig || {};
  if (pc.useLayers) tags.add(`css:useLayers=${typeof pc.useLayers === 'object' ? 'obj' : 'true'}`);
  for (const k of ['enableLTRRTLComments', 'legacyDisableLayers', 'useLegacyClassnamesSort']) if (pc[k]) tags.add(`css:${k}`);
  if (!spec.filename) tags.add('file:unnamed');
  for (const b of spec.typeBits || []) tags.add(`${spec.syntax === 'flow' ? 'flow' : 'ts'}:${b}`);
  for (const d of spec.decor || []) tags.add(`decor:${d}`);
  for (const it of spec.items) {
    if (it.paren) tags.add('create:paren');
    if (it.exportDefault && !spec.uses.some((u) => u.create === it.name)) tags.add('create:exportDefault');
  }
  for (const u of spec.uses) {
    if (u.componentSx && u.form === 'sx' && u.inJsx) tags.add('use:sx:component');
    if (u.classNameMerge && u.inJsx && u.form !== 'sx') tags.add('use:classNameMerge');
  }
  if (spec.mutation) tags.add(`invalid:${spec.mutation}`);
  if (spec.usesFirst && spec.uses.length) tags.add('order:useBeforeCreate');
  if (spec.debugFilePathFn) tags.add('opt:debugFilePath=fn');
  for (const it of spec.items) for (const ns of it.namespaces || []) if (ns.spreadBase) tags.add('create:spreadConst');
  for (const u of spec.uses) if (u.topLevel) tags.add('use:topLevel');
  if (spec.consts.length) tags.add('consts');
  const walkVal = (v) => {
    if (!v || typeof v !== 'object') return;
    if (v.t === 'cond') {
      tags.add('value:conditional');
      for (const [k, x] of v.entries) {
        if (typeof k === 'string') {
          if (k.startsWith('@media')) tags.add('cond:@media');
          else if (k.startsWith('@supports')) tags.add('cond:@supports');
          else if (k.startsWith('@')) tags.add(`cond:${k.split(' ')[0]}`);
          else if (k.startsWith(':')) tags.add('cond:pseudo');
          if (RARE_PSEUDO_CLASSES.includes(k) || RARE_AT_RULES.includes(k)) tags.add(`cond:rare:${k}`);
        } else if (k.computed === 'when') {
          tags.add('when');
          if (k.marker) tags.add('when:marker');
        } else if (k.computed === 'const') tags.add('computedKey:const');
        else if (k.computed === 'tokenConst') tags.add('computedKey:importedConst');
        walkVal(x);
      }
    } else if (v.t === 'arr') {
      tags.add('value:fallbackArray');
      v.items.forEach(walkVal);
    } else if (v.t === 'ftw') {
      tags.add('firstThatWorks');
      v.items.forEach(walkVal);
    } else if (v.t === 'obj') v.entries.forEach(([, x]) => walkVal(x));
    else if (v.t === 'uc') {
      tags.add('unstable_conditional');
      v.entries.forEach(([, x]) => walkVal(x));
    }
    else if (v.t === 'type') {
      tags.add('stylex.types');
      walkVal(v.value);
    } else if (v.t === 'lit' && v.v === null) tags.add('value:null');
    else if (v.t === 'token') tags.add('tokens:import');
    else if (v.t === 'env') tags.add('stylex.env');
    else if (v.t === 'envFn') tags.add('stylex.env:function');
    else if (v.t === 'tokenConst') tags.add('consts:import');
    else if (v.t === 'constRef') tags.add('consts:valueRef');
    else if (v.t === 'evalExpr') tags.add('eval:expr');
    else if (v.t === 'expr') tags.add('dynamic:expr');
    else if (v.t === 'ref') tags.add(`ref:${v.kind}`);
    else if (v.t === 'lit' && typeof v.v === 'number') tags.add('value:number');
  };
  for (const it of spec.items) {
    tags.add(`api:${it.kind}`);
    if (it.imported) tags.add('createTheme:importedVars');
    if (it.kind === 'create') {
      if (it.inFunction) tags.add('create:inFunction');
      if (it.exported) tags.add('create:exported');
      if (it.quotedKeys) tags.add('create:quotedKeys');
      for (const ns of it.namespaces) {
        if (ns.params) tags.add('create:dynamic');
        for (const [k, v] of ns.entries) {
          if (k.startsWith('::')) tags.add('pseudoElement');
          if (k.startsWith('--')) tags.add('customProperty');
          if (/^(Webkit|Moz)/.test(k)) tags.add('vendorProperty');
          tags.add(`prop:${k}`);
          walkVal(v);
        }
      }
    }
    for (const list of [it.entries, it.overrides, it.frames]) if (list) for (const [, v] of list) walkVal(v);
  }
  for (const u of spec.uses) {
    tags.add(`use:${u.form}${u.inJsx ? ':jsx' : ''}`);
    const walkArg = (a) => {
      tags.add(`arg:${a.t}`);
      if (a.arg) walkArg(a.arg);
      if (a.a) walkArg(a.a);
      if (a.b) walkArg(a.b);
      if (a.items) a.items.forEach(walkArg);
    };
    u.args.forEach(walkArg);
  }
  return [...tags].sort();
}

module.exports = { gen, render, features, makeRng };
