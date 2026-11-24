//! `stylex.props` / `stylex.attrs` for static arguments, through the `stylex-styleq` port.
//!
//! A front end resolves each argument to a compiled style object (what `create` returned for
//! the namespace, `$$css: true` included), `null`/`false`/`undefined`, or gives up. For
//! `cond && styles.a`, `cond ? a : b`, #2008's `stylex_merge` calls the merge once per
//! combination and emits a lookup; that fan-out is front-end work and calls [`props`] per
//! combination.

use std::{borrow::Cow, hash::Hash};

use indexmap::IndexMap;
use stylex_css::css::common::inline_style_to_css_string;
use stylex_macros::stylex_panic;
use stylex_structures::pair::PairCow;
use stylex_styleq::{StyleMap, StyleqArgument, StyleqValue};
use stylex_utils::number::to_js_string;
use stylex_value::{Object, Value};

/// One resolved argument.
#[derive(Debug, Clone, PartialEq)]
pub enum PropsArg {
  /// A compiled style object (`{ key: "x123", $$css: true }`), or an inline-style object.
  Style(Object),
  /// `null`, `undefined`, `false`: skipped.
  Nullish,
}

#[derive(Debug, Clone, PartialEq)]
struct StyleqVal(Value);

impl Hash for StyleqVal {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.0.to_json().to_string().hash(state)
  }
}

impl StyleqValue for StyleqVal {
  fn as_class_name(&self) -> Option<&str> {
    self.0.as_str()
  }
  fn is_null(&self) -> bool {
    matches!(self.0, Value::Null)
  }
  fn is_true_bool(&self) -> bool {
    matches!(self.0, Value::Bool(true))
  }
  fn is_undefined(&self) -> bool {
    matches!(self.0, Value::Undefined)
  }
}

struct Arg(Option<StyleMap<StyleqVal>>);

impl StyleqArgument<StyleqVal> for Arg {
  fn as_style(&self) -> Option<&StyleMap<StyleqVal>> {
    self.0.as_ref()
  }
  fn should_skip(&self) -> bool {
    self.0.is_none()
  }
}

/// What `stylex.props(...)` folds to: `className`, `style`, `data-style-src`, each present
/// only when non-empty (Babel's order).
pub fn props(args: &[PropsArg]) -> Object {
  let mut out = Object::new();
  if args.is_empty() {
    return out;
  }
  let converted: Vec<Arg> = args
    .iter()
    .map(|arg| match arg {
      PropsArg::Style(obj) => Arg(Some(
        obj.iter().map(|(k, v)| (k.clone(), StyleqVal(v.clone()))).collect(),
      )),
      PropsArg::Nullish => Arg(None),
    })
    .collect();
  let styleq = stylex_styleq::create_styleq(stylex_styleq::StyleqOptions {
    dedupe_class_name_chunks: true,
    disable_cache: true,
    ..Default::default()
  });
  let result = styleq.styleq(&converted);

  if !result.class_name.is_empty() {
    out.insert("className".into(), Value::String(result.class_name));
  }
  if let Some(inline) = result.inline_style.filter(|s| !s.is_empty()) {
    out.insert(
      "style".into(),
      Value::Object(inline.into_iter().map(|(k, v)| (k, v.0)).collect()),
    );
  }
  if !result.data_style_src.is_empty() {
    out.insert("data-style-src".into(), Value::String(result.data_style_src));
  }
  out
}

fn js_text(value: &Value) -> Cow<'_, str> {
  match value {
    Value::String(s) => Cow::Borrowed(s),
    Value::Number(n) => Cow::Owned(to_js_string(*n)),
    Value::Bool(b) => Cow::Owned(b.to_string()),
    Value::Null => Cow::Borrowed("null"),
    Value::Undefined => Cow::Borrowed("undefined"),
    _ => stylex_panic!("Only primitive inline style values can be folded into attrs()."),
  }
}

/// What `stylex.attrs(...)` folds to: `class`, `style` (a CSS string) and `data-style-src`.
pub fn attrs(args: &[PropsArg]) -> Object {
  let props = props(args);
  let mut out = IndexMap::new();
  if let Some(class_name) = props.get("className") {
    out.insert("class".to_owned(), class_name.clone());
  }
  if let Some(Value::Object(style)) = props.get("style") {
    let declarations: Vec<PairCow<'_>> = style
      .iter()
      .map(|(key, value)| PairCow {
        key: Cow::Borrowed(key.as_str()),
        value: js_text(value),
      })
      .collect();
    out.insert(
      "style".to_owned(),
      Value::String(inline_style_to_css_string(&declarations)),
    );
  }
  if let Some(src) = props.get("data-style-src") {
    out.insert("data-style-src".to_owned(), src.clone());
  }
  out
}
