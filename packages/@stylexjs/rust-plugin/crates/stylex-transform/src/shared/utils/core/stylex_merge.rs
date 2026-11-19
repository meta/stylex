use std::rc::Rc;

use stylex_ast::ast::convertors::{convert_lit_to_string, key_value_name, normalize_expr};

use swc_core::ecma::{
  ast::{
    BinExpr, BinaryOp, CallExpr, CondExpr, Expr, Ident, JSXAttrOrSpread, JSXAttrValue, Lit,
    ObjectLit, Prop, PropName, PropOrSpread,
  },
  visit::{VisitMut, VisitMutWith, VisitWith},
};

use crate::shared::{
  enums::data_structures::fn_result::FnResult,
  transformers::stylex_default_marker,
  utils::core::{
    make_string_expression::make_string_expression,
    member_expression::MemberTransform,
    parse_nullable_style::{ResolvedArg, StyleObject, parse_nullable_style},
  },
};
use stylex_ast::ast::factories::{create_jsx_attr, create_jsx_attr_or_spread};
use stylex_constants::constants::{api_names::STYLEX_DEFAULT_MARKER, common::COMPILED_KEY};
use stylex_enums::style_vars_to_keep::NonNullProps;
use stylex_state::{
  functions::{FunctionConfigType, FunctionMap},
  state_manager::{ImportKind, StateManager},
};

/// Merges the arguments of a `stylex.props`-family call into one value.
///
/// The caller supplies `hoist_expression`, which lifts an expression into a
/// module-scoped `const` and returns a reference to it.
pub(crate) fn stylex_merge(
  call: &mut CallExpr,
  transform: fn(&[ResolvedArg]) -> FnResult,
  hoist_expression: fn(Expr, &mut StateManager) -> Ident,
  flatten_array_arguments: bool,
  state: &mut StateManager,
) -> Option<Expr> {
  let mut bail_out = false;
  let mut conditional = 0;
  let mut current_index = -1;
  let mut bail_out_index = None;

  let mut evaluate_path_fn_config = FunctionMap {
    disable_imports: true,
    ..FunctionMap::default()
  };

  let marker_values = stylex_default_marker::shared_default_marker_values(state);

  if let Some(set) = state.get_stylex_api_import(ImportKind::DefaultMarker)
    && !set.is_empty()
  {
    for name in set {
      evaluate_path_fn_config.identifiers.insert(
        name.clone(),
        Box::new(FunctionConfigType::IndexMap(Rc::clone(&marker_values))),
      );
    }
  }

  for name in state.stylex_imports() {
    evaluate_path_fn_config
      .member_expressions
      .entry(name.clone())
      .or_default()
      .insert(
        STYLEX_DEFAULT_MARKER.into(),
        Box::new(FunctionConfigType::IndexMap(Rc::clone(&marker_values))),
      );
  }

  state.apply_stylex_env(&mut evaluate_path_fn_config);

  // Shared: the evaluator hands this map to every callback.
  let evaluate_path_fn_config = Rc::new(evaluate_path_fn_config);

  // Read through parentheses: `stylex.props(([a, b]))`, `stylex.props((styles.root))`.
  let mut args_path = Vec::with_capacity(call.args.len());
  for arg in &call.args {
    match normalize_expr(&arg.expr) {
      Expr::Array(array) if flatten_array_arguments => {
        args_path.extend(array.elems.iter().flatten())
      },
      _ => args_path.push(arg),
    }
  }
  let mut resolved_args = Vec::with_capacity(args_path.len());

  for arg_path in args_path.iter() {
    current_index += 1;

    let arg = normalize_expr(&arg_path.expr);

    match &arg {
      Expr::Object(_) | Expr::Ident(_) | Expr::Member(_) | Expr::Call(_) => {
        let resolved = parse_nullable_style(arg, state, &evaluate_path_fn_config);
        let is_unreadable = resolved == StyleObject::Other;

        if is_unreadable {
          bail_out_index = Some(current_index);
          bail_out = true;
        }

        // Objects and idents merge; members merge unless just kept for the runtime; calls never do.
        let merges = match &arg {
          Expr::Object(_) | Expr::Ident(_) => true,
          Expr::Member(_) => !is_unreadable,
          _ => false,
        };

        if merges {
          resolved_args.push(ResolvedArg::style_object(resolved));
        }
      },
      Expr::Cond(CondExpr {
        test,
        cons: consequent,
        alt: alternate,
        ..
      }) => {
        let primary = parse_nullable_style(consequent, state, &evaluate_path_fn_config);
        let fallback = parse_nullable_style(alternate, state, &evaluate_path_fn_config);

        if primary.eq(&StyleObject::Other) || fallback.eq(&StyleObject::Other) {
          bail_out_index = Some(current_index);
          bail_out = true;
        } else {
          resolved_args.push(ResolvedArg::conditional(
            *test.clone(),
            Some(primary),
            Some(fallback),
          ));

          conditional += 1;
        }
      },
      Expr::Bin(BinExpr {
        left: left_path,
        op,
        right: right_path,
        ..
      }) => {
        if !op.eq(&BinaryOp::LogicalAnd) {
          bail_out_index = Some(current_index);
          bail_out = true;
        } else {
          let left_resolved = parse_nullable_style(left_path, state, &evaluate_path_fn_config);
          let right_resolved = parse_nullable_style(right_path, state, &evaluate_path_fn_config);

          if !left_resolved.eq(&StyleObject::Other) || right_resolved.eq(&StyleObject::Other) {
            bail_out_index = Some(current_index);
            bail_out = true;
          } else {
            resolved_args.push(ResolvedArg::conditional(
              *left_path.clone(),
              Some(right_resolved),
              None,
            ));

            conditional += 1;
          }
        }
      },
      _ => {
        bail_out_index = Some(current_index);
        bail_out = true;
      },
    }

    if conditional > 4 {
      bail_out = true;
    }

    if bail_out {
      // Stop at the first bail: the bail path re-walks all `call.args` with `MemberTransform`.
      break;
    }
  }

  if !state.enable_inlined_conditional_merge() && conditional > 0 {
    bail_out = true;
  }

  if bail_out {
    let mut non_null_props: NonNullProps = NonNullProps::Vec(vec![]);
    let mut index = -1;

    for arg_path in call.args.iter_mut() {
      index += 1;

      let mut member_transform = MemberTransform {
        index,
        bail_out_index,
        non_null_props,
        state: &mut *state,
        functions: &evaluate_path_fn_config,
      };

      arg_path.expr.visit_with(&mut member_transform);

      index = member_transform.index;
      bail_out_index = member_transform.bail_out_index;
      non_null_props = member_transform.non_null_props;

      // Hoist inline compiled-style objects to module scope for a stable reference. Separate from
      // the member walk above, which must not descend into nested members.
      let mut object_hoister = CompiledStyleObjectHoister {
        state: &mut *state,
        hoist_expression,
      };
      arg_path.expr.visit_mut_with(&mut object_hoister);
    }
  } else {
    let string_expression = make_string_expression(&resolved_args, transform);

    if let Expr::Object(string_expression) = &string_expression
      && state.has_jsx_spread_call(call)
      && !string_expression.props.is_empty()
    {
      let jsx_attr_expressions = string_expression
        .props
        .iter()
        .map(static_jsx_attr_from_prop)
        .collect::<Option<Vec<_>>>();

      if let Some(jsx_attr_expressions) = jsx_attr_expressions {
        state.set_jsx_spread_replacement(call, jsx_attr_expressions);

        return None; // Early return to skip normal object creation
      }
    }

    return Some(string_expression);
  }

  None
}

