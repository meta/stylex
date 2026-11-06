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

fn pct(v: f64) -> crate::css_types::length_percentage::LengthPercentage {
  use crate::css_types::common_types::Percentage;
  use crate::css_types::length_percentage::LengthPercentage;
  LengthPercentage::Percentage(Percentage::new(v))
}

// ── inset_parser ──

#[test]
fn inset_parser_wrong_function_name() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("circle".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::inset_parser().run)(&mut tl);
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(msg.contains("Expected inset"), "msg: {msg}");
}

#[test]
fn inset_parser_eof_at_start() {
  let mut tl = make_token_list(vec![]);
  let result = (BasicShape::inset_parser().run)(&mut tl);
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(msg.contains("end of input"), "msg: {msg}");
}

#[test]
fn inset_parser_whitespace_after_top() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("inset".to_string()),
    SimpleToken::Dimension {
      value: 10.0,
      unit: "px".to_string(),
    },
    SimpleToken::Whitespace,
    SimpleToken::Whitespace,
    SimpleToken::Dimension {
      value: 20.0,
      unit: "px".to_string(),
    },
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::inset_parser().run)(&mut tl);
  assert!(result.is_ok(), "err: {:?}", result);
}

#[test]
fn inset_parser_whitespace_after_right() {
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
    SimpleToken::Whitespace,
    SimpleToken::Dimension {
      value: 30.0,
      unit: "px".to_string(),
    },
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::inset_parser().run)(&mut tl);
  assert!(result.is_ok(), "err: {:?}", result);
}

#[test]
fn inset_parser_whitespace_after_bottom() {
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
    SimpleToken::Whitespace,
    SimpleToken::Dimension {
      value: 40.0,
      unit: "px".to_string(),
    },
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::inset_parser().run)(&mut tl);
  assert!(result.is_ok(), "err: {:?}", result);
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
fn inset_parser_round_with_whitespace_after_keyword() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("inset".to_string()),
    SimpleToken::Dimension {
      value: 10.0,
      unit: "px".to_string(),
    },
    SimpleToken::Whitespace,
    SimpleToken::Ident("round".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Whitespace,
    SimpleToken::Dimension {
      value: 5.0,
      unit: "px".to_string(),
    },
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::inset_parser().run)(&mut tl);
  assert!(result.is_ok(), "err: {:?}", result);
  if let BasicShape::Inset { round, .. } = result.unwrap() {
    assert!(round.is_some());
  }
}

#[test]
fn inset_parser_non_round_keyword_gives_none_round() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("inset".to_string()),
    SimpleToken::Dimension {
      value: 10.0,
      unit: "px".to_string(),
    },
    SimpleToken::Whitespace,
    SimpleToken::Ident("other".to_string()),
  ]);
  let result = (BasicShape::inset_parser().run)(&mut tl);
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(
    msg.contains("closing paren") || msg.contains("Expected"),
    "msg: {msg}"
  );
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
fn circle_parser_wrong_function_name() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("inset".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::circle_parser().run)(&mut tl);
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(msg.contains("Expected circle"), "msg: {msg}");
}

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
fn circle_parser_closest_side_via_tokens() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("circle".to_string()),
    SimpleToken::Ident("closest-side".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::circle_parser().run)(&mut tl);
  assert!(result.is_ok(), "err: {:?}", result);
  assert_eq!(
    result.unwrap(),
    BasicShape::Circle {
      radius: CircleRadius::ClosestSide,
      position: None,
    }
  );
}

#[test]
fn circle_parser_farthest_side_via_tokens() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("circle".to_string()),
    SimpleToken::Ident("farthest-side".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::circle_parser().run)(&mut tl);
  assert!(result.is_ok(), "err: {:?}", result);
  assert_eq!(
    result.unwrap(),
    BasicShape::Circle {
      radius: CircleRadius::FarthestSide,
      position: None,
    }
  );
}

#[test]
fn circle_parser_whitespace_after_radius() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("circle".to_string()),
    SimpleToken::Dimension {
      value: 10.0,
      unit: "px".to_string(),
    },
    SimpleToken::Whitespace,
    SimpleToken::Whitespace,
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::circle_parser().run)(&mut tl);
  assert!(result.is_ok(), "err: {:?}", result);
}

