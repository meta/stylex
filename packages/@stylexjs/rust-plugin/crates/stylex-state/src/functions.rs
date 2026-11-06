use std::{fmt::Debug, rc::Rc, sync::Arc};

use indexmap::IndexMap;
use swc_core::ecma::ast::Expr;

use crate::{
  evaluate_result_value::EvaluateResultValue, state_manager::StateManager, theme_ref::ThemeRef,
  types::FlatCompiledStyles,
};
use stylex_enums::value_with_default::ValueWithDefault;

use crate::types::{FunctionConfigMap, FunctionMapIdentifiers, FunctionMapMemberExpression};
use stylex_structures::named_import_source::ImportSources;
use stylex_structures::stylex_env::JSFunction;

pub type StylexTypeFn = Rc<dyn Fn(ValueWithDefault) -> Expr + 'static>;
pub type StylexExprFn = fn(Expr, &mut StateManager) -> Expr;

/// The `stylex.when.*` functions, which alone among the StyleX helpers take a second argument: an optional custom marker to observe instead of the default one.
pub type StylexWhenFn =
  fn(EvaluateResultValue, Option<EvaluateResultValue>, &mut StateManager) -> Expr;

pub enum FunctionType {
  ArrayArgs(fn(Vec<Expr>, &mut StateManager, &FunctionMap) -> Expr),
  StylexExprFn(StylexExprFn),
  StylexWhenFn(StylexWhenFn),
  StylexTypeFn(StylexTypeFn),
  StylexFnsFactory(fn(input: String) -> StylexTypeFn),

