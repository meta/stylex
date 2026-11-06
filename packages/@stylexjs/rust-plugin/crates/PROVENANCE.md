# Vendored StyleX core crates

These crates are the StyleX compiler core from the public
[stylex-swc-plugin](https://github.com/Dwlad90/stylex-swc-plugin) repository
(MIT; see `LICENSE` and `NOTICE.md` in this directory), vendored at:

- tag `0.19.2`, commit `6e7d3983`
- full repository archive (`git archive 6e7d3983`) sha256
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

## Mechanical vendoring changes

- Only each crate's `src/`, `Cargo.toml` and license files were copied (plus
  `stylex-path-resolver/fixtures`, which its unit tests read). Upstream's
  benches, integration-test directories, docs and JavaScript tooling files were
  left out, and the `[[bench]]`/`[[test]]` tables naming them removed. The
  `tests/` directories now in `stylex-css`, `stylex-css-parser` and
  `stylex-evaluator` were added here.
- `stylex-path-resolver/fixtures` is copied whole, including its 112 files
  under `fixtures/*/node_modules/` (among them `.pnpm/` package copies; a
  plain tree, no symlinks). The repository's root `.gitignore` ignores
  `node_modules`, `dist` and `lib`, so `../.gitignore` re-includes them with
  upstream's negation `!crates/stylex-path-resolver/fixtures/**/node_modules`
  plus `!crates/stylex-path-resolver/fixtures/**/node_modules/**` for the
  `dist`/`lib` directories inside. The files are byte-identical to tag
  `0.19.2` (`6e7d3983`), taken from the codeload archive
  `stylex-swc-plugin-0.19.2.tar.gz` (sha256
  `cf4f84b6a32cfcdf4657cc0d9b831c7577cf401f15f05db9624822e3be02affa`).
- The crates inherit `[workspace.package]`, `[workspace.dependencies]` and
  `[workspace.lints]` from `../Cargo.toml`, copied from the upstream
  workspace manifest. `rust-version` is 1.96 (upstream declares 1.97); the
  crates build and their tests pass on 1.96.

- `rustfmt.toml` in this directory carries upstream's formatting (its stable
  options), and `stylex-state/package.json` keeps the package name upstream's
  canonical-path unit tests read.
- The crates' unit tests are run with `cargo test` and `RUST_MIN_STACK` set to
  16 MiB (`scripts/test.cjs`): upstream runs them under nextest, whose
  per-test main threads give the deep-nesting tests 8 MiB.

## Behaviour changes

Grouped by area; each keeps the Babel plugin's output and diagnostics where
upstream differed.

Evaluator (`stylex-evaluator`, `stylex-js`)
- The optional Boa engine is removed. A native evaluator folds what the Babel
  plugin's evaluator folds, with JavaScript semantics: sparse arrays and holes,
  `==`/`!=`, spread in array literals and in arguments to pure builtins,
  optional member reads, functions defined in the file and called later, and
  reference equality (distinct fresh allocations are unequal; refused when
  identity cannot be proven).
- A read before its declaration is refused in every order; builtin method values,
  arrow-function property reads and optional calls are refused.
- Work is bounded: allocation, length and recursion budgets refuse with typed
  reasons instead of exhausting memory or the stack.
- Results cross into the transform through one fallible data boundary
  (`materialize_result`); validators and `create`, `defineVars`, `createTheme`
  read data through it.
- Results are memoized per evaluation, invalidated by source writes and host
  operations; host reads and calls are never cached.
- Known follow-up: a Rust caller that writes bindings through `StateManager` or
  `FunctionMap` directly, or shares a budget across state owners, can read a
  stale memoized value; `transformSync` and the Babel facade cannot reach it.

Host boundary (`stylex-structures`, `stylex-state`)
- `env` values and callbacks are live host values (`HostEnv`), read and called
  in the order the Babel plugin does; their failures keep their cause.
- Effects of compiler callbacks run from the host (rules, imports) are
  journaled per evaluation scope and replayed into the file's state in order.
- `ImportResolver` lets the host resolve theme imports, name files canonically
  and rewrite aliased imports (`rewriteAliases`); `DebugFileNamer` names files
  for `debugFilePath`.

Module resolution and paths (`stylex-state`, `stylex-path-resolver`)
- Every option is set explicitly with the Babel plugin's defaults
  (`src/utils/transform/core_options.rs`); no `unstable_moduleResolution`
  means no resolution.
- `commonJS` resolves relative imports natively and asks the host otherwise; with
  a host installed, files are named by the host as Babel names them (a manifest
  without `name` gives `undefined:`).
- `experimental_crossFileParsing` resolves as `commonJS` does; upstream refused it
  (its unit test `cross_file_parsing_is_refused` became
  `cross_file_parsing_resolves_as_common_js`).
- A file outside any package resolves against its own directory (upstream
  refused it); package lookups start at the resolution's root directory.
- A resolved path is checked on disk in every build; a missing import is left
  unresolved instead of panicking. Resolved file paths are compared with forward
  slashes.

Transform and output (`stylex-transform`, `stylex-state`, `stylex-css`)
- Rule metadata is reported per `add_style` call in order, duplicates kept.
- Injected imports and helper names follow the Babel plugin's merge and naming
  order; theme-import compensation is placed at the import it compensates.
