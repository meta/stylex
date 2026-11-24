//! # stylex_core: the front-end boundary of the parser-independent StyleX core
//!
//! A front end (SWC, oxc, anything) finds StyleX calls, evaluates their arguments to plain
//! [`Value`]s with its own evaluator, and calls one method here per call. The core returns
//! the values to write back (class-name maps, token objects, generated names) and the CSS
//! rules to collect. It never sees an AST. See `docs/front-end-boundary.md` for the contract.
//!
//! One [`StyleXCore`] lives for one file compile. It owns the options, a per-file value cache
//! and the rules collected so far, in registration order (Babel's `metadata.stylex`).
//!
//! The logic is #2008's (`stylex-transform/src/shared/transformers/*` and the utilities they
//! call), ported from `Expr` to [`Value`]; every CSS decision goes through the vendored
//! `stylex-css` crate unchanged.

mod create;
mod css_value;
mod named;
pub mod props;
mod vars;

use std::panic::{AssertUnwindSafe, catch_unwind};

use indexmap::IndexMap;
use rustc_hash::FxHashMap;
use stylex_constants::constants::{
  common::COMPILED_KEY,
  messages::{EXPORT_ID_NOT_SET, VALUES_MUST_BE_OBJECT},
};
use stylex_macros::stylex_panic;
use stylex_css::utils::pseudo::is_pseudo_element;
use stylex_types::{
  enums::data_structures::injectable_style::InjectableStyleKind,
  structures::injectable_style::InjectableStyle,
};
use stylex_utils::{
  hash::{create_hash, create_short_hash},
  identifier::gen_file_based_identifier,
};

pub use create::DynamicLeaf;
pub use props::{PropsArg, attrs, props};
pub use stylex_enums::{
  property_validation_mode::PropertyValidationMode, style_resolution::StyleResolution,
};
pub use stylex_structures::stylex_state_options::StyleXStateOptions;

/// The Babel plugin's defaults on stylex main (`default-options.js`). They differ from
/// `StyleXStateOptions::default()` (and from #2008, based on 2026-11-13) in one place:
/// `styleResolution` became `application-order` in facebook/stylex#2056 (2026-11-18).
pub fn babel_main_defaults() -> StyleXStateOptions {
  StyleXStateOptions::default().with_style_resolution(StyleResolution::ApplicationOrder)
}
pub use stylex_stylesheet::{
  LayersConfig, ProcessStylexRulesConfig, RuleEntry, RuleFields, UseLayers, process_stylex_rules,
  process_stylex_rules_checked,
};
pub use stylex_value::{Object, OpaqueId, Value, object_from_js_entries};

/// A StyleX compile error: #2008's message, without a source location (the front end owns
/// spans and adds the frame).
#[derive(Debug, Clone, PartialEq)]
pub struct CoreError {
  pub message: String,
}

impl std::fmt::Display for CoreError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(&self.message)
  }
}

impl std::error::Error for CoreError {}

pub type CoreResult<T> = Result<T, CoreError>;

/// Runs a body that reports errors the way #2008 does (`stylex_panic!`) and turns the panic
/// into a [`CoreError`]. The vendored crates panic on invalid input; this keeps the boundary
/// a plain `Result`.
fn guarded<T>(body: impl FnOnce() -> T) -> CoreResult<T> {
  catch_unwind(AssertUnwindSafe(body)).map_err(|payload| {
    let message = payload
      .downcast_ref::<String>()
      .cloned()
      .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_owned()))
      .unwrap_or_else(|| "StyleX compile error".to_owned());
    CoreError { message }
  })
}

/// One `stylex.create` namespace, after the front end evaluated it.
#[derive(Debug, Clone, PartialEq)]
pub enum Namespace {
  /// `key: { color: 'red', ':hover': { ... } }`
  Static(Object),
  /// `key: (a, b) => ({ color: a })`. `body` is the arrow's object body evaluated with the
  /// parameters unbound: each leaf that depends on them (or that the evaluator could not
  /// fold) is a [`Value::Opaque`]. A leaf is the whole property value; an array or object
  /// value that is only partly static must be one opaque leaf.
  Dynamic { params: Vec<String>, body: Object },
}

