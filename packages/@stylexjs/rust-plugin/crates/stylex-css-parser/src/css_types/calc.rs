/*!
CSS Calc type parsing with full arithmetic support.

Implements complete calc() expression parsing with operator precedence.
*/

use stylex_macros::stylex_unreachable;
use stylex_utils::number::{to_js_string, write_js_number};

use crate::{
  CssParseError,
  css_types::{calc_constant::CalcConstant, common_types::Percentage},
  token_parser::TokenParser,
  token_types::{SimpleToken, TokenList},
};
use std::fmt::{self, Display};

/// Dimension with value and unit for calc expressions
#[derive(Debug, Clone, PartialEq)]
pub struct CalcDimension {
  pub value: f64,
  pub unit: String,
}

impl CalcDimension {
  pub fn new(value: f64, unit: impl Into<String>) -> Self {
    Self {
      value,
      unit: unit.into(),
    }
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for CalcDimension {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}{}", to_js_string(self.value), self.unit)
  }
}

/// Addition operation: { type: '+', left: CalcValue, right: CalcValue }
#[derive(Debug, Clone, PartialEq)]
pub struct Addition {
  pub left: Box<CalcValue>,
  pub right: Box<CalcValue>,
}

impl Addition {
  pub fn new(left: CalcValue, right: CalcValue) -> Self {
    Self {
      left: Box::new(left),
      right: Box::new(right),
    }
  }
}

/// Subtraction operation: { type: '-', left: CalcValue, right: CalcValue }
#[derive(Debug, Clone, PartialEq)]
pub struct Subtraction {
  pub left: Box<CalcValue>,
  pub right: Box<CalcValue>,
}

impl Subtraction {
  pub fn new(left: CalcValue, right: CalcValue) -> Self {
    Self {
      left: Box::new(left),
      right: Box::new(right),
    }
  }
}

/// Multiplication operation: { type: '*', left: CalcValue, right: CalcValue }
#[derive(Debug, Clone, PartialEq)]
pub struct Multiplication {
  pub left: Box<CalcValue>,
  pub right: Box<CalcValue>,
}

impl Multiplication {
  pub fn new(left: CalcValue, right: CalcValue) -> Self {
    Self {
      left: Box::new(left),
      right: Box::new(right),
    }
  }
}

/// Division operation: { type: '/', left: CalcValue, right: CalcValue }
#[derive(Debug, Clone, PartialEq)]
pub struct Division {
  pub left: Box<CalcValue>,
  pub right: Box<CalcValue>,
}

impl Division {
  pub fn new(left: CalcValue, right: CalcValue) -> Self {
    Self {
      left: Box::new(left),
      right: Box::new(right),
    }
  }
}

/// Group (parenthesized expression): { type: 'group', expr: CalcValue }
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
  pub expr: Box<CalcValue>,
}

impl Group {
  pub fn new(expr: CalcValue) -> Self {
    Self {
      expr: Box::new(expr),
    }
  }
}

/// Union type for calc values and operators during parsing
#[derive(Debug, Clone, PartialEq)]
pub enum CalcValueOrOperator {
  Value(CalcValue),
  Operator(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum CalcValue {
  /// number
  Number(f64),
  /// `TokenDimension[4]`
  Dimension(CalcDimension),
  /// Percentage
  Percentage(Percentage),
  /// CalcConstant
  Constant(CalcConstant),
  /// Addition
  Addition(Addition),
  /// Subtraction
  Subtraction(Subtraction),
  /// Multiplication
  Multiplication(Multiplication),
  /// Division
  Division(Division),
  /// Group
  Group(Group),
}

fn skip_whitespace(tokens: &mut TokenList) -> bool {
  let mut consumed = false;
  while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
    tokens.consume_next_token_infallible();
    consumed = true;
  }
  consumed
}

impl CalcValue {
  /// Parser for individual calc values (no operators)
  pub fn value_parser() -> TokenParser<CalcValue> {
    TokenParser::new(Self::parse_calc_value, "calc_value")
  }

