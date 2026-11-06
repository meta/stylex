//! Serialization-focused tests for MetaData.
//! Keeps JSON output behavior separated from constructor/accessor tests.

use serde_json::json;

use crate::{
  enums::data_structures::injectable_style::InjectableStyleKind,
  structures::{injectable_style::InjectableStyle, meta_data::MetaData},
};

/// Whole-number priorities should serialize as integer JSON values.
#[test]
fn serialize_priority_as_integer_when_whole_number() {
  let style = InjectableStyleKind::Regular(InjectableStyle {
    ltr: ".x{color:red}".to_string(),
    rtl: None,
    priority: Some(2.0),
  });
  let meta = MetaData::new("x123".to_string(), style);
  let value = serde_json::to_value(&meta).unwrap();
  assert_eq!(value["priority"], json!(2));
}

/// Fractional priorities should preserve decimal precision in JSON output.
#[test]
fn serialize_priority_as_float_when_fractional() {
  let style = InjectableStyleKind::Regular(InjectableStyle {
    ltr: ".x{color:red}".to_string(),
    rtl: None,
    priority: Some(2.5),
  });
  let meta = MetaData::new("x123".to_string(), style);
  let value = serde_json::to_value(&meta).unwrap();
  assert_eq!(value["priority"], json!(2.5));
}
