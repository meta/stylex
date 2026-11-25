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
      // Unknown or non-data results conservatively keep cancellation keys.
      let materialized = match style_value
        .as_ref()
        .map(|value| stylex_evaluator::evaluate::materialize_result(value, state))
      {
        Some(Ok(Some(expression))) => Some(expression),
        Some(Err(error)) => stylex_macros::stylex_panic!("{}", error.message),
        _ => None,
      };
      if !matches!(materialized, Some(Expr::Object(_))) {
        *non_null_props = NonNullProps::True;
        style_non_null_props = NonNullProps::True;
      } else {
        style_non_null_props = non_null_props.clone();
        if let NonNullProps::Vec(vec) = non_null_props
          && let Some(Expr::Object(ObjectLit { props, .. })) = materialized
        {
          vec.extend(declared_namespaces(&props));
        }
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

/// The namespaces an evaluated style object still declares (`null` values give none).
/// Reads quoted keys too; an unnamed property panics rather than dropping a live namespace.
fn declared_namespaces(props: &[PropOrSpread]) -> impl Iterator<Item = Atom> + '_ {
  props.iter().filter_map(|prop| {
    let (name, key_value) = named_key_value(prop);

    match key_value.value.as_ref() {
      Expr::Lit(Lit::Null(_)) => None,
      _ => Some(name),
    }
  })
}

/// Records which style variables and namespaces a `stylex.props`-family argument still needs.
pub(crate) struct MemberTransform<'a> {
  pub(crate) index: i32,
  pub(crate) bail_out_index: Option<i32>,
  pub(crate) non_null_props: NonNullProps,
  pub(crate) state: &'a mut StateManager,
  pub(crate) functions: &'a Rc<FunctionMap>,
}

impl Visit for MemberTransform<'_> {
  noop_visit_type!();

  // Don't walk children: a nested member expression would count twice.
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