  /// Helper: Parse a basic calc value (number, dimension, percentage, or
  /// constant)
  fn parse_calc_value(tokens: &mut TokenList) -> Result<CalcValue, CssParseError> {
    let token = tokens
      .consume_next_token_infallible()
      .ok_or(CssParseError::ParseError {
        message: "Expected Number, Dimension, Percentage, or Constant token".to_string(),
      })?;

    match token {
      SimpleToken::Number(value) => Ok(CalcValue::Number(value)),
      SimpleToken::Dimension { value, unit } => {
        Ok(CalcValue::Dimension(CalcDimension::new(value, unit)))
      },
      SimpleToken::Percentage(value) => {
        // The token already carries the authored percent: `50%` is `50`.
        Ok(CalcValue::Percentage(Percentage::new(value)))
      },
      SimpleToken::Ident(name) => {
        // Try to parse as calc constant (pi, e, infinity, -infinity, NaN)
        match CalcConstant::parse(&name) {
          Some(constant) => Ok(CalcValue::Constant(constant)),
          None => Err(CssParseError::ParseError {
            message: format!("Unknown calc constant: {}", name),
          }),
        }
      },
      SimpleToken::Function(name) if name == "calc" => {
        let value = Self::parse_calc_expression(tokens)?;
        let close_token =
          tokens
            .consume_next_token_infallible()
            .ok_or(CssParseError::ParseError {
              message: "Expected closing parenthesis".to_string(),
            })?;

        if matches!(close_token, SimpleToken::RightParen) {
          Ok(value)
        } else {
          Err(CssParseError::ParseError {
            message: "Expected closing parenthesis".to_string(),
          })
        }
      },
      _ => Err(CssParseError::ParseError {
        message: format!(
          "Expected Number, Dimension, Percentage, or Constant token, got {:?}",
          token
        ),
      }),
    }
  }

  pub fn parser() -> TokenParser<CalcValue> {
    TokenParser::new(Self::parse_calc_expression, "calc_expression")
  }

  /// Parse calc expression with operator precedence
  fn parse_calc_expression(tokens: &mut TokenList) -> Result<CalcValue, CssParseError> {
    // Skip any leading whitespace
    skip_whitespace(tokens);

    // Parse first value or group
    let first_value = match Self::try_parse_parenthesized_group(tokens) {
      Ok(grouped) => CalcValue::Group(grouped),
      Err(_) => Self::parse_calc_value(tokens)?,
    };

    // Collect values and operators
    let mut values_and_operators = vec![CalcValueOrOperator::Value(first_value)];

    loop {
      // Skip whitespace
      let had_ws_before = skip_whitespace(tokens);

      // Check if we're at the end of the expression (RightParen means end for calc)
      if let Ok(Some(SimpleToken::RightParen)) = tokens.peek() {
        // Don't consume the RightParen - let the parent parser handle it
        break;
      }

      // Try to parse an operator
      let checkpoint = tokens.current_index;
      match Self::try_parse_operator(tokens) {
        Ok(operator) => {
          let requires_surrounding_ws = matches!(operator.as_str(), "+" | "-");
          values_and_operators.push(CalcValueOrOperator::Operator(operator));

          // Skip whitespace after operator
          let had_ws_after = skip_whitespace(tokens);
          if requires_surrounding_ws && (!had_ws_before || !had_ws_after) {
            return Err(CssParseError::ParseError {
              message: "calc '+' and '-' operators require whitespace on both sides".to_string(),
            });
          }

          // Parse next value or group
          let next_value = match Self::try_parse_parenthesized_group(tokens) {
            Ok(grouped) => CalcValue::Group(grouped),
            Err(_) => Self::parse_calc_value(tokens)?,
          };
          values_and_operators.push(CalcValueOrOperator::Value(next_value));
        },
        Err(_) => {
          // No operator found, we're done
          tokens.set_current_index(checkpoint);
          break;
        },
      }
    }

    // Trailing whitespace before the terminator is already consumed by the
    // whitespace skip at the top of the loop above, so no extra skip is needed
    // here.

    // Apply operator precedence
    if values_and_operators.len() == 1 {
      Self::extract_single_calc_value(&values_and_operators[0])
    } else {
      Self::split_by_multiplication_or_division(values_and_operators)
    }
  }

