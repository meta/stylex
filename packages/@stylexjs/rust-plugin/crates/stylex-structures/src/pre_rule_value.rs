use swc_core::ecma::ast::Expr;

use crate::raw_value::TRawValue;

/// A style value on its way to becoming a CSS declaration:
/// `string | number | Array<string | number>`.
///
/// `Raw` keeps the authored JS type, which decides whether a unit suffix is
/// appended: `width: 1` compiles to `1px`, `width: '1'` to `1`.
#[derive(Debug, Clone, PartialEq)]
pub enum PreRuleValue {
  Expr(Expr),
  Raw(TRawValue),
  Vec(Vec<TRawValue>),
  Null,
}

impl PreRuleValue {
  pub fn string(value: impl Into<String>) -> Self {
    PreRuleValue::Raw(TRawValue::String(value.into()))
  }

  pub fn number(value: f64) -> Self {
    PreRuleValue::Raw(TRawValue::Number(value))
  }
}

#[cfg(test)]
#[path = "tests/pre_rule_value_test.rs"]
mod tests;
