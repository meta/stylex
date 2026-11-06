//! Tests for InjectableStyle and InjectableConstStyle factory methods,
//! defaults, and conversions to base types.

use crate::{
  enums::data_structures::injectable_style::InjectableStyleKind,
  structures::injectable_style::{
    InjectableConstStyle, InjectableStyle, InjectableStyleBase, InjectableStyleConstBase,
  },
};

#[test]
fn test_regular_factory() {
  let style = InjectableStyle::regular("color:red".to_string(), Some(0.5));
  match style.as_ref() {
    InjectableStyleKind::Regular(s) => {
      assert_eq!(s.ltr, "color:red");
      assert_eq!(s.rtl, None);
      assert_eq!(s.priority, Some(0.5));
    },
    _ => panic!("Expected Regular"),
  }
}

#[test]
fn test_with_rtl_factory() {
  let style = InjectableStyle::with_rtl(
    "margin-left:10px".to_string(),
    "margin-right:10px".to_string(),
    Some(1.0),
  );
  match style.as_ref() {
    InjectableStyleKind::Regular(s) => {
      assert_eq!(s.ltr, "margin-left:10px");
      assert_eq!(s.rtl, Some("margin-right:10px".to_string()));
      assert_eq!(s.priority, Some(1.0));
    },
    _ => panic!("Expected Regular"),
  }
}

#[test]
fn test_default_injectable_style() {
  let style = InjectableStyle::default();
  assert_eq!(style.ltr, "");
  assert_eq!(style.rtl, None);
  assert_eq!(style.priority, Some(0.0));
}

#[test]
fn test_default_injectable_const_style() {
  let style = InjectableConstStyle::default();
  assert_eq!(style.ltr, "");
  assert_eq!(style.rtl, None);
  assert_eq!(style.priority, Some(0.0));
  assert_eq!(style.const_key, "");
  // The field holds JSON, and an empty text spells none, so the value a
  // default carries is the JSON for a constant that holds nothing.
  assert_eq!(style.const_value, "null");
}

#[test]
fn test_from_injectable_style_to_base() {
  let style = InjectableStyle {
    ltr: "color:red".to_string(),
    rtl: Some("color:blue".to_string()),
    priority: Some(1.0),
  };
  let base: InjectableStyleBase = style.into();
  assert_eq!(base.ltr, "color:red");
  assert_eq!(base.rtl, Some("color:blue".to_string()));
}

#[test]
fn test_from_injectable_const_style_to_base() {
  let style = InjectableConstStyle {
    ltr: "color:red".to_string(),
    rtl: None,
    priority: Some(0.5),
    const_key: "k".to_string(),
    const_value: "v".to_string(),
  };
  let base: InjectableStyleConstBase = style.into();
  assert_eq!(base.ltr, "color:red");
  assert_eq!(base.rtl, None);
  assert_eq!(base.const_key, "k");
  assert_eq!(base.const_value, "v");
}
