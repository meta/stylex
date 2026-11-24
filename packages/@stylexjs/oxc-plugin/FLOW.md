# Flow and stylex-oxc

How oxs handles Flow, what it depends on from the Flow repo, and what we would ask the Flow team
for so open-source StyleX can ship this without path dependencies. Owner: flow.

## What oxs does with Flow

- **Detection** (`crates/stylex_oxc/src/parse.rs`). Option `flow`: `"detect"` (default), `"all"`
  or `true`, `false`. Detect follows Babel's flow plugin: a `@flow` pragma before the first
  token, or a `.js.flow` file. `@noflow` counts as no pragma, as in Babel and hermes-parser.
  Every file goes to oxc first. A file oxc parses has no type syntax and means the same to Flow,
  except `f<T>(x)`, which then goes to Flow. Files without a pragma that oxc rejects (JSX, then
  TSX) also go to Flow. Files that name no StyleX source skip parsing altogether.
- **Fast path** (`crates/flow_blank/src/ts_fast.rs`). A Flow file that only uses syntax
  TypeScript shares (annotations, generics, `import type`, type aliases, interfaces, `as`) is
  parsed with oxc's TS parser instead of Flow's, and blanked with the same rules. Anything else
  goes to Flow's parser: a TS parse error (every Flow-only construct), any TS-only construct, and
  the few shapes where the two parsers split the same text differently (`a ? (b): T => c : d`,
  `f<{...T}>(x)`, a type followed by a line starting with `<` or `[`, function types with
  destructured params, Flow keyword type names). The blanked text and the type names must equal
  Flow's byte for byte; `flow_blank fastcheck` and tests/flow/fuzz-types.cjs check that.
- **Blanking** (`crates/flow_blank`). Flow's own Rust parser (`flow_parser`) parses the file, and
  every type-only byte becomes a space, so offsets and lines stay put. oxc parses the result.
- **Lowering** (`crates/flow_blank/src/lower.rs`, about 1,150 lines, written for oxs). `component`,
  `hook`, `match` and records become plain JS the way hermes-parser does it in Babel mode, including
  `React.forwardRef` for `ref` params and hermes' generated names. Lowering is a list of text
  edits over the source, so untouched code is copied byte for byte and line numbers are kept.
  An offset map takes output positions back to the source. Enums stay in the output, as Babel
  prints them. `declare export` and the other `declare` forms are type-only and get blanked.
- **Type references.** Babel's scope counts names in Flow types (`x: styles`, `typeof styles`,
  `(styles: T) => U`, indexer keys, type params, but not object type keys) as references, so a
  style object named in a type keeps all its styles. Blanking hides types from oxc, so
  flow_blank collects those names and the transform applies the same rule.
- **Options.** `flowParserOptions` takes hermes-parser's parser option names, so a Babel config
  carries over: `reactRuntimeTarget: '19'` (a `ref` param stays a prop, no `forwardRef`) and
  `transformOptions: {TransformEnumSyntax: {enable: true}}` (enums become
  `require("flow-enums-runtime")` calls, as babel-plugin-transform-flow-enums does).

We do not use Flow's own lowering (`flow_parser_wasm::babel_adapter`). It reprints the whole
program, so every offset moves, and it pulls in `flow_common`, which does not build on stable
Rust 1.100 (`box_patterns` was removed). Writing the lowering against hermes-parser's behavior
was smaller than adapting it.

## Results

- Flow corpus (`node tests/flow/run.cjs all`), oxs against the Babel plugin behind
  `babel-plugin-syntax-hermes-parser`, comparing errors, metadata, CSS and normalized code:
  - synthetic fixtures with every construct, in three option sets (defaults, dev + debug, and
    React 19 with enum lowering): 24 of 24 runs identical.
  - StyleX's own Flow sources: 271 of 271 identical.
  - React `packages/*/src`: 1,415 of 1,415 identical.
- Lowering vs hermes-parser (`tests/flow/fuzz-lower.cjs`): 4,000 random `match`, `component`
  and `hook` programs, 2,885 identical after normalization, 0 different, 1,115 rejected by both.
  2,000 more with records: 1,645 identical, 0 different, 355 rejected by both.
- Random StyleX programs with `stylex.props` in components, hooks and `match` arms
  (`tests/flow/fuzz-stylex.cjs`): 2,200 of 2,200 identical to Babel, half of them in dev + debug
  mode (line numbers in `$$css` and `data-style-src` included).
- Cost on non-Flow files: none measured. 696 Babel-suite inputs take 16.7 ms (best of 30,
  three runs) with the Flow front end and 16.9 to 17.4 ms in a build without it (pinned, same
  allocator). The only added work is a scan of the leading comments.
