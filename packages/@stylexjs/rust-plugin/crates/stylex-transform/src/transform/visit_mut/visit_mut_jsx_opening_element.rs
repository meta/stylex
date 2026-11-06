use swc_core::{
  common::{DUMMY_SP, Span, comments::Comments},
  ecma::{
    ast::{
      Bool, CallExpr, Callee, Expr, ExprOrSpread, JSXAttrName, JSXAttrOrSpread, JSXAttrValue,
      JSXElementName, JSXExpr, JSXExprContainer, JSXOpeningElement, Lit, MemberExpr, MemberProp,
      Prop, PropOrSpread,
    },
    visit::VisitMutWith,
  },
};

use crate::StyleXTransform;
use stylex_ast::ast::convertors::{normalize_expr, normalize_expr_mut};
use stylex_ast::ast::factories::{
  create_arrow_expression, create_ident, create_ident_call_expr, create_ident_name,
  create_import_namespace_decl, create_jsx_spread_attr, create_member_call_expr,
  create_spread_prop,
};
use stylex_ast::ast::keys::try_namespace_name_from_prop_key;
use stylex_constants::constants::{api_names::STYLEX_PROPS, common::RUNTIME_JSX_CALL_NAMES};
use stylex_enums::{core::TransformationCycle, counter_mode::CounterMode};
use stylex_state::state_manager::InsertionSlot;
use stylex_structures::{named_import_source::ImportSources, uid_generator::UidGenerator};

