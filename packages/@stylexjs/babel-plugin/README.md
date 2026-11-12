# @stylexjs/babel-plugin

StyleX expects you to transform all `js`/`ts`/`tsx` files with `@stylexjs/babel-plugin`.
In addition to transforming JS code, this plugin also produces an Array of CSS rules. All the CSS rules
generated from all JS files within your project should be concatenated together and converted to a CSS
file using the `processStyles` function which is also exported from the same module.

`@stylexjs/babel-plugin` is fairly lightweight. It pre-computes `stylex` related functions like
`create` and `keyframes` by converting the argument AST to a JS object and transforming them
by passing them to the functions of the corresponding names within `@stylex/shared`


## Babel Metadata

The StyleX Babel plugin does more than transform JavaScript (or TypeScript) files. It also returns a list of injected styles. The way that such a value can be returned while transforming a JS file is by using Babel's `metadata` API.

e.g.
```tsx
const result = transformSync(sourceCode, {
  filename: opts.filename,
  parserOpts: { flow: { all: true } },
  plugins: [stylexPlugin, opts],
});

const transformedCode = result.code;
const injectedStyles = result.metadata.stylex;
```

## processStylexRules

`processStylexRules` is exported from `@stylexjs/babel-plugin` and takes an array of CSS rules collected during babel transformation to generate the final CSS output. This API is useful for building custom bundler integrations.

### Usage

```ts
import stylexBabelPlugin from '@stylexjs/babel-plugin';

// Rules collected from babel transformation metadata
const rules = [
  // ... metadata.stylex from babel transform results
];

const css = stylexBabelPlugin.processStylexRules(rules, {
  useLayers: true,
});
```

### Function Signature

```ts
type Rule = [
  string,                                    // className hash
  { ltr: string; rtl?: null | string },      // CSS rule object
  number                                     // priority
];

function processStylexRules(
  rules: Array<Rule>,
  config?: boolean | {
    useLayers?: boolean;
    legacyDisableLayers?: boolean;
  }
): string;
```

### How it works

When StyleX compiles your JavaScript files, the babel plugin returns metadata
containing an array of CSS rule objects. You collect these rules from all files in
your project, then pass them to `processStylexRules` which:

1. De-duplicates the rules
2. Sorts them by priority
3. Resolves `stylex.defineConsts` references
4. Generates the final CSS string

### Configuration options

#### `useLayers`

```ts
useLayers: boolean; // Default: false
```

When `true`, uses CSS `@layer` to group rules by priority level. This provides
native browser support for style priority ordering without specificity hacks.

Example output with `useLayers: true`:
```css
@layer priority1, priority2, priority3;

@layer priority2 {
  .margin-xymmreb { margin: 10px 20px }
}

@layer priority3 {
  .backgroundColor-xrkmrrc { background-color: red }
}
```

#### `legacyDisableLayers`

```ts
legacyDisableLayers: boolean; // Default: false
```

When `true`, disables the specificity polyfill (`:not(#\#)` selectors) that
StyleX uses when `useLayers` is `false`. Only use this if you have a custom
solution for handling style priority.

### Example: Custom Bundler Integration

```ts
import { transformAsync } from '@babel/core';
import stylexBabelPlugin from '@stylexjs/babel-plugin';

// Store rules from all files
const allRules = [];

// Transform each file
async function transformFile(code, filename) {
  const result = await transformAsync(code, {
    filename,
    plugins: [
      [stylexBabelPlugin, { /* your options */ }]
    ],
  });

  // Collect rules from metadata
  if (result.metadata?.stylex?.length > 0) {
    allRules.push(...result.metadata.stylex);
  }

  return result.code;
}

// After all files are processed, generate CSS
function generateCSS() {
  return stylexBabelPlugin.processStylexRules(allRules, {
    useLayers: true,
  });
}
```
