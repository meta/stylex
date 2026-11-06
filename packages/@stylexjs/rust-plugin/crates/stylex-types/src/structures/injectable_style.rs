use std::{hash::Hash, rc::Rc};

use serde::{Deserialize, Serialize};

use crate::enums::data_structures::injectable_style::InjectableStyleKind;
use stylex_utils::hash::hash_f64;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Hash, Eq)]
pub struct InjectableStyleBase {
  pub rtl: Option<String>,
  pub ltr: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Hash, Eq)]
pub struct InjectableStyleConstBase {
  pub rtl: Option<String>,
  pub ltr: String,
  pub const_key: String,
  /// The constant, spelled as JSON. See [`InjectableConstStyle::const_value`].
  pub const_value: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct InjectableStyle {
  pub ltr: String,
  pub rtl: Option<String>,
  pub priority: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct InjectableConstStyle {
  pub ltr: String,
  pub rtl: Option<String>,
  pub priority: Option<f64>,
  pub const_key: String,
  /// The constant, spelled as JSON.
  ///
  /// A constant keeps the kind the author gave it -- a number, a boolean, a
  /// `null`, an object, a list -- and two readers ask for it again: the
  /// metadata a build tool reads, and the rule the runtime is handed. This
  /// crate sits below the value vocabulary, so the kind travels as a spelling
  /// rather than as a value, and a writer of this field owes it that spelling.
  ///
  /// The four values JSON has no word for are spelled the way JavaScript
  /// spells them: `NaN`, `Infinity`, `-Infinity` and `undefined`.
  ///
  /// **The spelling stops here.** Both readers turn it back into a value before
  /// anything outside the compiler sees it, and both were measured against
  /// `@stylexjs/babel-plugin@0.19.0` on
  /// `defineConsts({ a: null, b: 1, c: 'red', d: true, e: 0 })`: the metadata
  /// carries `[null, 1, "red", true, 0]` and the injected call carries
  /// `1`, `"red"`, `"true"` and `0` with the null pair dropped. Both are
  /// byte-identical to the reference, so no reader is handed `"1"` where the
  /// reference hands it `1`. `index.spec.ts` and the
  /// `a_null_constant_value_writes_no_pair` snapshot pin the two halves.
  pub const_value: String,
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Hash for InjectableStyle {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.ltr.hash(state);
    self.rtl.hash(state);
    hash_f64(self.priority.unwrap_or(0.0));
  }
}

impl From<InjectableStyle> for InjectableStyleBase {
  fn from(style: InjectableStyle) -> Self {
    // Assuming InjectableStyleBase and InjectableStyle have similar fields
    InjectableStyleBase {
      ltr: style.ltr,
      rtl: style.rtl,
    }
  }
}

impl InjectableStyle {
  /// Creates a new `InjectableStyle` wrapped in `Rc<InjectableStyleKind>` with only
  /// LTR content.
  ///
  /// # Example
  /// ```ignore
  /// let style = InjectableStyle::regular(css_string, Some(0.5));
  /// ```
  #[inline]
  pub fn regular(ltr: impl Into<String>, priority: Option<f64>) -> Rc<InjectableStyleKind> {
    Rc::new(InjectableStyleKind::Regular(InjectableStyle {
      ltr: ltr.into(),
      rtl: None,
      priority,
    }))
  }

  /// Creates a new `InjectableStyle` wrapped in `Rc<InjectableStyleKind>` with both
  /// LTR and RTL content.
  ///
  /// # Example
  /// ```ignore
  /// let style = InjectableStyle::with_rtl(ltr_css, rtl_css, Some(0.5));
  /// ```
  #[inline]
  pub fn with_rtl(
    ltr: impl Into<String>,
    rtl: impl Into<String>,
    priority: Option<f64>,
  ) -> Rc<InjectableStyleKind> {
    Rc::new(InjectableStyleKind::Regular(InjectableStyle {
      ltr: ltr.into(),
      rtl: Some(rtl.into()),
      priority,
    }))
  }
}

impl Default for InjectableStyle {
  fn default() -> Self {
    InjectableStyle {
      ltr: String::new(),
      rtl: None,
      priority: Some(0.0),
    }
  }
}

impl From<InjectableConstStyle> for InjectableStyleConstBase {
  fn from(style: InjectableConstStyle) -> Self {
    InjectableStyleConstBase {
      ltr: style.ltr,
      rtl: style.rtl,
      const_key: style.const_key,
      const_value: style.const_value,
    }
  }
}

/// A rule with nothing in it, for a test that needs one.
///
/// No producer uses it: `stylex_define_consts` is the only writer and it fills
/// every field. `const_value` is `null` and not an empty text because the field
/// holds JSON, which has a word for "no value" and no word for "".
impl Default for InjectableConstStyle {
  fn default() -> Self {
    InjectableConstStyle {
      ltr: String::new(),
      rtl: None,
      priority: Some(0.0),
      const_key: String::new(),
      const_value: "null".to_owned(),
    }
  }
}
