//! Cutting a shorthand value into the parts its expansion hands out.
//!
//! Ports `splitValue` from the reference compiler's
//! `shared/utils/split-css-value.js`. The one deliberate divergence is recorded
//! under "What holds of every part" below: an empty part is a part here, where
//! upstream throws.
//!
//! A shorthand assigns one authored value to several longhands, so something
//! has to decide where one part ends and the next begins. That decision is a
//! compatibility contract rather than a detail: each part becomes the value of
//! a generated declaration, and a declaration's text is what its class name is
//! hashed from. Cut in a different place and every longhand gets a different
//! class name — and in the worst case a value that is not a declaration at all.
//!
//! The cut is made on the [value scanner]'s node kinds, which is the same
//! answer the reference compiler reaches, and it is made there for a reason a
//! character test cannot reproduce: `/` and `:` end a part at the top level and
//! are ordinary characters inside a function, so `10px/1.5` is two parts while
//! `calc(100% / 3)` is one. Only a scanner that has already decided what is a
//! separator and what is inside a function can tell those apart.
//!
//! # What holds of every part
//!
//! Three rules, stated here because each of them has been got wrong once at a
//! call site that did not know it:
//!
//! 1. **A part is echoed, never re-spelled.** An escape stays an escape, a
//!    quote keeps the character it was written with, and a number keeps the
//!    digits it was typed with. That is why there is no number formatting and
//!    no identifier serialization in this file.
//! 2. **A trailing importance annotation belongs to every part, not to one.**
//!    See [`apply_importance`].
//! 3. **An empty part is a part.** See below.
//!
//! All three are enforced by there being one way to get a part —
//! [`split_value_parts`] — rather than by anything a part carries as a value.
//! It applies the importance fold before returning, it spells a part only by
//! echoing what the scanner read, and it never drops one. A part is a `String`
//! for that reason: a wrapper around it could hold no rule the single producer
//! is not already the choke point for.
//!
//! The fold has one path around it, and it costs nothing: a value nested past
//! the depth budget returns before the split, as the single part it was handed.
//! [`apply_importance`] is a no-op on one part — it needs a second to move an
//! annotation onto — so the value that skips it is the value it would have
//! returned unchanged.
//!
//! # An empty part is a part
//!
//! A comment contributes its inner text, and an unterminated comment or an
//! empty one has none. So `padding: '1px /*'` is two parts, the second empty —
//! and that second part is *present*. It occupies its position, it counts
//! toward the arity, and every consumer treats it as the part it is:
//!
//! - the four-sided view assigns it to a side, whose declaration then emits
//!   nothing, because a declaration whose value is empty is not emitted;
//! - the importance fold moves a trailing annotation onto it like any other
//!   part;
//! - `contain-intrinsic-size`'s fold joins it onto a preceding `auto`;
//! - `list-style` lets it occupy the slot it lands in, and refuses a value
//!   where that means two images.
//!
//! No consumer skips one and none reads one as absent. There is no reference
//! answer to copy: the reference compiler throws `Cannot read properties of
//! undefined (reading 'type')` on `padding: '1px /*'`, so parity cannot
//! arbitrate this shape and the decision is this compiler's to make. It is
//! settled this way because the reference compiler's own guards over a part list
//! ask whether a part is *absent* — a missing index — and no part of a split
//! value is ever absent. Reading empty as absent instead loses whatever the
//! position meant: it is what made `contain-intrinsic-size: 'auto /*'` size only
//! the width where the reference compiler sizes both.
//!
//! **One consequence is worth stating where the decision is made.** A value
//! carrying an unterminated comment now reaches two different verdicts depending
//! on the style resolution, because only one of them splits it. Under
//! `legacy-expand-shorthands` `padding: '1px /*'` is *accepted* and emits a top
//! and a bottom, since the splitter reads the comment as an empty part and hands
//! the parts on; under the default `application-order` the same declaration is
//! refused for the unclosed comment, because nothing splits it and it arrives at
//! `normalize_css_property_value` whole.
//!
//! Not a defect of either path -- the reference compiler crashes on this input,
//! so there is nothing to be faithful to -- but two verdicts for one source is
//! the kind of thing a reader should meet here rather than discover from a
//! failing build.
//!
//! [value scanner]: postcss_value_parser

use crate::css::common::nests_too_deeply;
use postcss_value_parser::{Node, NodeKind, ValueParser};

