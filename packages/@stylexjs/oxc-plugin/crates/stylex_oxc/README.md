# stylex_oxc

The oxc front end. It parses with oxc, finds StyleX calls, evaluates their arguments, compiles
them with `css.rs` and the core crates, and splices the results into the original text.

One file goes through `parse`, then `imports::build_scope`, then `transform` pass 1 (top-level
declarations, dispatched to `api/*`), then pass 2 (props, attrs and sx, nested creates, and
references for unused-style elimination), then `emit` (splicing and `sourcemap`).

The Rust entry is `transform_json(code, &options_json)`. The napi entries are in `host.rs`.
Tests link the rlib with napi in `noop` mode.
