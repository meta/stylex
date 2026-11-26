# StyleX Installation Guide

This document provides comprehensive installation instructions for StyleX.

## Overview

StyleX requires a build-time compiler to transform styles into optimized atomic CSS. The recommended approach depends on your build tool and framework.

## Quick start

### 1. Install dependencies

For most projects, install the core package and the appropriate plugin:

```bash
# Core runtime (always needed)
npm install @stylexjs/stylex

# For Vite, Rollup, Webpack, esbuild, or Rspack
npm install --save-dev @stylexjs/unplugin

# For Next.js
npm install --save-dev @stylexjs/babel-plugin @stylexjs/postcss-plugin
```

### 2. Configure your build tool

#### Vite

```ts
// vite.config.ts
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import stylex from '@stylexjs/unplugin';

export default defineConfig({
  plugins: [
    stylex.vite({
      useCSSLayers: true,
    }),
    react(),
  ],
});
```

Keep the StyleX plugin before `@vitejs/plugin-react` to preserve Fast Refresh.

#### Next.js

Create or modify the `babel.config.js`:

```js
// babel.config.js
const path = require('path');
const dev = process.env.NODE_ENV !== 'production';

module.exports = {
  presets: ['next/babel'],
  plugins: [
    [
      '@stylexjs/babel-plugin',
      {
        dev,
        runtimeInjection: false,
        enableInlinedConditionalMerge: true,
        treeshakeCompensation: true,
        aliases: { '@/*': [path.join(__dirname, '*')] },
        unstable_moduleResolution: { type: 'commonJS' },
      },
    ],
  ],
};
```

Create or modify the `postcss.config.js`:

```js
// postcss.config.js
const babelConfig = require('./babel.config');

module.exports = {
  plugins: {
    '@stylexjs/postcss-plugin': {
      include: [
        'src/**/*.{js,jsx,ts,tsx}',
        'app/**/*.{js,jsx,ts,tsx}',
        'pages/**/*.{js,jsx,ts,tsx}',
        'components/**/*.{js,jsx,ts,tsx}',
      ],
      babelConfig: {
        babelrc: false,
        parserOpts: { plugins: ['typescript', 'jsx'] },
        plugins: babelConfig.plugins,
      },
      useCSSLayers: true,
    },
    autoprefixer: {},
  },
};
```

Add the `@stylex` directive to your CSS file:

```css
/* app/globals.css */
@stylex;
```

#### Webpack

```js
// webpack.config.js
const stylex = require('@stylexjs/unplugin');

module.exports = {
  plugins: [
    stylex.webpack({
      useCSSLayers: true,
    }),
  ],
};
```

#### Rspack

```js
// rspack.config.js
const stylex = require('@stylexjs/unplugin');

module.exports = {
  plugins: [
    stylex.rspack({
      useCSSLayers: true,
    }),
  ],
};
```

#### esbuild

```js
// build.js
const esbuild = require('esbuild');
const stylex = require('@stylexjs/unplugin');

esbuild.build({
  entryPoints: ['src/index.tsx'],
  bundle: true,
  outdir: 'dist',
  plugins: [
    stylex.esbuild({
      useCSSLayers: true,
    }),
  ],
});
```

#### Rollup

```js
// rollup.config.js
import stylex from '@stylexjs/unplugin';

export default {
  plugins: [
    stylex.rollup({
      useCSSLayers: true,
    }),
  ],
};
```

### 3. Create a CSS entrypoint

Import a CSS file from your app root. During build, the StyleX plugin appends its aggregated output to that CSS file:

```css
/* src/index.css */
@stylex;
```

Import this CSS file in your app entry:

```tsx
import './index.css';
```

## Common configuration options

### Babel plugin options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `dev` | boolean | false | Enable development mode with readable class names |
| `runtimeInjection` | boolean | false | Inject styles at runtime (not recommended for production) |
| `treeshakeCompensation` | boolean | false | Prevent tree-shaking from removing styles |
| `aliases` | object | {} | Path aliases matching your bundler config |
| `unstable_moduleResolution` | object | undefined | Module resolution strategy for theming APIs |
| `classNamePrefix` | string | 'x' | Prefix for generated class names |
| `importSources` | array | ['@stylexjs/stylex'] | Custom import sources for StyleX |
| `styleResolution` | string | 'application-order' | Style merge strategy: 'application-order' (last style wins) or 'property-specificity' (more specific property wins) |

### Unplugin and PostCSS options

These options are available for unplugin (Vite, Webpack, Rspack, esbuild, Rollup) and the PostCSS plugin:

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `useCSSLayers` | boolean | false | Wrap output in `@layer` for better cascade control |
| `include` | string[] | ['**/*.{js,jsx,ts,tsx}'] | Files to process |
| `exclude` | string[] | ['node_modules/**'] | Files to exclude |

## TypeScript setup

StyleX packages include TypeScript definitions. No additional configuration is needed.

For strict typing of style props, use the exported types:

```tsx
import type { StyleXStyles } from '@stylexjs/stylex';

type Props = {
  style?: StyleXStyles<{
    color?: string;
    backgroundColor?: string;
  }>;
};
```

## ESLint setup

Install the ESLint plugin:

```bash
npm install --save-dev @stylexjs/eslint-plugin
```

Configure ESLint:

```js
// .eslintrc.js
module.exports = {
  plugins: ['@stylexjs'],
  rules: {
    '@stylexjs/valid-styles': 'error',
    '@stylexjs/no-unused': 'error',
    '@stylexjs/valid-shorthands': 'warn',
    '@stylexjs/sort-keys': 'warn',
  },
};
```

Or for flat config:

```js
// eslint.config.js
import stylexPlugin from '@stylexjs/eslint-plugin';

export default [
  {
    plugins: {
      '@stylexjs': stylexPlugin,
    },
    rules: {
      '@stylexjs/valid-styles': 'error',
      '@stylexjs/no-unused': 'error',
      '@stylexjs/valid-shorthands': 'warn',
      '@stylexjs/sort-keys': 'warn',
    },
  },
];
```

Available rules:
- `@stylexjs/valid-styles` - Validates style definitions
- `@stylexjs/no-unused` - Flags unused style definitions
- `@stylexjs/valid-shorthands` - Warns about shorthand property usage
- `@stylexjs/sort-keys` - Enforces sorted style keys
- `@stylexjs/enforce-extension` - Enforces `.stylex.js` extension for theme files

## CLI tool

StyleX provides a CLI for processing files outside of a bundler:

```bash
npm install --save-dev @stylexjs/cli

npx stylex --input ./src --output ./dist
```

## Troubleshooting

### Styles not appearing

1. Ensure the CSS file with `@stylex` is imported
2. Check that files are included in the `include` pattern
3. Verify the plugin runs before other transforms

### StyleX precedence

When adding StyleX to an app with existing CSS, use the `useCSSLayers` config to determine style precedence. For StyleX to override existing styles, `useCSSLayers: false`. Otherwise, use `useCSSLayers: true` for all other cases.

### Build performance

- Use `include`/`exclude` options to limit processed files
- Set `treeshakeCompensation: true` if styles are being removed

## More resources

- Official documentation: https://stylexjs.com
- Example projects: https://github.com/facebook/stylex/tree/main/examples
