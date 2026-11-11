/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

'use strict';

/**
 * Keeps `CSSProperties` honest.
 *
 * Two things are hand-maintained and can drift:
 *
 *   1. `StyleXCSSTypes.d.ts` duplicates `StyleXCSSTypes.js`, because a Flow
 *      union containing `string` collapses to `string` and loses its literals,
 *      so TypeScript needs `(string & {})` instead. Nothing stops the two
 *      files diverging apart from this test.
 *   2. The property values themselves are written by hand, so a typo or an
 *      invented keyword goes unnoticed. `css-tree` ships the CSS value
 *      definition grammar, so it can say whether a keyword is real.
 */

const fs = require('fs');
const path = require('path');
const { parse } = require('@babel/parser');
const { lexer } = require('css-tree');
// `web-features` itself is ESM; its data is exported as JSON.
const { features } = require('web-features/data.json');
const { generate } = require('css-tree/definition-syntax');

const TYPES_DIR = path.resolve(__dirname, '../src/types');
const FLOW_FILE = path.join(TYPES_DIR, 'StyleXCSSTypes.js');
const TS_FILE = path.join(TYPES_DIR, 'StyleXCSSTypes.d.ts');

/** CSS-wide keywords, accepted by every property and never in a grammar. */
const GLOBAL_KEYWORDS = new Set([
  'inherit',
  'initial',
  'unset',
  'revert',
  'revert-layer',
]);

/* ------------------------------------------------------------------ *
 * Reading the type files
 * ------------------------------------------------------------------ */

/**
 * What a property accepts, reduced to the three things worth comparing: the
 * literal keywords, whether a bare number is allowed, and whether an arbitrary
 * string is allowed.
 */
const blank = () => ({ literals: new Set(), open: false, numeric: false });

const absorb = (into, from) => {
  from.literals.forEach((literal) => into.literals.add(literal));
  into.open = into.open || from.open;
  into.numeric = into.numeric || from.numeric;
};

const finish = (value) => ({
  literals: [...value.literals].sort(),
  open: value.open,
  numeric: value.numeric,
});

/** Parsing either file takes a moment, and neither changes mid-run. */
const parsed = new Map();
const readTypes = (file) => {
  if (!parsed.has(file)) parsed.set(file, parseTypes(file));
  return parsed.get(file);
};

/** A type alias, in either dialect: its name and the type it aliases. */
function asTypeAlias(statement) {
  const node =
    statement.type === 'ExportNamedDeclaration'
      ? statement.declaration
      : statement;
  if (!node) return null;
  if (node.type === 'TypeAlias') return [node.id.name, node.right];
  if (node.type === 'TSTypeAliasDeclaration') {
    return [node.id.name, node.typeAnnotation];
  }
  return null;
}

/** The `[name, type]` pairs of an object type, in either dialect. */
function objectTypeEntries(node) {
  if (node.type === 'ObjectTypeAnnotation') {
    return node.properties.map((property) => [
      property.key.name,
      property.value,
    ]);
  }
  return node.members.map((member) => [
    member.key.name,
    member.typeAnnotation.typeAnnotation,
  ]);
}

/** Unwraps the `Readonly<...>` around `CSSProperties`. */
function unwrapReadonly(node) {
  const args = node.typeParameters?.params;
  return args?.length === 1 ? args[0] : node;
}

/**
 * Reads `property -> { literals, open, numeric }` out of a types file.
 *
 * `@babel/parser` reads both dialects, so one function covers both files: the
 * Flow types with the `flow` plugin, the TypeScript definitions with
 * `typescript` in declaration mode. The two ASTs use disjoint node names, so a
 * single switch can handle either.
 */
