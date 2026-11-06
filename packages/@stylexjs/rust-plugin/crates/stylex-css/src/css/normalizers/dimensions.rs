//! The walk two of the ported normalizers share.

use postcss_value_parser::{Dimension, Node, NodeKind, ValueParser, unit};

/// Visits every word node that splits into a number and a unit.
///
/// [`super::timings`] and [`super::font_size_px_to_rem`] open with the same
/// three steps: walk the token list, skip anything that is not a word, and
/// split the word with [`unit`]. Only what they do afterwards differs, so the
/// opening is stated here once and each of them states only its own rewrite.
///
/// Two more normalizers look like they belong and do not. Both exclusions are
/// behavioural, so read them before adding a caller:
///
/// - [`super::zero_dimensions`] tracks function boundaries across *every* node,
///   words included, so its walk has to see what this one filters out.
/// - [`super::leading_zero`] acts on words this one skips. It selects on
///   `parse_js_float`, which — unlike [`unit`] — skips leading JavaScript
///   whitespace, and the value parser does not treat every such character as a
///   space: a word can begin with U+00A0, read back as a number, and split into
///   no dimension at all. Its unit-less branch is that word, and dropping it
///   would leave the character in place and change the class name.
///
/// The visitor is handed the word and its split and answers with the value to
/// replace it with, or `None` to leave it alone — rather than being handed the
/// node to mutate. A [`Dimension`] borrows the word it was split from, so a
/// visitor holding one cannot also hold the node mutably; returning the
/// rewrite hands the assignment back here, where the split is already done
/// with. It also states the shape both callers already had: read, decide,
/// build a string only when there is something to change.
pub(super) fn walk_dimensions<F>(ast: &mut ValueParser, mut visit: F)
where
  F: FnMut(&str, Dimension<'_>) -> Option<String>,
{
  ast.walk(
    |node: &mut Node, _| {
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
