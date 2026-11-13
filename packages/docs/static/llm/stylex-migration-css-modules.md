# Migrating CSS Modules to StyleX

This self-contained playbook guides an AI agent migrating CSS Modules to StyleX.

Convert what is provably safe; label and report everything else accurately.

**A labeled refusal is a success. A silent visual change is a failure.**

---

## 0. How to use this document

1. Read this entire document before editing any file.
2. Work in two passes: inventory first (no edits), then convert.
3. Convert one component at a time. Never batch-edit across components.
4. Produce the report described in section 10 even if you convert nothing.

This document assumes StyleX v0.18+. For older versions, verify every API used
here against the installed version.

---

## 1. The contract

These rules are mandatory.

- **R1. Never invent a value.** Copy or derive every value from repository
  evidence. If you cannot see it, do not write it.
- **R2. Refuse rather than approximate.** If a CSS construct has no exact StyleX
  equivalent, do not emit the "closest thing". Label it (section 7).
- **R3. Styling-only diffs.** Do not refactor: no renaming, reordering,
  extraction, prop additions/removals/renames, component splits, unrelated
  cleanup (including no-op class lookups), or formatting sweeps. Only R16 may
  remove a proven-unreachable private-app styling passthrough.
- **R4. Never delete a source rule you did not convert.** Deleting unconverted
  CSS silently removes styling.
- **R5. Conserve declarations.** Every source declaration must be converted,
  refused, or explicitly dropped with a reason; counts must balance (section 8).
  Freeze a parser-backed source total before editing, never from the result or
  report prose.
- **R6. Do not combine arbitrary `className` or `style` values with
  `stylex.props()` on one element.** They do not merge predictably. R17 permits
  only a proven behavior-only class token, never another styling class or an
  arbitrary caller value.
- **R7. Preserve effective directionality.** Inspect the CSS pipeline for
  transforms such as `rtlcss`. Normally keep physical properties physical
  (`marginLeft` stays `marginLeft`). If the exact pinned transform makes authored
  physical CSS flow-relative, use the matching logical StyleX property and
  record evidence. If CSS Module output is transformed but the transform is
  unavailable, retain the contagious rule as `UNRESOLVED_VALUE` until equivalent
  StyleX post-processing or RTL output is proven. A finite direction value
  already in JavaScript may select an RTL namespace for non-property behavior
  such as icon rotation; never add or mutate a document marker merely to observe
  global direction.
- **R8. Do not convert a component whose class names cross a file boundary**
  until its consumers are handled. See section 6.9.
- **R9. Preserve cascade outcomes, not cascade mechanics.** Determine which
  declaration wins in the original CSS, then order StyleX arguments so the same
  one wins (section 6.3).
- **R10. Report only what you verified.** State the verification level you
  actually reached (section 9). Never write "verified" for a step you did not
  run.
- **R11. One component per change set.** Each converted component should be
  reviewable and revertable on its own.
- **R12. When two readings of the source are possible, refuse.** Ambiguity is a
  blocker, not a judgment call.
- **R13. Verify version-matched third-party prop forwarding before converting.**
  Evidence is exact-version source/types you actually opened, with its path and
  symbol recorded. A lockfile, declared entrypoint, task statement, search result,
  typecheck, or `className`-only example is locator evidence, not forwarding
  proof. Both `className` and `style` from `stylex.props()` must reach the same DOM
  element. Read the manifest and test every declared `exports`, `main`, `module`,
  and `types` path directly; a bounded listing cannot prove absence. Refuse when
  forwarding is disproven or remains unverified.
- **R14. Verify property support against pinned StyleX rules.** Before
  conversion, check every property, exact value, and conditional branch with the
  installed version's full ESLint/type reference. Finding the property key is
  insufficient: validate its accepted value union. Check standard and vendor
  spellings separately; compiler acceptance is insufficient. Refuse the whole
  contagious rule when exact value support remains uncertain.
- **R15. Honor user-supplied facts.** Treat a stated StyleX version, compiler
  capability, or module-resolution mode as pre-flight evidence and label it
  user-supplied. A supplied dependency-evidence path is only a location to open;
  it does not prove that the path exists or what it contains. Do not reject stated
  facts because visible config is omitted, and never invent missing evidence.
- **R16. Remove a private optional styling contract only with closed-world
  proof.** The component must be private application code: not package-exported,
  published, copied, generated, dynamically loaded, or reached through an
  unresolved alias. Resolve its full call graph; prove every live caller omits
  optional `className` and `style` and no unknown spread can supply them. Only
  then remove the dead passthrough. Public/incomplete graphs remain
  `EXTERNAL_CONSUMER`.
- **R17. Preserve behavior-only class tokens explicitly.** A literal token may
  coexist with StyleX only when repository-wide and exact-version dependency
  evidence proves it is used solely by JavaScript behavior—not CSS, a stylesheet,
  a styling library, or caller styling. Destructure one `stylex.props()` result,
  join only its `className` with the literal token, and pass its `style` unchanged
  to the same DOM element. Record the proving searches and consumer. This never
  covers a CSS class, arbitrary expression, or prop value.

---

## 2. Workflow

### Pass 1: inventory (no edits)

Produce a plan before touching anything.

1. Find every `*.module.css`, `*.module.scss`, `*.module.less` file.
2. Find each file's consumers. Resolve imports using the repository's real
   language/bundler order. If `../theme` resolves to `../theme.js`, it does not
   import nearby `theme.module.css`; basename similarity proves nothing.
3. For dependency-owned consumers, resolve the exact lockfile version, inspect
   version-matched types/source per R13, and report file, version, and evidence.
4. Freeze the parser-backed authored-declaration total for every scoped file.
5. Classify each file as **convertible**, **partial**, or **blocked** using the
   ledger in section 5.
6. Order the convertible files leaf-first (section 6.10).
7. Write the report skeleton (section 10) with every file listed and no
   conversions yet.

If asked only for a plan, stop and present it. Also stop when a scope of **10+
stylesheets** is over 20% blocked; it likely needs token bootstrap first
(section 4). Ignore this threshold for smaller scopes: when asked to migrate,
continue even if every file will be refused, so required markers are written.
An inventory-only plan neither edits files nor completes the migration report.

