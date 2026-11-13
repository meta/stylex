//! The walk two of the ported normalizers share.

use postcss_value_parser::{Dimension, Node, NodeKind, ValueParser, unit};

/// Visits every word node that splits into a number and a unit; `visit` returns a replacement.
///
/// `zero_dimensions` and `leading_zero` deliberately keep their own walks: they need nodes this
/// one filters out (e.g. a word starting with U+00A0 that `unit` cannot split).
pub(super) fn walk_dimensions<F>(ast: &mut ValueParser, visit: F)
where
  F: FnMut(&str, Dimension<'_>) -> Option<String>,
{
  walk_dimensions_impl(ast, visit, false);
}

pub(super) fn walk_dimensions_outside_urls<F>(ast: &mut ValueParser, visit: F)
where
  F: FnMut(&str, Dimension<'_>) -> Option<String>,
{
  walk_dimensions_impl(ast, visit, true);
}

fn walk_dimensions_impl<F>(ast: &mut ValueParser, mut visit: F, skip_urls: bool)
where
  F: FnMut(&str, Dimension<'_>) -> Option<String>,
{
  ast.walk(
    |node: &mut Node, _| {
      if skip_urls && node.kind == NodeKind::Function && node.value.eq_ignore_ascii_case("url") {
        return false;
      }
      if node.kind != NodeKind::Word {
        return true;
      }

      let replacement = match unit(&node.value) {
        Some(dimension) => visit(&node.value, dimension),
        None => None,
      };

      if let Some(replacement) = replacement {
        node.value = replacement;
      }

      true
    },
    false,
  );
}
