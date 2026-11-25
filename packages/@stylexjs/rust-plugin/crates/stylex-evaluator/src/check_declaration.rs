use stylex_enums::declaration_type::DeclarationType;
use swc_core::ecma::ast::{Expr, Ident};

use crate::state::EvaluationState;
use stylex_constants::constants::evaluation_errors::{UNDEFINED_CONST, unsupported_expression};
use stylex_state::{evaluate_result_value::EvaluateResultValue, state_manager::StateManager};

use super::evaluate::{deopt, deopt_at_declaration};

/// The tail of the reference chain (`evaluate-path.js:685-690`, 0.19.0): a name whose binding holds no initializer to fold is refused for the kind of declaration it names, or for naming nothing this compiler can resolve.
pub(crate) fn check_ident_declaration(
  ident: &Ident,
  declared_as: Option<DeclarationType>,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  path: &Expr,
) -> Option<EvaluateResultValue> {
  match declared_as {
    Some(decl_type) => deopt_at_declaration(
      path,
      &ident.sym,
      state,
      traversal_state,
      &match decl_type {
        DeclarationType::Class => unsupported_expression("ClassDeclaration"),
        DeclarationType::Function => unsupported_expression("FunctionDeclaration"),
      },
    ),
    None => {
      if let Some(fault) = traversal_state.opaque_declaration_fault(ident) {
        let declaration = Expr::Invalid(swc_core::ecma::ast::Invalid { span: fault.span });
        return deopt(
          &declaration,
          state,
          &unsupported_expression(fault.kind.babel_node_kind()),
        );
      }
      deopt(path, state, UNDEFINED_CONST)
    },
  }
}
