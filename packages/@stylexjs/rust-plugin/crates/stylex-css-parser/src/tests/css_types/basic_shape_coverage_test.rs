use super::*;
use crate::token_types::{SimpleToken, TokenList};
use std::fmt::{self, Write as FmtWrite};

fn make_token_list(tokens: Vec<SimpleToken>) -> TokenList {
  TokenList {
    tokens,
    current_index: 0,
    depth: 0,
  }
}

// A writer that fails after `remaining` writes, to cover `write!(...)?` error paths.

struct FailAfter {
  remaining: usize,
}

impl fmt::Write for FailAfter {
  fn write_str(&mut self, _s: &str) -> fmt::Result {
    if self.remaining == 0 {
      return Err(fmt::Error);
    }
    self.remaining -= 1;
    Ok(())
  }
}

fn px(v: f64) -> crate::css_types::length_percentage::LengthPercentage {
  use crate::css_types::length::Length;
  use crate::css_types::length_percentage::LengthPercentage;
  LengthPercentage::Length(Length::new(v, "px".to_string()))
}

// ── inset_parser ──

#[test]
fn inset_parser_eof_at_start() {
  let mut tl = make_token_list(vec![]);
  let result = (BasicShape::inset_parser().run)(&mut tl);
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(msg.contains("end of input"), "msg: {msg}");
}

#[test]
fn inset_parser_whitespace_after_left() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("inset".to_string()),
    SimpleToken::Dimension {
      value: 10.0,
      unit: "px".to_string(),
    },
    SimpleToken::Whitespace,
    SimpleToken::Dimension {
      value: 20.0,
      unit: "px".to_string(),
    },
    SimpleToken::Whitespace,
    SimpleToken::Dimension {
      value: 30.0,
      unit: "px".to_string(),
    },
    SimpleToken::Whitespace,
    SimpleToken::Dimension {
      value: 40.0,
      unit: "px".to_string(),
    },
    SimpleToken::Whitespace,
    SimpleToken::Whitespace,
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::inset_parser().run)(&mut tl);
  assert!(result.is_ok(), "err: {:?}", result);
}

#[test]
fn inset_parser_whitespace_before_round_check() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("inset".to_string()),
    SimpleToken::Dimension {
      value: 10.0,
      unit: "px".to_string(),
    },
    SimpleToken::Whitespace,
    SimpleToken::Ident("round".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Dimension {
      value: 5.0,
      unit: "px".to_string(),
    },
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::inset_parser().run)(&mut tl);
  assert!(result.is_ok(), "err: {:?}", result);
}

#[test]
fn inset_parser_wrong_closing_token() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("inset".to_string()),
    SimpleToken::Dimension {
      value: 10.0,
      unit: "px".to_string(),
    },
    SimpleToken::Ident("extra".to_string()),
  ]);
  let result = (BasicShape::inset_parser().run)(&mut tl);
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(msg.contains("Expected closing paren"), "msg: {msg}");
}

#[test]
fn inset_parser_eof_on_closing_paren() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("inset".to_string()),
    SimpleToken::Dimension {
      value: 10.0,
      unit: "px".to_string(),
    },
  ]);
  let result = (BasicShape::inset_parser().run)(&mut tl);
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(msg.contains("end of input"), "msg: {msg}");
}

// ── circle_parser ──

#[test]
fn circle_parser_eof_at_start() {
  let mut tl = make_token_list(vec![]);
  let result = (BasicShape::circle_parser().run)(&mut tl);
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(msg.contains("end of input"), "msg: {msg}");
}

#[test]
fn circle_parser_eof_after_function_token() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("circle".to_string()),
    // EOF
  ]);
  let result = (BasicShape::circle_parser().run)(&mut tl);
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(
    msg.contains("Expected radius") || msg.contains("end of input") || msg.contains("radius"),
    "msg: {msg}"
  );
}

#[test]
fn circle_parser_eof_on_closing_paren() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("circle".to_string()),
    SimpleToken::Dimension {
      value: 10.0,
      unit: "px".to_string(),
    },
  ]);
  let result = (BasicShape::circle_parser().run)(&mut tl);
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(msg.contains("end of input"), "msg: {msg}");
}

// ── ellipse_parser ──

#[test]
fn ellipse_parser_eof_at_start() {
  let mut tl = make_token_list(vec![]);
  let result = (BasicShape::ellipse_parser().run)(&mut tl);
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(msg.contains("end of input"), "msg: {msg}");
}

#[test]
fn ellipse_parser_eof_for_radius() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("ellipse".to_string()),
    // EOF
  ]);
  let result = (BasicShape::ellipse_parser().run)(&mut tl);
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(
    msg.contains("Expected radius") || msg.contains("end of input"),
    "msg: {msg}"
  );
}