- Fast path equivalence: 134 of 270 StyleX Flow files, 1,075 of 1,415 React files and all 696
  suite inputs take it, and all of them blank byte for byte like Flow's parser, with the same
  type names. 240,000 random type-syntax files (fuzz-types, 80 seeds): 51,849 took it, all
  identical, none that Flow rejects.
- Early errors (duplicate constructors, `super()` outside a constructor, duplicate labels,
  `break` outside a loop, redeclarations and the rest) are raised on every file, whichever
  parser ran, with Babel's message and position. See `tests/early/run.cjs`.
- Fast path speed: perf/createTheme-complex.js (28 KB, `@flow strict`, only `import type`) went
  from 1,530 µs to 715 µs. createTheme-basic.js from 150 to 79 µs. The perf
  common set from 10.45 ms to 9.6 ms. StyleX's own Flow files that use StyleX from 15.6 to
  18.3 MB/s. "flow blanking" dropped from 6.0% of oxs samples to under 0.1% in
  tools/perf/breakdown.cjs.
- Cost on Flow files: Flow's parser runs at 13 to 23 MB/s and dominates when it runs. StyleX's own Flow files
  that use StyleX compile at about 15 MB/s end to end. Flow files without StyleX skip parsing.
- Build: the addon grows from 7.3 MB to 9.7 MB stripped and gains 47 crates. `flow_parser` is
  the longest compile unit (about 19 s).

## What we depend on

From `facebook/flow` `rust_port/crates`, as a git dependency pinned to one commit:

- `flow_parser`: `parse_program_without_file`, `ParseOptions` (with `components`, `enums`,
  `pattern_matching`, `records`), the `ast` types, `ast_visitor::AstVisitor` (about 35 hooks),
  `loc::{Loc, Position}`, `parse_error::ParseError`.
- Through it, `flow_data_structure_wrapper` (only for `smol_str`), which brings `im` with
  `rayon`, `rpds` and `parking_lot`.

## What would help upstream

In priority order. 1 to 3 unblock a crates.io release of open-source StyleX.

1. **Publish the parser on crates.io.** `flow_parser` is `version = "0.0.0"`, generated from
   another build system and unpublished, and the names `flow_parser`/`flow-parser` on crates.io belong to an unrelated
   project. A name like `flow-parser-rs`, versioned with the npm `flow-parser` release (0.342.x
   style), would let us depend on it normally and see breaking changes. A git dependency works
   today but cannot ship in a crates.io release and clones the whole Flow repo (341 MB).
2. **Drop `#![feature(never_type)]`.** `!` is stable since Rust 1.100, so the attribute is the
   only thing that needs `RUSTC_BOOTSTRAP=flow_parser` (it only warns on 1.100). Use
   `std::convert::Infallible` if older compilers matter.
3. **Trim dependencies.** Depend on `smol_str` directly instead of `flow_data_structure_wrapper`
   (most of the extra crates come from `im`, `rayon` and `rpds`). Move `regex` to the
   `parse_test262` binary that uses it. Put `serde`, `serde_json` and the ESTree translator
   behind a feature.
4. **Keep the visitor stable, or say when it breaks.** We rely on `AstVisitor` hooks and on the
   shape of about 30 AST structs (match patterns, component params, enum bodies, imports). The
   crate moves fast (dozens of commits a month). A changelog entry for AST changes would do.
5. **Settle column units.** `Loc` columns are UTF-8 bytes, but `offset_utils::OffsetTable`
   treats them as code points and is off by one byte per multi-byte character before the
   position. A byte offset on `Loc`, or a cheap offset API, would save every consumer a line
   table.
6. **If the runtime-syntax lowering is published**, make it a small crate (no `flow_common` or
   `flow_server_utils`) that builds on stable, and have it return edits or a source map. We
   would still need our own edit-based lowering unless it keeps untouched code in place.
7. **Longer term, parse speed.** Flow's parser is about 10x slower than oxc on the same text.
   Profiles show malloc/free near 16% and dropping the AST near 25% of parse time (every node is
   an `Arc`). An arena, or fewer `Arc`s, would make Flow files nearly as cheap as plain JS.

## Known gaps

- With `inputSourceMap`, emit composes the input map in lowered coordinates, so maps for lowered
  files (with `component`, `hook` or `match`) are off. Plain Flow files are fine. See
  INTERFACES.md.
- Generated code in a lowered file maps to the construct it replaced, not to the pattern piece
  each condition came from.
- A multi-line `match` argument in expression position moves to the end of the IIFE (hermes'
  shape). Lines after the `match` stay put, but lines inside its cases move up by the
  argument's line count.
- The harness's `__parserPlugins: [['flow', {all: true}]]` is not mapped to `flow: 'all'`
  (host). Detect mode gives the same results on every corpus file.