/// The JSX attribute a non-conditional compiled property spells; only literal values qualify.
fn static_jsx_attr_from_prop(prop: &PropOrSpread) -> Option<JSXAttrOrSpread> {
  prop
    .as_prop()
    .and_then(|prop| prop.as_key_value())
    .and_then(|key_value| {
      key_value
        .value
        .as_lit()
        .and_then(convert_lit_to_string)
        .map(|value| {
          create_jsx_attr_or_spread(create_jsx_attr(
            key_value_name(key_value).as_ref(),
            JSXAttrValue::Str(value.into()),
          ))
        })
    })
}

/// Hoists inline `$$css: true` objects out of a `stylex.props(...)` argument into module `const`s.
struct CompiledStyleObjectHoister<'a> {
  state: &'a mut StateManager,
  hoist_expression: fn(Expr, &mut StateManager) -> Ident,
}

impl VisitMut for CompiledStyleObjectHoister<'_> {
  fn visit_mut_expr(&mut self, expr: &mut Expr) {
    expr.visit_mut_children_with(self);

    if let Expr::Object(object) = expr
      && object_has_css_marker(object)
    {
      let object = std::mem::replace(
        expr,
        Expr::Invalid(swc_core::ecma::ast::Invalid {
          span: swc_core::common::DUMMY_SP,
        }),
      );
      *expr = Expr::Ident((self.hoist_expression)(object, self.state));
    }
  }
}

/// Whether an object literal carries `$$css: true`.
fn object_has_css_marker(object: &ObjectLit) -> bool {
  let mut compiled = false;
  for property in &object.props {
    let PropOrSpread::Prop(property) = property else {
      return false;
    };
    let Prop::KeyValue(property) = property.as_ref() else {
      return false;
    };
    let key = match &property.key {
      PropName::Ident(key) => key.sym.as_ref(),
      PropName::Str(key) => match key.value.as_str() {
        Some(key) => key,
        None => return false,
      },
      _ => return false,
    };
    if key == COMPILED_KEY {
      compiled = matches!(property.value.as_ref(), Expr::Lit(Lit::Bool(value)) if value.value)
        || matches!(property.value.as_ref(), Expr::Lit(Lit::Str(_)));
      if !compiled {
        return false;
      }
    } else if !matches!(
      property.value.as_ref(),
      Expr::Lit(Lit::Str(_) | Lit::Null(_))
    ) {
      // References local scope, so it can't be hoisted.
      return false;
    }
  }
  compiled
}

#[cfg(test)]
#[path = "tests/stylex_merge_tests.rs"]
mod tests;