#[test]
fn circle_parser_wrong_closing_token() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("circle".to_string()),
    SimpleToken::Dimension {
      value: 10.0,
      unit: "px".to_string(),
    },
    SimpleToken::Ident("bad".to_string()),
  ]);
  let result = (BasicShape::circle_parser().run)(&mut tl);
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(msg.contains("Expected closing paren"), "msg: {msg}");
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
fn ellipse_parser_wrong_function_name() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("circle".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::ellipse_parser().run)(&mut tl);
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(msg.contains("Expected ellipse"), "msg: {msg}");
}

#[test]
fn ellipse_parser_eof_at_start() {
  let mut tl = make_token_list(vec![]);
  let result = (BasicShape::ellipse_parser().run)(&mut tl);
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(msg.contains("end of input"), "msg: {msg}");
}

#[test]
fn ellipse_parser_closest_side_radius_x() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("ellipse".to_string()),
    SimpleToken::Ident("closest-side".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Ident("farthest-side".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::ellipse_parser().run)(&mut tl);
  assert!(result.is_ok(), "err: {:?}", result);
  if let BasicShape::Ellipse {
    radius_x, radius_y, ..
  } = result.unwrap()
  {
    assert_eq!(radius_x, CircleRadius::ClosestSide);
    assert_eq!(radius_y, CircleRadius::FarthestSide);
  }
}

#[test]
fn ellipse_parser_farthest_side_radius_x() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("ellipse".to_string()),
    SimpleToken::Ident("farthest-side".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Ident("closest-side".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::ellipse_parser().run)(&mut tl);
  assert!(result.is_ok(), "err: {:?}", result);
  if let BasicShape::Ellipse {
    radius_x, radius_y, ..
  } = result.unwrap()
  {
    assert_eq!(radius_x, CircleRadius::FarthestSide);
    assert_eq!(radius_y, CircleRadius::ClosestSide);
  }
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
fn ellipse_parser_whitespace_between_radii() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("ellipse".to_string()),
    SimpleToken::Dimension {
      value: 10.0,
      unit: "px".to_string(),
    },
    SimpleToken::Whitespace,
    SimpleToken::Whitespace,
    SimpleToken::Dimension {
      value: 20.0,
      unit: "px".to_string(),
    },
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::ellipse_parser().run)(&mut tl);
  assert!(result.is_ok(), "err: {:?}", result);
}

#[test]
fn ellipse_parser_whitespace_after_radius_y() {
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
    SimpleToken::Whitespace,
    SimpleToken::Whitespace,
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::ellipse_parser().run)(&mut tl);
  assert!(result.is_ok(), "err: {:?}", result);
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
fn polygon_parser_wrong_function_name() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("path".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::polygon_parser().run)(&mut tl);
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(msg.contains("Expected polygon"), "msg: {msg}");
}

#[test]
fn polygon_parser_eof_at_start() {
  let mut tl = make_token_list(vec![]);
  let result = (BasicShape::polygon_parser().run)(&mut tl);
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(msg.contains("end of input"), "msg: {msg}");
}

#[test]
fn polygon_parser_whitespace_after_open_paren() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("polygon".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Percentage(0.0),
    SimpleToken::Whitespace,
    SimpleToken::Percentage(0.0),
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::polygon_parser().run)(&mut tl);
  assert!(result.is_ok(), "err: {:?}", result);
}

#[test]
fn polygon_parser_nonzero_fill_rule() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("polygon".to_string()),
    SimpleToken::Ident("nonzero".to_string()),
    SimpleToken::Comma,
    SimpleToken::Whitespace,
    SimpleToken::Percentage(0.0),
    SimpleToken::Whitespace,
    SimpleToken::Percentage(0.0),
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::polygon_parser().run)(&mut tl);
  assert!(result.is_ok(), "err: {:?}", result);
  if let BasicShape::Polygon { fill_rule, .. } = result.unwrap() {
    assert_eq!(fill_rule, Some("nonzero".to_string()));
  }
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

#[test]
fn polygon_parser_comma_after_fill_rule() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("polygon".to_string()),
    SimpleToken::Ident("nonzero".to_string()),
    SimpleToken::Comma,
    SimpleToken::Percentage(0.0),
    SimpleToken::Whitespace,
    SimpleToken::Percentage(100.0),
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::polygon_parser().run)(&mut tl);
  assert!(result.is_ok(), "err: {:?}", result);
}

