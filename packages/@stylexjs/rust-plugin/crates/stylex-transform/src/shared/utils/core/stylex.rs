use crate::shared::enums::data_structures::fn_result::FnResult;
use stylex_ast::ast::convertors::create_string_expr;

use super::{parse_nullable_style::ResolvedArg, styleq::styleq};

/// The merged class name string a `stylex(...)` call is replaced by.
pub(crate) fn stylex(values: &[ResolvedArg]) -> FnResult {
  FnResult::ClassName(create_string_expr(styleq(values).class_name.as_str()))
}
