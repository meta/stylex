# @stylexjs/oxc-plugin

A prototype StyleX compiler written in Rust on [oxc](https://oxc.rs). It ships as a Node addon
with a `transformSync` API and a drop-in replacement for `@stylexjs/babel-plugin`. It is
private and not published.

## Status

- Passes 1037 of 1037 tests that the Babel plugin passes in its own Jest suite, with the plugin
  swapped for this one. Golden fixtures pass 8 of 8.
- Agrees with the Babel plugin on 100% of 30,000 fuzzed programs over two seeds.
- Eight example apps (Vite, Vite React, Vite RSC, webpack, Next.js, esbuild, Rspack, React
  Router) build byte-identical CSS and JS bundles.
- About 72x faster than the Babel plugin per file (geometric mean over 40 benchmark and example
  app files). The median file takes 117 µs. The SWC port in #2008 takes 878 µs and Babel takes
  5.4 ms.
- `yarn workspace @stylexjs/oxc-plugin build:pgo` makes an opt-in profile-guided build. It is
  about 17% faster with byte-identical output (cold-cache median 167 µs against 200 µs plain).
- Large create-heavy files are still 1.15 to 1.37x slower than the fastest native StyleX
  compiler we compared with, in the plain build.
- Loading the addon plus the first call takes about 5 ms, against about 2.5 ms for that
  compiler.

## Build and test

You need Node and a Rust toolchain (1.97 or later, which oxc requires). From the repo root,
after `yarn install`:

```sh
yarn workspace @stylexjs/babel-plugin build   # the reference the tests compare against
yarn workspace @stylexjs/oxc-plugin build     # cargo build --release, copies js/stylex_oxc.node
yarn workspace @stylexjs/oxc-plugin test
```

`test` runs cargo test, the golden fixtures, the Flow fixtures and fuzzers, the js tests, a
fuzz run against the built Babel plugin, and the Babel plugin's Jest suite run twice. One run
uses the real plugin and one uses `js/babel.js`, and every test must end the same way. The root
`build` and `test:packages` scripts skip this package, because it needs Rust.

Use it from Node or from a Babel config:

```js
const { transformSync, processStylexRules } = require('@stylexjs/oxc-plugin');
const { code, metadata } = transformSync(source, { filename, ...stylexOptions });
// or in babel.config.js, in place of '@stylexjs/babel-plugin', with the same options
plugins: [['@stylexjs/oxc-plugin/babel', stylexOptions]];
```

`yarn workspace @stylexjs/oxc-plugin fuzz --count 5000` runs the fuzzer. `perf` compares speed
with the Babel plugin. `build:pgo` needs `rustup component add llvm-tools` and a built Babel
plugin. It trains on the Babel plugin's test inputs and the Flow fixtures, not on the files the
speed numbers come from.

## Flow

Flow files are detected like Babel's Flow plugin does it. Their types are blanked with Flow's
own Rust parser, and `component`, `hook`, `match` and records are lowered the way
hermes-parser does. See [FLOW.md](FLOW.md). The parser comes from facebook/flow as a git
dependency, because it is not on crates.io yet. Flow's parser still uses a nightly feature line,
so `scripts/build.cjs` sets `RUSTC_BOOTSTRAP=flow_parser` for that one crate.

## Layout

- `crates/core` is the parser-independent StyleX core.
- `crates/stylex_oxc` is the oxc front end and the napi addon.
- `crates/flow_blank` is Flow type blanking and lowering.
- `js` holds the Node API and the Babel plugin.
- `tests` and `tools` hold the golden runner, the Flow corpora, the fuzzer and the perf script.

The code and the notes call the compiler oxs.

## Provenance

`crates/core` is carved from the SWC port in #2008, with the SWC parts removed. Those crates
come from [Dwlad90/stylex-swc-plugin](https://github.com/Dwlad90/stylex-swc-plugin) under the
MIT license. `crates/core/LICENSE`, `NOTICE.md` and `PROVENANCE.md` keep the license, the
third-party notices and the upstream tree ids. The rest was written for this package, with
the Babel plugin as the behavioral spec. It does not use code from any other oxc-based StyleX
compiler.

## Known differences

Output matches the Babel plugin except where [DIFFERENCES.md](DIFFERENCES.md) says otherwise.
Through `transformSync` the output is spliced into the source, so formatting can differ.
Through `js/babel.js` Babel prints it.