  /// Extract the CalcValue from a single CalcValueOrOperator.
  /// The first element of values_and_operators is always a Value (guaranteed by
  /// the parser), so the else arm is a defensive `stylex_unreachable!`.
  fn extract_single_calc_value(item: &CalcValueOrOperator) -> Result<CalcValue, CssParseError> {
    if let CalcValueOrOperator::Value(value) = item {
      Ok(value.clone())
    } else {
      stylex_unreachable!("First element should always be a value")
    }
  }

  /// Apply operator precedence: multiplication/division first, then
  /// addition/subtraction
  fn split_by_multiplication_or_division(
    values_and_operators: Vec<CalcValueOrOperator>,
  ) -> Result<CalcValue, CssParseError> {
    if values_and_operators.len() == 1 {
      match &values_and_operators[0] {
        CalcValueOrOperator::Value(value) => return Ok(value.clone()),
        CalcValueOrOperator::Operator(_) => {
          return Err(CssParseError::ParseError {
            message: "Invalid operator".to_string(),
          });
        },
      }
    }

    // Find first * or / operator
    let first_operator = values_and_operators
      .iter()
      .position(|item| matches!(item, CalcValueOrOperator::Operator(op) if op == "*" || op == "/"));

    match first_operator {
      None => {
        // No * or / found, handle + and -
        Self::compose_add_and_subtraction(values_and_operators)
      },
      Some(op_index) => {
        let left_slice = values_and_operators[..op_index].to_vec();
        let right_slice = values_and_operators[op_index + 1..].to_vec();

        let left = Self::compose_add_and_subtraction(left_slice)?;
        let right = Self::split_by_multiplication_or_division(right_slice)?;

        Self::apply_mult_div_operator(&values_and_operators[op_index], left, right)
      },
    }
  }

  /// Apply a `*` or `/` operator to two already-parsed values.
  ///
  /// `op` is the element at the index returned by `position()`, which only
  /// matches `CalcValueOrOperator::Operator(s)` where `s` is `"*"` or `"/"`.
  /// The other arms are defensive guards; they are extracted as a named
  /// function so they can be reached by a `#[should_panic]` test.
  fn apply_mult_div_operator(
    op: &CalcValueOrOperator,
    left: CalcValue,
    right: CalcValue,
  ) -> Result<CalcValue, CssParseError> {
    match op {
      CalcValueOrOperator::Operator(operator) => match operator.as_str() {
        "*" => Ok(CalcValue::Multiplication(Multiplication::new(left, right))),
        "/" => Ok(CalcValue::Division(Division::new(left, right))),
        _ => stylex_unreachable!(
          "position() only matches '*' or '/' operators; got '{}'",
          operator
        ),
      },
      CalcValueOrOperator::Value(_) => {
        stylex_unreachable!("position() guarantees a CalcValueOrOperator::Operator at op_index")
      },
    }
  }