#[test]
fn polygon_parser_whitespace_after_fill_rule_comma() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("polygon".to_string()),
    SimpleToken::Ident("evenodd".to_string()),
    SimpleToken::Comma,
    SimpleToken::Whitespace,
    SimpleToken::Whitespace,
    SimpleToken::Percentage(0.0),
    SimpleToken::Whitespace,
    SimpleToken::Percentage(0.0),
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::polygon_parser().run)(&mut tl);
  assert!(result.is_ok(), "err: {:?}", result);
}

// The fill-rule `else` fallback is unreachable: the peek guard ensures an Ident.

#[test]
fn polygon_parser_unknown_ident_defaults_fill_rule() {
  let result = BasicShape::parse().parse_to_end("polygon(other, 0% 0%)");
  let _ = result;
}

#[test]
fn polygon_parser_no_ident_defaults_fill_rule() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("polygon".to_string()),
    SimpleToken::Percentage(0.0),
    SimpleToken::Whitespace,
    SimpleToken::Percentage(0.0),
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::polygon_parser().run)(&mut tl);
  assert!(result.is_ok(), "err: {:?}", result);
  if let BasicShape::Polygon { fill_rule, .. } = result.unwrap() {
    assert_eq!(fill_rule, Some("nonzero".to_string()));
  }
}

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
fn polygon_parser_whitespace_between_x_and_y() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("polygon".to_string()),
    SimpleToken::Percentage(10.0),
    SimpleToken::Whitespace,
    SimpleToken::Whitespace,
    SimpleToken::Percentage(20.0),
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
fn polygon_parser_comma_between_points() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("polygon".to_string()),
    SimpleToken::Percentage(0.0),
    SimpleToken::Whitespace,
    SimpleToken::Percentage(0.0),
    SimpleToken::Comma,
    SimpleToken::Percentage(100.0),
    SimpleToken::Whitespace,
    SimpleToken::Percentage(100.0),
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::polygon_parser().run)(&mut tl);
  assert!(result.is_ok(), "err: {:?}", result);
  if let BasicShape::Polygon { points, .. } = result.unwrap() {
    assert_eq!(points.len(), 2);
  }
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

#[test]
fn polygon_parser_wrong_closing_token_after_loop() {
  // The closing-paren error arms after the loop are unreachable: the loop breaks on a peeked `)`.
  let result = BasicShape::parse().parse_to_end("polygon(0% 0%, 100% 100%)");
  assert!(result.is_ok());
}

// ── path_parser ──

#[test]
fn path_parser_wrong_function_name() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("polygon".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::path_parser().run)(&mut tl);
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(msg.contains("Expected path"), "msg: {msg}");
}

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
fn path_parser_nonzero_fill_rule() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("path".to_string()),
    SimpleToken::Ident("nonzero".to_string()),
    SimpleToken::Comma,
    SimpleToken::String("M0 0".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::path_parser().run)(&mut tl);
  assert!(result.is_ok(), "err: {:?}", result);
  if let BasicShape::Path { fill_rule, .. } = result.unwrap() {
    assert_eq!(fill_rule, Some("nonzero".to_string()));
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
fn path_parser_comma_after_fill_rule() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("path".to_string()),
    SimpleToken::Ident("evenodd".to_string()),
    SimpleToken::Comma,
    SimpleToken::String("M10 10".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::path_parser().run)(&mut tl);
  assert!(result.is_ok(), "err: {:?}", result);
}

#[test]
fn path_parser_whitespace_after_fill_rule_comma() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("path".to_string()),
    SimpleToken::Ident("nonzero".to_string()),
    SimpleToken::Comma,
    SimpleToken::Whitespace,
    SimpleToken::Whitespace,
    SimpleToken::String("M0 0".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::path_parser().run)(&mut tl);
  assert!(result.is_ok(), "err: {:?}", result);
}

#[test]
fn path_parser_unknown_ident_defaults_fill_rule() {
  let result = BasicShape::parse().parse_to_end("path(other, \"M0 0\")");
  let _ = result;
}

