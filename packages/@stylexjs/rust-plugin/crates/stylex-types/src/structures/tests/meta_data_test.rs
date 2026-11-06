//! Tests for MetaData construction, accessor methods, and
//! conversion from InjectableStylesMap.

use std::rc::Rc;

use indexmap::IndexMap;

use crate::{
  enums::data_structures::injectable_style::{InjectableStyleBaseKind, InjectableStyleKind},
  structures::{
    injectable_style::{InjectableConstStyle, InjectableStyle},
    meta_data::MetaData,
    style_key::RuleKey,
  },
  traits::InjectableStylesMap,
};

#[test]
fn test_new_with_regular_style() {
  let style = InjectableStyleKind::Regular(InjectableStyle {
    ltr: ".x{color:red}".to_string(),
    rtl: Some(".x{color:blue}".to_string()),
    priority: Some(1.0),
  });
  let meta = MetaData::new("x123".to_string(), style);
  assert_eq!(meta.get_class_name(), "x123");
  assert_eq!(meta.get_css(), ".x{color:red}");
  assert_eq!(meta.get_css_rtl(), Some(".x{color:blue}"));
  assert_eq!(*meta.get_priority(), 1.0);
  assert_eq!(meta.get_const_key(), None);
  assert_eq!(meta.get_const_value(), None);
}

/// Regular styles without explicit priority should default to 0.0.
#[test]
fn test_new_with_regular_style_without_priority_defaults_to_zero() {
  let style = InjectableStyleKind::Regular(InjectableStyle {
    ltr: ".x{color:red}".to_string(),
    rtl: None,
    priority: None,
  });
  let meta = MetaData::new("x000".to_string(), style);
  assert_eq!(*meta.get_priority(), 0.0);
}

#[test]
fn test_new_with_const_style() {
  let style = InjectableStyleKind::Const(InjectableConstStyle {
    ltr: ".y{font:bold}".to_string(),
    rtl: None,
    priority: None,
    const_key: "ck".to_string(),
    const_value: "cv".to_string(),
  });
  let meta = MetaData::new("y456".to_string(), style);
  assert_eq!(meta.get_class_name(), "y456");
  assert_eq!(meta.get_css(), ".y{font:bold}");
  assert_eq!(meta.get_css_rtl(), None);
  assert_eq!(*meta.get_priority(), 0.0);
  assert_eq!(meta.get_const_key(), Some("ck"));
  assert_eq!(meta.get_const_value(), Some("cv"));
}

#[test]
fn test_get_style_returns_ref() {
  let style = InjectableStyleKind::Regular(InjectableStyle {
    ltr: "a".to_string(),
    rtl: None,
    priority: Some(2.0),
  });
  let meta = MetaData::new("cls".to_string(), style);
  match meta.get_style() {
    InjectableStyleBaseKind::Regular(b) => assert_eq!(b.ltr, "a"),
    _ => panic!("Expected Regular"),
  }
}

#[test]
fn test_convert_from_injected_styles_map() {
  let mut map: InjectableStylesMap = IndexMap::new();
  map.insert(
    RuleKey::from("cls1"),
    Rc::new(InjectableStyleKind::Regular(InjectableStyle {
      ltr: "css1".to_string(),
      rtl: None,
      priority: Some(0.5),
    })),
  );
  map.insert(
    RuleKey::from("cls2"),
    Rc::new(InjectableStyleKind::Const(InjectableConstStyle {
      ltr: "css2".to_string(),
      rtl: Some("css2-rtl".to_string()),
      priority: Some(1.0),
      const_key: "k".to_string(),
      const_value: "v".to_string(),
    })),
  );
  let result = MetaData::convert_from_injected_styles_map(&map);
  assert_eq!(result.len(), 2);
  assert_eq!(result[0].get_class_name(), "cls1");
  assert_eq!(result[1].get_class_name(), "cls2");
}