- Call and JSX-spread sites are told apart by source span, not structure.
- Writes through known aliases and callbacks mark an object as mutated.
- Compiled objects keep required `null` keys and their order; conditional
  shorthands keep the last slot and reset keys their first appearance; unused
  reset keys are pruned in dev as in prod.
- `defineVars` normalizes stored callbacks after its first argument;
  `createTheme` and the nested forms read their arguments as data.
- With `test: true`, `createTheme` emits only the test label class.
- Dev and debug labels (`$$css`, `data-style-src`) are measured from the root
  or package as Node's `path.relative` measures them on the platform, with its
  separator: a file outside the root reads `../…` and a manifest without `name`
  gives `undefined:`. Upstream wrote `/` everywhere and fell back to the last two
  segments outside the root; its unit tests for those cases were ported to the
  Babel plugin's names.

Diagnostics (`stylex-diagnostics`, `stylex-macros`)
- Errors carry the Babel plugin's message and the span of the node it reports
  (a binding's declaration where Babel frames it); the host renders the frame
  and nothing is printed to stderr.

Manifests
- The workspace builds on SWC core 81 / parser 45; `boa_engine` is gone. Added
  workspace dependencies: `icu_collator`, `smallvec`, `stacker`, `xxhash-rust`.

## Performance-only changes

These change no output.

- `stylex-state`: `register_styles` builds the per-rule `_inject2(...)`
  statements only under runtime injection (the flush drops them otherwise),
  and the last placement key takes its statement by move.
- `stylex-transform`, `stylex-state`: the producer dispatch copies a
  declarator only for `keyframes`, `viewTransitionClass` and `positionTry`,
  and `update_references` hashes the call once for its four index lookups.
- `stylex-css`: `get_priority` runs the five relational-selector patterns (all
  anchored at `^:where\(`) only for keys starting with `:where(`.
- `stylex-state`: `register_styles` hashes placement keys only under runtime
  injection, records each rule with one copy and one set probe, and reuses the
  call key as the replaced initializers' key.
- `stylex-css-parser`: `validate_media_query` keeps a bounded per-thread memo
  of its answers by query text.
- `stylex-ast`: `convert_string_to_prop_name` accepts an ASCII identifier
  holding any character outside `a`-`z` without the reserved-word lookups
  (every reserved word is lowercase ASCII letters).
- `stylex-transform`: `convert_style_to_class_name` builds the hashed string
  and the class name in sized buffers, and skips sorting and joining for a
  declaration with no conditions.
- `stylex-utils`: `write_js_number` writes an integer below 2^53 as its
  decimal digits directly instead of re-reading `{:e}` output.
- `stylex-transform`: `stylex_create_set` builds minified namespace keys
  without the formatter.
- `stylex-transform`: `stylex_create_set` sizes its per-namespace and per-call
  maps up front.
- `stylex-css`: the default property priority is one lookup in a map folded
  from the four priority sets (first set wins, as the probes ran) instead of
  four set probes.
- `stylex-utils`: `dashify` hyphenates and lowercases ASCII input in one pass.
- `stylex-transform`: `transform_value` appends the unit to a number without
  the formatter, and `transform_value_cached` sizes its cache key once.
- `stylex-state`: the deduplicated rule metadata (`StateManager::metadata`) is
  derived from the reported rules when first read instead of being kept up per
  rule; contents and order are unchanged (unit test added).
- `stylex-state`: `var_group_member` and theme import resolution build their
  strings by concatenation instead of the formatter.
- `stylex-state`, `stylex-transform`: the producer dispatch computes the
  call's key once and shares it with the `create` transform's declaration
  lookup, style-variable insert and reference update (new keyed
  `find_call_declaration`, `insert_style_var` and `register_styles` variants).
- `stylex-utils`: `to_radix` takes its digit table as a fixed-size array, so
  each radix is a compile-time constant.
- `stylex-state`: `fill_top_level_expressions` does not record a top-level
  array literal, which none of the recorded-expression lookups can match (they
  match a call, or a style variable's initializer, never an array).
- `stylex-transform`, `stylex-state`: where a `create` call registers rules
  and no earlier style variable still holds the same call, `create` records
  its style variable once, after `register_styles`, holding the compiled
  initializer, and copies only the declarator's span, name and `definite` flag
  (otherwise it records it as before; `StateManager::style_var_for_call_keyed`
  asks); `createTheme`, `createThemeNested` and `defineVars` read the
  declarator's name or binding in place instead of copying the declarator.
- `stylex-css`: `normalize_css_property_value` answers a lowercase ASCII
  keyword (letters and hyphens, opening with a letter) as written, without the
  parse and print no pass changes it in; a test checks the shortcut against
  the full pipeline.
- `stylex-css`: compound pseudo priority skips the pseudo-part pattern scan
  for a key with a single run of colons, which can hold at most one part; a
  test checks the shortcut against the scan.
- `stylex-state`: `fill_top_level_expressions` also skips an object literal
  that cannot equal a compiled `create` (non-empty, no spread, no property
  that is a function or an object holding `$$css`), such as the constant
  objects a theme reads.
- `stylex-transform`: `createTheme` sorts ASCII variable names by bytes (their
  UTF-16 order) without re-encoding them, and builds each variable declaration
  by concatenation.
