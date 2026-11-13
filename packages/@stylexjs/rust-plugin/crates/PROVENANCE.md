# Vendored StyleX core crates

The StyleX compiler core from
[stylex-swc-plugin](https://github.com/Dwlad90/stylex-swc-plugin) (MIT; see
`LICENSE` and `NOTICE.md`), vendored at tag `0.19.2`, commit `6e7d3983`:

- repository archive (`git archive 6e7d3983`) sha256
  `8836aea57dfd4cee147df404753883a760422e628dc15df243fe89bb0dc5e1a5`
- 21-crate core subset archive sha256
  `f6d2d895a6360dff661926bc558a1d9035710c095386331725de164646d6a8e3`

Per-crate upstream git tree ids:

```
aa40421ccfb22fc832656110fb3d16bdaa6fbcf9 crates/postcss-value-parser
0193f86f7703fa8e7b0d0a24410034aa8add38a1 crates/stylex-ast
c719579e6281eaead18392dcac364c014392a6b2 crates/stylex-atoms
90d22a248e84d817874e9c319fd744e31850ad99 crates/stylex-constants
f8ff06c6579341dd8243b6c4e0d73aadeabf5556 crates/stylex-css-parser
da98d690ee160b4297286f2662f8a00e15b3ff00 crates/stylex-css
159239cec918c0d64336143f31159026771cf7bb crates/stylex-diagnostics
674005e71b6a7964c6ccc6d9a66e43ac93d60ae5 crates/stylex-enums
e15b924b54733f6f87e50e430a93bee340e8d22f crates/stylex-evaluator
3b16cb4df9f8794d7aab204c72028a6384a5b38a crates/stylex-js
48b9c59d722e81753f9cb6bb5e1babf3f581cc35 crates/stylex-macros
0d779b8461cc9ec24f130e74d105f6b43242349d crates/stylex-nested-config
6fd9b84cfb11de10af7c8e3562badd545903f192 crates/stylex-path-resolver
787c23cc19d566921080fe012e577dc5751832c2 crates/stylex-regex
6c33d2a9cdfc7bf81c53a92f7d71930cce45dbd9 crates/stylex-state-index
3f653b6ce199ecdf8dbcc28430af8bcf0101f42a crates/stylex-state
a722f63724ca83ada81c288a5c780d845ca98647 crates/stylex-structures
fb7cf2d616969dd5d1f4f49fc89af090c4ffffb5 crates/stylex-styleq
83fcfeb07d0f1aaaf01c6054594719a629878ef1 crates/stylex-transform
67fd65603219b1c419170752838c4aea241c4bb0 crates/stylex-types
4db9a5ed34b82852736c8a123a1ac23f8026dc93 crates/stylex-utils
```

## Vendoring

- Copied: each crate's `src/`, `Cargo.toml` and license files, and
  `stylex-path-resolver/fixtures` byte-identical to `0.19.2` (codeload archive
  sha256 `cf4f84b6a32cfcdf4657cc0d9b831c7577cf401f15f05db9624822e3be02affa`;
  `../.gitignore` re-includes its `node_modules`, `dist` and `lib`).
- Left out: upstream benches, integration-test directories, docs and JS tooling,
  and the `[[bench]]`/`[[test]]` tables naming them. The `tests/` directories in
  `stylex-css`, `stylex-css-parser` and `stylex-evaluator` are ours.
- Removed: the `stylex-css-parser` value-type and property parsers (colours,
  transforms, shapes, shadows and the like) and their tests. The compiler calls
  only its media-query parser, which needs `length`, `calc` and `common_types`.
- Workspace package, dependency and lint tables are copied from upstream into
  `../Cargo.toml`; `rust-version` is 1.96 (upstream 1.97).
- `rustfmt.toml` carries upstream's stable formatting options;
  `stylex-state/package.json` keeps the package name upstream's tests read.
- `scripts/test.cjs` runs the unit tests under `cargo test` with a 16 MiB
  `RUST_MIN_STACK` (upstream uses nextest); CI runs them at the default stack.
- Comments in the vendored crates were condensed; code is unchanged (verified
  token-for-token).

## Behaviour changes

Each keeps the Babel plugin's output and diagnostics where upstream differed.

