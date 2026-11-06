# `@stylexjs/swc-plugin`

SWC plugin package for StyleX.

Usage with `@swc/core`:

```js
const { transformSync } = require('@swc/core');

transformSync(source, {
  jsc: {
    experimental: {
      plugins: [[require.resolve('@stylexjs/swc-plugin'), {}]],
    },
  },
});
```

Build the wasm artifact locally with:

```bash
npm run build --workspace @stylexjs/swc-plugin
```

The generated wasm is written to `dist/` during the build and is not intended to
be committed to the repository.