  /// Handle addition and subtraction operations
  fn compose_add_and_subtraction(
    values_and_operators: Vec<CalcValueOrOperator>,
  ) -> Result<CalcValue, CssParseError> {
    if values_and_operators.len() == 1 {
      match &values_and_operators[0] {
        CalcValueOrOperator::Value(value) => return Ok(value.clone()),
        CalcValueOrOperator::Operator(op) => {
          return Err(CssParseError::ParseError {
            message: format!("Invalid operator: {}", op),
          });
        },
      }
    }

    // Find first + or - operator
    let first_operator = values_and_operators
      .iter()
      .position(|item| matches!(item, CalcValueOrOperator::Operator(op) if op == "+" || op == "-"));

    match first_operator {
      None => Err(CssParseError::ParseError {
        message: "No valid operator found".to_string(),
      }),
      Some(op_index) => {
        let left_slice = values_and_operators[..op_index].to_vec();
        let right_slice = values_and_operators[op_index + 1..].to_vec();

        let left = Self::compose_add_and_subtraction(left_slice)?;
        let right = Self::compose_add_and_subtraction(right_slice)?;

        Self::apply_add_sub_operator(&values_and_operators[op_index], left, right)
      },
    }
  }

  /// Apply a `+` or `-` operator to two already-parsed values.
  ///
  /// `op` is the element at the index returned by `position()`, which only
  /// matches `CalcValueOrOperator::Operator(s)` where `s` is `"+"` or `"-"`.
  /// The other arms are defensive guards extracted so they can be covered by
  /// `#[should_panic]` tests.
  fn apply_add_sub_operator(
    op: &CalcValueOrOperator,
    left: CalcValue,
    right: CalcValue,
  ) -> Result<CalcValue, CssParseError> {
    match op {
      CalcValueOrOperator::Operator(operator) => match operator.as_str() {
        "+" => Ok(CalcValue::Addition(Addition::new(left, right))),
        "-" => Ok(CalcValue::Subtraction(Subtraction::new(left, right))),
        _ => stylex_unreachable!(
          "position() only matches '+' or '-' operators; got '{}'",
          operator
        ),
      },
      CalcValueOrOperator::Value(_) => {
        stylex_unreachable!("position() guarantees a CalcValueOrOperator::Operator at op_index")
      },
    }
  }

  /// Try to parse a parenthesized group
  fn try_parse_parenthesized_group(tokens: &mut TokenList) -> Result<Group, CssParseError> {
    // Save checkpoint in case we need to rollback
    let checkpoint = tokens.current_index;

    // Expect '('
    let open_paren_token =
      tokens
        .consume_next_token_infallible()
        .ok_or(CssParseError::ParseError {
          message: "Expected opening parenthesis".to_string(),
        })?;

    if !matches!(open_paren_token, SimpleToken::LeftParen) {
      // Rollback and return error
      tokens.set_current_index(checkpoint);
      return Err(CssParseError::ParseError {
        message: "Expected '(' token".to_string(),
      });
    }

    // Skip optional whitespace
    skip_whitespace(tokens);

    // Parse the inner expression recursively. Its own trailing whitespace is
    // consumed by parse_calc_expression's internal loop, so the position is left
    // at the closing parenthesis.
    let inner_expr = Self::parse_calc_expression(tokens)?;

    // Expect ')'
    let close_paren_token =
      tokens
        .consume_next_token_infallible()
        .ok_or(CssParseError::ParseError {
          message: "Expected closing parenthesis".to_string(),
        })?;

    if !matches!(close_paren_token, SimpleToken::RightParen) {
      return Err(CssParseError::ParseError {
        message: "Expected ')' token".to_string(),
      });
    }

    Ok(Group::new(inner_expr))
  }

