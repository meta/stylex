use swc_core::ecma::ast::Id;

use stylex_enums::style_vars_to_keep::{NonNullProp, NonNullProps};

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct StyleVarsToKeep(pub Id, pub NonNullProp, pub NonNullProps);