/// One compiled namespace: what replaces `styles.key` in the output.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CompiledNamespace {
  /// Property key (possibly minified) to its space-joined class names, or `None` for a
  /// property that compiles to nothing (`null`). Ends with `$$css: true`, as Babel emits it.
  pub classes: IndexMap<String, Option<String>>,
  /// Class name to the authored key path that produced it (for `debug` labels and source maps).
  pub class_paths: IndexMap<String, Vec<String>>,
  /// Set for a dynamic namespace: the leaves the runtime function must write as inline vars.
  pub dynamic: Option<Vec<DynamicLeaf>>,
}

impl CompiledNamespace {
  /// The dynamic leaf whose value decides whether `class_name` applies: the leaf whose key
  /// path equals the class's original path (#2008's `class_names_for_prop`). A front end
  /// guards the class with `expr != null` unless the expression cannot be nullish.
  /// Not covered: `styleResolution: 'legacy-expand-shorthands'` re-keys leaves first
  /// (#2008's `legacy_expand_shorthands`), and rules whose `var()` points at a leaf with an
  /// explicit nullish fallback; both stay front-end work for now.
  pub fn controlling_leaf(&self, class_name: &str) -> Option<&DynamicLeaf> {
    let path = self.class_paths.get(class_name)?.join("_");
    self
      .dynamic
      .iter()
      .flatten()
      .find(|leaf| leaf.path.join("_") == path)
  }

  /// The namespace as a plain object (`{ key: "x1 x2", other: null, $$css: true }`).
  pub fn to_value(&self) -> Value {
    Value::Object(
      self
        .classes
        .iter()
        .map(|(k, v)| {
          let value = match v {
            Some(class) => Value::String(class.clone()),
            None if k == COMPILED_KEY => Value::Bool(true),
            None => Value::Null,
          };
          (k.clone(), value)
        })
        .collect(),
    )
  }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CreateOutput {
  pub namespaces: IndexMap<String, CompiledNamespace>,
  /// Rules this call added, in order, duplicates dropped (first wins, as #2008's map does).
  pub rules: Vec<RuleEntry>,
}

/// A generated name and the rule behind it (`keyframes`, `positionTry`,
/// `viewTransitionClass`, `defineMarker`).
#[derive(Debug, Clone, PartialEq)]
pub struct NamedRule {
  pub name: String,
  pub rule: RuleEntry,
}

/// The object a `defineVars`/`defineConsts`/`createTheme`/`defineMarker` call folds to, and
/// its rules.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectOutput {
  pub value: Object,
  pub rules: Vec<RuleEntry>,
}

pub struct StyleXCore {
  pub(crate) options: StyleXStateOptions,
  pub(crate) css_property_seen: FxHashMap<String, String>,
  rules: IndexMap<String, RuleEntry>,
}

fn to_rule_entry(key: &str, style: &InjectableStyleKind) -> RuleEntry {
  match style {
    InjectableStyleKind::Regular(s) => RuleEntry(
      key.to_owned(),
      RuleFields {
        ltr: s.ltr.clone(),
        rtl: s.rtl.clone(),
        const_key: None,
        const_val: None,
      },
      s.priority.unwrap_or(0.0),
    ),
    InjectableStyleKind::Const(s) => {
      let const_val = match serde_json::from_str::<serde_json::Value>(&s.const_value) {
        Ok(serde_json::Value::Number(n)) => serde_json::Value::Number(n),
        Ok(serde_json::Value::String(text)) => serde_json::Value::String(text),
        _ => serde_json::Value::String(s.const_value.clone()),
      };
      RuleEntry(
        key.to_owned(),
        RuleFields {
          ltr: s.ltr.clone(),
          rtl: s.rtl.clone(),
          const_key: Some(s.const_key.clone()),
          const_val: Some(const_val),
        },
        s.priority.unwrap_or(0.0),
      )
    },
  }
}

impl StyleXCore {
  pub fn new(options: StyleXStateOptions) -> Self {
    StyleXCore {
      options,
      css_property_seen: FxHashMap::default(),
      rules: IndexMap::new(),
    }
  }

