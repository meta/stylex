use std::rc::Rc;

use swc_core::{
  atoms::Atom,
  ecma::{
    ast::{Expr, Lit, MemberExpr, ObjectLit, PropOrSpread},
    visit::{Visit, noop_visit_type},
  },
};

use stylex_enums::style_vars_to_keep::{NonNullProp, NonNullProps};
use stylex_structures::style_vars_to_keep::StyleVarsToKeep;

use stylex_ast::ast::keys::{named_key_value, namespace_name_from_member_prop};

use stylex_evaluator::evaluate::evaluate_with_functions;
use stylex_state::{
  evaluate_result_value::EvaluateResultValue,
  functions::FunctionMap,
  state_manager::{DeclId, StateManager},
};

pub(crate) fn member_expression(
  member: &MemberExpr,
  index: &mut i32,
  bail_out_index: &mut Option<i32>,
  non_null_props: &mut NonNullProps,
  state: &mut StateManager,
  fns: &Rc<FunctionMap>,
) {
  let object = member.obj.as_ref();
  let property = &member.prop;

  let mut obj_id: Option<DeclId> = None;
  let mut prop_name: Option<Atom> = None;

  if let Expr::Ident(ident) = object
    && state.is_style_var_ident(ident)
  {
    obj_id = Some(ident.to_id());
    prop_name = namespace_name_from_member_prop(property);
  }

  let style_non_null_props: NonNullProps;

  if let Some(bail_out_index) = bail_out_index
    && index > bail_out_index
  {
    *non_null_props = NonNullProps::True;
  }

  if let NonNullProps::True = non_null_props {
    style_non_null_props = NonNullProps::True;
  } else {
    let evaluate_result =
      evaluate_with_functions(&Expr::from(member.clone()), state, Rc::clone(fns));

    let style_value = evaluate_result.value;
    let confident = evaluate_result.confident;

    if !confident {
      *non_null_props = NonNullProps::True;
      style_non_null_props = NonNullProps::True;
    } else {
      // Not `True` here: the branch above took that case, and nothing since has
      // written to the counter.
      style_non_null_props = non_null_props.clone();

      if let NonNullProps::Vec(vec) = non_null_props
        && let Some(EvaluateResultValue::Expr(Expr::Object(ObjectLit { props, .. }))) = style_value
      {
        vec.extend(declared_namespaces(&props));
      }
    }
  }

  if let Some(obj_id) = obj_id {
    let style_var_to_keep = StyleVarsToKeep(
      obj_id,
      match prop_name {
        Some(prop_name) => NonNullProp::Atom(prop_name),
        None => NonNullProp::True,
      },
      style_non_null_props,
    );

    state.style_vars_to_keep.insert(style_var_to_keep);
  }
}

/// The namespaces an evaluated style object still declares.
///
/// A property whose value is `null` declares nothing the runtime needs, so it
/// gives no name. Every other property gives the name it is declared under,
/// whichever shape its key was written in -- which is why the key goes through
/// the reader that names all of them. Read through `as_ident` alone, a quoted
/// key answered nothing and the namespace behind it went missing. That is no
/// theoretical shape: the fold rebuilds the keys of the object it writes but
/// not those of an object it carries through as a value, and `stylex.env`,
/// `stylex.types` and a folded function map each write a quoted key.
///
/// A property the reader cannot name is refused, on the sentence
/// `named_key_value` gives. Skipped instead, the namespace would never reach
/// `vars_to_keep`, and `retain_object_props` then deletes a namespace the
/// runtime still reads. A wrong output is worse than a stopped build.
///
/// No source reaches a refusal: the fold refuses a spread and a property that
/// is no key-value pair before it writes an object, and each producer above
/// writes key-value pairs only. So the cases for the refusals hand the reader
/// its object directly.
fn declared_namespaces(props: &[PropOrSpread]) -> impl Iterator<Item = Atom> + '_ {
  props.iter().filter_map(|prop| {
    let (name, key_value) = named_key_value(prop);

    match key_value.value.as_ref() {
      Expr::Lit(Lit::Null(_)) => None,
      _ => Some(name),
    }
  })
}

/// Walks the member expressions of a `stylex.props`-family call argument and
/// records which style variables and namespaces the runtime still needs.
///
/// A reader, not a writer: [`member_expression`] takes the node by shared
/// reference and writes only to the state and to the three counters here. The
/// walk says so through `Visit`, which is what keeps it independent of the
/// hoisting walk that runs beside it.
pub(crate) struct MemberTransform<'a> {
  pub(crate) index: i32,
  pub(crate) bail_out_index: Option<i32>,
  pub(crate) non_null_props: NonNullProps,
  pub(crate) state: &'a mut StateManager,
  pub(crate) functions: &'a Rc<FunctionMap>,
}

impl Visit for MemberTransform<'_> {
  noop_visit_type!();

  // Deliberately does not walk the children of a member expression. The index
  // counts one step per member expression the argument holds, and a nested one
  // would count twice and move the bail-out point.
  fn visit_member_expr(&mut self, member: &MemberExpr) {
    member_expression(
      member,
      &mut self.index,
      &mut self.bail_out_index,
      &mut self.non_null_props,
      &mut *self.state,
      self.functions,
    );
  }
}

#[cfg(test)]
#[path = "tests/member_expression_tests.rs"]
mod tests;
