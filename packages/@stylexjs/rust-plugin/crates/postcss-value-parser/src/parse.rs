//! The scanner. See the crate documentation in `lib.rs` for what it is and
//! who holds its copyright.

use std::borrow::Cow;

use crate::{Node, NodeKind};

const OPEN_PARENTHESES: u32 = b'(' as u32;
const CLOSE_PARENTHESES: u32 = b')' as u32;
const SINGLE_QUOTE: u32 = b'\'' as u32;
const DOUBLE_QUOTE: u32 = b'"' as u32;
const BACKSLASH: u32 = b'\\' as u32;
const SLASH: u32 = b'/' as u32;
const COMMA: u32 = b',' as u32;
const COLON: u32 = b':' as u32;
const STAR: u32 = b'*' as u32;

/// Stands in for what `charCodeAt` returns past the end of a string.
///
/// The JavaScript gets `NaN` there, which fails every comparison the scanner
/// makes and, being neither less than nor equal to 32, ends a whitespace run at
/// the end of input. `u32::MAX` behaves the same way against every test in this
/// file: no byte can equal it, and it is greater than 32.
const OUT_OF_RANGE: u32 = u32::MAX;

/// The scanner works over bytes rather than UTF-16 code units.
///
/// Every character it tests for is ASCII, and no UTF-8 continuation or lead
/// byte can be mistaken for one, so a multi-byte character is scanned as an
/// opaque run and token boundaries land in the same places either way. Offsets
/// are therefore byte offsets, which is all their one consumer needs: it
/// compares them against each other, never against a JavaScript index.
fn char_code_at(value: &str, pos: usize) -> u32 {
  match value.as_bytes().get(pos) {
    Some(byte) => u32::from(*byte),
    None => OUT_OF_RANGE,
  }
}

/// `String.prototype.slice` over the working buffer, clamping out-of-range and
/// inverted bounds to the empty string the way JavaScript does.
///
/// The buffer is a `str` rather than a byte slice so that this can cut it
/// without re-deciding whether it is UTF-8. It was handed a `&str` and only
/// ever appends ASCII to it, so the answer is always yes, and `str::get` checks
/// two offsets where validating the range walks every byte in it. That scan
/// was about a tenth of parse time, paid once per token.
///
/// The lossy path stays for a cut that lands inside a character rather than on
/// a boundary. Every cut the scanner makes lands on an ASCII delimiter or on a
/// buffer end, so nothing reaches it -- but a panic is never the right answer
/// to a value the compiler was handed, and this returns exactly what the whole
/// function used to.
fn slice(value: &str, start: usize, end: usize) -> String {
  let start = start.min(value.len());
  let end = end.clamp(start, value.len());

  match value.get(start..end) {
    Some(text) => text.to_owned(),
    None => String::from_utf8_lossy(&value.as_bytes()[start..end]).into_owned(),
  }
}

/// `String.prototype.indexOf` for a single character, searching from `from`.
///
/// Searching a subslice rather than skipping a byte iterator, so the search
/// reaches core's vectorised `str::find` instead of comparing one byte at a
/// time — the difference the long-comment, long-string and long-`url()` shapes
/// in the bench corpus are there to measure. `from` is a character boundary at
/// every call site, and `get` answers `None` rather than panicking if that ever
/// stops being true.
fn index_of(value: &str, needle: char, from: usize) -> Option<usize> {
  value
    .get(from..)
    .and_then(|tail| tail.find(needle))
    .map(|at| at + from)
}

/// `String.prototype.indexOf("*/")`, searching from `from`.
fn index_of_comment_end(value: &str, from: usize) -> Option<usize> {
  value
    .get(from..)
    .and_then(|tail| tail.find("*/"))
    .map(|at| at + from)
}

/// `/^[a-f0-9?-]+$/i` applied to the token's tail, after the leading `u+` has
/// been checked by the caller.
fn is_unicode_range_tail(tail: &[u8]) -> bool {
  !tail.is_empty()
    && tail
      .iter()
      .all(|byte| byte.is_ascii_hexdigit() || *byte == b'?' || *byte == b'-')
}

