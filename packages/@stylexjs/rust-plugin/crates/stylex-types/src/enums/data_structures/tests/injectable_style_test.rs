//! Tests for `InjectableStyleKind` → `InjectableStyleBaseKind` and the shared readers.

use crate::{
  enums::data_structures::injectable_style::InjectableStyleKind,
  structures::injectable_style::{InjectableConstStyle, InjectableStyle},
};

// rule_text

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