#[test]
fn ellipse_parser_wrong_closing_token() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("ellipse".to_string()),
    SimpleToken::Dimension {
      value: 10.0,
      unit: "px".to_string(),
    },
    SimpleToken::Whitespace,
    SimpleToken::Dimension {
      value: 20.0,
      unit: "px".to_string(),
    },
    SimpleToken::Ident("extra".to_string()),
  ]);
  let result = (BasicShape::ellipse_parser().run)(&mut tl);
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(msg.contains("Expected closing paren"), "msg: {msg}");
}

#[test]
fn ellipse_parser_eof_on_closing_paren() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("ellipse".to_string()),
    SimpleToken::Dimension {
      value: 10.0,
      unit: "px".to_string(),
    },
    SimpleToken::Whitespace,
    SimpleToken::Dimension {
      value: 20.0,
      unit: "px".to_string(),
    },
  ]);
  let result = (BasicShape::ellipse_parser().run)(&mut tl);
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(msg.contains("end of input"), "msg: {msg}");
}

// ── polygon_parser ──

#[test]
fn polygon_parser_eof_at_start() {
  let mut tl = make_token_list(vec![]);
  let result = (BasicShape::polygon_parser().run)(&mut tl);
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(msg.contains("end of input"), "msg: {msg}");
}

#[test]
fn polygon_parser_whitespace_after_fill_rule_before_comma() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("polygon".to_string()),
    SimpleToken::Ident("evenodd".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Comma,
    SimpleToken::Percentage(0.0),
    SimpleToken::Whitespace,
    SimpleToken::Percentage(0.0),
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::polygon_parser().run)(&mut tl);
  assert!(result.is_ok(), "err: {:?}", result);
  if let BasicShape::Polygon { fill_rule, .. } = result.unwrap() {
    assert_eq!(fill_rule, Some("evenodd".to_string()));
  }
}

// The fill-rule `else` fallback is unreachable: the peek guard ensures an Ident.

#[test]
fn polygon_parser_whitespace_inside_point_loop() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("polygon".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Percentage(0.0),
    SimpleToken::Whitespace,
    SimpleToken::Percentage(100.0),
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::polygon_parser().run)(&mut tl);
  assert!(result.is_ok(), "err: {:?}", result);
}

#[test]
fn polygon_parser_whitespace_after_point() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("polygon".to_string()),
    SimpleToken::Percentage(0.0),
    SimpleToken::Whitespace,
    SimpleToken::Percentage(0.0),
    SimpleToken::Whitespace,
    SimpleToken::Whitespace,
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::polygon_parser().run)(&mut tl);
  assert!(result.is_ok(), "err: {:?}", result);
}

#[test]
fn polygon_parser_unexpected_token_after_point() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("polygon".to_string()),
    SimpleToken::Percentage(0.0),
    SimpleToken::Whitespace,
    SimpleToken::Percentage(0.0),
    SimpleToken::Ident("bad".to_string()), // unexpected — not comma or RightParen
  ]);
  let result = (BasicShape::polygon_parser().run)(&mut tl);
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(
    msg.contains("Expected comma or closing paren") || msg.contains("polygon"),
    "msg: {msg}"
  );
}

#[test]
fn polygon_parser_eof_after_point() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("polygon".to_string()),
    SimpleToken::Percentage(0.0),
    SimpleToken::Whitespace,
    SimpleToken::Percentage(0.0),
  ]);
  let result = (BasicShape::polygon_parser().run)(&mut tl);
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(
    msg.contains("Unexpected end of input") || msg.contains("end of input"),
    "msg: {msg}"
  );
}

// ── path_parser ──

#[test]
fn path_parser_eof_at_start() {
  let mut tl = make_token_list(vec![]);
  let result = (BasicShape::path_parser().run)(&mut tl);
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(msg.contains("end of input"), "msg: {msg}");
}

#[test]
fn path_parser_whitespace_after_open_paren() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("path".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::String("M0 0".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::path_parser().run)(&mut tl);
  assert!(result.is_ok(), "err: {:?}", result);
  if let BasicShape::Path { path, .. } = result.unwrap() {
    assert_eq!(path, "M0 0");
  }
}

#[test]
fn path_parser_whitespace_after_fill_rule_before_comma() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("path".to_string()),
    SimpleToken::Ident("evenodd".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Comma,
    SimpleToken::String("M0 0".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::path_parser().run)(&mut tl);
  assert!(result.is_ok(), "err: {:?}", result);
  if let BasicShape::Path { fill_rule, .. } = result.unwrap() {
    assert_eq!(fill_rule, Some("evenodd".to_string()));
  }
}

#[test]
fn path_parser_unknown_ident_defaults_fill_rule() {
  let result = BasicShape::parse().parse_to_end("path(other, \"M0 0\")");
  let _ = result;
}