### Pass 2: convert

For each file in plan order:

1. Read the stylesheet and **all** of its consumers in full.
2. Convert (sections 4 to 6).
3. Run the verification ladder (section 9).
4. Update the report row for that file with real counts.
5. Continue only after the prior file verifies; otherwise fix or revert it.

---

## 3. Pre-flight

Before editing, establish and report these facts; never guess.

|Question|Evidence|
|---|---|
|Is StyleX installed and compiling?|`package.json` for `@stylexjs/stylex` plus StyleX build-plugin config|
|Version?|`package.json` / lockfile|
|JSX style prop?|Read `sxPropName`: default `sx`, configurable (for example `css`), or disabled with `false`; if unsure, use `stylex.props()`|
|Framework?|Non-React uses `stylex.attrs()`, not `stylex.props()`|
|Preprocessor?|`.module.scss` / `.module.less` invokes section 6.8|
|Token source?|`@value`, `:export`, custom properties, theme files|
|Layers/import order?|`@layer`, plugin `useCSSLayers`, global imports|
|Existing `.stylex.ts` tokens?|Reuse them; never duplicate the same tokens|
|Theming configured?|`defineVars`, `createTheme`, and markers require compiler module resolution; inspect its config|
|Supported properties/values?|Run all StyleX ESLint rules or inspect the installed ESLint/type reference; CSS standardization or compiler acceptance is not proof|

User-stated facts override missing local config. Record them as user-supplied;
do not refuse a safe rule merely because its manifest/plugin config is hidden.

---

## 4. Token bootstrap (do this first)

Bootstrap tokens in their own change set before components, avoiding duplicated
literals and later rework.

Map token sources as follows.

**CSS custom properties on `:root`**

```css
:root {
  --color-primary: #0055ff;
  --space-medium: 16px;
}

[data-theme='dark'] {
  --color-primary: #88aaff;
}
```

Token bootstrap is the sole exception to treating `:root` as global. First
inventory **all** definitions, overrides, and uses. Convert a `:root` block only
if it contains custom-property definitions exclusively and every runtime
override maps to a visible theme application site or the system-color path
below; otherwise refuse the whole block.

In a published library, theme, plugin, scaffold, or copied/swizzled tree, an
authored named `:root` property is a downstream CSS API unless explicitly
private. A hashed `defineVars` name breaks readers/overrides. Retain it as
`GLOBAL_SELECTOR`/`UNRESOLVED_VALUE` unless a versioned API migration handles
consumers; no local overrides does not permit renaming.

Put variable definitions in a `.stylex.ts` token file:

```ts
// tokens.stylex.ts
import * as stylex from '@stylexjs/stylex';

export const tokens = stylex.defineVars({
  colorPrimary: '#0055ff',
  spaceMedium: '16px',
});
```

Put `createTheme` in an **ordinary module**, never in the token file:

```ts
// themes.ts
import * as stylex from '@stylexjs/stylex';
import { tokens } from './tokens.stylex';

export const darkTheme = stylex.createTheme(tokens, {
  colorPrimary: '#88aaff',
  spaceMedium: '16px',
});
```

Apply the theme to the prior theme container. Keep its attribute/class when
behavior reads it. Use `defineVars`/`createTheme` only with compiler theming
module resolution.

**System color-scheme overrides**

`@media (prefers-color-scheme: light)` or
`@media (prefers-color-scheme: dark)` overrides of `:root` defaults may become
conditional `defineVars`; unlike attribute/class themes, the condition lives in
each variable value.

```css
:root {
  --text-primary: black;
  --background: white;
}

@media (prefers-color-scheme: dark) {
  :root {
    --text-primary: white;
    --background: black;
  }
}
```

```ts
// colors.stylex.ts
import * as stylex from '@stylexjs/stylex';

const DARK = '@media (prefers-color-scheme: dark)';

export const colors = stylex.defineVars({
  textPrimary: { default: 'black', [DARK]: 'white' },
  background: { default: 'white', [DARK]: 'black' },
});
```

Use this only for custom-property-only blocks after inventorying every default,
media override, and use. Preserve each query exactly; never invent a default.
Retain the family if the pinned version cannot represent its default, or if you
cannot prove original cascade/precedence against explicit themes, inline values,
or nested scopes. Use `GLOBAL_SELECTOR` or `UNRESOLVED_VALUE`. Media selection
does not waive the public CSS API rule above.

**`@value` declarations**

```css
@value primary: #0055ff;
@value spacingMedium: 16px;
```

If the value never changes at runtime, use `defineConsts`. If it is themed or
overridden, use `defineVars`. Follow the same file-placement rules above.

**`:export` blocks** become `defineConsts` or `defineVars` according to theming;
update JS imports to the token file.

**Media query breakpoints** become `defineConsts` entries so every component
references the same string.

**IMPORTANT constraints on token files:**

