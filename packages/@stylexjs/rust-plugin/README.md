# @stylexjs/rust-plugin

Native Rust implementation of the StyleX compiler. It accepts the same options
as `@stylexjs/babel-plugin` and is tested against it: the Babel plugin's unchanged
test suite runs against this compiler, and differential tests compare its output
with the Babel plugin's.

The compiler core in `crates/` is the
[stylex-swc-plugin](https://github.com/Dwlad90/stylex-swc-plugin) core (MIT),
vendored at 0.19.2 with StyleX's changes on top (see `crates/PROVENANCE.md`).
This package adds the source-preserving frontend, the Node host bridge and the
Babel adapter.

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

## Differences from the Babel plugin

The output matches `@stylexjs/babel-plugin`. It differs on purpose here:

- A shadowed `stylex` import or namespace parameter is left as authored.
- Styles that read function parameters, local types or `this` stay in their scope.
- Spread in array literals and in arguments to pure builtins evaluates.
- Optional member reads (`obj?.color`) evaluate.
- `==` and `!=` use JavaScript semantics.
- Functions defined in the file and called later evaluate; a read before its declaration is refused in every order.
- Property reads on arrow functions are refused.
- `stylex.when.ancestor(1)` reports an error instead of crashing.
- Theme variable groups joined or converted to text through array helpers, such as `[colors].map(g => g).join('')`, are refused; reading tokens off them, such as `[colors].map(g => g.primary)`, and direct reads such as `colors.primary` work as usual.
- `stylex.props`/`attrs` calls with nested conditional dynamic styles stay runtime calls.
- An inline object passed as `sx` compiles to its styles (the Babel plugin emits `style="[object Object]"`).
- `experimental_crossFileParsing` imports compile as `commonJS` ones do; the Babel plugin currently rejects every imported file in this mode.
- StyleX code another plugin adds during the traversal is compiled; the Babel plugin can leave it uncompiled.
- In the Babel adapter, a later plugin that rewrites a definition another depends on fails the build (a plugin that copies, rather than moves, compiled nodes is not detected).

## Development

```bash
npm run build:native --workspace @stylexjs/rust-plugin   # cargo build --release, copies native/*.node
npm run test:native --workspace @stylexjs/rust-plugin    # Rust tests, native API, bundler, Babel suite
npm run test:parity --workspace @stylexjs/rust-plugin -- babel  # reference run of the same suite
npm run bench --workspace @stylexjs/rust-plugin          # whole-file timings against Babel
```

`vendor/swc_ecma_parser` is the published 45.1.6 crate with three bounded
patches (see `vendor/README.md`). Third-party notices for the native binary are
in `THIRD_PARTY_NOTICES.md`.