#[test]
fn path_parser_non_ident_defaults_fill_rule() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("path".to_string()),
    SimpleToken::String("M0 0 L10 10".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::path_parser().run)(&mut tl);
  assert!(result.is_ok(), "err: {:?}", result);
  if let BasicShape::Path { fill_rule, path } = result.unwrap() {
    assert_eq!(fill_rule, Some("nonzero".to_string()));
    assert_eq!(path, "M0 0 L10 10");
  }
}

#[test]
fn path_parser_whitespace_before_string() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("path".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Whitespace,
    SimpleToken::String("M0 0".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::path_parser().run)(&mut tl);
  assert!(result.is_ok(), "err: {:?}", result);
}

#[test]
fn path_parser_wrong_token_at_string() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("path".to_string()),
    SimpleToken::Ident("not-a-string".to_string()), // won't match as fill-rule
  ]);
  let result = (BasicShape::path_parser().run)(&mut tl);
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(
    msg.contains("Expected string literal") || msg.contains("path"),
    "msg: {msg}"
  );
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

#[test]
fn parse_optional_position_at_keyword_exercises_position_parser() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("circle".to_string()),
    SimpleToken::Dimension {
      value: 10.0,
      unit: "px".to_string(),
    },
    SimpleToken::Whitespace,
    SimpleToken::Ident("at".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Ident("center".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::circle_parser().run)(&mut tl);
  let _ = result;
}

#[test]
fn parse_optional_position_whitespace_after_at() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("circle".to_string()),
    SimpleToken::Dimension {
      value: 10.0,
      unit: "px".to_string(),
    },
    SimpleToken::Whitespace,
    SimpleToken::Ident("at".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Whitespace,
    SimpleToken::Ident("center".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::circle_parser().run)(&mut tl);
  let _ = result;
}

// ── basic_shape_parser() ──

#[test]
fn basic_shape_parser_free_function_inset() {
  let result = basic_shape_parser().parse_to_end("inset(10px)");
  assert!(result.is_ok(), "err: {:?}", result);
  assert!(matches!(result.unwrap(), BasicShape::Inset { .. }));
}

#[test]
fn basic_shape_parser_free_function_circle() {
  let result = basic_shape_parser().parse_to_end("circle(50%)");
  assert!(result.is_ok(), "err: {:?}", result);
  assert!(matches!(result.unwrap(), BasicShape::Circle { .. }));
}

#[test]
fn basic_shape_parser_free_function_ellipse() {
  let result = basic_shape_parser().parse_to_end("ellipse(50% 50%)");
  assert!(result.is_ok(), "err: {:?}", result);
  assert!(matches!(result.unwrap(), BasicShape::Ellipse { .. }));
}

#[test]
fn basic_shape_parser_free_function_polygon() {
  let result = basic_shape_parser().parse_to_end("polygon(0% 0%, 100% 0%, 50% 100%)");
  assert!(result.is_ok(), "err: {:?}", result);
  assert!(matches!(result.unwrap(), BasicShape::Polygon { .. }));
}

#[test]
fn basic_shape_parser_free_function_path() {
  let result = basic_shape_parser().parse_to_end("path(\"M0 0L10 10\")");
  assert!(result.is_ok(), "err: {:?}", result);
  assert!(matches!(result.unwrap(), BasicShape::Path { .. }));
}

// ── Display error propagation ──

#[test]
fn display_inset_all_branches_propagates_writer_errors() {
  let value = BasicShape::Inset {
    top: px(10.0),
    right: px(20.0),
    bottom: px(30.0),
    left: px(40.0),
    round: None,
  };
  let (mut saw_ok, mut saw_err) = (false, false);
  for remaining in 0..128 {
    match write!(FailAfter { remaining }, "{}", value) {
      Ok(()) => saw_ok = true,
      Err(_) => saw_err = true,
    }
  }
  assert!(saw_err, "expected at least one Err");
  assert!(
    saw_ok,
    "expected at least one Ok — increase upper bound of range"
  );
}

