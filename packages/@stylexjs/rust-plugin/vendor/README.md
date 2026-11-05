# SWC parser bridge

`swc_ecma_parser` 38.0.2 is copied from its published Apache-2.0 crate archive.
Only the visibility of the existing `TokenFlags` re-export changes, so the native
compiler can enforce resource limits through the parser's actual lexer tokens.

Archive SHA-256: `a7c251d44e048647b5335861d1585b3e95fa8bc74f6e7a40570b0ea95d27ba66`.
The provenance test compares every vendored byte against the archived fixture.

Upstream source: https://github.com/swc-project/swc/tree/6c9032efaf1234d4f5138b102056835fb3a153d1/crates/swc_ecma_parser
