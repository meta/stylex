# @stylexjs/rust-plugin

Native Rust implementation of the StyleX compiler. It accepts the same options
as `@stylexjs/babel-plugin` and is tested against it: the Babel plugin's unchanged
test suite runs against this compiler, and differential tests compare its output
with the Babel plugin's.

The package is private for now: the native binary is built from source for the
host platform (`npm run build:native`); prebuilt binaries are not published.

## Usage

```js
const { transformSync, processStylexRules } = require('@stylexjs/rust-plugin');

const { code, map, metadata } = transformSync(source, {
  filename: '/src/Button.js',
  sourceMaps: true,
  unstable_moduleResolution: { type: 'commonJS', rootDir: process.cwd() },
});
const css = processStylexRules(metadata.stylex, { useLayers: true });
```

Function-valued options (`env` functions, custom module resolution, `debugFilePath`)
are called back on the calling JavaScript thread.

When other Babel plugins must run first, use the Babel adapter in place of
`@stylexjs/babel-plugin`:

```js
plugins: [myPlugin, [require('@stylexjs/rust-plugin/babel'), stylexOptions]];
```

`@stylexjs/unplugin` uses this package when configured with `compiler: 'rust'`.

## Development

```bash
npm run build:native --workspace @stylexjs/rust-plugin   # cargo build --release, copies native/*.node
npm run test:native --workspace @stylexjs/rust-plugin    # Rust tests, native API, bundler, Babel suite
npm run test:parity --workspace @stylexjs/rust-plugin -- babel  # reference run of the same suite
npm run bench --workspace @stylexjs/rust-plugin          # whole-file timings against Babel
```

`vendor/swc_ecma_parser` is the published 38.0.2 crate with one visibility change
(`TokenFlags` re-exported) so the parser can enforce nesting limits; see
`vendor/README.md`.
