# Notice

This project is MIT licensed; the terms are in [LICENSE](./LICENSE) and the
copyright there is the project's own. It contains third-party work, listed
below with its copyright holder and licence.

## StyleX

- **Upstream:** <https://github.com/facebook/stylex>
- **Copyright (c) Meta Platforms, Inc. and affiliates.**
- **Licence:** MIT

The compiler's observable behaviour (class-name hashing, property expansion,
value canonicalization, rule priorities) is StyleX's, and parts of the
implementation follow Meta's structure to match it exactly.

## styleq

- **Upstream:** <https://github.com/necolas/styleq>
- **Copyright (c) Nicolas Gallagher**
- **Licence:** MIT, text at
  [crates/stylex-styleq/LICENSE](./crates/stylex-styleq/LICENSE)

[`crates/stylex-styleq`](./crates/stylex-styleq) is a Rust port of the class-name
merger the StyleX runtime uses.

## postcss-value-parser

- **Upstream:** <https://github.com/TrySound/postcss-value-parser>
- **Copyright (c) Bogdan Chadkin**
- **Licence:** MIT, text at
  [crates/postcss-value-parser/LICENSE](./crates/postcss-value-parser/LICENSE)

[`crates/postcss-value-parser`](./crates/postcss-value-parser) carries this CSS
value scanner as its own dependency-free crate.

## Everything else

Registry dependencies carry their own licence files; `Cargo.lock` records their
versions.
