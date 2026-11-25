//! The scanner.

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

/// What `charCodeAt` returns past the end (JS `NaN`): equal to no byte and greater than 32.
const OUT_OF_RANGE: u32 = u32::MAX;

/// Scans bytes, not UTF-16 units: every tested character is ASCII, so offsets are byte offsets.
fn char_code_at(value: &str, pos: usize) -> u32 {
  match value.as_bytes().get(pos) {
    Some(byte) => u32::from(*byte),
    None => OUT_OF_RANGE,
  }
}

/// `String.prototype.slice`: out-of-range and inverted bounds clamp to `""`.
/// A cut inside a character falls back to a lossy conversion instead of panicking.
fn slice(value: &str, start: usize, end: usize) -> String {
  let start = start.min(value.len());
  let end = end.clamp(start, value.len());

  match value.get(start..end) {
    Some(text) => text.to_owned(),
    None => String::from_utf8_lossy(&value.as_bytes()[start..end]).into_owned(),
  }
}

/// `String.prototype.indexOf` for a single character, searching from `from`.
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

/// `/^[a-f0-9?-]+$/i` on the tail after the caller has checked the leading `u+`.
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

/// A function whose `)` has not been seen yet; its children move into the node on close.
struct OpenFunction {
  node: Node,
  children: Vec<Node>,
}

impl OpenFunction {
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

/// The last node written, if it is a div — the JavaScript's `prev`.
fn trailing_div<'value>(
  root: &'value mut Vec<Node>,
  stack: &'value mut [OpenFunction],
) -> Option<&'value mut Node> {
  tokens(root, stack)
    .last_mut()
    .filter(|node| node.kind == NodeKind::Div)
}

/// Whether the innermost open function is `calc()`, where `*` and `/` are operators.
fn parent_is_calc(stack: &[OpenFunction]) -> bool {
  matches!(stack.last(), Some(open) if open.node.value == "calc")
}

/// Whether the innermost open function is one other than `calc()`.
fn parent_is_non_calc_function(stack: &[OpenFunction]) -> bool {
  matches!(stack.last(), Some(open) if open.node.value != "calc")
}

/// Capacity hint for the top-level node list (a quarter of the byte length); only a hint.
fn estimated_nodes(bytes: usize) -> usize {
  (bytes / 4).clamp(1, 32)
}

/// Turns a declaration value into a loose token list. Never fails.
pub fn parse(input: &str) -> Vec<Node> {
  // Copied only when a closing delimiter has to be invented.
  let mut value: Cow<'_, str> = Cow::Borrowed(input);

  let mut pos: usize = 0;
  let mut code = char_code_at(&value, pos);
  // Captured before invented delimiters extend the buffer.
  let max = value.len();
  let mut root: Vec<Node> = Vec::with_capacity(estimated_nodes(input.len()));
  let mut stack: Vec<OpenFunction> = Vec::new();
  // JS `parent` is `undefined` until the first function opens; the `/` whitespace rule
  // tests `!parent`, so this is observable.
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
      // Matches upstream: the search starts at the opening `/`, so `/*/` closes itself and does not
      // round-trip.
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
        // `before` is always empty here; saturating to be safe.
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
        // A url body is taken whole.
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

        // Whitespace before the close, scanned backwards. Index 0 is unreachable (a `(` precedes);
        // the guard prevents an infinite loop if that ever changes.
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
          // JS's `pos <= whitespace_pos` guard is dead here: the body is non-empty.
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
        // JS appends to the parent on open; attaching on close lands in the same position.
        stack.push(OpenFunction {
          node: token,
          children: Vec::new(),
        });
        parent_seen = true;
      }
    } else if code == CLOSE_PARENTHESES
      && let Some(mut finished) = stack.pop()
    {
      // Close parentheses. An unmatched `)` falls through to the word scanner, as in JS.
      pos += 1;
      code = char_code_at(&value, pos);

      finished.node.after = Some(std::mem::take(&mut after));
      // Matches upstream: this sum is overwritten on the next line.
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

#[cfg(test)]
mod tests {
  use super::slice;

  /// Every cut the scanner makes lands on a character boundary.
  #[test]
  fn a_cut_on_a_character_boundary_takes_the_cheap_path() {
    assert_eq!(slice("hello", 1, 4), "ell");
    assert_eq!(slice("日本語", 0, 3), "日");
    assert_eq!(slice("a→b", 1, 4), "→");
  }

  /// Out-of-range and inverted bounds clamp to `""`, like `String.prototype.slice`.
  #[test]
  fn bounds_outside_the_buffer_clamp_instead_of_panicking() {
    assert_eq!(slice("abc", 0, 99), "abc");
    assert_eq!(slice("abc", 99, 99), "");
    assert_eq!(slice("abc", 2, 1), "");
    assert_eq!(slice("", 0, 5), "");
  }

  /// A cut inside a character (unreachable from [`parse`]) yields replacement characters.
  #[test]
  fn a_cut_inside_a_character_is_replaced_rather_than_fatal() {
    // One replacement character per broken sequence, not per byte.
    assert_eq!(slice("日", 0, 1), "\u{fffd}");
    assert_eq!(slice("a日b", 1, 3), "\u{fffd}");
    assert_eq!(slice("a日b", 0, 3), "a\u{fffd}");
  }
}
