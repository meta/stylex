# @stylexjs/rust-plugin

A native Rust implementation of the StyleX compiler. It takes the same options as
`@stylexjs/babel-plugin` and produces the same output; the Babel plugin's
unchanged test suite runs against it. The compiler core is based on
[stylex-swc-plugin](https://github.com/Dwlad90/stylex-swc-plugin) by Vladislav
Buinovski (MIT); see `THIRD_PARTY_NOTICES.md`. It is vendored in `crates/` at
0.19.2 with the changes listed in `crates/PROVENANCE.md`.

The package is private: the native addon is built from source
(`npm run build:native`); no prebuilt binaries are published.

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

Function-valued options (`env` functions, custom module resolution,
`debugFilePath`) are called on the calling JavaScript thread.

When other Babel plugins must run first, use the Babel adapter in place of
`@stylexjs/babel-plugin`:

```js
plugins: [myPlugin, [require('@stylexjs/rust-plugin/babel'), stylexOptions]];
```

`@stylexjs/unplugin` uses this package when configured with `compiler: 'rust'`.

## Differences from the Babel plugin

- A shadowed `stylex` import or namespace parameter is left as authored.
- Styles that read function parameters, local types or `this` stay in their scope.
- Spread in array literals and in arguments to pure builtins evaluates.
- Optional member reads (`obj?.color`) evaluate.
- `==` and `!=` use JavaScript semantics.
- Functions defined in the file and called later evaluate; a read before its declaration is refused in every order.
- Property reads on arrow functions are refused.
- `stylex.when.ancestor(1)` reports an error instead of crashing.
- Theme variable groups joined or converted to text through array helpers (`[colors].map(g => g).join('')`) are refused; token reads through them work.
- `stylex.props`/`attrs` calls with nested conditional dynamic styles stay runtime calls.
- An inline object passed as `sx` compiles to its styles (the Babel plugin emits `style="[object Object]"`).
- `experimental_crossFileParsing` imports compile as `commonJS` ones do; the Babel plugin rejects every imported file in this mode.
- StyleX code another plugin adds during the traversal is compiled; the Babel plugin can leave it uncompiled.
- In the Babel adapter, a later plugin that rewrites a definition another depends on fails the build (copied, rather than moved, compiled nodes are not detected).

## Development

```bash
npm run build:native --workspace @stylexjs/rust-plugin   # release build, copies native/*.node
npm run test:native --workspace @stylexjs/rust-plugin    # Rust tests, native API, Babel suite
npm run test:parity --workspace @stylexjs/rust-plugin -- babel  # reference run of the Babel suite
npm run bench --workspace @stylexjs/rust-plugin          # whole-file timings against Babel
```

- `THIRD_PARTY_NOTICES.md`: licenses of the native addon's dependencies.