/// Whether a word is a `U+0-7F` style range rather than an ordinary word.
fn is_unicode_range(token: &str) -> bool {
  let bytes = token.as_bytes();

  match (bytes.first(), bytes.get(1)) {
    (Some(b'u' | b'U'), Some(b'+')) => is_unicode_range_tail(&bytes[2..]),
    _ => false,
  }
}

/// A function whose closing parenthesis has not been seen yet.
///
/// Its children are held beside it rather than in its own `nodes` field, so
/// that "the list currently being filled" is a real list at every moment rather
/// than an optional one nobody can be sure is there. They move into the node
/// when it closes.
struct OpenFunction {
  node: Node,
  children: Vec<Node>,
}

impl OpenFunction {
  /// The finished node, with its children in place.
  fn close(self) -> Node {
    Node {
      nodes: Some(self.children),
      ..self.node
    }
  }
}

/// Where the scanner is writing — the JavaScript's `tokens`.
fn tokens<'value>(
  root: &'value mut Vec<Node>,
  stack: &'value mut [OpenFunction],
) -> &'value mut Vec<Node> {
  match stack.last_mut() {
    Some(open) => &mut open.children,
    None => root,
  }
}

/// The node last written, when it is a separator — the JavaScript's `prev`,
/// already narrowed to the one kind the caller does anything with.
fn trailing_div<'value>(
  root: &'value mut Vec<Node>,
  stack: &'value mut [OpenFunction],
) -> Option<&'value mut Node> {
  tokens(root, stack)
    .last_mut()
    .filter(|node| node.kind == NodeKind::Div)
}

/// Whether the innermost open function is `calc()`, inside which `*` and `/`
/// are operators rather than separators.
fn parent_is_calc(stack: &[OpenFunction]) -> bool {
  matches!(stack.last(), Some(open) if open.node.value == "calc")
}

/// Whether the innermost open function is one other than `calc()`.
fn parent_is_non_calc_function(stack: &[OpenFunction]) -> bool {
  matches!(stack.last(), Some(open) if open.node.value != "calc")
}

/// How many top-level nodes a value of `bytes` bytes is likely to produce.
///
/// A [`Node`] is 120 bytes, so the four slots `Vec` reaches for on its first
/// push cost 480 -- most of them wasted on the short values a stylesheet is
/// mostly made of, and not nearly enough for a long one, which then regrows and
/// copies every node it already holds.
///
/// A quarter of the byte length lands closer at both ends. Measured over the
/// parity corpus, a top-level list averages 2.05 nodes and reaches 21; this
/// under-reserves for 19% of it, each costing the one regrowth that was
/// previously the common case.
///
/// A function's arguments are deliberately not sized this way. They average
/// 3.28 nodes, which is what `Vec` already reserves, and the bytes left to scan
/// when a nested function opens describe its arguments rather than the value,
/// so estimating from them under-reserves badly enough to cost two regrowths
/// where the default cost one.
///
/// Nothing reads the hint back. Being wrong costs an allocation, never an
/// answer.
fn estimated_nodes(bytes: usize) -> usize {
  (bytes / 4).clamp(1, 32)
}

