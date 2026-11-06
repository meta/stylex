# Pinned SWC parser frontend

`swc_ecma_parser` 45.1.6 is copied in full from its published Apache-2.0 crate
archive to match the public StyleX core's `swc_core` 81 / AST 29 family.

Three bounded patches are applied:

- Export the existing `TokenFlags` type so the frontend can implement resource
  limits through the actual context-sensitive token protocol.
- Parse a `#name` through the existing private-name parser instead of rejecting
  every private brand check in Flow. The frontend separately validates that an
  expression private name is the direct left operand of `in` and is declared in
  its lexical class scope; invalid standalone, unary, grouped, and undeclared
  private expressions remain errors.
- Permit module-level `for await` using the real module/async context while
  rejecting scripts, synchronous functions, and class static blocks.

Archive SHA-256:
`a4af66bb9a4cbd2a600b41c2592e27b7947b7e30aa6918d321f987f9e0ff63f5`.
The portable provenance test reverses exactly these three hunks and checks the
length/hash of every published file and the complete 42-file inventory. Package
metadata and license notices are unchanged; no registry/toolchain files are
modified.

Upstream source:
https://github.com/swc-project/swc/tree/37ea5bd03c3cf14f81917b7ec8fbd6e90d203404/crates/swc_ecma_parser
