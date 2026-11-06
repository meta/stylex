use swc_core::ecma::ast::Expr;

use crate::raw_value::TRawValue;

/// A style value becoming a CSS declaration: `string | number | Array<string | number>`.
/// `Raw` keeps the JS type, which decides units: `width: 1` is `1px`, `width: '1'` is `1`.
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
