use std::rc::Rc;

use indexmap::IndexMap;
use rustc_hash::FxHashMap;
use swc_core::{
  common::comments::Comments,
  ecma::{
    ast::{Expr, Id, Module},
    visit::VisitMutWith,
  },
};

use crate::{
  StyleXTransform,
  shared::{
    transformers::stylex_create::stylex_create_set,
    utils::core::{
      dev_class_name::inject_sx_dev_class_name,
      evaluate_stylex_create_arg::evaluate_stylex_create_arg, js_to_ast::convert_values_to_ast,
    },
  },
};
use stylex_ast::ast::factories::{
  create_key_value_prop, create_object_expression, create_string_key_value_prop,
};
use stylex_atoms::transform::{
  AtomCompileResult, AtomFlatValue, Compile, InjectedAtomStyle, create_utility_styles_visitor,
};
use stylex_evaluator::state::EvaluationState;
use stylex_macros::stylex_panic;
use stylex_state::{
  flat_compiled_styles_value::FlatCompiledStylesValue,
  types::{FlatCompiledStyles, InjectableStylesMap},
};
use stylex_types::structures::{injectable_style::InjectableStyle, style_key::RuleKey};

use super::transform_stylex_create_call::{build_runtime_function_map, hoist_expression};

/// The namespace an atom is compiled under.
///
/// An atom declares one property, and the create pipeline compiles namespaces,
/// so the property is wrapped in a namespace of this name and read back out of
/// the compiled map under it. The name is written once, because the write and
/// the read have to agree.
pub(crate) const INLINE_NAMESPACE: &str = "__inline__";

/// The compiled namespace an atom was written into.
///
/// This is the whole of what is left out of the coverage measurement, and it
/// computes nothing -- it chooses between answers the caller has already worked
/// out. The name read here is the one the object above was written with, and
/// the create pipeline answers a namespace under the name it was given, which
/// `answers_a_namespace_under_the_name_it_was_given` measures.
/// `guidelines/stack/RUST.md` describes the allowance.
#[cfg_attr(coverage_nightly, coverage(off))]
fn or_refuse_missing_atom_namespace(
  namespace: Option<&Rc<FlatCompiledStyles>>,
) -> &FlatCompiledStyles {
  match namespace {
    Some(namespace) => namespace,
    None => stylex_panic!("An inline style compiled to no namespace."),
  }
}

impl<C> StyleXTransform<C>
where
  C: Comments,
{
  /// Runs the atoms transform pass over the module, rewriting inline atomic
  /// styles (`css.display.flex`, `css.color(value)`) into compiled style
  /// objects or hoisted dynamic-style calls.
  ///
  /// The `create_utility_styles_visitor` pass executes between the producer
  /// (`stylex.create`) and consumer (`stylex.props`) passes so that atoms are
  /// already compiled when `stylex.props` processes them.
  pub(crate) fn transform_atoms(&mut self, module: &mut Module) {
    if self.state.atom_imports.is_empty() {
      return;
    }

    let mut visitor = create_utility_styles_visitor(self);
    module.visit_mut_with(&mut visitor);
  }
}

impl<C> Compile for StyleXTransform<C>
where
  C: Comments,
{
  fn atom_imports(&self) -> &FxHashMap<Id, String> {
    &self.state.atom_imports
  }

  fn style_x_create_set(&mut self, property: &str, value: &str) -> Option<AtomCompileResult> {
    // Compile `{ __inline__: { [property]: value } }` exactly as `stylex.create`
    // would, reusing the full create pipeline.
    let inner = create_object_expression(vec![create_string_key_value_prop(property, value)]);
    let mut first_arg =
      create_object_expression(vec![create_key_value_prop(INLINE_NAMESPACE, inner)]);

    let function_map = build_runtime_function_map(self);

    // Save/restore rather than force `false`, so a future re-entrant caller that
    // was already mid-create is not silently knocked out of create mode. (Today
    // the atoms pass runs between the create and props passes, so the prior
    // value is always `false`.)
    let prev_in_stylex_create = self.state.in_stylex_create;
    self.state.in_stylex_create = true;
    let evaluated = evaluate_stylex_create_arg(&mut first_arg, &mut self.state, &function_map);

    // The atom is left as the author wrote it, for the runtime, where the fold
    // answers nothing. The object is two string literals, so the fold reads it
    // whenever it is allowed to descend that far. `maxEvaluationDepth` is what
    // says how far, and a project can set it to one.
    //
    // The `in_stylex_create` flag is restored here too, so a later pass is not
    // left in create mode.
    let Some(value_result) = evaluated.value else {
      self.state.in_stylex_create = prev_in_stylex_create;
      return None;
    };

    let (mut compiled, injected, _class_paths) = stylex_create_set(
      &value_result,
      &mut EvaluationState::new(),
      &mut self.state,
      &function_map,
    );

    if self.state.is_dev() && self.state.options.enable_dev_class_names {
      compiled = inject_sx_dev_class_name(compiled, &self.state);
    }
    self.state.in_stylex_create = prev_in_stylex_create;

    // Borrowed, because both readers below only read it.
    let namespace = or_refuse_missing_atom_namespace(compiled.get(INLINE_NAMESPACE));

    let compiled_ast = convert_values_to_ast(namespace);

    let compiled_flat = namespace
      .iter()
      .map(|(key, value)| {
        let flat_value = match value.as_ref() {
          FlatCompiledStylesValue::String(string) => AtomFlatValue::String(string.clone()),
          FlatCompiledStylesValue::Bool(bool_value) => AtomFlatValue::Bool(*bool_value),
          _ => AtomFlatValue::Null,
        };
        (key.clone(), flat_value)
      })
      .collect();

    let injected = injected
      .iter()
      .map(|(rule_key, kind)| {
        let (ltr, rtl) = kind.directional_rules();

        InjectedAtomStyle {
          class_name: rule_key.as_str().to_string(),
          priority: kind.priority(),
          ltr: ltr.to_string(),
          rtl: rtl.map(str::to_string),
        }
      })
      .collect();

    Some(AtomCompileResult {
      compiled_ast,
      compiled_flat,
      injected,
    })
  }

  fn register_styles(&mut self, injected: &[InjectedAtomStyle]) {
    if injected.is_empty() {
      return;
    }

    let mut style_map: InjectableStylesMap = IndexMap::new();

    for style in injected {
      let kind = match &style.rtl {
        Some(rtl) => {
          InjectableStyle::with_rtl(style.ltr.clone(), rtl.clone(), Some(style.priority))
        },
        None => InjectableStyle::regular(style.ltr.clone(), Some(style.priority)),
      };

      style_map.insert(RuleKey::from(style.class_name.clone()), kind);
    }

    self.state.register_atom_styles(&style_map);
  }

  fn hoist_expression(&mut self, expr: Expr) -> Expr {
    Expr::Ident(hoist_expression(expr, &mut self.state))
  }
}