Evaluator (`stylex-evaluator`, `stylex-js`)
- The Boa engine is removed; a native evaluator folds what the Babel plugin
  folds, with JavaScript semantics for host-provided sparse arrays, `==`/`!=`,
  spread in hole-free arrays and pure-builtin arguments, optional member reads,
  later-called local functions and reference equality.
- Source-literal holes refuse at their first offending element with Babel's
  unlocated fault (member calls keep Babel's CallExpression refusal); a hole
  Babel never reaches, inside an approved spread or call operand, may be named
  instead of Babel's spread or call refusal.
- Reads before declaration, builtin method values, arrow-function property
  reads and optional calls are refused.
- Allocation, length and recursion budgets refuse with typed reasons.
- Results reach the transform through one fallible boundary
  (`materialize_result`).
- Results are memoized per evaluation, keyed on the base context only;
  host reads and calls are never cached.
- Known limitation: a Rust caller that writes bindings through `StateManager`
  or `FunctionMap` directly, or shares a budget across state owners, can read a
  stale memoized value; `transformSync` and the Babel adapter cannot.

Host boundary (`stylex-structures`, `stylex-state`)
- `env` values and callbacks are live host values, read and called in Babel's
  order; their failures keep their cause.
- Host-run callback effects are journaled per evaluation scope and replayed in
  order.
- `ImportResolver` resolves theme imports, names files and rewrites aliased
  imports; `DebugFileNamer` serves `debugFilePath`.

Module resolution and paths (`stylex-state`, `stylex-path-resolver`)
- All options default as in the Babel plugin
  (`src/utils/transform/core_options.rs`); no `unstable_moduleResolution`
  means no resolution.
- `commonJS` resolves relative imports natively and asks the host otherwise; a
  manifest without `name` gives `undefined:`.
- `experimental_crossFileParsing` resolves as `commonJS` (upstream refused it;
  `cross_file_parsing_is_refused` became `cross_file_parsing_resolves_as_common_js`).
- A file outside any package resolves against its own directory; package
  lookups start at the resolution root.
- Missing imports are left unresolved instead of panicking.

Transform and output (`stylex-transform`, `stylex-state`, `stylex-css`)
- Rule metadata is reported per `add_style` call, in order, duplicates kept.
- Injected imports and helper names follow Babel's merge and naming order.
- Call and JSX-spread sites are distinguished by span.
- Writes through known aliases and callbacks mark an object as mutated.
- Compiled objects keep required `null` keys and Babel's key order; unused reset
  keys are pruned in dev as in prod.
- `defineVars`, `createTheme` and the nested forms read their arguments as data.
- With `test: true`, `createTheme` emits only the test label class.
- Debug labels (`$$css`, `data-style-src`) are `path.relative` from the root or
  package with the platform separator, and the compiled file's line; upstream's
  `/`-joined labels, last-two-segment fallback and input-source-map path (and
  their unit tests) are replaced.

Diagnostics (`stylex-diagnostics`, `stylex-macros`)
- Errors carry Babel's message and the span of the node Babel reports; the host
  renders the frame.

Manifests
- SWC core 81 / parser 45; `boa_engine` removed; added `icu_collator`,
  `smallvec`, `stacker`, `xxhash-rust`.

## Performance-only changes

These change no output.

- `stylex-state`: rule registration builds `_inject2(...)` statements and hashes
  placement keys only under runtime injection; rule metadata is derived on first
  read.
- `stylex-state`, `stylex-transform`: the producer dispatch copies declarators
  only where needed and computes each call's key once for all lookups.
- `stylex-state`: top-level expression recording skips array literals and
  object literals that cannot equal a compiled `create`.
- `stylex-css`: relational-selector and compound-pseudo priority scans run only
  on keys that can match; default priorities are one map lookup; lowercase CSS
  keywords skip the value parser.
- `stylex-css-parser`: `validate_media_query` keeps a bounded per-thread memo.
- `stylex-ast`, `stylex-utils`, `stylex-transform`: string and number formatting
  (`convert_string_to_prop_name`, `write_js_number`, `dashify`, `to_radix`,
  class names, `transform_value`, `createTheme` variables) avoids the formatter
  and re-encoding.
- `stylex-transform`: `stylex_create_set` sizes its maps up front.