function parseTypes(file) {
  const isDeclaration = file.endsWith('.d.ts');
  const ast = parse(fs.readFileSync(file, 'utf8'), {
    sourceType: 'module',
    plugins: isDeclaration ? [['typescript', { dts: true }]] : ['flow'],
  });

  const aliases = new Map();
  let cssProperties = null;
  for (const statement of ast.program.body) {
    const alias = asTypeAlias(statement);
    if (!alias) continue;
    aliases.set(alias[0], alias[1]);
    if (alias[0] === 'CSSProperties') cssProperties = alias[1];
  }

  const resolve = (node, seen) => {
    const result = blank();
    if (!node) return result;
    switch (node.type) {
      // A parenthesised type, e.g. `(string & {})`.
      case 'TSParenthesizedType':
        return resolve(node.typeAnnotation, seen);

      case 'UnionTypeAnnotation':
      case 'TSUnionType':
        for (const member of node.types) absorb(result, resolve(member, seen));
        return result;

      // `string & {}` in the TypeScript definitions: an arbitrary string that
      // leaves sibling literals visible to autocomplete. The `{}` half
      // contributes nothing, so this reads as a plain string.
      case 'IntersectionTypeAnnotation':
      case 'TSIntersectionType':
        for (const member of node.types) absorb(result, resolve(member, seen));
        return result;

      case 'StringLiteralTypeAnnotation':
        result.literals.add(node.value);
        return result;
      case 'TSLiteralType':
        if (node.literal.type === 'StringLiteral') {
          result.literals.add(node.literal.value);
        } else if (node.literal.type === 'NumericLiteral') {
          result.numeric = true;
        }
        return result;

      case 'StringTypeAnnotation':
      case 'TSStringKeyword':
        result.open = true;
        return result;

      case 'NumberTypeAnnotation':
      case 'NumberLiteralTypeAnnotation':
      case 'TSNumberKeyword':
        result.numeric = true;
        return result;

      // Every property accepts `null`; StyleX uses it to unset a value.
      case 'NullLiteralTypeAnnotation':
      case 'TSNullKeyword':
        return result;

      // The `{}` of `string & {}`, and Flow's equivalent.
      case 'ObjectTypeAnnotation':
      case 'TSTypeLiteral':
        return result;

      case 'GenericTypeAnnotation':
      case 'TSTypeReference': {
        const name =
          node.type === 'GenericTypeAnnotation'
            ? node.id.name
            : node.typeName.name;
        const args = node.typeParameters?.params ?? [];
        // `OptionalArray<T>` is `T` or an array of `T`. It does not widen what
        // a single value may be, so it contributes nothing of its own.
        if (name === 'OptionalArray' || name === 'Array') {
          for (const arg of args) absorb(result, resolve(arg, seen));
          return result;
        }
        if (aliases.has(name) && !seen.has(name)) {
          return resolve(aliases.get(name), new Set([...seen, name]));
        }
        // An unresolvable reference could be anything.
        result.open = true;
        return result;
      }

      default:
        result.open = true;
        return result;
    }
  };

  const properties = {};
  for (const [name, type] of objectTypeEntries(unwrapReadonly(cssProperties))) {
    properties[name] = finish(resolve(type, new Set()));
  }
  return properties;
}

/* ------------------------------------------------------------------ *
 * Reading the CSS grammar
 * ------------------------------------------------------------------ */

const camelToKebab = (name) =>
  name
    .replace(/^(Moz|Webkit|Ms|O)(?=[A-Z])/, (m) => '-' + m.toLowerCase() + '-')
    .replace(/([a-z0-9])([A-Z])/g, '$1-$2')
    .toLowerCase();

/** Grammar types that resolve no further, so walking into them finds nothing. */
const TERMINAL = new Set([
  'angle',
  'angle-percentage',
  'basic-shape',
  'counter-name',
  'counter-style-name',
  'custom-ident',
  'dashed-ident',
  'declaration-value',
  'dimension',
  'family-name',
  'feature-tag-value',
  'filter-function',
  'flex',
  'frequency',
  'gradient',
  'hex-color',
  'ident',
  'image',
  'integer',
  'keyframes-name',
  'length',
  'length-percentage',
  'number',
  'percentage',
  'position',
  'ratio',
  'resolution',
  'shape',
  'string',
  'time',
  'transform-function',
  'url',
  'zero',
]);

/**
 * Every keyword the grammar accepts for a property, or `null` when css-tree
 * has no grammar for it at all.
 */