  /// Try to parse an operator token
  fn try_parse_operator(tokens: &mut TokenList) -> Result<String, CssParseError> {
    let token = tokens
      .consume_next_token_infallible()
      .ok_or(CssParseError::ParseError {
        message: "Expected operator token".to_string(),
      })?;

    match token {
      SimpleToken::Delim('+') => Ok("+".to_string()),
      SimpleToken::Delim('-') => Ok("-".to_string()),
      SimpleToken::Delim('*') => Ok("*".to_string()),
      SimpleToken::Delim('/') => Ok("/".to_string()),
      _ => Err(CssParseError::ParseError {
        message: format!("Expected operator (+, -, *, /), got {:?}", token),
      }),
    }
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for CalcValue {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      CalcValue::Number(n) => write_js_number(f, *n),
      CalcValue::Dimension(d) => write!(f, "{}", d),
      CalcValue::Percentage(p) => write!(f, "{}", p),
      CalcValue::Constant(c) => write!(f, "{}", c),
      CalcValue::Addition(op) => write!(f, "{} + {}", op.left, op.right),
      CalcValue::Subtraction(op) => write!(f, "{} - {}", op.left, op.right),
      CalcValue::Multiplication(op) => write!(f, "{} * {}", op.left, op.right),
      CalcValue::Division(op) => write!(f, "{} / {}", op.left, op.right),
      CalcValue::Group(group) => write!(f, "({})", group.expr),
    }
  }
}

/// Main Calc expression container
#[derive(Debug, Clone, PartialEq)]
pub struct Calc {
  pub value: CalcValue,
}

impl Calc {
  /// Create a new calc expression
  pub fn new(value: CalcValue) -> Self {
    Calc { value }
  }

  /// Parse method for compatibility with existing tests
  pub fn parse() -> TokenParser<Calc> {
    Self::parser()
  }

  /// Parser for calc() expressions
  pub fn parser() -> TokenParser<Calc> {
    // Custom parser that handles the specific calc() tokenization structure
    TokenParser::new(Self::parse_calc, "calc_parser")
  }

  /// Parse a `calc(...)` token stream: `Function("calc")`, the inner expression,
  /// then the closing parenthesis.
  fn parse_calc(tokens: &mut TokenList) -> Result<Calc, CssParseError> {
    // First, expect Function("calc")
    let token = tokens
      .consume_next_token_infallible()
      .ok_or(CssParseError::ParseError {
        message: "Expected calc function".to_string(),
      })?;

    if let SimpleToken::Function(fn_name) = token {
      if fn_name != "calc" {
        return Err(CssParseError::ParseError {
          message: format!("Expected calc function, got {}", fn_name),
        });
      }
    } else {
      return Err(CssParseError::ParseError {
        message: "Expected function token".to_string(),
      });
    }

    // Parse the calc expression content (everything until RightParen)
    let calc_value = CalcValue::parse_calc_expression(tokens)?;

    // Consume the closing RightParen token
    let close_token = tokens
      .consume_next_token_infallible()
      .ok_or(CssParseError::ParseError {
        message: "Expected closing parenthesis".to_string(),
      })?;

    if !matches!(close_token, SimpleToken::RightParen) {
      return Err(CssParseError::ParseError {
        message: "Expected closing parenthesis".to_string(),
      });
    }

    Ok(Calc::new(calc_value))
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for Calc {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "calc({})", self.value)
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
pub fn calc_value_to_string(value: &CalcValue) -> String {
  match value {
    CalcValue::Number(n) => to_js_string(*n),
    CalcValue::Dimension(d) => d.to_string(),
    CalcValue::Percentage(p) => p.to_string(),
    CalcValue::Constant(c) => c.to_string(),
    CalcValue::Addition(op) => format!(
      "{} + {}",
      calc_value_to_string(&op.left),
      calc_value_to_string(&op.right)
    ),
    CalcValue::Subtraction(op) => format!(
      "{} - {}",
      calc_value_to_string(&op.left),
      calc_value_to_string(&op.right)
    ),
    CalcValue::Multiplication(op) => format!(
      "{} * {}",
      calc_value_to_string(&op.left),
      calc_value_to_string(&op.right)
    ),
    CalcValue::Division(op) => format!(
      "{} / {}",
      calc_value_to_string(&op.left),
      calc_value_to_string(&op.right)
    ),
    CalcValue::Group(group) => format!("({})", calc_value_to_string(&group.expr)),
  }
}

// Tests are in calc_parsing_tests.rs

#[cfg(test)]
#[path = "../tests/css_types/calc_test.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/css_types/calc_coverage_test.rs"]
mod calc_coverage_test;