impl<C> StyleXTransform<C>
where
  C: Comments,
{
  pub(crate) fn visit_mut_jsx_opening_element_impl(
    &mut self,
    jsx_opening_element: &mut JSXOpeningElement,
  ) {
    if self.state.cycle != TransformationCycle::Discover {
      jsx_opening_element.visit_mut_children_with(self);
      return;
    }

    // Used to detect local bindings shadowing the imported `stylex` name at this site.
    let element_span = jsx_opening_element.span;

    let sx_prop_name = match self.state.options.sx_prop_name.clone() {
      Some(name) => name,
      None => {
        jsx_opening_element.visit_mut_children_with(self);
        return;
      },
    };

    // Only transform lowercase (host) element names, not components.
    let is_lowercase_element = match &jsx_opening_element.name {
      JSXElementName::Ident(ident) => ident
        .sym
        .chars()
        .next()
        .map(|c| c.is_lowercase())
        .unwrap_or(false),
      _ => false,
    };

    if !is_lowercase_element {
      jsx_opening_element.visit_mut_children_with(self);
      return;
    }

    // The first attribute naming the prop; borrowed to avoid copying its subtree.
    let sx_attr = jsx_opening_element
      .attrs
      .iter()
      .enumerate()
      .find_map(|(idx, attr)| match attr {
        JSXAttrOrSpread::JSXAttr(jsx_attr) => match &jsx_attr.name {
          JSXAttrName::Ident(name) if name.sym.as_str() == sx_prop_name.as_str() => {
            Some((idx, jsx_attr.value.as_ref()))
          },
          _ => None,
        },
        JSXAttrOrSpread::SpreadElement(_) => None,
      });

    // Only an expression container is compiled; text, valueless or unreadable values are left
    // as written. Only the first matching attribute is read.
    if let Some((
      idx,
      Some(JSXAttrValue::JSXExprContainer(JSXExprContainer {
        expr: JSXExpr::Expr(value_expr),
        ..
      })),
    )) = sx_attr
    {
      let args = sx_value_to_props_args((**value_expr).clone());
      let stylex_local_name = self.get_stylex_runtime_binding(element_span);
      let call = Expr::Call(build_stylex_props_call(stylex_local_name, args));

      jsx_opening_element.attrs[idx] = create_jsx_spread_attr(call);
    }

    jsx_opening_element.visit_mut_children_with(self);
  }

  /// Rewrites the `sx` prop of a compiled JSX/VDOM call (React `_jsx`/`_jsxs`,
  /// `React.createElement`, Vue `_createElementBlock`/`_createElementVNode`) to
  /// `...stylex.props(expr)` in place, during `Discover`. Returns whether it matched.
  pub(crate) fn transform_sx_in_compiled_jsx(&mut self, expr: &mut Expr) -> bool {
    let Some(sx_prop_name) = self.state.options.sx_prop_name.as_deref() else {
      return false;
    };

    let Some(call) = expr.as_mut_call() else {
      return false;
    };

    if !is_jsx_runtime_call(call) {
      return false;
    }

    let span = call.span;

    // The first arg must be a lowercase string literal (a host element), ignoring parens.
    let Some(Expr::Lit(Lit::Str(element))) = call.args.first().map(|arg| normalize_expr(&arg.expr))
    else {
      return false;
    };

    if !element
      .value
      .as_str()
      .and_then(|name| name.chars().next())
      .is_some_and(char::is_lowercase)
    {
      return false;
    }

    let Some(obj_lit) = call
      .args
      .get_mut(1)
      .and_then(|arg| normalize_expr_mut(&mut arg.expr).as_mut_object())
    else {
      return false;
    };

    let Some((sx_prop_idx, sx_value)) = find_sx_prop(&obj_lit.props, sx_prop_name) else {
      return false;
    };

    // Resolve the binding only now: it injects an import a call without `sx` must not get.
    let stylex_local_name = self.get_stylex_runtime_binding(span);

    obj_lit.props[sx_prop_idx] = create_spread_prop(Expr::Call(build_stylex_props_call(
      stylex_local_name,
      sx_value_to_props_args(sx_value),
    )));

    true
  }

  /// Rewrites Solid's compiled `_$setAttribute(_el$, "sx", v)` to
  /// `_$spread(_el$, _$mergeProps(() => stylex.props(v)), false, true)`, during `Discover`.
  pub(crate) fn transform_sx_in_solid_set_attribute(&mut self, expr: &Expr) -> Option<Expr> {
    let sx_prop_name = self.state.options.sx_prop_name.as_deref()?;

    let call = expr.as_call()?;

    // Check callee is _$setAttribute, ignoring parentheses.
    let is_set_attribute = match &call.callee {
      Callee::Expr(e) => match normalize_expr(e) {
        Expr::Ident(ident) => ident.sym.as_str() == "_$setAttribute",
        _ => false,
      },
      _ => false,
    };
    if !is_set_attribute {
      return None;
    }

    // Need at least 3 args: (el, "sx", value)
    if call.args.len() < 3 {
      return None;
    }

    // Args[1] must be the string matching sx_prop_name, ignoring parentheses.
    let attr_name = match normalize_expr(&call.args[1].expr) {
      Expr::Lit(Lit::Str(s)) => s.value.as_str().unwrap_or(""),
      _ => return None,
    };
    if attr_name != sx_prop_name {
      return None;
    }

    let el_arg = call.args[0].clone();
    let value_expr = *call.args[2].expr.clone();

    let stylex_local_name = self.get_stylex_runtime_binding(call.span);
    let args = sx_value_to_props_args(value_expr);

    let props_call = Expr::Call(build_stylex_props_call(stylex_local_name, args));
    let arrow_fn = create_arrow_expression(props_call);

    let merge_props_call = Expr::Call(create_ident_call_expr(
      "_$mergeProps",
      vec![ExprOrSpread {
        spread: None,
        expr: Box::new(arrow_fn),
      }],
    ));

    use stylex_ast::ast::factories::create_expr_or_spread;
    Some(Expr::Call(create_ident_call_expr(
      "_$spread",
      vec![
        el_arg,
        create_expr_or_spread(merge_props_call),
        create_expr_or_spread(Expr::Lit(Lit::Bool(Bool {
          span: DUMMY_SP,
          value: false,
        }))),
        create_expr_or_spread(Expr::Lit(Lit::Bool(Bool {
          span: DUMMY_SP,
          value: true,
        }))),
      ],
    )))
  }

  /// Local name of the value-level `stylex` binding for an `sx` runtime call, injecting
  /// `import * as <name> from '<source>'` when none is usable at `site`.
  fn get_stylex_runtime_binding(&mut self, site: Span) -> swc_core::ecma::ast::Ident {
    let local_rebinding_shadows_site = |name: &str| {
      if site.is_dummy() {
        // DUMMY spans give no position: avoid any name with a local re-binding; over-injecting a
        // uid import is safer than targeting a shadowed binding.
        self.state.local_rebinding_scopes.contains_key(name)
      } else {
        self.state.is_locally_rebound_at(name, site)
      }
    };

    // 1. Reuse a value-level namespace/default import not shadowed at the `sx` site.
    if let Some(binding) = self
      .state
      .namespace_bindings()
      .iter()
      .find(|id| !local_rebinding_shadows_site(id.0.as_ref()))
      .cloned()
    {
      return swc_core::ecma::ast::Ident::new(binding.0, DUMMY_SP, binding.1);
    }

    // 2. First existing import (incl. type-only) from a configured source.
    let existing_import_source = self
      .state
      .existing_import_sources
      .iter()
      .find(|source| self.state.is_import_source(source))
      .cloned();

    // 3. importSource = existing ?? third configured source ?? first source.
    let import_source = existing_import_source.clone().unwrap_or_else(|| {
      self
        .state
        .import_source_names()
        .nth(2)
        .or_else(|| self.state.import_source_names().next())
        .map(str::to_string)
        .unwrap_or_else(|| "@stylexjs/stylex".to_string())
    });

    // 4. localName = uid (`_stylex`, `_stylex2`, …) if an import source exists or `stylex` is
    //    already bound, else `stylex`.
    let stylex_local_name = if existing_import_source.is_some() || self.state.has_binding("stylex")
    {
      let uid_generator = UidGenerator::new("stylex", CounterMode::Local);
      let mut candidate = uid_generator.generate();
      while self.state.has_binding(&candidate) {
        candidate = uid_generator.generate();
      }
      candidate
    } else {
      "stylex".to_string()
    };

    // 5. Prepend the import and register it so the pipeline runs and later `sx` sites reuse it.
    let import_item = create_import_namespace_decl(&stylex_local_name, &import_source);
    self
      .state
      .queue_insertion(InsertionSlot::PrependImport, import_item);
    self.state.insert_import_path(import_source);
    self
      .state
      .insert_stylex_import(ImportSources::Regular(stylex_local_name.clone()));

    // Reserve the name: `bound_names` doesn't see injected imports, so a later uid could collide.
    self.state.bound_names.insert(stylex_local_name.clone());

    let binding = create_ident(&stylex_local_name);
    self.state.insert_stylex_namespace_binding(binding.to_id());
    binding
  }
}

