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

/// The `stylex.when.*` functions, which alone among the StyleX helpers take a
/// second argument: an optional custom marker to observe instead of the
/// default one. Both arguments stay `EvaluateResultValue` because a marker
/// imported from another file resolves to a `ThemeRef`, which no `Expr` can
/// represent.
pub type StylexWhenFn =
  fn(EvaluateResultValue, Option<EvaluateResultValue>, &mut StateManager) -> Expr;

pub enum FunctionType {
  ArrayArgs(fn(Vec<Expr>, &mut StateManager, &FunctionMap) -> Expr),
  StylexExprFn(StylexExprFn),
  StylexWhenFn(StylexWhenFn),
  StylexTypeFn(StylexTypeFn),
  StylexFnsFactory(fn(input: String) -> StylexTypeFn),

  Mapper(Rc<dyn Fn() -> Expr + 'static>),
  ThemeRefMapper(Rc<dyn Fn() -> ThemeRef + 'static>),
  /// A function the module wrote, called with evaluated arguments.
  ///
  /// The expression itself rather than a named kind of callback: a method on a
  /// value is evaluated as JavaScript, and a global called as a function is
  /// too, so an arrow the author wrote is the only callback left to stand for.
  Callback(Box<Expr>),
  DefaultMarker(Arc<IndexMap<String, StylexWhenFn>>),
  /// An env function from the `env` config option.
  /// Takes evaluated arguments as `Expr`s and returns an `Expr`.
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
      FunctionType::ThemeRefMapper(_) => write!(f, "ThemeRefMapper"),
      FunctionType::Callback(_) => write!(f, "Callback"),
      FunctionType::DefaultMarker(_) => write!(f, "DefaultMarker"),
      FunctionType::EnvFunction(_) => write!(f, "EnvFunction"),
    }
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl PartialEq for FunctionType {
  /// Two `FunctionType` values compare equal when they have the same variant.
  /// We cannot compare the inner function pointers / `Rc<dyn Fn>` payloads, so
  /// discriminant equality is the strongest invariant we can uphold while
  /// satisfying `a == a` reflexivity. (This pairs with the discriminant-based
  /// `Hash` impl below so values can be used as map keys.)
  fn eq(&self, other: &Self) -> bool {
    std::mem::discriminant(self) == std::mem::discriminant(other)
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl std::hash::Hash for FunctionType {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    std::mem::discriminant(self).hash(state);
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
  /// A compiled style object registered under a name, shared rather than
  /// copied.
  ///
  /// The default marker is the only such object today. One file registers it
  /// again for each name it imports the marker by, and no reader changes it, so
  /// every registration points at one map.
  IndexMap(Rc<FlatCompiledStyles>),
  /// An env object from the `env` config option. Contains both values and
  /// functions.
  /// The `env` option's object, shared rather than copied.
  ///
  /// Registered once per `stylex` import name per `create` call, so a copy here
  /// is a copy per style object. It is read-only after options construction,
  /// which is what makes the sharing sound.
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
      (Self::IndexMap(a), Self::IndexMap(b)) => a == b,
      // One pointer compare, and an exact answer rather than a conservative
      // one. The payload was a bare map when this arm was written, where
      // answering meant a deep compare per probe; it is shared from the options
      // now, so two `EnvObject`s are the same object or they are not.
      (Self::EnvObject(a), Self::EnvObject(b)) => Rc::ptr_eq(a, b),
      _ => false,
    }
  }
}

impl FunctionConfigType {
  /// Which variant this is, for the memo fingerprint: a name bound to a
  /// callable and the same name bound to a style object are two entries.
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
  pub member_expressions: FunctionMapMemberExpression,
  /// If `true`, disables the generation or processing of imports for this
  /// function map. Set to `true` when imports should not be generated (e.g.,
  /// for built-in or inlined functions). Set to `false` to allow normal
  /// import handling.
  pub disable_imports: bool,
}

impl FunctionMap {
  /// What this map contributes to a fold's answer, as one number: the names it
  /// registers, as identifiers and as members of an import source, with the
  /// kind of entry each is bound to, and whether imports resolve.
  ///
  /// Two maps with the same fingerprint fold any one subtree of a module to the
  /// same value, which is what lets the evaluator's module-lifetime memo answer
  /// a fold made under one of them with an entry written under the other. Two
  /// maps with different fingerprints may not: a `create` call registers
  /// `firstThatWorks`, so it folds `stylex.firstThatWorks('a', 'b')` to a value,
  /// where the `props` merge, which registers no such helper, leaves the call
  /// to the runtime as the reference implementation does.
  ///
  /// Entry order does not matter: each entry is hashed on its own and the
  /// hashes are summed, so two maps built in different orders read the same.
  /// The entries' functions are not part of it; a name is bound to one
  /// function throughout a module, so the name stands for it.
  pub fn memo_fingerprint(&self) -> u64 {
    use std::hash::{Hash, Hasher};

    fn entry_hash(parts: &[&str], tag: u8) -> u64 {
      let mut hasher = rustc_hash::FxHasher::default();
      parts.hash(&mut hasher);
      tag.hash(&mut hasher);
      hasher.finish()
    }

    let mut sum: u64 = 0;

    for (name, config) in &self.identifiers {
      sum = sum.wrapping_add(entry_hash(&[name.as_str()], config.kind_tag()));
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
        ));
      }
    }

    let mut hasher = rustc_hash::FxHasher::default();
    sum.hash(&mut hasher);
    self.disable_imports.hash(&mut hasher);
    hasher.finish()
  }
}

/// Which StyleX helpers a rule call folds inside its argument, and the key the
/// map that registers them is kept under.
///
/// `keyframes` and `positionTry` fold `firstThatWorks` alone: a `keyframes`
/// call written inside either of them must refuse, so its name stays
/// unregistered there. `viewTransitionClass` folds `keyframes` too, because one
/// of its steps may name a keyframes rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleCallHelpers {
  FirstThatWorks,
  FirstThatWorksAndKeyframes,
}

/// The function maps the rule calls evaluate their argument with, one per set
/// of helpers.
#[derive(Clone, Debug, Default)]
pub struct RuleCallFunctionMaps {
  first_that_works: Option<Rc<FunctionMap>>,
  with_keyframes: Option<Rc<FunctionMap>>,
}

impl RuleCallFunctionMaps {
  /// The slot `helpers` is kept in.
  ///
  /// One match for the read and the write alike, and an exhaustive one, so a
  /// new set of helpers does not compile until it has a slot to live in.
  pub fn slot(&mut self, helpers: RuleCallHelpers) -> &mut Option<Rc<FunctionMap>> {
    match helpers {
      RuleCallHelpers::FirstThatWorks => &mut self.first_that_works,
      RuleCallHelpers::FirstThatWorksAndKeyframes => &mut self.with_keyframes,
    }
  }
}