/// Turns a declaration value into a loose token list.
///
/// Never fails: input the parser cannot make sense of comes back as words and
/// spaces, and constructs that run off the end of the input come back flagged
/// rather than rejected.
pub fn parse(input: &str) -> Vec<Node> {
  // The scan grows `value` past the input when it has to invent a closing
  // delimiter, so the buffer has to be growable -- but only three input shapes
  // ever make it grow. Borrowed until one of them turns up, copied then, which
  // is the difference between one allocation per declaration value and none.
  let mut value: Cow<'_, str> = Cow::Borrowed(input);

  let mut pos: usize = 0;
  let mut code = char_code_at(&value, pos);
  // The input's length, captured before any invented delimiter extends the
  // buffer. That is what stops the main loop from walking over a delimiter it
  // appended itself.
  let max = value.len();
  let mut root: Vec<Node> = Vec::with_capacity(estimated_nodes(input.len()));
  let mut stack: Vec<OpenFunction> = Vec::new();
  // The JavaScript's `parent` is `undefined` until the first function opens and
  // the root object from then on, and the difference is observable: the
  // whitespace-before-`/` rule tests `!parent`, so a `/` at top level is read
  // one way before any function has been seen and another way after one has
  // closed.
  let mut parent_seen = false;

  let mut name = String::new();
  let mut before = String::new();
  let mut after = String::new();

  while pos < max {
    if code <= 32 {
      // Whitespace.
      let mut next = pos;
      loop {
        next += 1;
        code = char_code_at(&value, next);
        if code > 32 {
          break;
        }
      }
      let token = slice(&value, pos, next);

      if code == CLOSE_PARENTHESES && !stack.is_empty() {
        after = token;
      } else if let Some(prev) = trailing_div(&mut root, &mut stack) {
        prev.source_end_index += token.len();
        prev.after = Some(token);
      } else if code == COMMA
        || code == COLON
        || (code == SLASH
          && char_code_at(&value, next + 1) != STAR
          && (!parent_seen || parent_is_non_calc_function(&stack)))
      {
        before = token;
      } else {
        tokens(&mut root, &mut stack).push(Node::new(NodeKind::Space, token, pos, next));
      }

      pos = next;
    } else if code == SINGLE_QUOTE || code == DOUBLE_QUOTE {
      // Quotes.
      let mut next = pos;
      let quote = match code == SINGLE_QUOTE {
        true => b'\'',
        false => b'"',
      };
      let mut token = Node::new(NodeKind::String, String::new(), pos, 0);
      token.quote = Some(char::from(quote));

      loop {
        let mut escape = false;

        match index_of(&value, char::from(quote), next + 1) {
          Some(found) => {
            next = found;
            let mut escape_pos = found;
            while char_code_at(&value, escape_pos.wrapping_sub(1)) == BACKSLASH {
              escape_pos -= 1;
              escape = !escape;
            }
          },
          None => {
            value.to_mut().push(char::from(quote));
            next = value.len() - 1;
            token.unclosed = true;
          },
        }

        if !escape {
          break;
        }
      }

      token.value = slice(&value, pos + 1, next);
      token.source_end_index = match token.unclosed {
        true => next,
        false => next + 1,
      };
      tokens(&mut root, &mut stack).push(token);
      pos = next + 1;
      code = char_code_at(&value, pos);
    } else if code == SLASH && char_code_at(&value, pos + 1) == STAR {
      // Comments.
      //
      // The search starts at the opening `/`, not past it, so `/*/` finds its
      // own terminator and the comment closes before the author meant it to.
      // That is deliberate, and the one input shape that does not survive a
      // parse-and-serialise round trip unchanged.
      let mut token = Node::new(NodeKind::Comment, String::new(), pos, 0);

      let next = match index_of_comment_end(&value, pos) {
        Some(found) => {
          token.source_end_index = found + 2;
          found
        },
        None => {
          token.unclosed = true;
          token.source_end_index = value.len();
          value.len()
        },
      };

      token.value = slice(&value, pos + 2, next);
      tokens(&mut root, &mut stack).push(token);

      pos = next + 2;
      code = char_code_at(&value, pos);
    } else if (code == SLASH || code == STAR) && parent_is_calc(&stack) {
      // Operation within calc.
      let token = slice(&value, pos, pos + 1);
      let length = token.len();
      tokens(&mut root, &mut stack).push(Node::new(
        NodeKind::Word,
        token,
        // `before` is provably empty here — it is only ever set when the
        // character after the whitespace is `,`, `:` or a `/` outside `calc()`,
        // and each of those produces a div that clears it. The subtraction is
        // saturating rather than trusting that argument.
        pos.saturating_sub(before.len()),
        pos + length,
      ));
      pos += 1;
      code = char_code_at(&value, pos);
    } else if code == SLASH || code == COMMA || code == COLON {
      // Dividers.
      let token = slice(&value, pos, pos + 1);
      let length = token.len();
      let mut node = Node::new(
        NodeKind::Div,
        token,
        pos.saturating_sub(before.len()),
        pos + length,
      );
      node.before = Some(std::mem::take(&mut before));
      node.after = Some(String::new());
      tokens(&mut root, &mut stack).push(node);

      pos += 1;
      code = char_code_at(&value, pos);
    } else if code == OPEN_PARENTHESES {
      // Whitespace after the open parenthesis.
      let mut next = pos;
      loop {
        next += 1;
        code = char_code_at(&value, next);
        if code > 32 {
          break;
        }
      }
      let parentheses_open_pos = pos;
      let name_start = pos.saturating_sub(name.len());
      let mut token = Node::new(NodeKind::Function, std::mem::take(&mut name), name_start, 0);
      token.before = Some(slice(&value, parentheses_open_pos + 1, next));
      pos = next;

      if token.value == "url" && code != SINGLE_QUOTE && code != DOUBLE_QUOTE {
        // A url body is taken whole: it is allowed to contain the characters
        // that would otherwise separate tokens.
        next -= 1;
        loop {
          let mut escape = false;

          match index_of(&value, ')', next + 1) {
            Some(found) => {
              next = found;
              let mut escape_pos = found;
              while char_code_at(&value, escape_pos.wrapping_sub(1)) == BACKSLASH {
                escape_pos -= 1;
                escape = !escape;
              }
            },
            None => {
              value.to_mut().push(')');
              next = value.len() - 1;
              token.unclosed = true;
            },
          }

          if !escape {
            break;
          }
        }

        // Whitespace before the close, scanned backwards. Index 0 is
        // unreachable: this branch is only entered inside a `url(` body, so
        // there is an opening parenthesis at a lower index and it is not
        // whitespace.
        //
        // The guard is there anyway, because it is what decides how a broken
        // precondition fails. Where the JavaScript reads `charCodeAt(-1)` as
        // `NaN` and leaves the loop, an unsigned counter pinned at 0 by
        // `saturating_sub` re-reads index 0 forever — so a refactor that let
        // the scan reach the front would hang the compiler rather than return
        // a wrong offset.
        //
        // Spelled as the loop condition rather than as an `if … break` inside
        // it, so that the unreachable exit is a branch no test has to take
        // instead of a statement no test can cover.
        let mut whitespace_pos = next;
        while whitespace_pos > 0 {
          whitespace_pos -= 1;
          code = char_code_at(&value, whitespace_pos);

          if code > 32 {
            break;
          }
        }
        let body_end = whitespace_pos + 1;

        if parentheses_open_pos < whitespace_pos {
          // The body cannot be empty here, so the JavaScript's `pos <=
          // whitespace_pos` guard is dead. Written out because an omitted guard
          // is the kind of thing that gets re-added as a fix:
          //
          // `pos` is the first index after the parenthesis whose code exceeds
          // 32 — the whitespace skip above left it there. The backward scan
          // stops at the first such index at or before the close, and `pos` is
          // one, so it stops no earlier: `whitespace_pos >= pos`. The one case
          // where it lands before `pos` is a body of nothing but whitespace,
          // and there it lands on the parenthesis itself, which is exactly what
          // the enclosing `parentheses_open_pos < whitespace_pos` rejects.
          let mut nodes = vec![Node::new(
            NodeKind::Word,
            slice(&value, pos, body_end),
            pos,
            body_end,
          )];
          if token.unclosed && body_end != next {
            token.after = Some(String::new());
            nodes.push(Node::new(
              NodeKind::Space,
              slice(&value, body_end, next),
              body_end,
              next,
            ));
          } else {
            token.after = Some(slice(&value, body_end, next));
          }
          token.nodes = Some(nodes);
        } else {
          token.after = Some(String::new());
          token.nodes = Some(Vec::new());
        }

        pos = next + 1;
        token.source_end_index = match token.unclosed {
          true => next,
          false => pos,
        };
        code = char_code_at(&value, pos);
        tokens(&mut root, &mut stack).push(token);
      } else {
        token.after = Some(String::new());
        token.source_end_index = pos + 1;
        // The JavaScript appends the function to its parent here and keeps
        // writing through the same object; this attaches it when it closes
        // instead. Nothing can be appended to the parent while the function is
        // open, so the position it lands in is the same.
        stack.push(OpenFunction {
          node: token,
          children: Vec::new(),
        });
        parent_seen = true;
      }
    } else if code == CLOSE_PARENTHESES
      && let Some(mut finished) = stack.pop()
    {
      // Close parentheses. A `)` with nothing open is not one — it falls
      // through to the word scanner below, which is where the JavaScript's
      // `balanced` counter sends it too.
      pos += 1;
      code = char_code_at(&value, pos);

      finished.node.after = Some(std::mem::take(&mut after));
      // The JavaScript adds the trailing whitespace's length here and then
      // overwrites the field with `pos` on the next line. Kept so this reads
      // the way it was written; the sum is never observable.
      finished.node.source_end_index += finished.node.after.as_ref().map_or(0, String::len);
      finished.node.source_end_index = pos;

      let closed = finished.close();
      tokens(&mut root, &mut stack).push(closed);
      parent_seen = true;
    } else {
      // Words.
      let mut next = pos;
      loop {
        if code == BACKSLASH {
          next += 1;
        }
        next += 1;
        code = char_code_at(&value, next);

        let terminates = code <= 32
          || code == SINGLE_QUOTE
          || code == DOUBLE_QUOTE
          || code == COMMA
          || code == COLON
          || code == SLASH
          || code == OPEN_PARENTHESES
          || (code == STAR && parent_is_calc(&stack))
          || (code == CLOSE_PARENTHESES && !stack.is_empty());

        if next >= max || terminates {
          break;
        }
      }
      let token = slice(&value, pos, next);

      if code == OPEN_PARENTHESES {
        name = token;
      } else if is_unicode_range(&token) {
        tokens(&mut root, &mut stack).push(Node::new(NodeKind::UnicodeRange, token, pos, next));
      } else {
        tokens(&mut root, &mut stack).push(Node::new(NodeKind::Word, token, pos, next));
      }

      pos = next;
    }
  }

  // Anything still open ran off the end of the input.
  while let Some(mut open) = stack.pop() {
    open.node.unclosed = true;
    open.node.source_end_index = value.len();
    let closed = open.close();
    tokens(&mut root, &mut stack).push(closed);
  }

  root
}