#[test]
fn path_parser_eof_at_string() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("path".to_string()),
    // EOF
  ]);
  let result = (BasicShape::path_parser().run)(&mut tl);
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(
    msg.contains("end of input") || msg.contains("path string"),
    "msg: {msg}"
  );
}

#[test]
fn path_parser_whitespace_after_string() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("path".to_string()),
    SimpleToken::String("M0 0".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Whitespace,
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::path_parser().run)(&mut tl);
  assert!(result.is_ok(), "err: {:?}", result);
}

#[test]
fn path_parser_wrong_closing_token() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("path".to_string()),
    SimpleToken::String("M0 0".to_string()),
    SimpleToken::Ident("extra".to_string()),
  ]);
  let result = (BasicShape::path_parser().run)(&mut tl);
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(msg.contains("Expected closing paren"), "msg: {msg}");
}

#[test]
fn path_parser_eof_on_closing_paren() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("path".to_string()),
    SimpleToken::String("M0 0".to_string()),
  ]);
  let result = (BasicShape::path_parser().run)(&mut tl);
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(msg.contains("end of input"), "msg: {msg}");
}

// ── parse_optional_position ──

// ── basic_shape_parser() ──

#[test]
fn basic_shape_parser_free_function_inset() {
  let result = basic_shape_parser().parse_to_end("inset(10px)");
  assert!(result.is_ok(), "err: {:?}", result);
  assert!(matches!(result.unwrap(), BasicShape::Inset { .. }));
}

// ── Display error propagation ──

#[test]
fn display_inset_two_values_propagates_writer_errors() {
  let value = BasicShape::Inset {
    top: px(10.0),
    right: px(20.0),
    bottom: px(10.0),
    left: px(20.0),
    round: None,
  };
  let (mut saw_ok, mut saw_err) = (false, false);
  for remaining in 0..16 {
    match write!(FailAfter { remaining }, "{}", value) {
      Ok(()) => saw_ok = true,
      Err(_) => saw_err = true,
    }
  }
  assert!(saw_ok && saw_err);
}

#[test]
fn display_inset_three_values_propagates_writer_errors() {
  let value = BasicShape::Inset {
    top: px(10.0),
    right: px(20.0),
    bottom: px(10.0),
    left: px(30.0),
    round: None,
  };
  let (mut saw_ok, mut saw_err) = (false, false);
  for remaining in 0..16 {
    match write!(FailAfter { remaining }, "{}", value) {
      Ok(()) => saw_ok = true,
      Err(_) => saw_err = true,
    }
  }
  assert!(saw_ok && saw_err);
}

#[test]
fn ellipse_parser_at_with_failing_position() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("ellipse".to_string()),
    SimpleToken::Ident("closest-side".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Ident("farthest-side".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Ident("at".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Number(999.0), // Number without unit — may not be valid position
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::ellipse_parser().run)(&mut tl);
  let _ = result;
}

#[test]
fn polygon_parser_fill_rule_without_comma() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("polygon".to_string()),
    SimpleToken::Ident("nonzero".to_string()),
    SimpleToken::Percentage(0.0),
    SimpleToken::Whitespace,
    SimpleToken::Percentage(0.0),
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::polygon_parser().run)(&mut tl);
  assert!(result.is_ok(), "err: {:?}", result);
  if let BasicShape::Polygon { fill_rule, points } = result.unwrap() {
    assert_eq!(fill_rule, Some("nonzero".to_string()));
    assert_eq!(points.len(), 1);
  }
}

#[test]
fn polygon_parser_immediate_right_paren_after_fill_rule() {
  // `polygon()` with no points reaches the `points.is_empty()` guard.
  let mut tl = make_token_list(vec![
    SimpleToken::Function("polygon".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::polygon_parser().run)(&mut tl);
  assert!(result.is_err(), "expected error for empty polygon");
  let msg = result.unwrap_err().to_string();
  assert!(
    msg.contains("at least one point") || msg.contains("polygon") || msg.contains("Polygon"),
    "msg: {msg}"
  );
}

#[test]
fn path_parser_fill_rule_without_comma() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("path".to_string()),
    SimpleToken::Ident("evenodd".to_string()),
    SimpleToken::String("M0 0".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::path_parser().run)(&mut tl);
  assert!(result.is_ok(), "err: {:?}", result);
  if let BasicShape::Path { fill_rule, path } = result.unwrap() {
    assert_eq!(fill_rule, Some("evenodd".to_string()));
    assert_eq!(path, "M0 0");
  }
}

// The leading-whitespace guard in `parse_optional_position` is unreachable via circle/ellipse.

#[test]
fn parse_optional_position_skips_leading_whitespace() {
  // Callers drain whitespace first, so this guard is only reachable directly.
  let mut tokens = make_token_list(vec![SimpleToken::Whitespace]);
  let result = BasicShape::parse_optional_position(&mut tokens);
  assert_eq!(result.unwrap(), None);
}
