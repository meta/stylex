//! Tests for InjectableStyleKind → InjectableStyleBaseKind conversion
//! (From trait implementation).

use crate::{
  enums::data_structures::injectable_style::{InjectableStyleBaseKind, InjectableStyleKind},
  structures::injectable_style::{InjectableConstStyle, InjectableStyle},
};

#[test]
fn test_from_regular_kind() {
  let style = InjectableStyleKind::Regular(InjectableStyle {
    ltr: "color:red".to_string(),
    rtl: Some("color:blue".to_string()),
    priority: Some(1.0),
  });
  let base: InjectableStyleBaseKind = style.into();
  match base {
    InjectableStyleBaseKind::Regular(b) => {
      assert_eq!(b.ltr, "color:red");
      assert_eq!(b.rtl, Some("color:blue".to_string()));
    },
    _ => panic!("Expected Regular variant"),
  }
}

#[test]
fn test_from_const_kind() {
  let style = InjectableStyleKind::Const(InjectableConstStyle {
    ltr: "color:red".to_string(),
    rtl: None,
    priority: Some(0.5),
    const_key: "key".to_string(),
    const_value: "val".to_string(),
  });
  let base: InjectableStyleBaseKind = style.into();
  match base {
    InjectableStyleBaseKind::Const(b) => {
      assert_eq!(b.ltr, "color:red");
      assert_eq!(b.rtl, None);
      assert_eq!(b.const_key, "key");
      assert_eq!(b.const_value, "val");
    },
    _ => panic!("Expected Const variant"),
  }
}

// ── rule_text ───────────────────────────────────────────────────────

/// `rule_text` picks whichever direction actually carries the rule, and both
/// kinds answer the same way. The empty-`ltr` case is the one it exists for: a
/// direction-specific rule is spelled with an empty `ltr` and the text in `rtl`,
/// so a caller reading `ltr` alone would get nothing back.
/// The four directional shapes `rule_text` has to answer for, as `(ltr, rtl)`
/// beside the rule each should produce.
///
/// The interesting one is the third: a direction-specific rule is spelled with
/// an empty `ltr` and the text in `rtl`, so a caller reading `ltr` alone gets
/// nothing back. The fourth carries nothing either way, which is the empty
/// string rather than a panic or a `None` the caller would have to handle.
const DIRECTIONAL_CASES: [(&str, Option<&str>, &str); 4] = [
  ("color:red", None, "color:red"),
  ("color:red", Some("color:blue"), "color:red"),
  ("", Some("color:blue"), "color:blue"),
  ("", None, ""),
];

fn regular_kind(ltr: &str, rtl: Option<&str>) -> InjectableStyleKind {
  InjectableStyleKind::Regular(InjectableStyle {
    ltr: ltr.to_string(),
    rtl: rtl.map(str::to_string),
    priority: Some(1.0),
  })
}

fn const_kind(ltr: &str, rtl: Option<&str>) -> InjectableStyleKind {
  InjectableStyleKind::Const(InjectableConstStyle {
    ltr: ltr.to_string(),
    rtl: rtl.map(str::to_string),
    priority: Some(1.0),
    const_key: "--k".to_string(),
    const_value: "v".to_string(),
  })
}

/// `rule_text` picks whichever direction actually carries the rule, and both
/// kinds answer the same way -- which is the property that let the two duplicated
/// match arms it replaced be one function.
#[test]
fn rule_text_prefers_ltr_and_falls_back_to_rtl() {
  for (ltr, rtl, expected) in DIRECTIONAL_CASES {
    assert_eq!(
      regular_kind(ltr, rtl).rule_text(),
      expected,
      "Regular({ltr:?}, {rtl:?})"
    );
    assert_eq!(
      const_kind(ltr, rtl).rule_text(),
      expected,
      "Const({ltr:?}, {rtl:?})"
    );
  }
}

/// The two kinds are interchangeable to this reader, which is the property that
/// let the two duplicated match arms it replaced be one function.
#[test]
fn rule_text_does_not_depend_on_the_kind() {
  let regular = InjectableStyleKind::Regular(InjectableStyle {
    ltr: String::new(),
    rtl: Some("margin-right:4px".to_string()),
    priority: Some(3000.0),
  });
  let konst = InjectableStyleKind::Const(InjectableConstStyle {
    ltr: String::new(),
    rtl: Some("margin-right:4px".to_string()),
    priority: Some(3000.0),
    const_key: "--spacing".to_string(),
    const_value: "4px".to_string(),
  });

  assert_eq!(regular.rule_text(), konst.rule_text());
}
