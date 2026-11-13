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
  /// The constant, spelled as JSON (`NaN`, `Infinity`, `-Infinity` and `undefined` as in JS).
  /// Readers convert it back to a value before output, matching `@stylexjs/babel-plugin@0.19.0`.
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
    InjectableStyleBase {
      ltr: style.ltr,
      rtl: style.rtl,
    }
  }
}

impl InjectableStyle {
  /// Creates an LTR-only `InjectableStyle` wrapped in `Rc<InjectableStyleKind>`.
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

  /// Creates an `InjectableStyle` with LTR and RTL content, wrapped in `Rc<InjectableStyleKind>`.
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

/// An empty rule for tests; `const_value` is `null` because the field holds JSON.
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
