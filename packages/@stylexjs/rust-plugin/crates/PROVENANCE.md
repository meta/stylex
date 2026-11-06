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

- Only each crate's `src/`, `Cargo.toml` and license files are copied (plus
  `stylex-path-resolver/fixtures`, which its unit tests read). Benches,
  integration-test directories, docs and JavaScript tooling files are omitted,
  and the `[[bench]]`/`[[test]]` tables naming them are removed.
- The crates inherit `[workspace.package]`, `[workspace.dependencies]` and
  `[workspace.lints]` from `../Cargo.toml`, copied from the upstream
  workspace manifest. `rust-version` is 1.96 (upstream declares 1.97); the
  selected crates have been checked to build on 1.96 (their full upstream
  test suites have not been run on 1.96 yet).

## StyleX changes

Every later change to these crates is a separate commit in this repository so
that each difference from upstream is reviewable on its own.