function grammarKeywords(cssName, maxDepth = 6) {
  const descriptor = lexer.properties[cssName];
  if (!descriptor) return null;

  const keywords = new Set();
  const seen = new Set();

  const walk = (node, depth) => {
    if (!node || depth > maxDepth) return;
    switch (node.type) {
      case 'Keyword':
        keywords.add(node.name);
        return;
      case 'String':
        keywords.add(node.value.replace(/^["']|["']$/g, ''));
        return;
      case 'Type': {
        // Functional notations serialise to arbitrary strings; walking into
        // them harvests nonsense from the math grammar (`e`, `pi`, `NaN`).
        if (node.name.endsWith('()') || TERMINAL.has(node.name)) return;
        const type = lexer.types[node.name];
        if (!type || seen.has(node.name)) return;
        seen.add(node.name);
        walk(type.syntax, depth + 1);
        seen.delete(node.name);
        return;
      }
      case 'Property': {
        // Logical properties and shorthands are defined almost entirely as
        // `<'other-property'>` references.
        const referenced = lexer.properties[node.name];
        const key = `property:${node.name}`;
        if (!referenced || seen.has(key)) return;
        seen.add(key);
        walk(referenced.syntax, depth + 1);
        seen.delete(key);
        return;
      }
      case 'Group':
        for (const term of node.terms) walk(term, depth);
        return;
      case 'Multiplier':
        walk(node.term, depth);
        return;
      default:
        return;
    }
  };

  walk(descriptor.syntax, 0);
  return keywords;
}

/**
 * Whether a property's grammar admits anything that is not a bare keyword --
 * a function, a URL, a length, an identifier. If it does, a union of literals
 * cannot describe the property and the type has to accept arbitrary strings.
 */
function needsArbitraryString(cssName) {
  const descriptor = lexer.properties[cssName];
  if (!descriptor) return false;

  const seen = new Set();
  const walk = (node) => {
    if (!node) return false;
    switch (node.type) {
      case 'Type': {
        if (node.name.endsWith('()') || TERMINAL.has(node.name)) return true;
        const type = lexer.types[node.name];
        if (!type || seen.has(node.name)) return false;
        seen.add(node.name);
        const found = walk(type.syntax);
        seen.delete(node.name);
        return found;
      }
      case 'Property': {
        const referenced = lexer.properties[node.name];
        const key = `property:${node.name}`;
        if (!referenced || seen.has(key)) return false;
        seen.add(key);
        const found = walk(referenced.syntax);
        seen.delete(key);
        return found;
      }
      case 'Function':
      case 'Token':
        return true;
      case 'Group':
        return node.terms.some(walk);
      case 'Multiplier':
        return walk(node.term);
      default:
        return false;
    }
  };

  return walk(descriptor.syntax);
}

/* ------------------------------------------------------------------ *
 * Deliberate deviations from the grammar
 * ------------------------------------------------------------------ */

/**
 * Properties css-tree has no grammar for. Mostly abandoned drafts
 * (`motion-*` became `offset-*`, the CSS Display Level 3 longhands were
 * dropped), `@font-face` descriptors that were never properties (`src`,
 * `unicode-range`), and names that never shipped unprefixed.
 */
const NO_GRAMMAR = new Set([
  'azimuth',
  'boxSuppress',
  'displayInside',
  'displayList',
  'displayOutside',
  'end',
  'markerOffset',
  'motion',
  'motionOffset',
  'motionPath',
  'motionRotation',
  'overflowBlockX',
  'src',
  'start',
  'textFillColor',
  'unicodeRange',
  'WebkitBoxOrient',
]);

/**
 * Keywords kept even though the current grammar has dropped them. Each is a
 * value real stylesheets still use, so removing it would break code.
 */
const KEPT_KEYWORDS = {
  // The spec was rewritten to `none | in-flow | all`, but Safari ships the
  // original `block`/`inline` syntax.
  marginTrim: [
    'block',
    'block-end',
    'block-start',
    'inline',
    'inline-end',
    'inline-start',
  ],
  // Renamed to `inline-start`/`inline-end`.
  float: ['end', 'start'],
  // Renamed to `never`/`always`.
  speak: ['none', 'normal'],
  // Deprecated alias of `inter-character`.
  textJustify: ['distribute'],
  // Logical values from a dropped CSS Logical draft.
  captionSide: ['block-end', 'block-start', 'inline-end', 'inline-start'],
  // In the CSS UI draft, though not in css-tree.
  userSelect: ['contain'],
  // The one case where the grammar is stricter and correct: the spec excludes
  // `hidden` from `outline-style`. Kept because removing it would break code.
  outlineStyle: ['hidden'],
  // CSS 2.1 allowed `invert`; the current spec is `auto | <color>`.
  outlineColor: ['invert'],
  // `-webkit-appearance` never standardised `auto`, but it is widely written.
  WebkitAppearance: ['auto'],
  // `mask-origin` narrowed from `<geometry-box>` to `<coord-box>`, dropping
  // `margin-box`.
  maskOrigin: ['margin-box'],
  // Unprefixed intrinsic sizing keywords, which only ever shipped prefixed
  // (`-webkit-fill-available`, `-moz-available`).
  blockSize: ['available'],
  inlineSize: ['available'],
  width: ['available'],
  maxBlockSize: ['fill-available'],
  maxHeight: ['fill-available'],
  maxInlineSize: ['fill-available'],
  maxWidth: ['fill-available'],
  minBlockSize: ['fill-available'],
  minHeight: ['fill-available'],
  minInlineSize: ['fill-available'],
  minWidth: ['fill-available'],
};

/**
 * Baseline status per CSS property, from `web-features` -- the data behind
 * MDN's "Widely available" / "Newly available" / "Limited availability"
 * banners. `high` means supported across the core browsers for over a year,
 * `low` means it just got there, and `false` means it has not.
 *
 * css-tree carries only grammar, with no support data at all, and mdn-data's
 * `status` describes the spec rather than browsers -- it calls `font-stretch`
 * obsolete even though every browser has supported it since 2020, while
 * marking its replacement `font-width` experimental. Baseline is the signal
 * that actually says whether a property is safe to use.
 */
const BASELINE = new Map();
for (const feature of Object.values(features)) {
  for (const key of feature.compat_features ?? []) {
    const match = key.match(/^css\.properties\.([a-z-]+)$/);
    if (match) BASELINE.set(match[1], feature.status?.baseline);
  }
}

const kebabToCamel = (name) =>
  name.replace(/-([a-z])/g, (_, letter) => letter.toUpperCase());

/**
 * Properties that are Baseline-available but deliberately not typed.
 *
 * Anything Baseline `false` needs no entry here -- it is not safe to use yet,
 * so not typing it is the default. That keeps this list to actual decisions.
 */
const NOT_TYPED = new Set([
  // Shipped and worth typing -- a backlog rather than a decision.
  // `whiteSpaceCollapse` is in fact commented out in `StyleXCSSTypes.js`
  // awaiting exactly this, and `overflowInline` is the logical counterpart of
  // `overflowBlock`, which is typed.
  'fieldSizing',
  'overflowInline',
  'textWrapMode',
  'textWrapStyle',
  'transitionBehavior',
  'viewTransitionClass',
  'whiteSpaceCollapse',

  // SVG presentation attributes, valid as CSS on SVG elements. StyleX types
  // `fill`, `stroke`, `strokeWidth`, `fillOpacity` and `strokeOpacity`, so
  // excluding the rest is arbitrary -- but `x`, `y`, `d` and `r` read oddly as
  // style keys, so it stays a choice rather than an oversight.
  'colorInterpolation',
  'colorInterpolationFilters',
  'cx',
  'cy',
  'd',
  'floodColor',
  'floodOpacity',
  'lightingColor',
  'r',
  'rx',
  'ry',
  'stopColor',
  'stopOpacity',
  'strokeColor',
  'vectorEffect',
  'x',
  'y',

  // `all` resets every property at once, which styleq cannot express: it
  // resolves overrides by deduping on the property key, so a class keyed `all`
  // would not suppress one keyed `color`.
  'all',
]);

/**
 * Keywords the grammar has that the types do not -- the mirror of
 * `KEPT_KEYWORDS`, which lists keywords the types have that the grammar does
 * not.
 *
 * Only closed unions are checked. Where a property accepts an arbitrary
 * string, every keyword is already accepted and completeness means nothing --
 * which is most shorthands and every colour property, and is why asserting
 * this over all properties would demand over ten thousand additions.
 *
 * Each of these is a value a user cannot currently write. They are listed
 * rather than fixed so that the check can land green; emptying the list is the
 * work it exists to prompt.
 */
const INCOMPLETE_UNIONS = {
  MozOsxFontSmoothing: ['auto'],
  WebkitBackgroundClip: ['border', 'content', 'padding'],
  WebkitFontSmoothing: ['auto', 'none', 'subpixel-antialiased'],
  alignItems: ['anchor-center'],
  alignSelf: ['anchor-center'],
  backgroundClip: ['border-area'],
  breakAfter: ['all', 'always'],
  breakBefore: ['all', 'always'],
  // StyleX has `-webkit-grab` and `-webkit-grabbing` but not the `-moz-`
  // equivalents or the zoom variants. `hand` is the pre-standard IE spelling.
  cursor: [
    '-moz-grab',
    '-moz-grabbing',
    '-moz-zoom-in',
    '-moz-zoom-out',
    '-webkit-zoom-in',
    '-webkit-zoom-out',
    'hand',
  ],
  display: [
    '-moz-box',
    '-moz-inline-box',
    '-moz-inline-stack',
    '-ms-grid',
    '-ms-inline-flexbox',
    '-ms-inline-grid',
    '-webkit-flex',
    '-webkit-inline-box',
    '-webkit-inline-flex',
    'flow',
    'table-caption',
  ],
  fontSizeAdjust: [
    'cap-height',
    'ch-width',
    'ex-height',
    'from-font',
    'ic-height',
    'ic-width',
  ],
  forcedColorAdjust: ['preserve-parent-color'],
  justifyItems: ['anchor-center'],
  justifySelf: ['anchor-center'],
  lineBreak: ['anywhere'],
  marginTrim: ['all', 'in-flow'],
  mixBlendMode: ['plus-darker', 'plus-lighter'],
  overflow: [
    '-moz-hidden-unscrollable',
    '-moz-scrollbars-horizontal',
    '-moz-scrollbars-none',
    '-moz-scrollbars-vertical',
    'overlay',
  ],
  overflowBlock: [
    '-moz-hidden-unscrollable',
    '-moz-scrollbars-horizontal',
    '-moz-scrollbars-none',
    '-moz-scrollbars-vertical',
    'overlay',
  ],
  overflowX: [
    '-moz-hidden-unscrollable',
    '-moz-scrollbars-horizontal',
    '-moz-scrollbars-none',
    '-moz-scrollbars-vertical',
    'overlay',
  ],
  overflowY: [
    '-moz-hidden-unscrollable',
    '-moz-scrollbars-horizontal',
    '-moz-scrollbars-none',
    '-moz-scrollbars-vertical',
    'overlay',
  ],
  pageBreakAfter: ['recto', 'verso'],
  pageBreakBefore: ['recto', 'verso'],
  position: ['-webkit-sticky'],
  // The full anchor-positioning grid. StyleX has a partial list.
  positionArea: [
    'end',
    'none',
    'self-block-end',
    'self-block-start',
    'self-end',
    'self-inline-end',
    'self-inline-start',
    'self-start',
    'span-all',
    'span-bottom',
    'span-end',
    'span-left',
    'span-right',
    'span-self-block-end',
    'span-self-block-start',
    'span-self-end',
    'span-self-inline-end',
    'span-self-inline-start',
    'span-self-start',
    'span-start',
    'span-top',
    'span-x-end',
    'span-x-self-end',
    'span-x-self-start',
    'span-x-start',
    'span-y-end',
    'span-y-self-end',
    'span-y-self-start',
    'span-y-start',
    'start',
    'x-end',
    'x-self-end',
    'x-self-start',
    'x-start',
    'y-end',
    'y-self-end',
    'y-self-start',
    'y-start',
  ],
  positionVisibility: ['anchors-valid'],
  resize: ['block', 'inline'],
  rubyPosition: ['alternate'],
  speak: ['always', 'never'],
  textTransform: ['full-size-kana', 'math-auto'],
  textWrap: ['auto'],
  transformBox: ['content-box', 'stroke-box'],
  unicodeBidi: [
    '-moz-isolate',
    '-moz-isolate-override',
    '-moz-plaintext',
    '-webkit-isolate',
    '-webkit-isolate-override',
    '-webkit-plaintext',
  ],
  whiteSpace: [
    'break-spaces',
    'collapse',
    'preserve',
    'preserve-breaks',
    'preserve-spaces',
    'wrap',
  ],
  wordBreak: ['auto-phrase'],
};

/**
 * Properties whose grammar admits non-keyword values but whose type does not
 * accept an arbitrary string.
 *
 * Each is a value a user cannot express. Most already accept a bare number,
 * which covers the common case but not `calc()`, `var()`, or any unit.
 */
const CLOSED_DESPITE_GRAMMAR = new Set([
  // `[ <url> [ <x> <y> ]? , ]*` -- custom cursor images cannot be written.
  // https://github.com/facebook/stylex/issues/1463
  'cursor',
  // `[ light | dark | <custom-ident> ]+` -- custom scheme names.
  'colorScheme',
  // `oblique <angle>` -- `oblique 14deg` cannot be written.
  'fontStyle',

  // These accept a bare number, so only `calc()`, `var()` and explicit units
  // are out of reach.
  'animationIterationCount',
  'fontSizeAdjust',
  'perspective',
  'zIndex',

  // Aural properties, all `<time>` or `<number>` based.
  'pause',
  'pauseAfter',
  'pauseBefore',
  'rest',
  'restAfter',
  'restBefore',
  'voiceBalance',
]);

/* ------------------------------------------------------------------ *
 * Tests
 * ------------------------------------------------------------------ */

describe('CSSProperties', () => {
  test('the TypeScript definitions match the Flow types', () => {
    // `StyleXCSSTypes.d.ts` exists only to swap `string` for `(string & {})`.
    // Normalised back, the two files must describe the same properties with
    // the same values -- otherwise one has been edited and the other has not.
    expect(readTypes(TS_FILE)).toEqual(readTypes(FLOW_FILE));
  });

  test('every property accepts the CSS-wide keywords', () => {
    // `initial`, `inherit` and `unset` are valid on every property and appear
    // in no grammar, so nothing else here can check them. Properties pick them
    // up from the `all` alias; one written `null | 'a' | 'b'` instead of
    // `all | 'a' | 'b'` silently rejects them.
    const problems = [];
    for (const [name, { literals }] of Object.entries(readTypes(FLOW_FILE))) {
      const missing = [...GLOBAL_KEYWORDS].filter((k) => !literals.includes(k));
      if (missing.length) {
        problems.push(
          `${name}: does not accept ${missing.map((k) => `'${k}'`).join(', ')}. ` +
            'Every property should compose the `all` alias.',
        );
      }
    }
    expect(problems).toEqual([]);
  });

  test('every keyword is real, according to the CSS grammar', () => {
    const types = readTypes(FLOW_FILE);
    const invalid = [];

    for (const [name, { literals }] of Object.entries(types)) {
      if (name === 'theme' || NO_GRAMMAR.has(name)) continue;

      const keywords = grammarKeywords(camelToKebab(name));
      if (keywords == null) {
        invalid.push(
          `${name}: css-tree has no grammar for this property. Either drop it ` +
            'from both type files, or add it to NO_GRAMMAR with a reason.',
        );
        continue;
      }

      const kept = KEPT_KEYWORDS[name] ?? [];
      const unknown = literals.filter(
        (l) =>
          !keywords.has(l) &&
          !GLOBAL_KEYWORDS.has(l) &&
          !kept.includes(l) &&
          // Space-separated values are combinations of single keywords.
          !l.split(' ').every((word) => keywords.has(word)),
      );

      if (unknown.length) {
        invalid.push(
          `${name}: ${unknown.map((l) => `'${l}'`).join(', ')} ` +
            'not in the CSS grammar. Either drop from both type files, or add ' +
            'to KEPT_KEYWORDS with a reason.',
        );
      }
    }

    expect(invalid).toEqual([]);
  });

  test('closed unions list every keyword the grammar has', () => {
    // The mirror of the grammar check above: that one asks whether every typed
    // keyword is real, this one asks whether every real keyword is typed.
    // Restricted to closed unions -- a property accepting arbitrary strings
    // already accepts everything.
    const problems = [];

    for (const [name, type] of Object.entries(readTypes(FLOW_FILE))) {
      if (name === 'theme' || NO_GRAMMAR.has(name) || type.open) continue;
      const keywords = grammarKeywords(camelToKebab(name));
      if (keywords == null) continue;

      // Multi-word values are typed whole ('first baseline'), so compare words.
      const words = new Set(type.literals.flatMap((l) => l.split(' ')));
      const known = INCOMPLETE_UNIONS[name] ?? [];
      const missing = [...keywords].filter(
        (k) => !words.has(k) && !known.includes(k),
      );
      if (missing.length) {
        problems.push(
          `${name}: the grammar allows ${missing
            .map((k) => `'${k}'`)
            .join(', ')} but the type does not. Either add to both type ` +
            'files, or list in INCOMPLETE_UNIONS.',
        );
      }

      const stale = known.filter((k) => words.has(k));
      if (stale.length) {
        problems.push(
          `${name}: ${stale.map((k) => `'${k}'`).join(', ')} now typed, so ` +
            'remove from its INCOMPLETE_UNIONS entry.',
        );
      }
    }

    const gone = Object.keys(INCOMPLETE_UNIONS).filter(
      (name) => !(name in readTypes(FLOW_FILE)),
    );
    for (const name of gone) {
      problems.push(
        `${name}: no longer a property, so remove it from INCOMPLETE_UNIONS.`,
      );
    }

    expect(problems).toEqual([]);
  });

  test('properties whose grammar needs arbitrary strings accept them', () => {
    // The other direction from the two keyword checks: not "are the literals
    // right?" but "should this be a union of literals at all?". A grammar with
    // a `<url>`, `<length>` or function in it cannot be expressed as literals.
    const problems = [];

    for (const [name, type] of Object.entries(readTypes(FLOW_FILE))) {
      if (name === 'theme' || NO_GRAMMAR.has(name) || type.open) continue;
      if (!needsArbitraryString(camelToKebab(name))) continue;
      if (CLOSED_DESPITE_GRAMMAR.has(name)) continue;
      problems.push(
        `${name}: its grammar is \`${generate(
          lexer.properties[camelToKebab(name)].syntax,
        )}\`, which admits more than keywords, but the type accepts no ` +
          'arbitrary string. Either add `string` in the Flow types and ' +
          '`(string & {})` in the definitions, or list it in ' +
          'CLOSED_DESPITE_GRAMMAR with a reason.',
      );
    }

    const stale = [...CLOSED_DESPITE_GRAMMAR].filter((name) => {
      const type = readTypes(FLOW_FILE)[name];
      return type == null || type.open;
    });
    for (const name of stale) {
      problems.push(
        `${name}: now accepts an arbitrary string, so remove it from CLOSED_DESPITE_GRAMMAR.`,
      );
    }

    expect(problems).toEqual([]);
  });

  test('every Baseline-available property is typed', () => {
    // Bumping `web-features` is what makes this fire: a property that reaches
    // Baseline has to be typed or listed, and one that is already typed has to
    // come off the list. The dependency is pinned exactly so that only a
    // deliberate bump can change the answer.
    const typed = new Set(Object.keys(readTypes(FLOW_FILE)).map(camelToKebab));
    const available = [...BASELINE.entries()].filter(
      ([cssName, baseline]) =>
        (baseline === 'high' || baseline === 'low') &&
        // Vendor-prefixed properties are exposed case by case, and
        // `custom-property` is a compat-data entry rather than a property.
        !cssName.startsWith('-') &&
        cssName !== 'custom-property',
    );

    const problems = [
      ...available
        .filter(
          ([cssName]) =>
            !typed.has(cssName) && !NOT_TYPED.has(kebabToCamel(cssName)),
        )
        .map(
          ([cssName, baseline]) =>
            `${kebabToCamel(cssName)}: Baseline ${baseline}, so it is safe to ` +
            `use, but it is not typed. Its grammar is \`${generate(
              lexer.properties[cssName].syntax,
            )}\`. Either add it to both type files, or list it in NOT_TYPED ` +
            'with a reason.',
        ),
      ...[...NOT_TYPED]
        .filter((name) => typed.has(camelToKebab(name)))
        .map((name) => `${name}: now typed, so remove it from NOT_TYPED.`),
    ];

    expect(problems).toEqual([]);
  });
});
