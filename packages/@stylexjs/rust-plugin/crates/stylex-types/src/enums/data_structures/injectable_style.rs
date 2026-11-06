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
  /// The rule text this style contributes, whichever direction carries it.
  ///
  /// Both kinds hold an `ltr` and an optional `rtl`, and a directional rule is
  /// spelled with an empty `ltr` — so a reader wanting "the rule" has to pick,
  /// and picking is the same two lines at every site. Answered here so the kinds
  /// stay this module's business.
  ///
  /// The empty-`ltr` arm is this compiler's spelling of a directional rule, not
  /// a fallback the reference implementation has: upstream takes `ltr` whenever
  /// it is a string, and `generateCSSRule` always produces a non-empty one, so
  /// it never reads `rtl` here. Faithful to the code this was lifted out of, and
  /// unreachable from `generate_css_rule` output — but a new caller should not
  /// read it as upstream behaviour.
  pub fn rule_text(&self) -> &str {
    let (ltr, rtl) = self.directional_rules();

    if ltr.is_empty() {
      rtl.unwrap_or_default()
    } else {
      ltr
    }
  }

  /// The `ltr` rule and the optional `rtl` one, whichever kind carries them.
  ///
  /// Both kinds hold the same pair, so every reader that wants it wrote the
  /// same two arms. Answered here for the reason [`Self::rule_text`] is: the
  /// kinds are this module's business, and a reader that only ever meets one of
  /// them leaves the other arm with no test to reach it.
  pub fn directional_rules(&self) -> (&str, Option<&str>) {
    match self {
      Self::Regular(style) => (style.ltr.as_str(), style.rtl.as_deref()),
      Self::Const(style) => (style.ltr.as_str(), style.rtl.as_deref()),
    }
  }

  /// The order this style is injected in, or `0.0` where the kind names none.
  ///
  /// The same two arms as [`Self::directional_rules`], and answered here for
  /// the same reason.
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