  Mapper(Rc<dyn Fn() -> Expr + 'static>),
  ValueMapper(Rc<dyn Fn() -> EvaluateResultValue + 'static>),
  ThemeRefMapper(Rc<dyn Fn() -> ThemeRef + 'static>),
  /// A function the module wrote, called with evaluated arguments.
  Callback(Box<Expr>),
  DefaultMarker(Arc<IndexMap<String, StylexWhenFn>>),
  /// An env function from the `env` config option.
  EnvFunction(JSFunction),
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Clone for FunctionType {
  fn clone(&self) -> Self {
    match self {
      Self::ArrayArgs(e) => Self::ArrayArgs(*e),
      Self::StylexExprFn(e) => Self::StylexExprFn(*e),
      Self::StylexWhenFn(e) => Self::StylexWhenFn(*e),
      Self::StylexTypeFn(e) => Self::StylexTypeFn(e.clone()),
      Self::StylexFnsFactory(e) => Self::StylexFnsFactory(*e),
      Self::Callback(v) => Self::Callback(v.clone()),
      Self::Mapper(c) => Self::Mapper(Rc::clone(c)),
      Self::ValueMapper(c) => Self::ValueMapper(Rc::clone(c)),
      Self::ThemeRefMapper(c) => Self::ThemeRefMapper(Rc::clone(c)),
      Self::DefaultMarker(e) => Self::DefaultMarker(Arc::clone(e)),
      Self::EnvFunction(e) => Self::EnvFunction(e.clone()),
    }
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl std::fmt::Debug for FunctionType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      FunctionType::ArrayArgs(_) => write!(f, "ArrayArgs"),
      FunctionType::StylexExprFn(_) => write!(f, "StylexExprWithStateFn"),
      FunctionType::StylexWhenFn(_) => write!(f, "StylexWhenFn"),
      FunctionType::StylexTypeFn(_) => write!(f, "StylexExprFn"),
      FunctionType::StylexFnsFactory(_) => write!(f, "StylexFnsFactory"),
      FunctionType::Mapper(_) => write!(f, "Mapper"),
      FunctionType::ValueMapper(_) => write!(f, "ValueMapper"),
      FunctionType::ThemeRefMapper(_) => write!(f, "ThemeRefMapper"),
      FunctionType::Callback(_) => write!(f, "Callback"),
      FunctionType::DefaultMarker(_) => write!(f, "DefaultMarker"),
      FunctionType::EnvFunction(_) => write!(f, "EnvFunction"),
    }
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl PartialEq for FunctionType {
  /// Equality names the callable instance, never merely its enum variant.
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (Self::ArrayArgs(a), Self::ArrayArgs(b)) => std::ptr::fn_addr_eq(*a, *b),
      (Self::StylexExprFn(a), Self::StylexExprFn(b)) => std::ptr::fn_addr_eq(*a, *b),
      (Self::StylexWhenFn(a), Self::StylexWhenFn(b)) => std::ptr::fn_addr_eq(*a, *b),
      (Self::StylexFnsFactory(a), Self::StylexFnsFactory(b)) => std::ptr::fn_addr_eq(*a, *b),
      (Self::StylexTypeFn(a), Self::StylexTypeFn(b)) => Rc::ptr_eq(a, b),
      (Self::Mapper(a), Self::Mapper(b)) => Rc::ptr_eq(a, b),
      (Self::ValueMapper(a), Self::ValueMapper(b)) => Rc::ptr_eq(a, b),
      (Self::ThemeRefMapper(a), Self::ThemeRefMapper(b)) => Rc::ptr_eq(a, b),
      (Self::Callback(a), Self::Callback(b)) => a == b,
      (Self::DefaultMarker(a), Self::DefaultMarker(b)) => Arc::ptr_eq(a, b),
      (Self::EnvFunction(a), Self::EnvFunction(b)) => a.same_instance(b),
      _ => false,
    }
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl std::hash::Hash for FunctionType {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    std::mem::discriminant(self).hash(state);
    match self {
      Self::ArrayArgs(f) => (*f as usize).hash(state),
      Self::StylexExprFn(f) => (*f as usize).hash(state),
      Self::StylexWhenFn(f) => (*f as usize).hash(state),
      Self::StylexFnsFactory(f) => (*f as usize).hash(state),
      Self::StylexTypeFn(f) => (Rc::as_ptr(f) as *const () as usize).hash(state),
      Self::Mapper(f) => (Rc::as_ptr(f) as *const () as usize).hash(state),
      Self::ValueMapper(f) => (Rc::as_ptr(f) as *const () as usize).hash(state),
      Self::ThemeRefMapper(f) => (Rc::as_ptr(f) as *const () as usize).hash(state),
      Self::Callback(expr) => expr.hash(state),
      Self::DefaultMarker(f) => (Arc::as_ptr(f) as usize).hash(state),
      Self::EnvFunction(f) => f.identity().hash(state),
    }
  }
}

#[derive(Debug, Hash, PartialEq, Clone)]
pub struct FunctionConfig {
  pub fn_ptr: FunctionType,
  pub takes_path: bool,
}

pub enum FunctionConfigType {
  Regular(FunctionConfig),
  Map(FunctionConfigMap),
  /// A compiled style object registered under a name, shared rather than copied.
  IndexMap(Rc<FlatCompiledStyles>),
  /// An env object from the `env` config option.
  EnvObject(Rc<IndexMap<String, stylex_structures::stylex_env::EnvEntry>>),
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl std::fmt::Debug for FunctionConfigType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Regular(config) => f.debug_tuple("Regular").field(config).finish(),
      Self::Map(map) => f.debug_tuple("Map").field(map).finish(),
      Self::IndexMap(map) => f.debug_tuple("IndexMap").field(map).finish(),
      Self::EnvObject(map) => f.debug_tuple("EnvObject").field(map).finish(),
    }
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Clone for FunctionConfigType {
  fn clone(&self) -> Self {
    match self {
      Self::Regular(config) => Self::Regular(config.clone()),
      Self::Map(map) => Self::Map(map.clone()),
      Self::IndexMap(map) => Self::IndexMap(map.clone()),
      Self::EnvObject(map) => Self::EnvObject(map.clone()),
    }
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl PartialEq for FunctionConfigType {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (Self::Regular(a), Self::Regular(b)) => a == b,
      (Self::Map(a), Self::Map(b)) => a == b,
      (Self::IndexMap(a), Self::IndexMap(b)) => Rc::ptr_eq(a, b),

      (Self::EnvObject(a), Self::EnvObject(b)) => Rc::ptr_eq(a, b),
      _ => false,
    }
  }
}

impl std::hash::Hash for FunctionConfigType {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.kind_tag().hash(state);
    match self {
      Self::Regular(config) => config.hash(state),
      Self::Map(map) => {
        use std::hash::Hasher;
        let mut sum = 0u64;
        for (name, config) in map {
          let mut entry = rustc_hash::FxHasher::default();
          name.hash(&mut entry);
          config.hash(&mut entry);
          sum = sum.wrapping_add(entry.finish());
        }
        sum.hash(state);
      },
      Self::IndexMap(map) => (Rc::as_ptr(map) as usize).hash(state),
      Self::EnvObject(map) => (Rc::as_ptr(map) as usize).hash(state),
    }
  }
}
impl FunctionConfigType {
  /// Which variant this is, for the memo fingerprint: a name bound to a callable and the same name bound to a style object are two entries.
  fn kind_tag(&self) -> u8 {
    match self {
      Self::Regular(_) => 0,
      Self::Map(_) => 1,
      Self::IndexMap(_) => 2,
      Self::EnvObject(_) => 3,
    }
  }

  pub fn as_map_mut(&mut self) -> Option<&mut FunctionConfigMap> {
    match self {
      Self::Map(map) => Some(map),
      Self::Regular(_) | Self::IndexMap(_) | Self::EnvObject(_) => None,
    }
  }
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct FunctionMap {
  pub identifiers: FunctionMapIdentifiers,
  pub bindings: rustc_hash::FxHashMap<swc_core::ecma::ast::Id, Box<FunctionConfigType>>,
  pub member_expressions: FunctionMapMemberExpression,
  /// If `true`, disables the generation or processing of imports for this function map.
  pub disable_imports: bool,
}

impl FunctionMap {
  /// Only static module inventories may participate in AST memoization.
  pub fn is_static_memo_context(&self) -> bool {
    fn static_config(config: &FunctionConfigType) -> bool {
      match config {
        FunctionConfigType::Regular(config) => !matches!(
          config.fn_ptr,
          FunctionType::Mapper(_) | FunctionType::ValueMapper(_) | FunctionType::ThemeRefMapper(_)
        ),
        FunctionConfigType::Map(map) => map.values().all(static_config),
        FunctionConfigType::IndexMap(_) | FunctionConfigType::EnvObject(_) => true,
      }
    }
    self.bindings.is_empty()
      && self
        .identifiers
        .values()
        .all(|config| static_config(config))
      && self
        .member_expressions
        .values()
        .all(|members| members.values().all(|config| static_config(config)))
  }

  /// Exact static-inventory comparison.
  pub fn same_memo_inventory(&self, other: &Self) -> bool {
    fn equal_config(a: &FunctionConfigType, b: &FunctionConfigType) -> bool {
      match (a, b) {
        (FunctionConfigType::Map(a), FunctionConfigType::Map(b)) => {
          a.len() == b.len()
            && a
              .iter()
              .zip(b.iter())
              .all(|((ak, av), (bk, bv))| ak == bk && equal_config(av, bv))
        },
        _ => a == b,
      }
    }
    self.disable_imports == other.disable_imports
      && self.bindings.is_empty()
      && other.bindings.is_empty()
      && self.identifiers.len() == other.identifiers.len()
      && self.identifiers.iter().all(|(key, value)| {
        other
          .identifiers
          .get(key)
          .is_some_and(|other| equal_config(value, other))
      })
      && self.member_expressions.len() == other.member_expressions.len()
      && self.member_expressions.iter().all(|(key, members)| {
        other.member_expressions.get(key).is_some_and(|others| {
          members.len() == others.len()
            && members.iter().all(|(key, value)| {
              others
                .get(key)
                .is_some_and(|other| equal_config(value, other))
            })
        })
      })
  }

  /// What this map contributes to a fold's answer, as one number: the names it registers, as identifiers and as members of an import source, with the kind of entry each is bound to, and whether imports resolve.
  pub fn memo_fingerprint(&self) -> u64 {
    use std::hash::{Hash, Hasher};

    fn entry_hash(parts: &[&str], tag: u8, config: &FunctionConfigType) -> u64 {
      let mut hasher = rustc_hash::FxHasher::default();
      parts.hash(&mut hasher);
      tag.hash(&mut hasher);
      config.hash(&mut hasher);
      hasher.finish()
    }

    let mut sum: u64 = 0;

    for (id, config) in &self.bindings {
      let mut entry = rustc_hash::FxHasher::default();
      id.hash(&mut entry);
      config.hash(&mut entry);
      sum = sum.wrapping_add(entry.finish());
    }
    for (name, config) in &self.identifiers {
      sum = sum.wrapping_add(entry_hash(&[name.as_str()], config.kind_tag(), config));
    }

    for (source, members) in &self.member_expressions {
      let source_name = match source {
        ImportSources::Regular(name) => name.as_str(),
        ImportSources::Named(named) => named.r#as.as_str(),
      };

      for (member, config) in members.iter() {
        sum = sum.wrapping_add(entry_hash(
          &[source_name, member.as_str()],
          config.kind_tag() | 0x80,
          config,
        ));
      }
    }

    let mut hasher = rustc_hash::FxHasher::default();
    sum.hash(&mut hasher);
    self.disable_imports.hash(&mut hasher);
    hasher.finish()
  }
}

/// Which StyleX helpers a rule call folds inside its argument, and the key the map that registers them is kept under.
/// `keyframes` and `positionTry` fold `firstThatWorks` alone: a `keyframes` call written inside either of them must refuse, so its name stays unregistered there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleCallHelpers {
  FirstThatWorks,
  FirstThatWorksAndKeyframes,
}

/// The function maps the rule calls evaluate their argument with, one per set of helpers.
#[derive(Clone, Debug, Default)]
pub struct RuleCallFunctionMaps {
  first_that_works: Option<Rc<FunctionMap>>,
  with_keyframes: Option<Rc<FunctionMap>>,
}

impl RuleCallFunctionMaps {
  /// The slot `helpers` is kept in.
  pub fn slot(&mut self, helpers: RuleCallHelpers) -> &mut Option<Rc<FunctionMap>> {
    match helpers {
      RuleCallHelpers::FirstThatWorks => &mut self.first_that_works,
      RuleCallHelpers::FirstThatWorksAndKeyframes => &mut self.with_keyframes,
    }
  }
}
