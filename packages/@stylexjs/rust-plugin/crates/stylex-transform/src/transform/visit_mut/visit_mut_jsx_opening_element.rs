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
    // Only transform during Discover cycle
    if self.state.cycle != TransformationCycle::Discover {
      jsx_opening_element.visit_mut_children_with(self);
      return;
    }

    // The `sx` site span — used to resolve the runtime binding against any
    // local binding that shadows the imported `stylex` name at this position.
    let element_span = jsx_opening_element.span;

    // Check if sxPropName is enabled
    let sx_prop_name = match self.state.options.sx_prop_name.clone() {
      Some(name) => name,
      None => {
        jsx_opening_element.visit_mut_children_with(self);
        return;
      },
    };

    // Only transform lowercase JSX element names (HTML elements, not React
    // components)
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

    // The first attribute that names the prop, with the value it carries. The
    // value is borrowed, so an attribute the compiler goes on to leave alone
    // costs no copy of the subtree under it.
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

    // Only an expression container names styles to compile. An attribute
    // written as text, or with no value at all, stays where the author wrote
    // it. So does a prop of a compiled call whose value the compiler cannot
    // read. The first attribute with the name wins either way, so a second one
    // is not read.
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

  /// Transform a compiled JSX/VDOM call that carries an `sx` prop, in place.
  ///
  /// Handles:
  /// - React: `_jsx("div", { sx: expr })` / `_jsxs("div", { sx: expr })`
  /// - React classic: `React.createElement("div", { sx: expr })`
  /// - Vue: `_createElementBlock("div", …)` / `_createElementVNode("div", …)`
  ///
  /// Rewrites the one prop to `...stylex.props(expr)` and leaves every other
  /// prop, and every other argument, untouched. Returns whether it matched.
  ///
  /// Runs in the `Discover` cycle.
  pub(crate) fn transform_sx_in_compiled_jsx(&mut self, expr: &mut Expr) -> bool {
    // Read first, because it is the cheapest way out and it settles the whole
    // feature. The borrow ends at the search below, well before the runtime
    // binding is resolved.
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

    // The first arg must be a lowercase string literal: a host element. Read
    // through its parentheses, because they name no other element.
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

    // The props object holds the element's whole subtree, so it is reached by
    // borrow. A call the scan passes over costs no copy of it.
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

    // The props object stays borrowed across this call, because it belongs to
    // the expression the caller owns and not to `self`. The runtime binding is
    // resolved only now, because resolving it injects an import that a call
    // with no `sx` prop must not get.
    let stylex_local_name = self.get_stylex_runtime_binding(span);

    obj_lit.props[sx_prop_idx] = create_spread_prop(Expr::Call(build_stylex_props_call(
      stylex_local_name,
      sx_value_to_props_args(sx_value),
    )));

    true
  }

  /// Transform a Solid.js compiled `sx` attribute, in the `Discover` cycle.
  ///
  /// Solid.js compiles `<div sx={styles.main}>` to:
  /// `_$setAttribute(_el$, "sx", styles.main)`
  ///
  /// We transform it to:
  /// `_$spread(_el$, _$mergeProps(() => stylex.props(styles.main)), false,
  /// true)`
  pub(crate) fn transform_sx_in_solid_set_attribute(&mut self, expr: &Expr) -> Option<Expr> {
    let sx_prop_name = self.state.options.sx_prop_name.as_deref()?;

    let call = expr.as_call()?;

    // Check callee is _$setAttribute, read through its parentheses for the
    // reason `is_jsx_runtime_call` gives.
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

    // Args[1] must be the string matching sx_prop_name, read through its
    // parentheses.
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

    // Build: () => stylex.props(args...)
    let props_call = Expr::Call(build_stylex_props_call(stylex_local_name, args));
    let arrow_fn = create_arrow_expression(props_call);

    // Build: _$mergeProps(() => stylex.props(...))
    let merge_props_call = Expr::Call(create_ident_call_expr(
      "_$mergeProps",
      vec![ExprOrSpread {
        spread: None,
        expr: Box::new(arrow_fn),
      }],
    ));

    // Build: _$spread(el, _$mergeProps(() => stylex.props(...)), false, true)
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

  /// Resolve the local name of the value-level `stylex` namespace binding to
  /// use for an `sx` runtime call, injecting an `import * as <name> from
  /// '<source>'` declaration when none exists.
  ///
  /// 1. Reuse an existing value-level namespace/default import if present and
  ///    not shadowed by a local binding in scope at the `sx` site.
  /// 2. Otherwise find the first existing import declaration (including
  ///    type-only) whose source is a configured import source.
  /// 3. Pick the import source: existing import source, else the first
  ///    configured custom source, else the default `@stylexjs/stylex`.
  /// 4. Pick the local name: a generated uid (`_stylex`) when an existing
  ///    import source was found or a `stylex` binding already exists,
  ///    otherwise plain `stylex`.
  /// 5. Prepend the namespace import, register it, and return the local name.
  fn get_stylex_runtime_binding(&mut self, site: Span) -> String {
    let local_rebinding_shadows_site = |name: &str| {
      if site.is_dummy() {
        // Generated compiled-JSX/Solid calls can carry DUMMY spans. Without a
        // reliable position, do not reuse an imported name that has any local
        // non-import re-binding; over-injecting a uid import is safer than
        // emitting `stylex.props(...)` against a shadowed binding.
        self.state.local_rebinding_scopes.contains_key(name)
      } else {
        self.state.is_locally_rebound_at(name, site)
      }
    };

    // 1. Reuse an existing value-level namespace/default import, skipping any
    //    whose local name is shadowed by a non-import binding in scope at the
    //    `sx` site. The shadow check is position-aware, using the pre-scanned
    //    scope spans and the `sx` site span.
    if let Some(local_name) = self
      .state
      .stylex_import_names()
      .find(|name| !local_rebinding_shadows_site(name))
      .map(str::to_string)
    {
      return local_name;
    }

    // 2. First existing import declaration (incl. type-only) whose source is
    //    a configured import source.
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

    // 4. localName = uid when an existing import source was found or a
    //    `stylex` binding already exists, else plain `stylex`. The uid is
    //    `_stylex`, then `_stylex2`, ..., skipping any name already bound in
    //    the module.
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

    // 5. Prepend `import * as <local> from '<source>'`, register it so the
    //    pipeline proceeds (the module visitor bails when no import path was
    //    discovered) and subsequent `sx` attributes reuse this binding.
    let import_item = create_import_namespace_decl(&stylex_local_name, &import_source);
    self
      .state
      .queue_insertion(InsertionSlot::PrependImport, import_item);
    self.state.insert_import_path(import_source);
    self
      .state
      .insert_stylex_import(ImportSources::Regular(stylex_local_name.clone()));

    // Reserve the injected name so a later `sx` site that is forced down the uid
    // path again (e.g. this binding is itself shadowed there) cannot regenerate
    // the same name and emit a duplicate `import * as <name>`. The pre-scan
    // `bound_names` does not see injected imports otherwise.
    self.state.bound_names.insert(stylex_local_name.clone());

    stylex_local_name
  }
}

