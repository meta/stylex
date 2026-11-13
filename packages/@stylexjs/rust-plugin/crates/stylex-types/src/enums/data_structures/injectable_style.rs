use serde::{Deserialize, Serialize};

use crate::structures::injectable_style::{
  InjectableConstStyle, InjectableStyle, InjectableStyleBase, InjectableStyleConstBase,
};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum InjectableStyleKind {
  Regular(InjectableStyle),
  Const(InjectableConstStyle),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Hash, Eq)]
pub enum InjectableStyleBaseKind {
  Regular(InjectableStyleBase),
  Const(InjectableStyleConstBase),
}

impl InjectableStyleKind {
  /// The rule text: `ltr`, or `rtl` when `ltr` is empty (this compiler's spelling of a
  /// directional rule; upstream always reads `ltr`).
  pub fn rule_text(&self) -> &str {
    let (ltr, rtl) = self.directional_rules();

    if ltr.is_empty() {
      rtl.unwrap_or_default()
    } else {
      ltr
    }
  }

  /// The `ltr` rule and the optional `rtl` one.
  pub fn directional_rules(&self) -> (&str, Option<&str>) {
    match self {
      Self::Regular(style) => (style.ltr.as_str(), style.rtl.as_deref()),
      Self::Const(style) => (style.ltr.as_str(), style.rtl.as_deref()),
    }
  }

  /// The injection priority, or `0.0` where the kind has none.
  pub fn priority(&self) -> f64 {
    match self {
      Self::Regular(style) => style.priority.unwrap_or(0.0),
      Self::Const(style) => style.priority.unwrap_or(0.0),
    }
  }
}

impl From<InjectableStyleKind> for InjectableStyleBaseKind {
  fn from(style: InjectableStyleKind) -> Self {
    match style {
      InjectableStyleKind::Regular(style) => {
        InjectableStyleBaseKind::Regular(InjectableStyleBase {
          ltr: style.ltr,
          rtl: style.rtl,
        })
      },
      InjectableStyleKind::Const(style) => {
        InjectableStyleBaseKind::Const(InjectableStyleConstBase {
          ltr: style.ltr,
          rtl: style.rtl,
          const_key: style.const_key,
          const_value: style.const_value,
        })
      },
    }
  }
}