#[test]
fn display_inset_with_round_propagates_writer_errors() {
  let value = BasicShape::Inset {
    top: px(10.0),
    right: px(20.0),
    bottom: px(30.0),
    left: px(40.0),
    round: Some(px(5.0)),
  };
  let (mut saw_ok, mut saw_err) = (false, false);
  for remaining in 0..128 {
    match write!(FailAfter { remaining }, "{}", value) {
      Ok(()) => saw_ok = true,
      Err(_) => saw_err = true,
    }
  }
  assert!(saw_err, "expected at least one Err");
  assert!(
    saw_ok,
    "expected at least one Ok — increase upper bound of range"
  );
}

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
fn display_circle_propagates_writer_errors() {
  let value = BasicShape::Circle {
    radius: CircleRadius::Length(px(50.0)),
    position: None,
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
fn display_ellipse_propagates_writer_errors() {
  let value = BasicShape::Ellipse {
    radius_x: CircleRadius::ClosestSide,
    radius_y: CircleRadius::FarthestSide,
    position: None,
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
fn display_polygon_with_fill_rule_propagates_writer_errors() {
  let value = BasicShape::Polygon {
    fill_rule: Some("evenodd".to_string()),
    points: vec![(px(0.0), pct(0.0)), (pct(100.0), px(100.0))],
  };
  let (mut saw_ok, mut saw_err) = (false, false);
  for remaining in 0..64 {
    match write!(FailAfter { remaining }, "{}", value) {
      Ok(()) => saw_ok = true,
      Err(_) => saw_err = true,
    }
  }
  assert!(saw_ok && saw_err);
}

#[test]
fn display_polygon_no_fill_rule_propagates_writer_errors() {
  let value = BasicShape::Polygon {
    fill_rule: None,
    points: vec![
      (px(0.0), px(0.0)),
      (px(100.0), px(0.0)),
      (px(50.0), px(100.0)),
    ],
  };
  let (mut saw_ok, mut saw_err) = (false, false);
  for remaining in 0..64 {
    match write!(FailAfter { remaining }, "{}", value) {
      Ok(()) => saw_ok = true,
      Err(_) => saw_err = true,
    }
  }
  assert!(saw_ok && saw_err);
}

#[test]
fn display_path_with_fill_rule_propagates_writer_errors() {
  let value = BasicShape::Path {
    fill_rule: Some("evenodd".to_string()),
    path: "M0 0 L100 100 Z".to_string(),
  };
  let (mut saw_ok, mut saw_err) = (false, false);
  for remaining in 0..64 {
    match write!(FailAfter { remaining }, "{}", value) {
      Ok(()) => saw_ok = true,
      Err(_) => saw_err = true,
    }
  }
  assert!(saw_ok && saw_err);
}

#[test]
fn display_path_no_fill_rule_propagates_writer_errors() {
  let value = BasicShape::Path {
    fill_rule: None,
    path: "M0 0 L100 100".to_string(),
  };
  let (mut saw_ok, mut saw_err) = (false, false);
  for remaining in 0..64 {
    match write!(FailAfter { remaining }, "{}", value) {
      Ok(()) => saw_ok = true,
      Err(_) => saw_err = true,
    }
  }
  assert!(saw_ok && saw_err);
}

#[test]
fn display_circle_radius_length_propagates_writer_errors() {
  let value = CircleRadius::Length(px(42.0));
  let (mut saw_ok, mut saw_err) = (false, false);
  for remaining in 0..8 {
    match write!(FailAfter { remaining }, "{}", value) {
      Ok(()) => saw_ok = true,
      Err(_) => saw_err = true,
    }
  }
  assert!(saw_ok && saw_err);
}

#[test]
fn inset_display_single_value_all_equal_with_round_equal() {
  let value = BasicShape::Inset {
    top: px(10.0),
    right: px(10.0),
    bottom: px(10.0),
    left: px(10.0),
    round: Some(px(10.0)),
  };
  let s = value.to_string();
  assert!(s.starts_with("inset("), "s: {s}");
}

#[test]
fn inset_display_single_value_all_equal_no_round() {
  let value = BasicShape::Inset {
    top: px(5.0),
    right: px(5.0),
    bottom: px(5.0),
    left: px(5.0),
    round: None,
  };
  assert_eq!(value.to_string(), "inset(5px)");
}

#[test]
fn inset_display_two_values_with_round() {
  let value = BasicShape::Inset {
    top: px(10.0),
    right: px(20.0),
    bottom: px(10.0),
    left: px(20.0),
    round: Some(px(3.0)),
  };
  let s = value.to_string();
  assert!(s.contains("round"), "expected round in: {s}");
}

#[test]
fn inset_display_three_values_with_round() {
  let value = BasicShape::Inset {
    top: px(10.0),
    right: px(20.0),
    bottom: px(10.0),
    left: px(30.0),
    round: Some(px(2.0)),
  };
  let s = value.to_string();
  assert!(s.contains("round"), "expected round in: {s}");
}

#[test]
fn inset_display_four_values_with_round() {
  let value = BasicShape::Inset {
    top: px(1.0),
    right: px(2.0),
    bottom: px(3.0),
    left: px(4.0),
    round: Some(px(5.0)),
  };
  let s = value.to_string();
  assert!(s.contains("round"), "expected round in: {s}");
  assert!(s.contains("1px"), "expected 1px in: {s}");
}

#[test]
fn circle_radius_clone_and_debug() {
  let r = CircleRadius::ClosestSide;
  let r2 = r.clone();
  assert_eq!(r, r2);
  let _ = format!("{:?}", r);
}

#[test]
fn basic_shape_clone_and_debug() {
  let shape = BasicShape::Path {
    fill_rule: Some("nonzero".to_string()),
    path: "M0 0".to_string(),
  };
  let shape2 = shape.clone();
  assert_eq!(shape, shape2);
  let _ = format!("{:?}", shape);
}

#[test]
fn circle_radius_display_all_variants() {
  assert_eq!(CircleRadius::ClosestSide.to_string(), "closest-side");
  assert_eq!(CircleRadius::FarthestSide.to_string(), "farthest-side");
  assert_eq!(CircleRadius::Length(px(10.0)).to_string(), "10px");
}

#[test]
fn inset_parser_round_keyword_then_invalid_radius() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("inset".to_string()),
    SimpleToken::Dimension {
      value: 10.0,
      unit: "px".to_string(),
    },
    SimpleToken::Whitespace,
    SimpleToken::Ident("round".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Ident("bad-radius".to_string()), // not a valid length
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::inset_parser().run)(&mut tl);
  assert!(
    result.is_err(),
    "expected error when round radius is invalid"
  );
}

#[test]
fn ellipse_parser_at_position_propagates_error() {
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
    SimpleToken::Whitespace,
    SimpleToken::Ident("at".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::ellipse_parser().run)(&mut tl);
  let _ = result;
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
fn polygon_parser_invalid_y_coordinate() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("polygon".to_string()),
    SimpleToken::Percentage(0.0), // valid x
    SimpleToken::Whitespace,
    SimpleToken::Ident("bad-y".to_string()), // invalid y
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::polygon_parser().run)(&mut tl);
  assert!(result.is_err(), "expected error for invalid y coordinate");
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

#[test]
fn path_parser_whitespace_before_string_explicit() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("path".to_string()),
    SimpleToken::Whitespace, // consumed at WS loop 602
    SimpleToken::Whitespace, // consumed at WS loop 602
    SimpleToken::String("M10 20".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::path_parser().run)(&mut tl);
  assert!(result.is_ok(), "err: {:?}", result);
  if let BasicShape::Path { path, fill_rule } = result.unwrap() {
    assert_eq!(path, "M10 20");
    assert_eq!(fill_rule, Some("nonzero".to_string()));
  }
}

// The leading-whitespace guard in `parse_optional_position` is unreachable via circle/ellipse.

#[test]
fn parse_optional_position_whitespace_after_at_explicit() {
  let mut tl = make_token_list(vec![
    SimpleToken::Function("circle".to_string()),
    SimpleToken::Dimension {
      value: 10.0,
      unit: "px".to_string(),
    },
    SimpleToken::Ident("at".to_string()),
    SimpleToken::Whitespace, // 665: this WS is consumed
    SimpleToken::Ident("center".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (BasicShape::circle_parser().run)(&mut tl);
  let _ = result;
}

#[test]
fn parse_optional_position_skips_leading_whitespace() {
  // Callers drain whitespace first, so this guard is only reachable directly.
  let mut tokens = make_token_list(vec![SimpleToken::Whitespace]);
  let result = BasicShape::parse_optional_position(&mut tokens);
  assert_eq!(result.unwrap(), None);
}
