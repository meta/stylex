//! One module per StyleX API. Owner: surface.
//!
//! Each top-level API call found in pass 1 dispatches to a `Cx` method here, and pass 2's
//! visitor delegates props/attrs/sx and marker calls to `Pass2` methods here.

pub mod atoms;
pub mod consts;
pub mod create;
pub mod dynamic;
pub mod keyframes;
pub mod position_try;
pub mod props;
pub mod vars;
pub mod view_transition;
pub mod when_markers;

use oxc_ast::ast::*;

use crate::diagnostics::{self as err, Res, bail};

pub(crate) fn first_arg<'b, 'a>(c: &'b CallExpression<'a>, name: &str) -> Res<&'b Expression<'a>> {
  if c.arguments.len() != 1 {
    bail(&err::arg_len(name, 1))?;
  }
  match c.arguments[0].as_expression() {
    Some(e) => Ok(e),
    None => bail(&err::arg_len(name, 1)),
  }
}