/// Convert an `sx` value expression into args for `stylex.props(...)`.
/// The value is passed through as-is: `expr` → `[expr]`.
fn sx_value_to_props_args(value_expr: Expr) -> Vec<ExprOrSpread> {
  vec![ExprOrSpread {
    spread: None,
    expr: Box::new(value_expr),
  }]
}

/// Build a `<stylex_local_name>.props(args...)` call expression, where
/// `stylex_local_name` is the value-level namespace binding resolved by
/// [`StyleXTransform::get_stylex_runtime_binding`].
fn build_stylex_props_call(stylex_local_name: String, args: Vec<ExprOrSpread>) -> CallExpr {
  let member = MemberExpr {
    span: DUMMY_SP,
    obj: Box::new(Expr::Ident(create_ident(&stylex_local_name))),
    prop: MemberProp::Ident(create_ident_name(STYLEX_PROPS)),
  };
  create_member_call_expr(member, args)
}

/// Find the first prop that names `sx_prop_name`, with the value it forwards.
///
/// A key-value prop gives its value, whatever shape its key is written in, as
/// long as the text of that key is known at compile time. A shorthand gives
/// the identifier it names, because `{ sx }` and `{ sx: sx }` name the same
/// prop.
///
/// Every other shape is skipped. A getter, a setter and a method carry no
/// value expression to forward, like an attribute whose value is not an
/// expression container. A spread is not inspected at all, like a spread
/// attribute, and its keys are not knowable at compile time anyway. A key
/// whose text cannot be read -- a lone surrogate, say -- is simply not the
/// prop asked about, so the scan passes over it rather than refusing it.
///
/// The first match wins, like the raw markup path, which stops at the first
/// matching attribute. Where the two paths cannot correspond they do not: a
/// shorthand has no markup spelling, and a numeric or computed key is not a
/// shape JSX can write.
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

/// Check if a `CallExpr` is a JSX/VDOM runtime call that takes `(elementName,
/// props, ...)`.
///
/// The callee and the receiver inside it are read through their parentheses. A
/// parenthesis is not a different callee, so `(React).createElement(...)` names
/// the function `React.createElement(...)` names. Read bare, the call is not
/// recognised, and the `sx` prop reaches the runtime with no error for the
/// author to read.
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