/// Convert an `sx` value into `stylex.props(...)` args: `expr` → `[expr]`.
fn sx_value_to_props_args(value_expr: Expr) -> Vec<ExprOrSpread> {
  vec![ExprOrSpread {
    spread: None,
    expr: Box::new(value_expr),
  }]
}

/// Build `<stylex_local_name>.props(args...)`, with the binding from
/// [`StyleXTransform::get_stylex_runtime_binding`].
fn build_stylex_props_call(
  stylex_local_name: swc_core::ecma::ast::Ident,
  args: Vec<ExprOrSpread>,
) -> CallExpr {
  let member = MemberExpr {
    span: DUMMY_SP,
    obj: Box::new(Expr::Ident(stylex_local_name)),
    prop: MemberProp::Ident(create_ident_name(STYLEX_PROPS)),
  };
  create_member_call_expr(member, args)
}

/// First prop named `sx_prop_name` (key-value with a static key, or shorthand) and its value.
/// Getters, setters, methods, spreads and unreadable keys are skipped; first match wins.
fn find_sx_prop(props: &[PropOrSpread], sx_prop_name: &str) -> Option<(usize, Expr)> {
  props.iter().enumerate().find_map(|(idx, prop)| {
    let PropOrSpread::Prop(prop) = prop else {
      return None;
    };

    let value = match prop.as_ref() {
      Prop::KeyValue(key_value) => (try_namespace_name_from_prop_key(&key_value.key)?
        == sx_prop_name)
        .then(|| key_value.value.as_ref().clone()),
      Prop::Shorthand(ident) => {
        (ident.sym.as_str() == sx_prop_name).then(|| Expr::Ident(ident.clone()))
      },
      _ => None,
    }?;

    Some((idx, value))
  })
}

/// Whether `call` is a JSX/VDOM runtime call `(elementName, props, ...)`, ignoring parentheses
/// around the callee and receiver.
fn is_jsx_runtime_call(call: &CallExpr) -> bool {
  match &call.callee {
    Callee::Expr(e) => match normalize_expr(e) {
      Expr::Ident(ident) => {
        let name = ident.sym.as_str();

        let name = name.strip_suffix("DEV").unwrap_or(name);
        let name = name.strip_prefix('_').unwrap_or(name);

        RUNTIME_JSX_CALL_NAMES.contains(&name)
      },
      Expr::Member(member) => {
        if let (Expr::Ident(obj), MemberProp::Ident(prop)) =
          (normalize_expr(&member.obj), &member.prop)
        {
          obj.sym.as_str() == "React" && prop.sym.as_str() == "createElement"
        } else {
          false
        }
      },
      _ => false,
    },
    _ => false,
  }
}

#[cfg(test)]
#[path = "tests/sx_runtime_binding_test.rs"]
mod tests;