  pub fn options(&self) -> &StyleXStateOptions {
    &self.options
  }

  /// Every rule registered so far, in order. Feed these to [`process_stylex_rules`].
  pub fn rules(&self) -> Vec<RuleEntry> {
    self.rules.values().cloned().collect()
  }

  /// `file_name//export_name`, the id `defineVars`/`defineConsts`/`defineMarker` hash.
  /// `file_name` is the front end's hashing name (relative to the package or root, per
  /// `unstable_moduleResolution`; see #2008's `get_filename_for_hashing`).
  pub fn export_id(file_name: &str, export_name: &str) -> String {
    gen_file_based_identifier(file_name, export_name, None)
  }

  fn register(&mut self, rules: impl IntoIterator<Item = (String, InjectableStyleKind)>) -> Vec<RuleEntry> {
    let mut added = Vec::new();
    for (key, style) in rules {
      if self.rules.contains_key(&key) {
        continue;
      }
      let entry = to_rule_entry(&key, &style);
      self.rules.insert(key, entry.clone());
      added.push(entry);
    }
    added
  }

  // -------------------------------------------------------------------------------------------
  // stylex.create
  // -------------------------------------------------------------------------------------------

  pub fn create(&mut self, namespaces: &IndexMap<String, Namespace>) -> CoreResult<CreateOutput> {
    guarded(|| self.create_impl(namespaces))
  }

  fn create_impl(&mut self, namespaces: &IndexMap<String, Namespace>) -> CreateOutput {
    let mut out = CreateOutput::default();
    let mut injected: IndexMap<String, InjectableStyleKind> = IndexMap::new();

    for (namespace_name, namespace) in namespaces {
      if namespace_name == "__proto__" {
        continue;
      }
      let (style, dynamic) = match namespace {
        Namespace::Static(obj) => (obj.clone(), None),
        Namespace::Dynamic { body, .. } => {
          let mut leaves = Vec::new();
          let replaced = create::substitute_dynamic_leaves(body, &[], &mut leaves);
          (replaced, Some(leaves))
        },
      };

      let mut flattened = create::flatten_raw_style_object(self, &style);
      let mut compiled = CompiledNamespace {
        dynamic,
        ..Default::default()
      };

      let tuples: Vec<(String, Option<Vec<create::ComputedStyle>>)> = flattened
        .iter_mut()
        .map(|(key, value)| {
          let key = if self.options.enable_minified_keys && !key.starts_with("--") {
            let hashed = create_short_hash(&format!("<>{key}"));
            if self.options.debug {
              format!("{key}-k{hashed}")
            } else {
              format!("k{hashed}")
            }
          } else {
            key.clone()
          };
          (key, value.compiled(self))
        })
        .collect();

      for (key, value) in tuples {
        match value {
          Some(styles) => {
            let mut unique = indexmap::IndexSet::new();
            for s in &styles {
              unique.insert(s.class_name.clone());
            }
            let class_name = unique.into_iter().collect::<Vec<_>>().join(" ");
            compiled
              .classes
              .insert(key, (!class_name.is_empty()).then_some(class_name));
            for s in styles {
              compiled
                .class_paths
                .insert(s.class_name.clone(), s.original_path.clone());
              injected
                .entry(s.class_name)
                .or_insert(InjectableStyleKind::Regular(s.style));
            }
          },
          None => {
            compiled.classes.insert(key, None);
          },
        }
      }
      compiled.classes.insert(COMPILED_KEY.to_owned(), None);
      out.namespaces.insert(namespace_name.clone(), compiled);
    }

    // Dynamic leaves get an `@property` rule after the class rules (#2008's
    // `injected_inherit_styles`); pseudo elements read the variable by inheritance.
    for compiled in out.namespaces.values() {
      for leaf in compiled.dynamic.iter().flatten() {
        let inherits = leaf.path.iter().any(|segment| is_pseudo_element(segment));
        injected.insert(
          leaf.var_name.clone(),
          InjectableStyleKind::Regular(InjectableStyle {
            ltr: format!(
              "@property {} {{ syntax: \"*\"; inherits: {};}}",
              leaf.var_name,
              if inherits { "true" } else { "false" }
            ),
            rtl: None,
            priority: Some(0.0),
          }),
        );
      }
    }

    out.rules = self.register(injected);
    out
  }

