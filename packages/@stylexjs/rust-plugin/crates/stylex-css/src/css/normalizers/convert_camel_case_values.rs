//! Ported normalizer 8 of 9 in upstream's sequence, which is what that count
//! names. See `normalize_value.rs` for the order the passes run in here — a
//! tenth pass that upstream has no equivalent for runs among them.

use std::borrow::Cow;

use postcss_value_parser::{NodeKind, ValueParser};
use stylex_utils::string::dashify;

/// Rewrites a camel-cased property *name* appearing in a value — `backgroundColor`
/// in `transitionProperty: 'backgroundColor'` — into the CSS spelling.
///
/// Only two properties take property names as their value, and only those two
/// are rewritten. A custom property named in one of them keeps its case, since
/// `--fooBar` and `--foobar` are different properties.
///
/// Unlike every other normalizer here this reads only the top-level nodes, so
/// a name written inside a function is left alone. It also applies to every
/// word token at that level rather than only to the ones that are really
/// identifiers, which is why a bare number in one of these values would be run
/// through the same rewrite — harmlessly, since it has no upper case to fold.
pub fn convert_camel_cased_values(ast: &mut ValueParser, key: &str) {
  if key != "transitionProperty" && key != "willChange" {
    return;
  }

  for node in &mut ast.nodes {
    if node.kind == NodeKind::Word && !node.value.starts_with("--") {
      // Only the owned answer is a rewrite. `dashify` borrows back whatever it
      // would not have changed — ASCII with no upper case, which is almost
      // every word these two properties carry — and `into_owned` would copy
      // that word to replace it with itself.
      if let Cow::Owned(dashed) = dashify(&node.value) {
        node.value = dashed;
      }
    }
  }
}