- Must be named `*.stylex.ts` or `*.stylex.js`
- Named exports only, no default export
- May export only values returned by `defineVars`, `defineConsts`, or
  `defineMarker` (subject to the project's `enforce-extension` options)
- `createTheme`, `stylex.create`, and `stylex.keyframes` belong in ordinary
  modules, not token files
- Import them directly. Re-exporting through a barrel/index file breaks
  compile-time resolution

Report token path/count and each `@value` or property not classifiable as const
vs var; retain and label it `UNRESOLVED_VALUE`.

**Component-scoped custom properties**

For custom properties defined on a local class and read only by owned
descendants: put defaults in token-file `defineVars`, authored local values in
ordinary-module `createTheme`, apply the theme to the same container, and replace
`var(--name)` reads with its token. This preserves inheritance without raw
unsupported `'--name'` keys. Require complete definition/override/consumer proof;
otherwise retain the family as `UNRESOLVED_VALUE`.

**Private-app global-state value bridge**

An app owning its loaded global stylesheet may bridge a finite global-state
selector by copying authored values into named custom properties on that
selector for StyleX. Another system may write the state, but exact-version source
must prove every value, state, placement, and scope.

This bridge is not a new truth: copy values exactly and override every finite
state at every possible writer scope. If `data-theme` may occur on arbitrary
subtrees, light/dark requires `:root` defaults plus **both**
`[data-theme='light']` and `[data-theme='dark']` rules, so light resets inside
dark ancestors. Keep the global CSS loaded and limit changes to the private app.
Never use this for a library, theme, plugin, scaffold, copied/swizzled output,
or downstream CSS API. Incomplete source, states, placement, or scope means
`GLOBAL_SELECTOR`/`UNRESOLVED_VALUE`.

---

## 5. Expressibility ledger

Classify every rule in a stylesheet against this table before converting it.

### Direct equivalents

|CSS Modules|StyleX|
|---|---|
|`.foo { color: red }`|`stylex.create()` namespace|
|Multiple declarations|Multiple namespace properties|
|`.foo:hover`, `:focus`, `:active`, `:disabled`, `:focus-visible`|Condition nested in property value|
|`.foo::before`, `::after`, `::placeholder`, `::selection`|Top-level namespace key|
|`@media`, `@supports`, `@container`|Condition nested in property value|
|`@keyframes` + `animation-name`|`stylex.keyframes()` as `animationName`|
|Custom-property definitions|Token-file `defineVars()`|
|`@value`|`defineConsts()` or `defineVars()`|
|Multiple fallback values|`stylex.firstThatWorks()`|

### Expressible with restructuring (allowed, with preconditions)

|CSS Modules|StyleX|Precondition|
|---|---|---|
|`.card h2 { ... }`|Style every matching `<h2>` directly|All matches and render branches are visible in edited files; none comes through `children`, render props, or third-party DOM|
|`.card > .icon { ... }`|Style child directly|Same|
|`.card:hover .icon { ... }`|Icon: `stylex.when.ancestor(':hover')`; card: `stylex.defaultMarker()`|Observed ancestor and styled element are controlled|
|`.input:focus + .label { ... }`|`stylex.when.siblingBefore(':focus')` + marker|Same|
|`.row[data-open='true'] .cell { ... }`|`stylex.when.ancestor('[data-open="true"]')` + marker|Same|
|`.title, .subtitle { ... }`|Shared/equivalent namespace at every use|Every branch/use is convertible; count each authored declaration once|

Structural-descendant locality:

- Refuse targets from `children`, render props, or third-party output.
- A target behind a visible local conditional is convertible only when every
  matching branch is visible and styled.
- A local `.map()` target is convertible when its element and all matching
  branches are visible; style that element directly.
- Style every local match; refuse unbounded/non-enumerable sets.

Directly applying `.card h2` preserves current rendering but not automatic
styling of future `<h2>` children. Report each structural rewrite so future
matches receive the namespace; this note alone does not require refusal.

For same-element attributes, nest the selector in the value:
`.trigger[data-state='open']` becomes `[data-state="open"]` plus `default`;
handle `aria-*` likewise. For ancestor attributes, use
`stylex.when.ancestor(...)` on the descendant and a marker in the ancestor's
`stylex.props()`. Remove a CSS class used only to identify that ancestor after
the marker replaces it.

`stylex.when.*` observes **state** (pseudo/attribute) on a marked element, not
pure structure. `.card h2` must be styled on the child or refused.

`stylex.when.descendant`, `anySibling`, and `siblingAfter` compile to `:has()`.
For projects supporting browsers without it, refuse and note why.

### Refuse

|CSS Modules|Reason|
|---|---|
|`:global(...)` / `:global { ... }`|`GLOBAL_SELECTOR`|
|`html`, `body`, `:root`, `*`, or bare tags (except section 4 bootstrap)|`GLOBAL_SELECTOR`|
|Third-party-rendered DOM target|`GLOBAL_SELECTOR`|
|Non-local structural target|`STRUCTURAL_SELECTOR`|
|`+`, `~`, `.stack > * + *` spacing|`STRUCTURAL_SELECTOR`|
|Structural `:nth-child`, `:first-child`, `:last-child`, `:not(...)`|`STRUCTURAL_SELECTOR`|
|`!important`|`IMPORTANT`|
|`composes` from `global` or unreadable file|`COMPOSES_UNRESOLVED`|
|Preprocessor mixin/function/`@extend`/loop|`UNRESOLVED_VALUE`|
|`@font-face`, `@page`, `@property`, `@import`|`UNSUPPORTED_AT_RULE`|
|`@layer` without StyleX layer config/global order|`UNSUPPORTED_AT_RULE` / `CASCADE_ORDER`|
|Vendor-only property without standard equivalent|`UNSUPPORTED_PROPERTY`|
|Dynamic access (`styles[name]`)|`DYNAMIC_CLASS`|
|Class on unowned component or crossing file boundary|`EXTERNAL_CONSUMER`|
|Conflicts without determinable static order|`CASCADE_ORDER`|

A refusal is contagious: any class named in a refused rule's selector must keep
being applied, so it cannot be converted either. See 6.2.1.

---

## 6. Conversion rules

### 6.1 Declarations

- Expand multi-value shorthands. `padding: 8px 16px` becomes
  `paddingTop: 8`, `paddingRight: 16`, `paddingBottom: 8`, `paddingLeft: 16`.
  `border: 1px solid red` becomes `borderWidth`, `borderStyle`, `borderColor`.
  Single-value shorthands (`padding: 8px`) may stay as `padding: 8`.
- Before expanding shorthand containing fallback-free `var(--name)`, prove it
  resolves in every applicable state. An unresolved variable invalidates the
  **whole shorthand**; splitting may wrongly leave longhands active. If its
  definition or every runtime assignment is unproven, retain the contagious
  family as `UNRESOLVED_VALUE`.
- Convert property names to camelCase.
- Check every camel-cased property/value against the pinned version's complete
  `valid-styles` and type reference. One unsupported declaration refuses its
  whole rule and contagious family. Check standard/vendor spellings separately:
  `WebkitLineClamp`/`WebkitBoxOrient` support proves nothing about `lineClamp`,
  or vice versa. Name the failing property in `UNSUPPORTED_PROPERTY`.
- Inspect the final validator, not one table entry: CSS-wide keywords (`inherit`,
  `initial`, `unset`, `revert`) may be added after table construction. A local
  `showError(...)` does not prove `font: inherit` is rejected; run the rule or
  follow final composition.
- Sort each namespace with configured `@stylexjs/sort-keys`, else its default.
  At L1, autofix then rerun. If unavailable, do not guess priorities: mark order
  unverified and not release-ready until the rule fixes and rechecks it.
- Preserve string values verbatim except: bare `px` may become a number
  (`16px` → `16`), and authored unitless numbers must be numeric when the pinned
  checker requires it (`line-height: 1.2` → `lineHeight: 1.2`). Never change units.
- `content` values keep their quotes: `content: ''` becomes `content: '""'`.
- Keep colors, `calc()`, gradients, and `transform` strings exactly as authored.
- Drop only with recorded unreachability proof. A rule fully overridden later
  in-file qualifies. An unused rule qualifies only in a private app after
  repository-wide searches exclude static/dynamic use, exports, generated or
  template copies, package output, and runtime loaders. In libraries, published
  packages, themes, plugins, scaffolds, or copied/swizzled trees, absence of a
  local import is insufficient: retain/refuse. Apply consistently and count each declaration
  _dropped_ with evidence. Decide reachability before support: dead private-app
  rules are dropped even with unsupported properties. In monorepos classify the
  target subtree, tracing its entrypoints, exports, file globs, copy steps, and
  loaders; an unrelated public package does not make a private app public.

### 6.2 Selector classification

Parse CSS Modules scope before classification. Treat `:local(.foo)` as `.foo`
while preserving its pseudo-classes/elements, lists, and relationships. Never
blind-replace or unwrap `:global()`.

Examples:

- `:local(.button):hover` is classified like `.button:hover`.
- `:local(.title), :local(.subtitle)` is a selector list whose branches must
  each be independently convertible.
- `:local(.card .title)` remains a structural descendant selector.
- `:local(.card) :global(.tooltip)` still contains a global target and is
  refused as `GLOBAL_SELECTOR`.

For each rule, stop at the first match:

1. Selector contains `:global` or targets `html`/`body`/`:root`/`*`/a bare tag →
   refuse `GLOBAL_SELECTOR`, except for a fully classified section 4
   custom-property bootstrap or private-app global-state value bridge.
2. Declaration block contains `!important` → refuse `IMPORTANT`.
3. Selector is a single class (`.foo`), optionally with a supported pseudo-class
   or pseudo-element → **convert**.
4. Selector is a class plus a state on the same element (`.foo[data-x='y']`,
   `.foo:hover`) → **convert** as a nested condition.
5. Relationship with **state on the left** and a local right side → **convert**
   with `stylex.when.*` + marker.
6. Pure structure with all targets local → **convert** on each target. If a
   boolean/finite variant for a class-keyed ancestor is already in JavaScript,
   select a target namespace (for example `as === 'h1' && styles.titleH1`), not
   `stylex.when.*`.
7. Global direction/finite state whose exact value is already in component JS →
   **convert** by selecting a namespace; do not add markers to unowned document
   nodes such as `html`/`body`.
8. Selector is a comma-separated list and every branch is independently
   convertible → **convert** every branch without multiplying ledger counts.
9. Anything else → refuse `STRUCTURAL_SELECTOR`.

If any selector-list branch is refused, refuse the whole rule; splitting may
change cascade/specificity. A preparatory source split requires prior proof of
identical selector and cascade outcomes.

### 6.2.1 Dependency check (run before converting any class)

A refused rule works only while its selector classes remain applied. Therefore:

**Before converting `.x`, find every rule mentioning it. If any is refused,
`.x` cannot be converted.**

```css
.card {
  padding: 16px;
} /* looks convertible */
.card :global(.tooltip) {
  z-index: 10;
} /* refused: GLOBAL_SELECTOR */
```

Converting `.card` would silently disable the tooltip rule. Keep both rules and
the class; do not migrate that element. Label it with the blocking code.

Migrate per element: siblings may use different systems; R6 forbids mixing only
on the _same_ element.

**Refuse whole rules, not declarations.** One blocked declaration retains and
counts the whole rule; extracting safe declarations would mix CSS Modules and
`stylex.props()` on one element. Apply contagion transitively to every named class.

### 6.3 Multiple classes on one element

In CSS Modules, `cx(styles.a, styles.b)` does **not** pick the winner; specificity
then source order do. In StyleX, the last argument wins.

Procedure:

1. List the properties set by more than one applied class.
2. For each, determine the CSS winner (higher specificity first, then later
   source position).
3. Order the `stylex.props()` arguments so the winner is last.
4. If the classes are applied conditionally such that no single static order
   reproduces the original for all combinations, refuse `CASCADE_ORDER`.

```css
/* Button.module.css — .primary appears later, so it wins */
.base {
  background: gray;
  padding: 8px;
}
.primary {
  background: blue;
}
```

```tsx
// Correct: primary last
<button {...stylex.props(styles.base, isPrimary && styles.primary)} />
```

### 6.4 `composes`

- `composes: other;` (same file) → include both namespaces in the
  `stylex.props()` call, composed namespace first.
- `composes: other from './other.module.css';` → convert it first, import and
  place its namespace first. Shared namespaces live in ordinary modules (for
  example `surface.styles.ts`), **not** token-only `.stylex.ts` files.
- `composes: x from global;` → refuse `COMPOSES_UNRESOLVED`.
- If the composed file is not yet converted, refuse this rule for now rather
  than mixing a raw class name into a StyleX element (R6).

### 6.5 Values coming from JavaScript

CSS Modules code often passes runtime values through inline custom properties:

```tsx
<div className={styles.bar} style={{ '--bar-width': `${pct}%` }} />
```

```css
.bar {
  width: var(--bar-width);
}
```

Convert to a dynamic style function:

```tsx
const styles = stylex.create({
  bar: (width: string) => ({ width }),
});

<div {...stylex.props(styles.bar(`${pct}%`))} />;
```

If multiple stylesheets read it or a non-local component sets it, refuse
`EXTERNAL_CONSUMER`.

### 6.6 Media queries, `@supports`, `@container`

Nest these inside property values, never atop a namespace. `default` is required;
use `null` when the default applies nothing.

```css
.card {
  padding: 8px;
}
@media (min-width: 768px) {
  .card {
    padding: 16px;
  }
}
```

```ts
const styles = stylex.create({
  card: {
    padding: { default: 8, '@media (min-width: 768px)': 16 },
  },
});
```

Put breakpoints repeated across files in bootstrap `defineConsts`.

Count source, not output: the example has **two converted declarations**, though
they merge into one nested property.

### 6.6.1 Cascade layers

Never unwrap `@layer`. Treat a rule as layered only when authored that way or
the active checked-in build demonstrably wraps it—not because of an optional
plugin, future flag, generated mode, or package-wide feature. Record optional
modes as unverified L2/L3 states. For actual layers, read `useCSSLayers` and
global layer order; convert only if StyleX preserves priority against every
competitor. Missing mapping means retain/refuse `UNSUPPORTED_AT_RULE`; unknown
priority means `CASCADE_ORDER`. Layered conversions require L3 competitor checks.

### 6.7 Keyframes and animation

```css
@keyframes fade {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}
.toast {
  animation: fade 0.3s ease-out;
}
```

```ts
const fade = stylex.keyframes({ from: { opacity: 0 }, to: { opacity: 1 } });

const styles = stylex.create({
  toast: {
    animationName: fade,
    animationDuration: '0.3s',
    animationTimingFunction: 'ease-out',
  },
});
```

Expand `animation` to longhands. If value order is ambiguous, refuse
`UNSUPPORTED_PROPERTY`; never guess duration vs delay.

### 6.8 Preprocessor files (`.module.scss`, `.module.less`)

- `&:hover`, `&:focus`, `&[data-x]` → nested conditions, same as plain CSS.
- `& .child`, `&__element` (BEM nesting) → apply section 6.2 to the
  **flattened** selector. Flatten first, classify second.
- `$variables` / `@variables` with literal values → resolve by reading the
  definition, then treat as a token (section 4).
- Mixins (`@include`)/functions, `@extend`, `@each`, `@for` →
  `UNRESOLVED_VALUE`; never compute `darken()`, `lighten()`, or `math.div()`.
- Treat `@apply` and other generated styling as contagious with every declaration
  in their rule. Convert only from CSS compiled with the project's exact
  dependency version and configuration. Configuration values, framework
  conventions, or remembered utility behavior are not compiled-CSS evidence.
  Without exact output, retain the whole rule as
  `UNRESOLVED_VALUE — preprocessor:`.
- When available, convert from exact compiled module CSS; it is ground truth.

### 6.9 Use sites (the plumbing)

Resolve workspace aliases/local exports to checked-in source before calling a
component third-party. An owned component imported as `@theme/Heading`,
`@docusaurus/Link`, or another alias is still local when its implementation is
in the checkout. Inspect every live render branch: all must forward `className`
and `style` from one props object to one DOM element. Enter external dependencies
only on branches that cross that boundary. Use concrete call-site props to
eliminate dead branches and report the proof (for example, an `id`-guarded class
cannot block a call that omits `id`).

Update every consumer:

|Pattern|Action|
|---|---|
|`className={styles.foo}`|`{...stylex.props(styles.foo)}`|
|`className={cx(styles.a, styles.b)}`|One `stylex.props()` ordered per 6.3; remove unused `cx`/`clsx`/`classnames` import|
|`className={cx(styles.a, condition && styles.b)}`|`stylex.props(styles.a, condition && styles.b)`|
|`className={cx(styles.a, props.className)}`|`EXTERNAL_CONSUMER` (R6)|
|`className={cx(styles.a, 'behavior-hook')}`|Only R17: join returned StyleX class with proven literal and forward returned style|
|`className={styles[variant]}`|`DYNAMIC_CLASS`, unless replaced in-file by an explicit static map|
|`<ThirdPartyThing className={styles.foo} />`|Exact-version proof that both `className` and `style` reach one DOM node; else `EXTERNAL_CONSUMER`|
|Class exported/passed to another file|`EXTERNAL_CONSUMER`|
|Element also has `style={{...}}`|Fold static values into namespace or use dynamic function; never retain both|

An owned component's optional `className`/`style` remains external unless R16
closed-world proof succeeds. Remove it/its merge only after enumerating callers
and excluding exports, copy/generation, dynamic loads, and unknown spreads.
Never apply R16 to reusable/public output.

For R17, do not add a literal class beside a JSX spread. Make the exceptional
merge visible and auditable:

```tsx
const {className, style} = stylex.props(styles.frame);
return <iframe className={`${className} rr-block`} style={style} />;
```

The literal requires a proven behavior consumer and no styling role; CSS classes,
expressions, and caller values remain forbidden.

Before deleting a multi-consumer stylesheet, update **all** consumers. Share one
named `stylex.create()` export from an ordinary module such as `Badge.styles.ts`;
never duplicate it or place it in token-only `.stylex.ts`.

Third-party evidence order: installed source, installed declarations, then
official repository/docs at the exact lockfile version. Current `main`, another
version, a `className`-only example, or a passing consumer typecheck proves
nothing; evidence must show the full `stylex.props()` result reaches one DOM
element. Read installed `package.json`, then resolve/test every relevant
`exports`, `main`, `module`, and `types` path directly. Limited `find`/`tree`/`ls`/
glob output cannot prove absence; record direct checks of missing declared paths.
If primary evidence is unavailable, retain as `EXTERNAL_CONSUMER` and report
package, version, and lookup. A task's claim that evidence is mounted does not
replace opening it. Inspect supplied offline exact-version evidence; network is
unnecessary when pinned manifest/runtime paths exist in `node_modules`.

Delete a stylesheet only after all consumers and rules convert. Otherwise keep
unconverted rules/TODOs, their import, and their unmigrated elements.

A zero-declaration/comment-only tombstone is not deletion. Remove the converted
file and verify absence; if deletion is impossible, report it and do not claim
completion.

If only a global side effect (`@font-face`, `@property`, permitted `@import`)
remains, keep an explicit import such as `import './Fonts.module.css';`; L2 must
confirm emission.

### 6.10 Ordering

Convert leaf-first. Rank candidate files by:

1. No `:global`, no `!important`, no `@font-face` (fully convertible)
2. Class names never leave the component's own files
3. Single consumer
4. Fewest structural selectors
5. No `composes` from unconverted files

Start with the highest rank. Shared primitives and parent `className` consumers
go last.

---

## 7. Refusal protocol

On refusal, preserve behavior and mark the exact site.

**Format** (one line, no wrapping):

```
TODO(stylex-migrate): <CODE> — <what blocked it> | <where> | <suggested fix>
```

In retained CSS, comment above the rule; in JS/TS, above the element/namespace.
Keep JSX valid: use `{/* ... */}` inside parents; for a one-node arrow return,
put `//` before the callback or wrap comment+node in a fragment. Never replace a
callback's sole node with a bare JSX comment expression.

**Mark every Pass-2 refusal, including whole-file refusal.** Even at zero
conversion, mark each blocking rule (or its single containing at-rule) and any
blocked use site. Reports do not replace source markers. Inventory-only planning
is exempt because it makes no edits.

Trace the full propagation graph. Mark the CSS rule and every export, return,
forward, or caller supply of its class/blocking `className`/`style` contract.
Do not stop at the importer or edit basename-similar non-dependencies.

```css
/* TODO(stylex-migrate): GLOBAL_SELECTOR — :global(.tooltip) targets DOM from react-tooltip | .card :global(.tooltip) | keep in CSS Modules as an escape hatch */
.card :global(.tooltip) {
  z-index: 10;
}
```

```tsx
// TODO(stylex-migrate): EXTERNAL_CONSUMER — merges props.className with local styles | <Card> root | migrate consumers first, then convert
<div className={cx(styles.card, props.className)} />
```

**Reason codes** (use these exact strings, no others):

|Code|Meaning|
|---|---|
|`GLOBAL_SELECTOR`|Global/root/tag/third-party DOM selector|
|`STRUCTURAL_SELECTOR`|Unexpressible or non-local relationship|
|`EXTERNAL_CONSUMER`|Class crosses component/file boundary|
|`DYNAMIC_CLASS`|Computed class key|
|`UNRESOLVED_VALUE`|Repository evidence cannot resolve a construct/value|
|`COMPOSES_UNRESOLVED`|`composes` from global/unavailable/unconverted source|
|`IMPORTANT`|`!important`|
|`CASCADE_ORDER`|Conflict without deterministic static order|
|`UNSUPPORTED_PROPERTY`|StyleX rejects property/value|
|`UNSUPPORTED_AT_RULE`|No StyleX at-rule equivalent|

Never invent codes; use the closest and explain `<what blocked it>`.

Start each `UNRESOLVED_VALUE` explanation with a stable subreason:

|Subreason|Use when unproven|
|---|---|
|`preprocessor:`|`@apply`/mixin/function/loop/other generated construct|
|`custom-property-resolution:`|Custom-property definition/assignment/fallback/scope|
|`direction-transform:`|Effective RTL transform/equivalent StyleX output|
|`token-classification:`|Whether `@value`/export/token is constant or overridable|

Example:

```css
/* TODO(stylex-migrate): UNRESOLVED_VALUE — custom-property-resolution: --border-base has no proven definition in every state | .card border shorthand | define the variable or add a fallback before converting */
```

---

## 8. Declaration conservation

Prevent silent loss with per-file declaration counts.

One **declaration** is one authored, source-located `property: value` before
shorthand expansion. Count it once despite expansion, conditional merging,
selector-list duplication, or cross-consumer sharing. `composes`, `@value`, and
selectors are directives: note but never count them.

Before editing, freeze an AST/parser-backed total in the report; without a parser,
freeze one enumeration of source locations. Never recount from output. Assign
each frozen location one status and reject any final source-total mismatch.

For every stylesheet:

```
total = converted + refused + dropped
```

- `converted` — represented by an applied namespace on the same element(s)
- `refused` — still in CSS, with a TODO marker; if one declaration blocks a
  rule, every declaration in that rule counts as refused
- `dropped` — removed as provably unreachable, with one-line reason each

If it does not balance, find the loss before continuing; report all three counts.

Example: this source has five declarations, not four—two base declarations, one
hover declaration, one media declaration, and one media-plus-hover declaration:

```css
.nav {
  padding: 8px;
  color: black;
}
.nav:hover {
  color: blue;
}
@media (min-width: 768px) {
  .nav {
    padding: 16px;
  }
  .nav:hover {
    color: green;
  }
}
```

Though StyleX merges them into two properties, the ledger is
`5 total = 5 converted + 0 refused + 0 dropped`.

---

## 9. Verification ladder

Run as far as possible and record the level. **Never claim a higher level.**

**L0 — Self-check (always, requires no tooling)**

- Declaration counts balance (section 8)
- No arbitrary `className`/`style` shares an element with `stylex.props()`; R17
  uses explicit destructuring and recorded proof
- Every converted class is removed from the stylesheet; every refused class is
  still there and still applied
- No refused rule names a class you converted (6.2.1)
- No remaining import of a deleted stylesheet
- Every TODO marker uses a valid code and names a real location
- Edited JS/TS/JSX/TSX is syntactically valid. Without tooling, re-read each
  expression for balanced delimiters/valid JSX comments; unchecked edits fail L0.
- Snapshot changed files before commands; direct caches/incremental metadata
  outside the repo where supported. Compare afterward, remove only command-made
  artifacts (for example `.tsbuildinfo`), and record cleanup.
- Final diff is styling-only (R3); report scope matches changed files

**L1 — Static analysis**

- Typecheck (`tsc --noEmit` or the project's equivalent)
- ESLint with these `@stylexjs/eslint-plugin` rules:
  `@stylexjs/valid-styles`, `@stylexjs/valid-shorthands` (autofixes shorthand
  expansion), `@stylexjs/no-legacy-contextual-styles`,
  `@stylexjs/no-conflicting-props`, `@stylexjs/no-unused`,
  `@stylexjs/no-lookahead-selectors` (flags `:has()`-dependent selectors),
  `@stylexjs/no-nonstandard-styles`, `@stylexjs/sort-keys`, and
  `@stylexjs/enforce-extension` (catches invalid or mixed token-file exports)

Autofix `valid-shorthands`/`sort-keys`, then rerun all rules. Correctness errors
block conversion; `sort-keys` warnings block release readiness. A candidate with
any correctness error is not safety-qualified, even if its report reached L0 or
the compiler accepted it.

Compiler success does not replace ESLint; it may transform a `.stylex.ts` file
that `enforce-extension` correctly rejects.

**L2 — Build**

- The project builds and CSS is emitted

**L3 — Behavior**

- Unit/integration tests pass
- Visual check: screenshot or computed-style comparison of the component before
  and after, in each state the stylesheet targeted (default, hover, focus,
  disabled, each breakpoint)

Only L3 catches wrong cascade/missed state. Without it, report every unverified
state plainly.

---

## 10. Report

Write `stylex-migration-report.md` at the requested location or repository root;
regenerate after each file and include verified facts only.

```markdown
# CSS Modules to StyleX migration report

## Environment

- StyleX version:
- Framework / build tool:
- Preprocessor:
- Token file:
- Highest verification level reached:

## Summary

|Metric|Value|
|---|---|
|Stylesheets found||
|Fully converted||
|Partially converted||
|Blocked (no conversion)||
|Source declarations||
|Declarations converted||
|Gross conversion yield||
|Safety-qualified declarations||
|Safety-qualified yield||
|Declarations refused||
|Declarations dropped||

## Files

|File|Status|Total|Converted|Refused|Dropped|Codes|Verified|
|---|---|---|---|---|---|---|---|
|src/Card/Card.module.css|partial|17|14|3|0|GLOBAL_SELECTOR, IMPORTANT|L1|

## Dependency and generated-CSS evidence

|Subject|Version/configuration|Inspected path and symbol|What it proves|
|---|---|---|---|

## Blockers by code and subreason

|Code|Subreason|Occurrences|Files|Suggested unblock|
|---|---|---|---|---|

## Structural rewrites

List each structural selector that was replaced by direct StyleX application,
the elements updated today, and the requirement to style future matching
elements explicitly.

## Unverified states

List every component state (hover, breakpoint, theme) that was converted but not
visually checked.

## Follow-ups

Ordered list of what a human should do next, highest leverage first.
```

`Declarations converted` is the gross conservation count. A converted
declaration is safety-qualified only when its candidate passes the StyleX
compiler and full correctness lint with no errors, balances its ledger, and has
exact evidence for generated CSS and third-party forwarding where applicable.
If a required gate is unavailable, report zero safety-qualified declarations for
that candidate; do not relabel its gross conversions as refused. Report each
yield as its declaration count divided by source declarations.

The blocker table ranks fixes by files unlocked without merging unrelated
`UNRESOLVED_VALUE` problems.

### 10.1 Production validation and rollout

Before production rollout:

1. Gate CI on types, full L1 StyleX ESLint, production build, and emission of
   both StyleX and intentionally retained CSS.
2. Compare computed styles for every affected node/pseudo-element across default,
   hover, focus, focus-visible, disabled, authored `data-*`/`aria-*`, themes,
   breakpoints, LTR, and RTL; screenshots are secondary.
3. For partial work, test layer/import-order competition and retained side
   effects such as fonts. Ship small revertable changes with monitoring/rollback.

---

## 11. Worked example

**Before — `Card.module.css`**

```css
@value spacingMd: 16px;

.card {
  composes: surface from './surface.module.css';
  padding: spacingMd;
  border: 1px solid #ddd;
  border-radius: 8px;
}

.card:hover {
  border-color: #999;
}

.card:hover .icon {
  opacity: 1;
}

.icon {
  opacity: 0.5;
}

.title {
  font-size: 18px;
  font-weight: 600;
}

.featured {
  background-color: #fffdf5;
}

.tooltipHost :global(.tooltip) {
  z-index: 10 !important;
}
```

**Before — `Card.tsx`**

```tsx
import cx from 'classnames';
import styles from './Card.module.css';

export function Card({ title, isFeatured, children }) {
  return (
    <div className={cx(styles.card, isFeatured && styles.featured)}>
      <span className={styles.icon} />
      <h2 className={styles.title}>{title}</h2>
      <div className={styles.tooltipHost}>{children}</div>
    </div>
  );
}
```

**After — `tokens.stylex.ts`** (token bootstrap)

```ts
import * as stylex from '@stylexjs/stylex';

export const spacing = stylex.defineConsts({
  md: '16px',
});
```

**After — `Card.tsx`**

```tsx
import * as stylex from '@stylexjs/stylex';
import legacy from './Card.module.css';
import { surface } from './surface.styles';
import { spacing } from './tokens.stylex';

const styles = stylex.create({
  card: {
    padding: spacing.md,
    borderWidth: 1,
    borderStyle: 'solid',
    borderColor: { default: '#ddd', ':hover': '#999' },
    borderRadius: 8,
  },
  featured: {
    backgroundColor: '#fffdf5',
  },
  icon: {
    opacity: { default: 0.5, [stylex.when.ancestor(':hover')]: 1 },
  },
  title: {
    fontSize: 18,
    fontWeight: 600,
  },
});

export function Card({ title, isFeatured, children }) {
  return (
    <div
      {...stylex.props(
        surface,
        styles.card,
        isFeatured && styles.featured,
        stylex.defaultMarker(),
      )}
    >
      <span {...stylex.props(styles.icon)} />
      <h2 {...stylex.props(styles.title)}>{title}</h2>
      {/* TODO(stylex-migrate): GLOBAL_SELECTOR — .tooltipHost is kept in CSS Modules because a refused rule depends on it | Card.module.css .tooltipHost :global(.tooltip) | remove the global tooltip override upstream, then migrate this element */}
      <div className={legacy.tooltipHost}>{children}</div>
    </div>
  );
}
```

**After — `Card.module.css`** (what remains)

```css
/* TODO(stylex-migrate): GLOBAL_SELECTOR — targets third-party DOM from the tooltip library | .tooltipHost :global(.tooltip) | requires a global stylesheet or a library-level fix */
/* TODO(stylex-migrate): IMPORTANT — z-index uses !important | .tooltipHost :global(.tooltip) | remove !important upstream, then re-evaluate */
.tooltipHost :global(.tooltip) {
  z-index: 10 !important;
}
```

Outcomes:

- `composes` resolved to an ordinary-module namespace placed **first**.
- `.card:hover` folded into a nested condition on `borderColor`.
- `.card:hover .icon` used `stylex.when.ancestor(':hover')` plus a card marker: left-side
  state, local icon.
- `border` expanded into three longhands.
- `spacingMd` became a token, not an inlined `16px`.
- `.featured` has no property conflict, so argument order suffices. If it set
  `border-color`, `.card:hover` must still win: add the matching `:hover`
  condition or refuse `CASCADE_ORDER`.
- `.tooltipHost` stayed because a refused rule names it (6.2.1); only that
  element remains on CSS Modules.
- The stylesheet/import stayed (`legacy` signals mixed state); remove both when
  no module class remains.

**Report row:**
`src/Card/Card.module.css | partial | 10 | 9 | 1 | 0 | GLOBAL_SELECTOR, IMPORTANT | L1`

---

## 12. Per-file checklist

Before marking any file done:

- [ ] Stylesheet and all consumers read in full
- [ ] Extensionless imports resolved; no basename-only assumptions
- [ ] Parser-backed source total frozen before edits
- [ ] Every property/value checked against pinned StyleX rules/types
- [ ] Exact values and every conditional branch match accepted value unions
- [ ] Final validator composition checked for CSS-wide keyword handling
- [ ] Installed manifest entrypoints checked by exact path, not bounded listing
- [ ] Every dependency claim cites exact-version content actually opened
- [ ] Every `@apply`/generated rule uses exact compiled CSS or remains refused
- [ ] `:local()`/`:global()` parsed before selector classification
- [ ] Every rule classified with section 6.2
- [ ] Dependency check run: no converted class is named by a refused rule
      (6.2.1)
- [ ] Shorthands expanded
- [ ] Multi-class elements ordered by CSS winner (6.3)
- [ ] Tokens referenced, not inlined
- [ ] Every refusal labeled with a valid code and left working
- [ ] Every `UNRESOLVED_VALUE` marker starts with a standard subreason
- [ ] Declaration counts balance
- [ ] Dropped unused rules have private-app reachability proof; public/copied output retained
- [ ] Monorepo visibility uses target exports/copy paths, not repository-wide status
- [ ] Token/theme files pass `@stylexjs/enforce-extension`
- [ ] Retained global CSS still has an active side-effect or binding import
- [ ] No arbitrary `className`/`style` with `stylex.props()`; R17 merges have
      repository-wide, exact-version proof
- [ ] R16 removals prove all callers, exports, copies, dynamic loads, and spreads
- [ ] Physical→logical changes cite exact RTL transform; JS direction uses an
      existing finite value
- [ ] Private-app global bridge copies every value/state/scope/reset and keeps
      global CSS loaded
- [ ] Color-scheme tokens preserve defaults, media overrides, explicit-theme
      precedence, and public custom-property contracts
- [ ] Every structural direct-application rewrite is listed in the report
- [ ] Verification ladder run; level recorded
- [ ] Edited JS/TS/JSX/TSX is syntactically valid, including refusal comments
- [ ] Verification left no cache, incremental, or generated artifacts
- [ ] `valid-styles` clean; `sort-keys` fixed/rechecked before release
- [ ] Gross and safety-qualified conversion yields reported separately
- [ ] Report row updated with real numbers

---

## 13. Runtime setup

This Markdown works with any agent. Put it where the tool reads instructions:

- **Any agent:** attach/paste it with the migration files.
- **Claude Code:** save it in-repo; add to `CLAUDE.md`: "For CSS Modules to
  StyleX migrations, follow `<path>/stylex-migration-css-modules.md`."
- **Codex / AGENTS.md tools:** save it in-repo; add to `AGENTS.md`: "For CSS
  Modules to StyleX migrations, follow `<path>/stylex-migration-css-modules.md`."
- **Cursor:** save as `.cursor/rules/stylex-migration-css-modules.mdc` and add
  `globs` if you want it auto-attached for `*.module.css` files.
- **Other tools:** reference it from their instructions or attach per session.

---

## 14. StyleX reference

Common migration rules follow. Full API: adjacent `stylex-authoring.md` or
https://stylexjs.com/docs/api.

- Define with `stylex.create()`; apply with React `stylex.props()` or
  Solid/Svelte/Qwik/Vue `stylex.attrs()`.
- With compiler support, `<div sx={styles.root} />` is shorthand for spreading
  `stylex.props()` on a lowercase DOM element.
- Later arguments win: `stylex.props(base, override)`.
- `null` unsets a property; `false`/`undefined` arguments are ignored.
- Numbers are pixels.
- Prefer longhands; multi-value shorthands are not supported.
- Conditions (pseudos, media, `when.*`) nest **inside** property values with
  `default`, never atop a namespace.
- Pseudo-elements **are** top-level keys of a namespace.
- `stylex.defineVars()`/`stylex.defineConsts()`/`stylex.defineMarker()`: named,
  direct-imported exports from `*.stylex.ts`.
  `stylex.createTheme()`/`stylex.create()`/`stylex.keyframes()`: ordinary modules.
- `stylex.when.*` observes pseudo/attribute state on a
  `stylex.defaultMarker()` or custom `stylex.defineMarker()` element.
- Do not put arbitrary `className` or `style` on an element that receives
  `stylex.props()`; R17's proven behavior-only literal is the only exception.