/// The parts of `css_string`, in the order the expansion consumes them.
///
/// The only way to get a part, which is what makes the three rules in this
/// module's documentation hold of every one of them.
///
/// Whitespace and separators are structure here, not content: they decide where
/// a part ends and then contribute nothing to it. Everything else is echoed
/// with the author's own characters — an escape stays an escape, a quote keeps
/// the character it was written with, a hex colour keeps its spelling, and a
/// number keeps the digits it was typed with. Nothing here re-spells anything,
/// which is why there is no number formatting and no identifier serialization
/// in this file.
///
/// A part can be empty, and an empty one is returned rather than dropped — see
/// *An empty part is a part* above.
pub fn split_value_parts(css_string: &str) -> Vec<String> {
  let trimmed = js_trim(css_string);

  // Not a split at all for a value nested past the compiler's budget, and not a
  // refusal either. Scanning one builds a tree whose *destructor* recurses once
  // per level, which overruns the stack and aborts the process -- unwindable
  // panics never enter it, so no diagnostic would ever be produced. Expansion
  // runs before the guard in front of normalization, so this is the earliest
  // place the question can be asked at all.
  //
  // Handing the value on whole rather than refusing here keeps the diagnostic
  // where it is documented: normalization asks the same question of the same
  // scan and rejects, so the author gets the nesting-depth message either way.
  //
  // It costs a second byte scan of every shorthand value, since normalization
  // scans again for its own reasons. Paid rather than avoided: the alternative
  // is a depth count kept here that has to agree with the one the diagnostic is
  // raised from, and a splitter that declined at a depth normalization accepts
  // would refuse a value nobody rejected.
  //
  // A value with no `(` in it opens no function, so it nests nothing and the
  // second scan is skipped for it -- which is nearly every shorthand value there
  // is: `1px 2px`, `auto`, `red solid 1px`. The guarantee is unchanged, since
  // `nests_too_deeply` can only answer `true` for a value that opens one.
  if trimmed.contains('(') && nests_too_deeply(trimmed) {
    return vec![trimmed.to_string()];
  }

  let parts: Vec<String> = ValueParser::new(trimmed)
    .nodes
    .iter()
    .filter(|node| !matches!(node.kind, NodeKind::Space | NodeKind::Div))
    .map(print_node)
    .collect();

  apply_importance(parts)
}

/// Moves a trailing `!important` onto every part it qualifies.
///
/// An author writes it once and means it of the whole shorthand, so it cannot
/// stay a part of its own: as one it would become the value of whichever
/// longhand happened to be next in line, and `padding: '1px !important'` would
/// emit `padding-inline-end: important`. Re-attached to each part instead, it
/// survives on all four sides.
///
/// A lone `!important` with nothing to qualify is left alone — there is no part
/// to move it onto, and dropping it would silently discard what the author
/// wrote.
///
/// An empty part is qualified like any other: `padding: '1px /**/ !important'`
/// is three parts, and the annotation lands on the empty one too. That reads
/// oddly and it is what the reference compiler does — the middle part becomes
/// ` !important`, which normalization spells `!important`, and both compilers
/// hash the same class name for it.
fn apply_importance(mut parts: Vec<String>) -> Vec<String> {
  // The `!` test is what keeps the case fold off the common path: no character
  // lowercases *into* `!`, so a part that does not start with one cannot be an
  // importance annotation however it is spelled.
  let qualifies = parts.len() >= 2
    && parts
      .last()
      .is_some_and(|last| last.starts_with('!') && last.to_lowercase() == "!important");

  if !qualifies {
    return parts;
  }

  // Annotated in place. The fold this replaces allocated a fresh `Vec` and a
  // fresh `String` per part, on the path every shorthand declaration takes, to
  // append the same eleven bytes each time.
  parts.pop();

  for part in &mut parts {
    part.push_str(" !important");
  }

  parts
}

/// One node as the text it contributes to its part.
///
/// Written against an explicit stack rather than recursively: the scanner's own
/// pass over the input is iterative and accepts nesting far deeper than a call
/// stack survives, and a value reaches here *before* the depth guard in front
/// of normalization has had a chance to reject anything.
fn print_node(node: &Node) -> String {
  let mut out = String::with_capacity(node.source_end_index.saturating_sub(node.source_index));
  let mut pending: Vec<Emit<'_>> = vec![Emit::Node(node)];

  while let Some(step) = pending.pop() {
    match step {
      Emit::CloseParen => out.push(')'),
      Emit::Node(node) => match (node.kind, node.quote, node.nodes.as_deref()) {
        (NodeKind::String, Some(quote), _) => {
          out.push(quote);
          out.push_str(&node.value);
          out.push(quote);
        },
        (NodeKind::Function, _, Some(arguments)) => {
          out.push_str(&node.value);
          out.push('(');
          pending.push(Emit::CloseParen);
          pending.extend(arguments.iter().rev().map(Emit::Node));
        },
        // Every other kind contributes its own text, and so does a string or a
        // function missing the field the arm above reads. The fallback is not a
        // guess: the reference implementation matches on the fields being
        // present too, and an absent one takes it down the same path.
        _ => out.push_str(&node.value),
      },
    }
  }

  out
}

/// A step of [`print_node`]'s traversal: a node to spell, or the parenthesis
/// that closes the function whose arguments were just queued.
enum Emit<'a> {
  Node(&'a Node),
  CloseParen,
}

/// `css_string` with the whitespace JavaScript's `String.prototype.trim`
/// removes taken off both ends.
///
/// Not `str::trim`, and the difference is observable twice. A byte-order mark
/// is trimmed by JavaScript and is not Unicode whitespace, so `str::trim`
/// leaves it attached to the first part; `U+0085` is Unicode whitespace and is
/// not in JavaScript's set, so `str::trim` removes one the reference compiler
/// keeps. Neither character is whitespace to the scanner, so whichever survives
/// the trim ends up inside a word and inside a class name.
fn js_trim(css_string: &str) -> &str {
  css_string.trim_matches(|character: char| {
    character == '\u{feff}' || (character.is_whitespace() && character != '\u{85}')
  })
}

#[cfg(test)]
#[path = "../tests/values_parser_tests.rs"]
mod tests;
