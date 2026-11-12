# @stylexjs/eslint-plugin

The ESLint rule is a standalone ESLint plugin that mostly maintains an
`allowlist` for valid styles and their valid values.

## Installation

```sh
npm install --save-dev @stylexjs/eslint-plugin
```

## Enable Flow Types

If you need to import this package in an environment where you need the Flow
type definitions shipped with this package to work, please add the following
options to your `.flowconfig` file:

```
module.system.node.resolve_dirname=flow_modules
module.system.node.resolve_dirname=node_modules
```

This will fix the missing types for `eslint` and `estree`.

## Enabling the plugin and rules

Once you've installed the npm package you can enable the plugin and rules by
opening your ESLint configuration file and adding the plugin and rules.

```json
{
  "rules": {
    "@stylexjs/valid-styles": "error",
    "@stylexjs/sort-keys": "warn"
  },
  "plugins": ["@stylexjs"]
}
```

## All the rules

### `@stylexjs/valid-styles`

StyleX requires styles that are statically analyzable. This rule will detect
invalid styles that StyleX cannot handle and provides basic type checking for
style values.

### `@stylexjs/sort-keys`

This rule helps to sort the StyleX property keys according to
[property priorities](https://github.com/facebook/stylex/blob/main/packages/%40stylexjs/shared/src/utils/property-priorities.js).

#### Config options

```js
{
  order: 'default',                            // Sort order: 'default', 'clean', or 'recess'
  minKeys: 2,                                  // Minimum number of keys before sorting is enforced
  allowLineSeparatedGroups: false,             // Treat blank-line-separated blocks as independent groups
  enableMediaQueryOrder: false,                // Preserve authored `@media` order to match StyleX's `enableMediaQueryOrder`
  validImports: ['stylex', '@stylexjs/stylex']
}
```

When you compile with StyleX's `enableMediaQueryOrder`, overlapping media
queries are resolved by source order (the last matching query wins). Sorting
`@media` keys alphabetically inverts that cascade, so set
`enableMediaQueryOrder: true` here to keep the rule from reordering media
queries and let you control the cascade through their authored order.

### `@stylexjs/valid-shorthands`

This ESLint rule enforces the use of individual longhand CSS properties in place
of multivalue shorthands when using `create` for reasons of consistency
and performance. The rule provides an autofix to replace the shorthand with the
equivalent longhand properties.

#### Disallowed: `margin`, `padding` with multiple values

Using multivalue shorthands that StyleX cannot safely split into equivalent
longhands:

- `margin: '8px 16px'`
- `padding: '8px 16px 8px 16px'`

**Fix:** Replace with equivalent longhands. Note: this is autofixable.

##### Disallowed: `font`

**Why:** `font` is a shorthand that overrides multiple font settings at once.

**Fix:** Replace with individual font properties. Note: this is autofixable.

- `fontSize`
- `fontFamily`
- `fontStyle`
- `fontWeight`

##### Disallowed: `border`, `borderTop`, `borderRight`, `borderBottom`, `borderLeft`

**Fix:** Replace with individual sub-properties. Note: this is autofixable.

- `borderWidth`
- `borderStyle`
- `borderColor`

#### Config options

This rule has a few custom config options that can be set.

```js
{
  allowImportant: false,                       // Whether `!important` is allowed
  preferInline: false,                         // Whether the expansion uses logical direction properties over physical
  validImports: ['stylex', '@stylexjs/stylex']
}
```

### `@stylexjs/enforce-extension`

This rule ensures consistent naming for StyleX theme files that export variables
using `defineVars` or `defineConsts`.

#### Not allowed

- Exporting `defineVars` or `defineConsts` in a file **not** ending in `.stylex.js` or
  `.stylex.ts`
- Using the `.stylex.js` / `.stylex.ts` extension without exporting
  `defineVars` or `defineConsts`
- Mixing `defineVars` or `defineConsts` with other exports in the same file (unless `legacyAllowMixedExports` is enabled)
- Mixing `defineConsts` with `defineVars` when `enforceDefineConstsExtension` is enabled

#### Instead...

- Use `.stylex.js` or `.stylex.ts` for files that only export
  `defineVars` or `defineConsts`
- Export **only** theme vars/consts from these files
- When `enforceDefineConstsExtension` is enabled, use `.stylex.const.js` for `defineConsts` exports

#### Config options

```json
{
  "themeFileExtension": ".stylex", // default, can be customized
  "legacyAllowMixedExports": false, // allow mixed exports (legacy support)
  "enforceDefineConstsExtension": false, // enforce separate .const suffix for defineConsts
  "validImports": ["stylex", "@stylexjs/stylex"]
}
```

### `@stylexjs/no-unused`

This rule flags unused styles created with `create(...)`. If a style key
is defined but never used, the rule auto-strips them from the create call.

#### Config options

```json
{
  "validImports": ["stylex", "@stylexjs/stylex"]
}
```

### `@stylexjs/no-legacy-contextual-styles`

This rule flags usages of the original media query/pseudo class syntax that
wraps multiple property values within a single @-rule or pseudo class like this:

#### Config options

```json
{
  "validImports": ["stylex", "@stylexjs/stylex"]
}
```

```js
const styles = stylex.create({
  root: {
    width: '100%',
    '@media (min-width: 600px)': {
      width: '50%',
    },
  },
});
```

This syntax is deprecated and should be replaced with the new syntax specified
[here](https://stylexjs.com/docs/learn/styling-ui/defining-styles/#media-queries-and-other--rules)

```js
const styles = stylex.create({
  root: {
    width: {
      default: '100%',
      '@media (min-width: 600px)': '50%',
    },
  },
});
```

### `@stylexjs/no-lookahead-selectors`

This rule warns against usage of `stylex.when.anySibling`, `stylex.when.descendant`,
and `stylex.when.siblingAfter` due to their reliance on the CSS `:has()`
selector, which does not yet have widespread browser support.

#### Limited browser support

```js
const styles = stylex.create({
  foo: {
    backgroundColor: {
      default: 'blue',
      [stylex.when.anySibling('.sibling')]: 'red',
    },
    color: {
      default: 'black',
      [stylex.when.descendant('.child')]: 'purple',
    },
    marginTop: {
      default: '0px',
      [stylex.when.siblingAfter('.next')]: '8px',
    },
  },
});
```

See [caniuse.com/css-has](https://caniuse.com/css-has) for current browser
compatibility.

#### Config options

```json
{
  "validImports": ["stylex", "@stylexjs/stylex"]
}
```

### `@stylexjs/no-nonstandard-styles`

This rule enforces that you create standard CSS values and properties for StyleX.
It validates that all CSS properties and values conform to standard CSS
specifications.

#### Features

- Detects invalid CSS property names and suggests standard alternatives
- Validates CSS values against property specifications
- Provides auto-fixes for common issues (e.g., non-standard `float: start` → `float: inline-start`)
- Supports StyleX-specific functions like `stylex.keyframes()` and `stylex.defineVars()`

#### Config options

```json
{
  "validImports": ["stylex", "@stylexjs/stylex"]
}
```

### `@stylexjs/no-conflicting-props`

This rule disallows using `className` or `style` props on elements that spread
`stylex.props()` to avoid conflicts and unexpected behavior.

#### Invalid examples

```jsx
<div {...stylex.props(styles.foo)} className="extra" />

<div {...stylex.props(styles.foo)} style={{ color: 'red' }} />
```

#### Config options

```json
{
  "validImports": ["stylex", "@stylexjs/stylex"]
}
```
