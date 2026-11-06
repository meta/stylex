# Pinned SWC parser

`swc_ecma_parser` 45.1.6, copied whole from its published Apache-2.0 crate archive,
with three patches (each patched file opens with a `// Modified for StyleX` notice):

- `TokenFlags` is exported, for the frontend's resource limits.
- A Flow `#name` parses through the existing private-name parser; the frontend
  checks that it is the direct left operand of `in` and declared in its class.
- Module-level `for await` is accepted in modules, using the real async context.

Archive SHA-256: `a4af66bb9a4cbd2a600b41c2592e27b7947b7e30aa6918d321f987f9e0ff63f5`
(`tests/vendor_parser_provenance.rs` reverses the three patches and checks every file).

Source: https://github.com/swc-project/swc/tree/37ea5bd03c3cf14f81917b7ec8fbd6e90d203404/crates/swc_ecma_parser
