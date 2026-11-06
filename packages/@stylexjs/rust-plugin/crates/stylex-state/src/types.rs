use std::cell::OnceCell;
use std::rc::Rc;
use std::sync::Arc;

use indexmap::IndexMap;
use rustc_hash::FxHashMap;
use stylex_utils::collections::FxIndexMap;
use swc_core::{
  atoms::Atom,
  common::SourceFile,
  ecma::ast::{Ident, Module},
};

use crate::{
  evaluate_result_value::EvaluateResultValue, flat_compiled_styles_value::FlatCompiledStylesValue,
};
use stylex_types::enums::data_structures::injectable_style::InjectableStyleKind;
use stylex_types::structures::style_key::{ClassName, RuleKey};

use crate::{functions::FunctionConfigType, state_manager::StateManager};
use stylex_state_index::key_span_index::KeySpanIndex;
use stylex_structures::{inline_style::InlineStyle, named_import_source::ImportSources};

pub type FlatCompiledStyles = IndexMap<String, Rc<FlatCompiledStylesValue>>;
/// A dynamic style retains the complete authored arrow (including TS/Flow annotations), with only its returned body rewritten by creation.
#[derive(Clone, Debug, PartialEq)]
pub struct DynamicFunction {
  pub original: swc_core::ecma::ast::ArrowExpr,
  pub inline_styles: TInlineStyles,
}
pub type DynamicFns = IndexMap<String, DynamicFunction>;

/// An author's own arrow, ready to be applied to evaluated arguments.
#[derive(Clone, Debug, PartialEq)]
pub struct EvaluationFailure {
  pub message: String,
  pub span: swc_core::common::Span,
  pub plain: bool,
}
pub type CallbackOperation = dyn Fn(
  Vec<EvaluateResultValue>,
  &mut StateManager,
) -> Result<EvaluateResultValue, EvaluationFailure>;
pub type OverlayOperation = dyn Fn(
  Vec<EvaluateResultValue>,
  &mut StateManager,
  &rustc_hash::FxHashMap<swc_core::ecma::ast::Id, EvaluateResultValue>,
) -> Result<EvaluateResultValue, EvaluationFailure>;
pub struct EvaluatedFunction {
  pub original: Rc<swc_core::ecma::ast::ArrowExpr>,
  pub invoke: Rc<CallbackOperation>,
  pub invoke_with_overlays: Rc<OverlayOperation>,
  pub budget: Option<stylex_js::native::BudgetLease>,
}
impl std::ops::Deref for EvaluatedFunction {
  type Target = dyn Fn(
    Vec<EvaluateResultValue>,
    &mut StateManager,
  ) -> Result<EvaluateResultValue, EvaluationFailure>;
  fn deref(&self) -> &Self::Target {
    self.invoke.as_ref()
  }
}
pub type EvaluationCallback = Rc<EvaluatedFunction>;
pub type FunctionMapMemberExpression =
  FxHashMap<ImportSources, Box<FxHashMap<Atom, Box<FunctionConfigType>>>>;
pub type FunctionMapIdentifiers = FxHashMap<Atom, Box<FunctionConfigType>>;

/// The entries a single function-map name carries, standing for a plain JS object on the reference implementation's side.
pub type FunctionConfigMap = FxIndexMap<Atom, FunctionConfigType>;

pub type StylesObjectMap = IndexMap<String, Rc<FlatCompiledStyles>>;
pub type InjectableStylesMap = IndexMap<RuleKey, Rc<InjectableStyleKind>>;
pub type ClassPathsMap = IndexMap<String, Rc<ClassPathsInNamespace>>;
pub(crate) type ClassesToOriginalPaths = IndexMap<String, Vec<String>>;
pub type ClassNameToOriginalPaths = IndexMap<ClassName, Vec<String>>;
pub type ClassPathsInNamespace = ClassesToOriginalPaths;
pub type TInlineStyles = IndexMap<String, Box<InlineStyle>>;

#[derive(Clone, Debug)]
pub(crate) struct InjectImportIdents {
  pub(crate) module: Ident,
  pub(crate) var: Ident,
}

#[derive(Clone, Debug)]
pub(crate) struct SeenModuleSource {
  /// The module's own source, parsed.
  pub(crate) module: Module,
  /// The file the module was parsed from, kept whole so its text costs nothing to hand back.
  pub(crate) source_file: Option<Arc<SourceFile>>,
  /// Where every namespace key of `module` is written, built on the first debug-path lookup that needs it and dropped with the module it indexes.
  pub(crate) key_span_index: OnceCell<KeySpanIndex>,
}