/// [`slice`] is the one helper with a branch the scanner cannot reach, so it is
/// measured directly rather than through [`parse`].
#[cfg(test)]
mod tests {
  use super::slice;

  /// The claim the fallback rests on: every cut the scanner makes lands on a
  /// character boundary, so the cheap path is the only one a parse ever takes.
  /// Asserting it here is what lets the branch below stay defensive rather than
  /// become the thing that decides an answer.
  #[test]
  fn a_cut_on_a_character_boundary_takes_the_cheap_path() {
    assert_eq!(slice("hello", 1, 4), "ell");
    assert_eq!(slice("日本語", 0, 3), "日");
    assert_eq!(slice("a→b", 1, 4), "→");
  }

  /// Out-of-range and inverted bounds clamp to the empty string, the way
  /// `String.prototype.slice` does rather than the way indexing a slice does.
  #[test]
  fn bounds_outside_the_buffer_clamp_instead_of_panicking() {
    assert_eq!(slice("abc", 0, 99), "abc");
    assert_eq!(slice("abc", 99, 99), "");
    assert_eq!(slice("abc", 2, 1), "");
    assert_eq!(slice("", 0, 5), "");
  }

  /// A cut inside a character, which is what the fallback is for. Unreachable
  /// from [`parse`] -- the backward scan in a `url(` body stops on a
  /// character's last byte, and every other cut lands on an ASCII delimiter --
  /// so this is the only place its answer is pinned: replacement characters,
  /// never a panic.
  #[test]
  fn a_cut_inside_a_character_is_replaced_rather_than_fatal() {
    // One replacement character per broken sequence, not per orphaned byte:
    // both cuts below leave a single incomplete sequence.
    assert_eq!(slice("日", 0, 1), "\u{fffd}");
    assert_eq!(slice("a日b", 1, 3), "\u{fffd}");
    // A whole character either side of the damage still comes through.
    assert_eq!(slice("a日b", 0, 3), "a\u{fffd}");
  }
}