  // -------------------------------------------------------------------------------------------
  // Variables and themes
  // -------------------------------------------------------------------------------------------

  /// `stylex.defineVars(vars)`. `export_id` from [`StyleXCore::export_id`].
  pub fn define_vars(&mut self, export_id: &str, vars: &Value) -> CoreResult<ObjectOutput> {
    guarded(|| {
      let vars = object_or_refuse(vars);
      if export_id.is_empty() {
        stylex_panic!("{}", EXPORT_ID_NOT_SET);
      }
      let (value, rules) = vars::define_vars(&self.options.class_name_prefix, export_id, vars);
      let rules = self.register(rules);
      ObjectOutput { value, rules }
    })
  }

  /// `stylex.defineConsts(consts)`.
  pub fn define_consts(&mut self, export_id: &str, consts: &Value) -> CoreResult<ObjectOutput> {
    guarded(|| {
      let consts = object_or_refuse(consts);
      if export_id.is_empty() {
        stylex_panic!("{}", EXPORT_ID_NOT_SET);
      }
      let (value, rules) = vars::define_consts(&self.options.class_name_prefix, export_id, consts);
      let rules = self.register(rules);
      ObjectOutput { value, rules }
    })
  }

  /// `stylex.createTheme(vars, overrides)`. `theme_vars` is the imported `defineVars` result
  /// as the front end resolved it (keys to `var(--…)`, plus `__varGroupHash__`).
  pub fn create_theme(&mut self, theme_vars: &Value, overrides: &Value) -> CoreResult<ObjectOutput> {
    guarded(|| {
      let theme_vars = object_or_refuse(theme_vars);
      let overrides = object_or_refuse(overrides);
      let (value, rules) =
        vars::create_theme(&self.options.class_name_prefix, theme_vars, overrides);
      let rules = self.register(rules);
      ObjectOutput { value, rules }
    })
  }

  /// `stylex.defineMarker()`: `{ [id]: id, $$css: true }`. No rule.
  pub fn define_marker(&self, export_id: &str) -> ObjectOutput {
    let id = format!("{}{}", self.options.class_name_prefix, create_hash(export_id));
    let mut value = Object::new();
    value.insert(id.clone(), Value::String(id));
    value.insert(COMPILED_KEY.to_owned(), Value::Bool(true));
    ObjectOutput {
      value,
      rules: vec![],
    }
  }

  // -------------------------------------------------------------------------------------------
  // Named rules. A front end calls these when its evaluator meets the call, top level or nested
  // inside a `create` argument, and folds the call to the returned name.
  // -------------------------------------------------------------------------------------------

  pub fn keyframes(&mut self, frames: &Value) -> CoreResult<NamedRule> {
    guarded(|| {
      let frames = object_or_refuse(frames);
      let (name, style) = self.keyframes_impl(frames);
      self.named(name, style)
    })
  }

  pub fn position_try(&mut self, styles: &Value) -> CoreResult<NamedRule> {
    guarded(|| {
      let styles = object_or_refuse(styles);
      let (name, style) = self.position_try_impl(styles);
      self.named(name, style)
    })
  }

  pub fn view_transition_class(&mut self, styles: &Value) -> CoreResult<NamedRule> {
    guarded(|| {
      let styles = object_or_refuse(styles);
      let (name, style) = self.view_transition_class_impl(styles);
      self.named(name, style)
    })
  }

  fn named(&mut self, name: String, style: InjectableStyle) -> NamedRule {
    let style = InjectableStyleKind::Regular(style);
    let rule = to_rule_entry(&name, &style);
    self.register([(name.clone(), style)]);
    NamedRule { name, rule }
  }
}

fn object_or_refuse(value: &Value) -> &Object {
  match value {
    Value::Object(obj) => obj,
    _ => stylex_panic!("{}", VALUES_MUST_BE_OBJECT),
  }
}

#[cfg(test)]
mod tests;
