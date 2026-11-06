use stylex_enums::declaration_type::DeclarationType;
use swc_core::ecma::ast::{Expr, Ident};

use crate::state::EvaluationState;
use stylex_constants::constants::evaluation_errors::{UNDEFINED_CONST, unsupported_expression};
use stylex_state::{evaluate_result_value::EvaluateResultValue, state_manager::StateManager};

use super::evaluate::{deopt, deopt_at_declaration};

/// The tail of the reference chain (`evaluate-path.js:685-690`, 0.19.0): a name
/// whose binding holds no initializer to fold is refused for the kind of
/// declaration it names, or for naming nothing this compiler can resolve.
///
/// The two arms report against different nodes, and upstream's do too. A
/// declaration kind is refused where upstream refuses it — inside
/// `evaluateCached` over the node `path.resolve()` handed back, which is the
/// declaration — so its frame names the `function` or `class` line. Measured on
/// 0.19.0, `create({ x: { color: f } })` under `function f() {}` frames the
/// declaration and carries a caret over the whole of it. `UNDEFINED_CONST` is
/// upstream's one refusal on the reference itself (`:687`), because by then
/// there is no declaration to name: the reference resolved to itself.
///
/// Takes the kind already resolved rather than the lists to search. The refusal
/// writes to the state, and a `&[Ident]` borrowed out of it cannot be held open
/// across that write -- so passing the lists meant cloning both of them, on a
/// path the comment there called rare and which is in fact how every dynamic
/// style's parameter is detected. A `DeclarationType` borrows nothing.
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
    None => deopt(path, state, UNDEFINED_CONST),
  }
}
