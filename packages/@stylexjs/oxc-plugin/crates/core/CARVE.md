# What the carve changed

The crates here are #2008's vendored copies (Dwlad90/stylex-swc-plugin 0.19.2, see
`PROVENANCE.md`) taken from `packages/@stylexjs/rust-plugin/crates` at commit `9553897528`.
This file lists every change made to drop SWC. Removed code is kept under `../cut/`.

## Unchanged (byte-identical sources)

`postcss-value-parser`, `stylex-constants`, `stylex-css` (see "Changes after the carve"), `stylex-macros`,
`stylex-path-resolver`, `stylex-regex`, `stylex-styleq`, `stylex-types`.
Manifests only: unused `swc_malloc` and `criterion` dev-dependencies dropped (they served the
upstream benches, which were never vendored).

## Leaks and replacements

| # | where | SWC type | replacement |
|---|---|---|---|
| 1 | `stylex-utils/src/collection.rs` | `swc_core::atoms::Atom` keys of `get_hash_map_value_difference`, `sum_hash_map_values` | generic `K: Eq + Hash + Clone` |
| 2 | `stylex-utils/src/hash.rs` | `Expr`, `CallExpr`, `MemberExpr`, ~40 node types, `drop_span` in `stable_hash_unspanned*` | cut to `cut/stylex-utils/hash_unspanned.rs` (AST hashing is front-end work). `WideHasher`, `new`, `finish_wide` and `MAX_UNSPANNED_HASH_COLLECTION_LEN` made `pub` so a front end reuses them |
| 3 | `stylex-utils/src/swc.rs` | `Expr`, `BinaryOp`, `Syntax` helpers | cut (front end) |
| 4 | `stylex-utils/src/tests/support.rs` | SWC parser for tests | cut with the tests that used it |
| 5 | `stylex-enums/src/style_vars_to_keep.rs` | `Atom` | `NonNullProp<A = String>`, `NonNullProps<A = String>` |
| 6 | `stylex-structures/src/style_vars_to_keep.rs` | `Id`, `Atom` | `StyleVarsToKeep<I, A = String>` |
| 7 | `stylex-structures/src/top_level_expression.rs` | `Expr`, `Atom` | `TopLevelExpression<E, A = String>` |
| 8 | `stylex-structures/src/dynamic_style.rs` | `Expr` | `DynamicStyle<E>` |
| 9 | `stylex-structures/src/inline_style.rs` | `Expr` | `InlineStyle<E>` |
| 10 | `stylex-structures/src/pre_rule_value.rs` | `PreRuleValue::Expr(Expr)` | `PreRuleValue<E>`; the core uses `PreRuleValue<stylex_value::Value>` |
| 11 | `stylex-structures/src/plugin_pass.rs` | `swc_core::common::FileName` | own `FileName { Real(PathBuf), Anon }` |
| 12 | `stylex-structures/src/uid_generator.rs` | `generate_ident() -> Ident` | removed; `generate() -> String` stays, the front end wraps it |
| 13 | `stylex-structures/src/stylex_env.rs` | `EnvEntry::Expr(Expr)`, `JSFunction(Fn(Vec<Expr>) -> Expr)` | `EnvEntry::Value(Value)`, `JSFunction(Fn(Vec<Value>) -> Value)`. Reaches `CoreStyleXOptions::env` and `debug_file_path` |
| 14 | `stylex-structures/src/base_css_type.rs` | `ObjectLit`, `Expr`, `KeyValueProp`, `PropOrSpread`, `stylex_ast` convertors and factories | rewritten over `Value`: `BaseCSSType::from_object`, `to_value`, `value_to_entries`, `get_css_value(&Value)`. SWC version in `cut/` |
| 15 | `stylex-css-parser/src/at_queries/media_query_transform.rs` | `KeyValueProp`, `ObjectLit`, `PropName`, `Str`, `Wtf8Atom`, `DUMMY_SP` | generic over a `MediaQueryProp` trait; implemented for `(String, Value)`. Its two test suites run against a 150-line test-only stand-in for the SWC shapes (`tests/at_queries/mini_swc_ast.rs`), assertions unchanged |
| 16 | `stylex-css-parser/src/lib.rs` | doc example built `KeyValueProp` | doc example uses the value tree |
| 17 | manifests | `swc_core` (css-parser, enums, structures, utils), `swc_ecma_parser` (utils), `stylex_ast` (structures) | dropped |

`stylex-stylesheet` is #2008's `src/stylesheet` moved into a crate (`mod.rs` became `lib.rs`).
It needed `ryu-js` and `icu_locale_core`, which the root crate supplied.

## Tests

- Removed: 33 `stylex-utils` tests (24 in `swc_test.rs`, 6 unspanned-hash tests, 3 parser
  self-tests in `support.rs`) and 13 `base_css_type_test.rs` tests.
- Added: 4 `BaseCSSType` tests over `Value`.
- Adapted, same assertions: `collection_test.rs` (`Atom` -> `String`), `stylex_env_test.rs`
  and `core_stylex_options_test.rs` (`Expr` -> `Value`), `plugin_pass_test.rs`,
  `uid_generator_test.rs` (`generate_ident().sym` -> `generate()`), and the 77 media-query
  transform tests (mini AST).

## Changes after the carve (perf)

- Hashing (2026-11-23). `stylex-css/src/css/common.rs` (`DEFAULT_PRIORITIES`) and
  `stylex-stylesheet/src/process.rs` (constants, width keys, dedupe) use the workspace's Fx types
  from `stylex_utils::collections` instead of std SipHash maps. `stylex-stylesheet` gained a
  `stylex_utils` dependency for it. The other core crates already used Fx. Output is unchanged:
  map iteration order never reaches the CSS (it was random per process before). `stylex-css` is
  no longer byte-identical to upstream because of this one import.

## Changes after the carve (accuracy)

- Rule-breaking values (2026-11-23, team closer). `stylex-css/src/css/common.rs`
  `normalize_css_property_value` no longer refuses a value with a `{`, `}` or `;` outside strings
  and comments, or an unclosed `/*`. #2008 refused them to keep a value from splicing CSS into
  the stylesheet. Babel's plugin accepts them and prints the value as written (for example
  `.x1vhilna{font-family:x{y}}`), so oxs does too. `ValueStructure` keeps only the nesting depth.
  Tests `rule_breaking_values_are_kept_like_babel`,
  `rule_breaking_relative_color_values_are_kept_like_babel` and
  `keeps_a_braced_value_under_a_pseudo_key` replace the refusal tests, with Babel's output as the
  expected values. The totality test no longer lists the two diagnostics.
- Value normalization fast paths (2026-11-23). `stylex-css/src/css/common.rs`
  `normalize_css_property_value` returns `var(--name)` and plain dimensions (`16px`, `1.5rem`,
  `100%`, `2`; not `ms`, not below 1) unchanged without parsing, next to the existing plain-keyword
  path. `transitionProperty`, `willChange` and `fontSize` with px-to-rem on still go through the
  passes. `common_test.rs` checks both paths against the full pipeline for every key the passes
  treat specially.
