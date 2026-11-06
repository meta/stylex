//! Media query transform tests.

use crate::at_queries::media_query_transform::last_media_query_wins_transform;
use serde_json::{Value, json};
use swc_core::{
  atoms::Wtf8Atom,
  common::DUMMY_SP,
  ecma::ast::{Expr, KeyValueProp, ObjectLit, Prop, PropName, PropOrSpread, Str},
};

// Helper functions

/// Helper function to create KeyValueProp from key and JSON value
fn create_key_value_prop(key: &str, value: Value) -> KeyValueProp {
  KeyValueProp {
    key: PropName::Str(Str {
      span: DUMMY_SP,
      value: Wtf8Atom::from(key),
      raw: None,
    }),
    value: Box::new(json_to_expr(value)),
  }
}

/// Convert JSON Value to SWC AST Expr
fn json_to_expr(value: Value) -> Expr {
  match value {
    Value::String(s) => Expr::Lit(swc_core::ecma::ast::Lit::Str(Str {
      span: DUMMY_SP,
      value: Wtf8Atom::from(s),
      raw: None,
    })),
    Value::Object(map) => {
      let props = map
        .into_iter()
        .map(|(k, v)| {
          PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
            key: PropName::Str(Str {
              span: DUMMY_SP,
              value: Wtf8Atom::from(k),
              raw: None,
            }),
            value: Box::new(json_to_expr(v)),
          })))
        })
        .collect();

      Expr::Object(ObjectLit {
        span: DUMMY_SP,
        props,
      })
    },
    _ => Expr::Lit(swc_core::ecma::ast::Lit::Str(Str {
      span: DUMMY_SP,
      value: Wtf8Atom::from(value.to_string()),
      raw: None,
    })),
  }
}

/// Helper to convert Wtf8Atom to String
fn convert_atom_to_string(atom: &Wtf8Atom) -> String {
  atom
    .as_str()
    .expect("Failed to convert Wtf8Atom to &str")
    .to_string()
}

/// Helper to convert KeyValueProp back to JSON (for backwards compatibility)
fn key_value_prop_to_json(props: &[KeyValueProp]) -> Value {
  let mut map = serde_json::Map::new();

  for prop in props {
    let key = match &prop.key {
      PropName::Str(s) => convert_atom_to_string(&s.value),
      PropName::Ident(id) => id.sym.to_string(),
      _ => continue,
    };

    let value = expr_to_json(&prop.value);
    map.insert(key, value);
  }

  Value::Object(map)
}

/// Helper to convert SWC AST Expr back to JSON
fn expr_to_json(expr: &Expr) -> Value {
  match expr {
    Expr::Lit(lit) => match lit {
      swc_core::ecma::ast::Lit::Str(s) => Value::String(convert_atom_to_string(&s.value)),
      _ => Value::String(format!("{:?}", lit)),
    },
    Expr::Object(obj) => {
      let mut result = serde_json::Map::new();
      for prop in &obj.props {
        if let PropOrSpread::Prop(p) = prop
          && let Prop::KeyValue(kv) = &**p
        {
          let key = match &kv.key {
            PropName::Str(s) => convert_atom_to_string(&s.value),
            PropName::Ident(id) => id.sym.to_string(),
            _ => continue,
          };
          let value = expr_to_json(&kv.value);
          result.insert(key, value);
        }
      }
      Value::Object(result)
    },
    _ => Value::String(format!("{:?}", expr)),
  }
}

#[cfg(test)]
mod media_query_transformer {
  use super::*;

  /// Test: basic usage: multiple widths
  #[test]
  fn basic_usage_multiple_widths() {
    let original_styles = json!({
      "gridColumn": {
        "default": "1 / 2",
        "@media (max-width: 1440px)": "1 / 4",
        "@media (max-width: 1024px)": "1 / 3",
        "@media (max-width: 768px)": "1 / -1"
      }
    });

    let expected_styles = json!({
      "gridColumn": {
        "default": "1 / 2",
        "@media (min-width: 1024.01px) and (max-width: 1440px)": "1 / 4",
        "@media (min-width: 768.01px) and (max-width: 1024px)": "1 / 3",
        "@media (max-width: 768px)": "1 / -1"
      }
    });

    let input_props = if let Value::Object(obj) = original_styles {
      obj
        .into_iter()
        .map(|(k, v)| create_key_value_prop(&k, v))
        .collect::<Vec<_>>()
    } else {
      vec![]
    };

    let result = last_media_query_wins_transform(&input_props);
    let result_json = key_value_prop_to_json(&result);

    assert_eq!(
      serde_json::to_string(&result_json).unwrap(),
      serde_json::to_string(&expected_styles).unwrap()
    );
  }

  /// Test: keeps Chromium from rounding generated max-width boundaries
  #[test]
  fn keeps_chromium_from_rounding_generated_max_width_boundaries() {
    let original_styles = json!({
      "color": {
        "default": "blue",
        "@media (min-width: 400px)": "red",
        "@media (min-width: 600px)": "green"
      }
    });

    let expected_styles = json!({
      "color": {
        "default": "blue",
        "@media (min-width: 400px) and (max-width: 599.98px)": "red",
        "@media (min-width: 600px)": "green"
      }
    });

    let input_props = if let Value::Object(obj) = original_styles {
      obj
        .into_iter()
        .map(|(k, v)| create_key_value_prop(&k, v))
        .collect::<Vec<_>>()
    } else {
      vec![]
    };

    let result = last_media_query_wins_transform(&input_props);
    let result_json = key_value_prop_to_json(&result);

    assert_eq!(
      serde_json::to_string(&result_json).unwrap(),
      serde_json::to_string(&expected_styles).unwrap()
    );
  }

  /// Test: basic usage: nested query
  #[test]
  fn basic_usage_nested_query() {
    let original_styles = json!({
      "gridColumn": {
        "default": "1 / 2",
        "@media (max-width: 1440px)": {
          "@media (max-height: 1024px)": "1 / 3",
          "@media (max-height: 768px)": "1 / -1"
        },
        "@media (max-width: 1024px)": "1 / 3",
        "@media (max-width: 768px)": "1 / -1"
      }
    });

    let expected_styles = json!({
      "gridColumn": {
        "default": "1 / 2",
        "@media (min-width: 1024.01px) and (max-width: 1440px)": {
          "@media (min-height: 768.01px) and (max-height: 1024px)": "1 / 3",
          "@media (max-height: 768px)": "1 / -1"
        },
        "@media (min-width: 768.01px) and (max-width: 1024px)": "1 / 3",
        "@media (max-width: 768px)": "1 / -1"
      }
    });

    let input_props = if let Value::Object(obj) = original_styles {
      obj
        .into_iter()
        .map(|(k, v)| create_key_value_prop(&k, v))
        .collect::<Vec<_>>()
    } else {
      vec![]
    };

    let result = last_media_query_wins_transform(&input_props);
    let result_json = key_value_prop_to_json(&result);

    assert_eq!(
      serde_json::to_string(&result_json).unwrap(),
      serde_json::to_string(&expected_styles).unwrap(),
      "Nested queries should be transformed correctly"
    );
  }

  /// Test: basic usage: nested query
  #[test]
  fn basic_usage_nested_query_with_padding() {
    let original_styles = json!({
      "gridColumn": {
        "default": "1 / 2",
        "@media (max-width: 1440px)": {
          "@media (max-width: 1024px)": "1 / 3",
          "@media (max-width: 768px)": "1 / -1"
        },
        "@media (max-width: 1024px)": "1 / 3",
        "@media (max-width: 768px)": "1 / -1"
      },
      "padding": "10px"
    });

    let expected_styles = json!({
      "gridColumn": {
        "default": "1 / 2",
        "@media (min-width: 1024.01px) and (max-width: 1440px)": {
          "@media (min-width: 768.01px) and (max-width: 1024px)": "1 / 3",
          "@media (max-width: 768px)": "1 / -1"
        },
        "@media (min-width: 768.01px) and (max-width: 1024px)": "1 / 3",
        "@media (max-width: 768px)": "1 / -1"
      },
      "padding": "10px"
    });

    let input_props = if let Value::Object(obj) = original_styles {
      obj
        .into_iter()
        .map(|(k, v)| create_key_value_prop(&k, v))
        .collect::<Vec<_>>()
    } else {
      vec![]
    };

    let result = last_media_query_wins_transform(&input_props);
    let result_json = key_value_prop_to_json(&result);

    assert_eq!(
      serde_json::to_string(&result_json).unwrap(),
      serde_json::to_string(&expected_styles).unwrap(),
      "Nested queries with additional properties should be transformed correctly"
    );
  }

  /// Test: basic usage: complex object
  #[test]
  fn basic_usage_complex_object() {
    let original_styles = json!({
      "gridColumn": {
        "default": "1 / 2",
        "@media (max-width: 1440px)": "1 / 4",
        "@media (max-width: 1024px)": "1 / 3",
        "@media (max-width: 768px)": "1 / -1"
      },
      "grid": {
        "default": "1 / 2",
        "@media (max-width: 1440px)": "1 / 4"
      },
      "gridRow": {
        "default": "1 / 2",
        "padding": "10px"
      }
    });

    let expected_styles = json!({
      "gridColumn": {
        "default": "1 / 2",
        "@media (min-width: 1024.01px) and (max-width: 1440px)": "1 / 4",
        "@media (min-width: 768.01px) and (max-width: 1024px)": "1 / 3",
        "@media (max-width: 768px)": "1 / -1"
      },
      "grid": {
        "default": "1 / 2",
        "@media (max-width: 1440px)": "1 / 4"
      },
      "gridRow": {
        "default": "1 / 2",
        "padding": "10px"
      }
    });

    let input_props = if let Value::Object(obj) = original_styles {
      obj
        .into_iter()
        .map(|(k, v)| create_key_value_prop(&k, v))
        .collect::<Vec<_>>()
    } else {
      vec![]
    };

    let result = last_media_query_wins_transform(&input_props);
    let result_json = key_value_prop_to_json(&result);

    assert_eq!(
      serde_json::to_string(&result_json).unwrap(),
      serde_json::to_string(&expected_styles).unwrap(),
      "Complex object with multiple properties should be handled correctly"
    );
  }

  /// Test: basic usage: lots and lots of max-widths
  #[test]
  fn basic_usage_lots_and_lots_of_max_widths() {
    let original_styles = json!({
      "gridColumn": {
        "default": "1 / 2",
        "@media (max-width: 1440px)": "1 / 4",
        "@media (max-width: 1024px)": "1 / 3",
        "@media (max-width: 768px)": "1 / -1",
        "@media (max-width: 458px)": "1 / -1"
      }
    });

    let expected_styles = json!({
      "gridColumn": {
        "default": "1 / 2",
        "@media (min-width: 1024.01px) and (max-width: 1440px)": "1 / 4",
        "@media (min-width: 768.01px) and (max-width: 1024px)": "1 / 3",
        "@media (min-width: 458.01px) and (max-width: 768px)": "1 / -1",
        "@media (max-width: 458px)": "1 / -1"
      }
    });

    let input_props = if let Value::Object(obj) = original_styles {
      obj
        .into_iter()
        .map(|(k, v)| create_key_value_prop(&k, v))
        .collect::<Vec<_>>()
    } else {
      vec![]
    };

    let result = last_media_query_wins_transform(&input_props);
    let result_json = key_value_prop_to_json(&result);

    assert_eq!(
      serde_json::to_string(&result_json).unwrap(),
      serde_json::to_string(&expected_styles).unwrap(),
      "Multiple max-width queries should be transformed into ranges"
    );
  }

  /// Test: basic usage: lots and lots of min-widths
  #[test]
  fn basic_usage_lots_and_lots_of_min_widths() {
    let original_styles = json!({
      "gridColumn": {
        "default": "1 / 2",
        "@media (min-width: 768px)": "1 / -1",
        "@media (min-width: 1024px)": "1 / 3",
        "@media (min-width: 1440px)": "1 / 4"
      }
    });

    let expected_styles = json!({
      "gridColumn": {
        "default": "1 / 2",
        "@media (min-width: 768px) and (max-width: 1023.98px)": "1 / -1",
        "@media (min-width: 1024px) and (max-width: 1439.98px)": "1 / 3",
        "@media (min-width: 1440px)": "1 / 4"
      }
    });

    let input_props = if let Value::Object(obj) = original_styles {
      obj
        .into_iter()
        .map(|(k, v)| create_key_value_prop(&k, v))
        .collect::<Vec<_>>()
    } else {
      vec![]
    };

    let result = last_media_query_wins_transform(&input_props);
    let result_json = key_value_prop_to_json(&result);

    assert_eq!(
      serde_json::to_string(&result_json).unwrap(),
      serde_json::to_string(&expected_styles).unwrap(),
      "Multiple min-width queries should be transformed into ranges with .99px boundaries"
    );
  }

  /// Test: basic usage: multiple heights
  #[test]
  fn basic_usage_multiple_heights() {
    let original_styles = json!({
      "foo": {
        "gridRow": {
          "default": "1 / 2",
          "@media (max-height: 1200px)": "1 / 4",
          "@media (max-height: 900px)": "1 / 3",
          "@media (max-height: 600px)": "1 / -1"
        }
      }
    });

    let expected_styles = json!({
      "foo": {
        "gridRow": {
          "default": "1 / 2",
          "@media (min-height: 900.01px) and (max-height: 1200px)": "1 / 4",
          "@media (min-height: 600.01px) and (max-height: 900px)": "1 / 3",
          "@media (max-height: 600px)": "1 / -1"
        }
      }
    });

    let input_props = if let Value::Object(obj) = original_styles {
      obj
        .into_iter()
        .map(|(k, v)| create_key_value_prop(&k, v))
        .collect::<Vec<_>>()
    } else {
      vec![]
    };

    let result = last_media_query_wins_transform(&input_props);
    let result_json = key_value_prop_to_json(&result);

    assert_eq!(
      serde_json::to_string(&result_json).unwrap(),
      serde_json::to_string(&expected_styles).unwrap(),
      "Multiple height queries should be transformed into ranges"
    );
  }

  /// Test: basic usage: min/max heights
  #[test]
  fn basic_usage_min_max_heights() {
    let original_styles = json!({
      "foo": {
        "gridRow": {
          "default": "1 / 2",
          "@media (min-height: 1200px) and (max-height: 1400px)": "1 / 4",
          "@media (max-height: 900px)": "1 / 3",
          "@media (max-height: 600px)": "1 / -1"
        }
      }
    });

    let expected_styles = json!({
      "foo": {
        "gridRow": {
          "default": "1 / 2",
          "@media (min-height: 1200px) and (max-height: 1400px)": "1 / 4",
          "@media (min-height: 600.01px) and (max-height: 900px)": "1 / 3",
          "@media (max-height: 600px)": "1 / -1"
        }
      }
    });

    let input_props = if let Value::Object(obj) = original_styles {
      obj
        .into_iter()
        .map(|(k, v)| create_key_value_prop(&k, v))
        .collect::<Vec<_>>()
    } else {
      vec![]
    };

    let result = last_media_query_wins_transform(&input_props);
    let result_json = key_value_prop_to_json(&result);

    assert_eq!(
      serde_json::to_string(&result_json).unwrap(),
      serde_json::to_string(&expected_styles).unwrap(),
      "Min/max height queries should preserve existing ranges"
    );
  }

  /// Test: single word condition
  #[test]
  fn single_word_condition() {
    let original_styles = json!({
      "mode": {
        "default": "normal",
        "@media (color)": "colorful",
        "@media (monochrome)": "grayscale"
      }
    });

    let expected_styles = json!({
      "mode": {
        "default": "normal",
        "@media (color) and (not (monochrome))": "colorful",
        "@media (monochrome)": "grayscale"
      }
    });

    let input_props = if let Value::Object(obj) = original_styles {
      obj
        .into_iter()
        .map(|(k, v)| create_key_value_prop(&k, v))
        .collect::<Vec<_>>()
    } else {
      vec![]
    };

    let result = last_media_query_wins_transform(&input_props);
    let result_json = key_value_prop_to_json(&result);

    assert_eq!(
      serde_json::to_string(&result_json).unwrap(),
      serde_json::to_string(&expected_styles).unwrap(),
      "Single word conditions should include negations"
    );
  }

  /// Test: handles comma-separated (or) media queries
  #[test]
  fn handles_comma_separated_or_media_queries() {
    let original_styles = json!({
      "width": {
        "default": "100%",
        "@media screen, (max-width: 800px)": "80%",
        "@media (max-width: 500px)": "60%"
      }
    });

    let expected_styles = json!({
      "width": {
        "default": "100%",
        "@media (screen) and (not (max-width: 500px)), (min-width: 500.01px) and (max-width: 800px)": "80%",
        "@media (max-width: 500px)": "60%"
      }
    });

    let input_props = if let Value::Object(obj) = original_styles {
      obj
        .into_iter()
        .map(|(k, v)| create_key_value_prop(&k, v))
        .collect::<Vec<_>>()
    } else {
      vec![]
    };

    let result = last_media_query_wins_transform(&input_props);
    let result_json = key_value_prop_to_json(&result);

    assert_eq!(
      serde_json::to_string(&result_json).unwrap(),
      serde_json::to_string(&expected_styles).unwrap(),
      "Comma-separated media queries should handle OR logic with negations"
    );
  }

  /// Test: basic usage: does not modify single queries
  #[test]
  fn basic_usage_does_not_modify_single_queries() {
    let original_styles = json!({
      "gridColumn": {
        "default": "1 / 2",
        "@media (max-width: 1440px)": "1 / 4"
      }
    });

    let expected_styles = json!({
      "gridColumn": {
        "default": "1 / 2",
        "@media (max-width: 1440px)": "1 / 4"
      }
    });

    let input_props = if let Value::Object(obj) = original_styles {
      obj
        .into_iter()
        .map(|(k, v)| create_key_value_prop(&k, v))
        .collect::<Vec<_>>()
    } else {
      vec![]
    };

    let result = last_media_query_wins_transform(&input_props);
    let result_json = key_value_prop_to_json(&result);

    assert_eq!(
      serde_json::to_string(&result_json).unwrap(),
      serde_json::to_string(&expected_styles).unwrap(),
      "Single queries should not be modified"
    );
  }

  /// Test: ignores legacy media query syntax
  #[test]
  fn ignores_legacy_media_query_syntax() {
    let original_styles = json!({
      "width": "100%",
      "@media (min-width: 600px)": {
        "width": "50%"
      }
    });

    let expected_styles = json!({
      "width": "100%",
      "@media (min-width: 600px)": {
        "width": "50%"
      }
    });

    let input_props = if let Value::Object(obj) = original_styles {
      obj
        .into_iter()
        .map(|(k, v)| create_key_value_prop(&k, v))
        .collect::<Vec<_>>()
    } else {
      vec![]
    };

    let result = last_media_query_wins_transform(&input_props);
    let result_json = key_value_prop_to_json(&result);

    assert_eq!(
      serde_json::to_string(&result_json).unwrap(),
      serde_json::to_string(&expected_styles).unwrap(),
      "Legacy media query syntax should be ignored"
    );
  }

  /// Test: mixed min/max width and height
  #[test]
  fn mixed_min_max_width_and_height() {
    let original_styles = json!({
      "foo": {
        "gridColumn": {
          "default": "1 / 2",
          "@media (max-width: 1440px) and (max-height: 900px)": "1 / 4",
          "@media (max-width: 1024px)": "1 / 3",
          "@media (max-width: 768px)": "1 / -1"
        }
      }
    });

    let expected_styles = json!({
      "foo": {
        "gridColumn": {
          "default": "1 / 2",
          "@media (min-width: 1024.01px) and (max-width: 1440px) and (max-height: 900px)": "1 / 4",
          "@media (min-width: 768.01px) and (max-width: 1024px)": "1 / 3",
          "@media (max-width: 768px)": "1 / -1"
        }
      }
    });

    let input_props = if let Value::Object(obj) = original_styles {
      obj
        .into_iter()
        .map(|(k, v)| create_key_value_prop(&k, v))
        .collect::<Vec<_>>()
    } else {
      vec![]
    };

    let result = last_media_query_wins_transform(&input_props);
    let result_json = key_value_prop_to_json(&result);

    assert_eq!(
      serde_json::to_string(&result_json).unwrap(),
      serde_json::to_string(&expected_styles).unwrap(),
      "Mixed width/height queries should preserve height conditions"
    );
  }

  /// Test: mixed min/max width and height with only height changing
  #[test]
  fn mixed_min_max_width_and_height_with_only_height_changing() {
    let original_styles = json!({
      "foo": {
        "gridColumn": {
          "default": "1 / 2",
          "@media (max-width: 1440px) and (max-height: 900px)": "1 / 4",
          "@media (max-height: 700px)": "1 / 3",
          "@media (max-height: 500px)": "1 / -1"
        }
      }
    });

    let expected_styles = json!({
      "foo": {
        "gridColumn": {
          "default": "1 / 2",
          "@media (max-width: 1440px) and (min-height: 700.01px) and (max-height: 900px)": "1 / 4",
          "@media (min-height: 500.01px) and (max-height: 700px)": "1 / 3",
          "@media (max-height: 500px)": "1 / -1"
        }
      }
    });

    let input_props = if let Value::Object(obj) = original_styles {
      obj
        .into_iter()
        .map(|(k, v)| create_key_value_prop(&k, v))
        .collect::<Vec<_>>()
    } else {
      vec![]
    };

    let result = last_media_query_wins_transform(&input_props);
    let result_json = key_value_prop_to_json(&result);

    assert_eq!(
      serde_json::to_string(&result_json).unwrap(),
      serde_json::to_string(&expected_styles).unwrap(),
      "Mixed queries with only height changing should transform height ranges"
    );
  }

  /// Test: mixed min/max width with disjoint ranges
  #[test]
  fn mixed_min_max_width_with_disjoint_ranges() {
    let original_styles = json!({
      "foo": {
        "gridColumn": {
          "default": "1 / 2",
          "@media (max-width: 1440px) and (min-width: 900px)": "1 / 4",
          "@media (max-width: 800px) and (min-width: 600px)": "1 / 3"
        }
      }
    });

    let expected_styles = json!({
      "foo": {
        "gridColumn": {
          "default": "1 / 2",
          "@media (min-width: 900px) and (max-width: 1440px)": "1 / 4",
          "@media (min-width: 600px) and (max-width: 800px)": "1 / 3"
        }
      }
    });

    let input_props = if let Value::Object(obj) = original_styles {
      obj
        .into_iter()
        .map(|(k, v)| create_key_value_prop(&k, v))
        .collect::<Vec<_>>()
    } else {
      vec![]
    };

    let result = last_media_query_wins_transform(&input_props);
    let result_json = key_value_prop_to_json(&result);

    assert_eq!(
      serde_json::to_string(&result_json).unwrap(),
      serde_json::to_string(&expected_styles).unwrap(),
      "Mixed width disjoint ranges should be transformed with negation logic"
    );
  }

  /// Test: mixed min/max width with many disjoint ranges
  #[test]
  fn mixed_min_max_width_with_many_disjoint_ranges() {
    let original_styles = json!({
      "foo": {
        "gridColumn": {
          "default": "1 / 2",
          "@media (max-width: 1440px) and (min-width: 900px)": "1 / 4",
          "@media (max-width: 800px) and (min-width: 600px)": "1 / 3",
          "@media (max-width: 500px) and (min-width: 400px)": "1 / 1",
        }
      }
    });

    let expected_styles = json!({
      "foo": {
        "gridColumn": {
          "default": "1 / 2",
          "@media (min-width: 900px) and (max-width: 1440px)": "1 / 4",
          "@media (min-width: 600px) and (max-width: 800px)": "1 / 3",
          "@media (min-width: 400px) and (max-width: 500px)": "1 / 1",
        }
      }
    });

    let input_props = if let Value::Object(obj) = original_styles {
      obj
        .into_iter()
        .map(|(k, v)| create_key_value_prop(&k, v))
        .collect::<Vec<_>>()
    } else {
      vec![]
    };

    let result = last_media_query_wins_transform(&input_props);
    let result_json = key_value_prop_to_json(&result);

    assert_eq!(
      serde_json::to_string(&result_json).unwrap(),
      serde_json::to_string(&expected_styles).unwrap(),
      "Mixed width many disjoint ranges should be transformed with negation logic"
    );
  }

  /// Test: mixed min/max width with mixed ranges
  #[test]
  fn mixed_min_max_width_with_mixed_ranges() {
    let original_styles = json!({
      "foo": {
        "gridColumn": {
          "default": "1 / 2",
          "@media (max-width: 1440px) and (min-width: 900px)": "1 / 4",
          "@media (max-width: 1100px) and (min-width: 1000px)": "1 / 3",
          "@media (max-width: 500px) and (min-width: 400px)": "1 / 1"
        }
      }
    });

    let expected_styles = json!({
      "foo": {
        "gridColumn": {
          "default": "1 / 2",
          "@media ((min-width: 900px) and (max-width: 999.98px)) or ((min-width: 1100.01px) and (max-width: 1440px))": "1 / 4",
          "@media (min-width: 1000px) and (max-width: 1100px)": "1 / 3",
          "@media (min-width: 400px) and (max-width: 500px)": "1 / 1"
        }
      }
    });

    let input_props = if let Value::Object(obj) = original_styles {
      obj
        .into_iter()
        .map(|(k, v)| create_key_value_prop(&k, v))
        .collect::<Vec<_>>()
    } else {
      vec![]
    };

    let result = last_media_query_wins_transform(&input_props);
    let result_json = key_value_prop_to_json(&result);

    assert_eq!(
      serde_json::to_string(&result_json).unwrap(),
      serde_json::to_string(&expected_styles).unwrap(),
      "Mixed width ranges with intersections should use OR logic"
    );
  }

  /// Test: mixed min/max width with intersecting ranges
  #[test]
  fn mixed_min_max_width_with_intersecting_ranges() {
    let original_styles = json!({
      "foo": {
        "gridColumn": {
          "default": "1 / 2",
          "@media (max-width: 1440px) and (min-width: 900px)": "1 / 4",
          "@media (max-width: 1100px) and (min-width: 1000px)": "1 / 3"
        }
      }
    });

    let expected_styles = json!({
      "foo": {
        "gridColumn": {
          "default": "1 / 2",
          "@media ((min-width: 900px) and (max-width: 999.98px)) or ((min-width: 1100.01px) and (max-width: 1440px))": "1 / 4",
          "@media (min-width: 1000px) and (max-width: 1100px)": "1 / 3"
        }
      }
    });

    let input_props = if let Value::Object(obj) = original_styles {
      obj
        .into_iter()
        .map(|(k, v)| create_key_value_prop(&k, v))
        .collect::<Vec<_>>()
    } else {
      vec![]
    };

    let result = last_media_query_wins_transform(&input_props);
    let result_json = key_value_prop_to_json(&result);

    assert_eq!(
      serde_json::to_string(&result_json).unwrap(),
      serde_json::to_string(&expected_styles).unwrap(),
      "Mixed width intersecting ranges should split with OR logic"
    );
  }

  /// Test: mixed min/max width with many intersecting ranges
  #[test]
  fn mixed_min_max_width_with_many_intersecting_ranges() {
    let original_styles = json!({
      "foo": {
        "gridColumn": {
          "default": "1 / 2",
          "@media (max-width: 1440px) and (min-width: 900px)": "1 / 4",
          "@media (max-width: 1100px) and (min-width: 1000px)": "1 / 3",
          "@media (max-width: 1050px) and (min-width: 1010px)": "1 / -1"
        }
      }
    });

    let expected_styles = json!({
      "foo": {
        "gridColumn": {
          "default": "1 / 2",
          "@media ((min-width: 900px) and (max-width: 999.98px)) or ((min-width: 1100.01px) and (max-width: 1440px))": "1 / 4",
          "@media ((min-width: 1000px) and (max-width: 1009.98px)) or ((min-width: 1050.01px) and (max-width: 1100px))": "1 / 3",
          "@media (min-width: 1010px) and (max-width: 1050px)": "1 / -1"
        }
      }
    });

    let input_props = if let Value::Object(obj) = original_styles {
      obj
        .into_iter()
        .map(|(k, v)| create_key_value_prop(&k, v))
        .collect::<Vec<_>>()
    } else {
      vec![]
    };

    let result = last_media_query_wins_transform(&input_props);
    let result_json = key_value_prop_to_json(&result);

    assert_eq!(
      serde_json::to_string(&result_json).unwrap(),
      serde_json::to_string(&expected_styles).unwrap(),
      "Mixed width many intersecting ranges should split with complex OR logic"
    );
  }

  /// Test: mixed min/max width with overlapping ranges
  #[test]
  fn mixed_min_max_width_with_overlapping_ranges() {
    let original_styles = json!({
      "foo": {
        "gridColumn": {
          "default": "1 / 2",
          "@media (max-width: 1440px) and (min-width: 900px)": "1 / 4",
          "@media (max-width: 1040px) and (min-width: 600px)": "1 / 3"
        }
      }
    });

    let expected_styles = json!({
      "foo": {
        "gridColumn": {
          "default": "1 / 2",
          "@media (min-width: 1040.01px) and (max-width: 1440px)": "1 / 4",
          "@media (min-width: 600px) and (max-width: 1040px)": "1 / 3"
        }
      }
    });

    let input_props = if let Value::Object(obj) = original_styles {
      obj
        .into_iter()
        .map(|(k, v)| create_key_value_prop(&k, v))
        .collect::<Vec<_>>()
    } else {
      vec![]
    };

    let result = last_media_query_wins_transform(&input_props);
    let result_json = key_value_prop_to_json(&result);

    assert_eq!(
      serde_json::to_string(&result_json).unwrap(),
      serde_json::to_string(&expected_styles).unwrap(),
      "Mixed width overlapping ranges should split at boundaries"
    );
  }

  /// Test: handles and media queries
  #[test]
  fn handles_and_media_queries() {
    let original_styles = json!({
      "width": {
        "default": "100%",
        "@media (min-width: 900px)": "80%",
        "@media (min-width: 500px) and (max-width: 899px) and (max-height: 300px)": "50%"
      }
    });

    let expected_styles = json!({
      "width": {
        "default": "100%",
        "@media (min-width: 900px) and (not ((min-width: 500px) and (max-width: 899px) and (max-height: 300px)))": "80%",
        "@media (min-width: 500px) and (max-width: 899px) and (max-height: 300px)": "50%"
      }
    });

    let input_props = if let Value::Object(obj) = original_styles {
      obj
        .into_iter()
        .map(|(k, v)| create_key_value_prop(&k, v))
        .collect::<Vec<_>>()
    } else {
      vec![]
    };

    let result = last_media_query_wins_transform(&input_props);
    let result_json = key_value_prop_to_json(&result);

    assert_eq!(
      serde_json::to_string(&result_json).unwrap(),
      serde_json::to_string(&expected_styles).unwrap(),
      "Complex AND combinations should handle negations correctly"
    );
  }

  /// Test: combination of keywords and rules
  #[test]
  fn combination_of_keywords_and_rules() {
    let original_styles = json!({
      "width": {
        "default": "100%",
        "@media screen and (min-width: 900px)": "80%",
        "@media print and (max-width: 500px)": "50%"
      }
    });

    let expected_styles = json!({
      "width": {
        "default": "100%",
        "@media ((screen) and (min-width: 900px) and (not (print))) or ((screen) and (min-width: 900px) and (not (max-width: 500px)))": "80%",
        "@media (print) and (max-width: 500px)": "50%"
      }
    });

    let input_props = if let Value::Object(obj) = original_styles {
      obj
        .into_iter()
        .map(|(k, v)| create_key_value_prop(&k, v))
        .collect::<Vec<_>>()
    } else {
      vec![]
    };

    let result = last_media_query_wins_transform(&input_props);
    let result_json = key_value_prop_to_json(&result);

    assert_eq!(
      serde_json::to_string(&result_json).unwrap(),
      serde_json::to_string(&expected_styles).unwrap(),
      "Combination of keywords and rules should handle negations correctly"
    );
  }

  #[test]
  fn media_queries_with_em_units() {
    let original_styles = json!({
      "foo": {
        "gridColumn": {
          "default": "1 / 2",
          "@media (max-width: 90em) and (min-width: 60em)": "1 / 4",
          "@media (max-width: 70em) and (min-width: 65em)": "1 / 3"
        }
      }
    });

    let expected_styles = json!({
      "foo": {
        "gridColumn": {
          "default": "1 / 2",
          "@media ((min-width: 60em) and (max-width: 64.99em)) or ((min-width: 70.01em) and (max-width: 90em))": "1 / 4",
          "@media (min-width: 65em) and (max-width: 70em)": "1 / 3"
        }
      }
    });

    let input_props = if let Value::Object(obj) = original_styles {
      obj
        .into_iter()
        .map(|(k, v)| create_key_value_prop(&k, v))
        .collect::<Vec<_>>()
    } else {
      vec![]
    };

    let result = last_media_query_wins_transform(&input_props);
    let result_json = key_value_prop_to_json(&result);

    assert_eq!(
      serde_json::to_string(&result_json).unwrap(),
      serde_json::to_string(&expected_styles).unwrap()
    );
  }

  #[test]
  fn media_queries_with_mixed_units() {
    let original_styles = json!({
      "foo": {
        "gridColumn": {
          "default": "1 / 2",
          "@media (max-width: 1200px) and (min-height: 50vh)": "1 / 4",
          "@media (max-width: 800px) and (min-height: 30vh)": "1 / 3"
        }
      }
    });

    let expected_styles = json!({
      "foo": {
        "gridColumn": {
          "default": "1 / 2",
          "@media (min-width: 800.01px) and (max-width: 1200px) and (min-height: 50vh)": "1 / 4",
          "@media (max-width: 800px) and (min-height: 30vh)": "1 / 3"
        }
      }
    });

    let input_props = if let Value::Object(obj) = original_styles {
      obj
        .into_iter()
        .map(|(k, v)| create_key_value_prop(&k, v))
        .collect::<Vec<_>>()
    } else {
      vec![]
    };

    let result = last_media_query_wins_transform(&input_props);
    let result_json = key_value_prop_to_json(&result);

    assert_eq!(
      serde_json::to_string(&result_json).unwrap(),
      serde_json::to_string(&expected_styles).unwrap()
    );
  }

  #[test]
  fn skips_range_simplification_for_media_queries_with_conflicting_units_in_same_dimension_across_queries()
   {
    let original_styles = json!({
      "foo": {
        "gridColumn": {
          "default": "1 / 2",
          "@media (min-width: 768px) and (max-width: 1200px)": "1 / 4",
          "@media (min-width: 50em)": "1 / 3"
        }
      }
    });

    let expected_styles = json!({
      "foo": {
        "gridColumn": {
          "default": "1 / 2",
          "@media (min-width: 768px) and (max-width: 1200px) and (not (min-width: 50em))": "1 / 4",
          "@media (min-width: 50em)": "1 / 3"
        }
      }
    });

    let input_props = if let Value::Object(obj) = original_styles {
      obj
        .into_iter()
        .map(|(k, v)| create_key_value_prop(&k, v))
        .collect::<Vec<_>>()
    } else {
      vec![]
    };

    let result = last_media_query_wins_transform(&input_props);
    let result_json = key_value_prop_to_json(&result);

    assert_eq!(
      serde_json::to_string(&result_json).unwrap(),
      serde_json::to_string(&expected_styles).unwrap()
    );
  }

  #[test]
  fn skips_range_simplification_for_media_queries_with_conflicting_units_in_same_dimension_and_query()
   {
    let original_styles = json!({
      "foo": {
        "gridColumn": {
          "default": "1 / 2",
          "@media (min-width: 768px) and (max-width: 1200em)": "1 / 4",
          "@media (min-width: 50em)": "1 / 3"
        }
      }
    });

    let expected_styles = json!({
      "foo": {
        "gridColumn": {
          "default": "1 / 2",
          "@media (min-width: 768px) and (max-width: 1200em) and (not (min-width: 50em))": "1 / 4",
          "@media (min-width: 50em)": "1 / 3"
        }
      }
    });

    let input_props = if let Value::Object(obj) = original_styles {
      obj
        .into_iter()
        .map(|(k, v)| create_key_value_prop(&k, v))
        .collect::<Vec<_>>()
    } else {
      vec![]
    };

    let result = last_media_query_wins_transform(&input_props);
    let result_json = key_value_prop_to_json(&result);

    assert_eq!(
      serde_json::to_string(&result_json).unwrap(),
      serde_json::to_string(&expected_styles).unwrap()
    );
  }

  #[test]
  fn handles_only_screen_media_queries_without_parenthesizing_the_media_type() {
    let original_styles = json!({
      "color": {
        "default": null,
        "@media only screen and (max-width: 600px)": "red",
        "@media only screen and (max-width: 400px)": "blue"
      }
    });

    let input_props = if let Value::Object(obj) = original_styles {
      obj
        .into_iter()
        .map(|(k, v)| create_key_value_prop(&k, v))
        .collect::<Vec<_>>()
    } else {
      vec![]
    };

    let result = last_media_query_wins_transform(&input_props);
    let result_json = key_value_prop_to_json(&result);
    let result_str = serde_json::to_string(&result_json).unwrap();

    assert!(!result_str.contains("only (screen)"));
    assert!(result_str.contains("only screen"));
  }
}

/// Every `@media` key is deleted and re-added, which moves it to the end of the
/// object — including when there is only one, where nothing else about the key
/// changes. The resulting order reaches the emitted CSS, so it is asserted
/// here.
#[test]
fn single_media_query_moves_after_the_default() {
  let props = vec![create_key_value_prop(
    "color",
    json!({
      "@media (max-width: 900px) and (min-width: 100px)": "red",
      "default": "blue",
    }),
  )];

  let result = last_media_query_wins_transform(&props);
  let color = match &*result[0].value {
    Expr::Object(object) => object,
    _ => panic!("expected an object value"),
  };

  let keys: Vec<String> = color
    .props
    .iter()
    .filter_map(|prop| match prop {
      PropOrSpread::Prop(prop) => match &**prop {
        Prop::KeyValue(kv) => match &kv.key {
          PropName::Str(key) => Some(convert_atom_to_string(&key.value)),
          _ => None,
        },
        _ => None,
      },
      _ => None,
    })
    .collect();

  assert_eq!(
    keys,
    vec![
      "default".to_string(),
      // The lone query is re-stringified too, which reorders its features.
      "@media (min-width: 100px) and (max-width: 900px)".to_string(),
    ]
  );
}

// ---------------------------------------------------------------------------
// Computed bounds at double precision — https://github.com/Dwlad90/stylex-swc-plugin/issues/1267
// ---------------------------------------------------------------------------

/// A style object as the props the transform takes.
fn props_of(styles: Value) -> Vec<KeyValueProp> {
  match styles {
    Value::Object(obj) => obj
      .into_iter()
      .map(|(k, v)| create_key_value_prop(&k, v))
      .collect(),
    other => panic!("expected an object, got {other:?}"),
  }
}

/// The whole transformed object, keys and values both.
///
/// `transformed_keys` below answers what the rewritten queries are; this
/// answers what survived under them, which is the half a collision moves.
fn transformed_styles(styles: Value) -> Value {
  key_value_prop_to_json(&last_media_query_wins_transform(&props_of(styles)))
}

/// The single property's entries, in order, as key and value pairs.
///
/// A pair list rather than the map itself, because order is half of what these
/// tests assert and two maps holding the same entries in different orders
/// compare equal.
fn transformed_entries(styles: Value) -> Vec<(String, Value)> {
  match transformed_styles(styles) {
    Value::Object(obj) => match obj.into_iter().next() {
      Some((_, Value::Object(inner))) => inner.into_iter().collect(),
      other => panic!("expected one property holding an object, got {other:?}"),
    },
    other => panic!("expected an object, got {other:?}"),
  }
}

/// One authored `@media` key, with a later key after it to negate, and the
/// rewritten text the first one came out as.
///
/// The trailing key is what makes the first one interesting: a key with nothing
/// after it is handed back as authored, which tests nothing about the rewrite.
fn rewritten_first_key(query: &str) -> String {
  transformed_keys(json!({
    "color": { "default": "black", query: "red", "@media (max-width: 50px)": "blue" }
  }))[1]
    .clone()
}

/// Whether the transform refuses `query` outright.
///
/// The refusal reaches a caller as a panic, which the compiler turns into the
/// invalid-media-query-syntax error. The message is checked rather than the
/// mere fact of a panic, so an unrelated one -- an index out of range, say --
/// cannot read as a refusal.
fn refuses_query(query: &str) -> bool {
  let styles = json!({
    "color": { "default": "black", query: "red", "@media (max-width: 50px)": "blue" }
  });

  match std::panic::catch_unwind(|| transformed_keys(styles)) {
    Ok(_) => false,
    Err(payload) => match payload.downcast_ref::<String>() {
      Some(message) => message.contains("Invalid media query"),
      None => match payload.downcast_ref::<&str>() {
        Some(message) => message.contains("Invalid media query"),
        None => false,
      },
    },
  }
}

/// Run the transform over `styles` and return the keys of the single property
/// it contains, in order.
fn transformed_keys(styles: Value) -> Vec<String> {
  transformed_entries(styles)
    .into_iter()
    .map(|(key, _)| key)
    .collect()
}

#[cfg(test)]
mod computed_bounds_carry_the_authored_digits {
  use super::*;

  /// The reproduction from issue #1267. Each derived upper bound is
  /// `next - 0.01` in double precision, which is what
  /// `@stylexjs/babel-plugin@0.19.0` emits for the same input — a rounder
  /// string such as `28.8rem` is the bug, not the baseline.
  #[test]
  fn fractional_rem_breakpoints_derive_the_bounds_babel_derives() {
    assert_eq!(
      transformed_keys(json!({
        "minHeight": {
          "default": "100px",
          "@media (min-width: 25rem)": "200px",
          "@media (min-width: 28.81rem)": "300px",
          "@media (min-width: 32.88rem)": "400px"
        }
      })),
      vec![
        "default",
        "@media (min-width: 25rem) and (max-width: 28.799999999999997rem)",
        "@media (min-width: 28.81rem) and (max-width: 32.870000000000005rem)",
        "@media (min-width: 32.88rem)",
      ]
    );
  }

  /// Every bound in a chain of five fractional breakpoints, so that a passing
  /// assertion cannot be explained by the values that happen to survive single
  /// precision. Two of these five print short and three print long.
  #[test]
  fn every_bound_in_a_long_fractional_chain_matches() {
    assert_eq!(
      transformed_keys(json!({
        "width": {
          "default": "1px",
          "@media (min-width: 1.1rem)": "2px",
          "@media (min-width: 2.2rem)": "3px",
          "@media (min-width: 3.3rem)": "4px",
          "@media (min-width: 4.4rem)": "5px"
        }
      })),
      vec![
        "default",
        "@media (min-width: 1.1rem) and (max-width: 2.1900000000000004rem)",
        "@media (min-width: 2.2rem) and (max-width: 3.29rem)",
        "@media (min-width: 3.3rem) and (max-width: 4.390000000000001rem)",
        "@media (min-width: 4.4rem)",
      ]
    );
  }

  /// A fractional aspect-ratio survives the round trip.
  ///
  /// This is the test that says the fraction is reachable at all. The transform
  /// reprints every `@media` key it is handed, including one it had nothing to
  /// negate -- `combine_media_query_with_negations` returns the query unchanged
  /// and the printer still runs -- so a fraction held at the wrong width did not
  /// stay inside the parser. `16.5/9` reprinted as `16 / 9`, and a ratio of
  /// sixteen to nine is a different shape of screen from one of eleven to six.
  ///
  /// The second key is here for the saturating half of the same bug: past
  /// `i32::MAX` every numerator collapsed onto `2147483647`.
  ///
  /// The negation on the first key is the transform doing its own job -- the
  /// later query wins, so the earlier one is narrowed by its negation. It is
  /// incidental here, and left in rather than filtered out so the assertion
  /// reads against what the transform actually emits.
  #[test]
  fn a_fractional_aspect_ratio_reprints_at_the_width_it_was_written() {
    assert_eq!(
      transformed_keys(json!({
        "width": {
          "default": "1px",
          "@media (aspect-ratio: 16.5/9)": "2px",
          "@media (aspect-ratio: 3000000000/1)": "3px"
        }
      })),
      vec![
        "default",
        "@media (aspect-ratio: 16.5 / 9) and (not (aspect-ratio: 3000000000 / 1))",
        "@media (aspect-ratio: 3000000000 / 1)",
      ]
    );
  }

  /// Round breakpoints print identically at either width. Pinned so that the
  /// widening is shown to move only the values that were wrong.
  #[test]
  fn round_breakpoints_are_undisturbed() {
    assert_eq!(
      transformed_keys(json!({
        "width": {
          "default": "1px",
          "@media (min-width: 1024px)": "2px",
          "@media (min-width: 1440px)": "3px"
        }
      })),
      vec![
        "default",
        "@media (min-width: 1024px) and (max-width: 1439.98px)",
        "@media (min-width: 1440px)",
      ]
    );
  }
}

/// The reported ladder of exclusive breakpoints, at the transform's own seam.
///
/// Regression coverage for
/// https://github.com/Dwlad90/stylex-swc-plugin/issues/1268. Every expectation
/// here is quoted from row `r01` of the ticket 02 divergence table, which
/// recorded what `@stylexjs/babel-plugin@0.19.0` emits for this input.
#[cfg(test)]
mod a_ladder_of_exclusive_breakpoints {
  use super::*;

  /// A ladder whose rungs never touch, ending in a `max-width`-only rung.
  ///
  /// Every distributed branch of the two widest rungs contradicts, and a
  /// contradiction is retained rather than pruned: it prints as `not all`, and
  /// the disjunction nesting built around it survives into the key. The two
  /// narrowest rungs have nothing after them to negate, so they are handed back
  /// as authored.
  ///
  /// The reason this matters at all is the class hash: the key text is what is
  /// hashed, so dropping the wrapper costs two of the seven class names for
  /// this input.
  #[test]
  fn contradictory_branches_are_retained_as_not_all() {
    assert_eq!(
      transformed_keys(json!({
        "color": {
          "default": "black",
          "@media (min-width: 1440px)": "c1",
          "@media (min-width: 1200px) and (max-width: 1439px)": "c2",
          "@media (min-width: 1024px) and (max-width: 1199px)": "c3",
          "@media (min-width: 768px) and (max-width: 1023px)": "c4",
          "@media (min-width: 480px) and (max-width: 767px)": "c5",
          "@media (max-width: 479px)": "c6"
        }
      })),
      vec![
        "default",
        "@media ((not all) or (not all)) or ((not all) or ((min-width: 1440px)))",
        "@media (not all) or ((min-width: 1200px) and (max-width: 1439px))",
        "@media (min-width: 1024px) and (max-width: 1199px)",
        "@media (min-width: 768px) and (max-width: 1023px)",
        "@media (min-width: 480px) and (max-width: 767px)",
        "@media (max-width: 479px)",
      ]
    );
  }
}

/// What happens when two rewritten query keys land on the same text.
#[cfg(test)]
mod colliding_rewritten_keys {
  use super::*;

  /// Two entries that canonicalize to one query text leave one entry.
  ///
  /// The rewritten keys are written into a map, so the second entry to reach
  /// `@media not all` replaces the first one's value and keeps its position.
  /// `red` is gone from the output, `blue` sits where `red` would have, and the
  /// rule count is one lower than the author wrote — all three quoted from
  /// `@stylexjs/babel-plugin@0.19.0`.
  ///
  /// The `min-height` key between the two colliding ones is what makes the
  /// position observable: neighbours would collide into the same slot whichever
  /// of the two positions survived.
  #[test]
  fn a_collision_keeps_the_earlier_position_and_the_later_value() {
    let entries = transformed_entries(json!({
      "color": {
        "default": "black",
        "@media (min-width: 200px)": "red",
        "@media (min-height: 100px)": "green",
        "@media (min-width: 300px)": "blue",
        "@media (min-width: 100px)": "purple"
      }
    }));

    assert_eq!(
      entries,
      vec![
        ("default".to_string(), json!("black")),
        // `red` is gone; `blue` took its key, and its place.
        ("@media not all".to_string(), json!("blue")),
        (
          "@media (max-width: 99.98px) and (min-height: 100px)".to_string(),
          json!("green")
        ),
        ("@media (min-width: 100px)".to_string(), json!("purple")),
      ]
    );
  }
}

/// The bound past which the range merge stops expanding.
#[cfg(test)]
mod a_ladder_too_deep_to_expand {
  use super::*;

  /// The reported ladder shape at `rungs` rungs: exclusive `min-width` /
  /// `max-width` pairs from widest to narrowest, the first `min-width`-only and
  /// the last `max-width`-only. No two rungs touch, so every distributed branch
  /// contradicts and the expansion is as large as a ladder can make it.
  fn ladder(rungs: usize) -> Value {
    // Signed, because a long ladder walks the widths past zero and a negative
    // breakpoint is still a query the merge reads.
    let width = |step: usize| 1000_i64 - step as i64 * 50;

    let mut value = serde_json::Map::new();
    value.insert("default".to_string(), Value::from("black"));

    for i in 0..rungs - 1 {
      let lower = width(i);
      let key = match i {
        0 => format!("@media (min-width: {lower}px)"),
        _ => {
          let upper = width(i - 1) - 1;
          format!("@media (min-width: {lower}px) and (max-width: {upper}px)")
        },
      };
      value.insert(key, Value::from(format!("c{i}")));
    }

    value.insert(
      format!("@media (max-width: {}px)", width(rungs - 2) - 1),
      Value::from(format!("c{}", rungs - 1)),
    );

    json!({ "color": Value::Object(value) })
  }

  /// Past the bound the rules come back as they went in, so the first rung's
  /// key is its authored query followed by one negation per later rung, printed
  /// rather than merged.
  ///
  /// Twenty-one rungs is the shortest ladder that exceeds the bound, and it is
  /// used rather than a longer one because every ladder past the bound still
  /// contains a twenty-rung one among its later rungs, which expands in full.
  /// The three questions worth asking are asked of a single transform for the
  /// same reason.
  ///
  /// The expectation is built from the input rather than written out, because
  /// what is being asserted is that nothing happened to it. Without the bound
  /// the first key would instead be about two megabytes of nested disjunctions.
  #[test]
  fn a_ladder_past_the_bound_comes_back_unmerged() {
    let rungs = 21;
    let input = ladder(rungs);

    let authored: Vec<String> = match &input["color"] {
      Value::Object(map) => map.keys().skip(1).cloned().collect(),
      other => panic!("expected an object, got {other:?}"),
    };

    let negations = authored[1..]
      .iter()
      .map(|key| {
        let query = key.trim_start_matches("@media ");
        // A `not` prints a pair of parentheses around a compound operand; a
        // single condition already carries the only pair it needs.
        match query.contains(" and ") {
          true => format!(" and (not ({query}))"),
          false => format!(" and (not {query})"),
        }
      })
      .collect::<String>();

    let keys = transformed_keys(input);

    // Nothing was dropped, nothing collapsed to a contradiction, and the last
    // rung -- which had nothing after it to negate -- is untouched either way.
    assert_eq!(keys.len(), rungs + 1);
    assert_eq!(keys[1], format!("{}{negations}", authored[0]));
    assert!(!keys[1].contains("not all"));
    assert_eq!(keys[rungs], authored[rungs - 1]);
  }
}

/// Queries the transform refuses, and the ones it must not.
///
/// The refusal is the outer of the two failure modes: it rejects the whole
/// declaration, where the depth bound quietly hands rules back. Every
/// expectation here was compiled through `@stylexjs/babel-plugin` 0.19.0 as
/// well before being written down, so each is a recorded agreement rather than
/// a belief about what should happen.
#[cfg(test)]
mod malformed_queries {
  use super::*;

  /// A closing parenthesis the author never wrote.
  ///
  /// The tokenizer synthesizes one at end of input, so these parse cleanly and
  /// would reach the stylesheet as queries nobody wrote. The balanced-
  /// parenthesis check in front of the parse is what refuses them, and it is
  /// the only reason they are refused — which is why each shape is listed
  /// rather than one standing for the rest.
  #[test]
  fn an_unbalanced_parenthesis_is_refused() {
    assert!(refuses_query("@media (min-width: 100px"));
    assert!(refuses_query("@media ((min-width: 100px)"));
    assert!(refuses_query("@media (width: calc(100px)"));
    assert!(refuses_query("@media min-width: 100px)"));
    assert!(refuses_query("@media (min-width: 100px))"));
  }

  /// An unclosed string swallows the rest of the query, including whatever
  /// would have closed the parenthesis it sits in, so it is unbalanced in its
  /// own right and refused for the same reason as an unclosed parenthesis.
  #[test]
  fn an_unclosed_quote_is_refused() {
    assert!(refuses_query("@media (min-width: \"100px)"));
    assert!(refuses_query("@media (min-width: '100px)"));
  }

  /// A parenthesis that is a character rather than syntax does not count
  /// towards the balance, and must not: counting it would refuse queries the
  /// reference implementation accepts, which is a divergence like any other.
  ///
  /// Its own counter is naive and would call the first of these unbalanced —
  /// but that counter never runs on this path, so what it actually does with
  /// the input is accept it, and that is what is matched here.
  #[test]
  fn a_parenthesis_that_is_not_syntax_does_not_count() {
    // An escaped open parenthesis, which prints as the bare character.
    assert!(!refuses_query("@media (min-width: 100px) and (\\(: 1)"));
    // Inside a closed string the balance check gets out of the way, and the
    // grammar is what refuses — as it does in the reference implementation.
    assert!(refuses_query("@media (min-width: 100px) and (foo: \"(\")"));
  }

  /// Token sequences that are balanced but say nothing the grammar reads.
  #[test]
  fn an_invalid_token_sequence_is_refused() {
    assert!(refuses_query("@media ()"));
    assert!(refuses_query("@media (:)"));
    assert!(refuses_query("@media (min-width:)"));
    assert!(refuses_query(
      "@media (min-width: 100px) and and (max-width: 200px)"
    ));
    assert!(refuses_query("@media (min-width: 100px) and"));
    assert!(refuses_query("@media and (min-width: 100px)"));
    assert!(refuses_query("@media ,"));
    assert!(refuses_query("@media ???"));
    assert!(refuses_query("@media not"));
    assert!(refuses_query("@media only"));
  }

  /// Refusing too much is the other way to diverge. These are accepted by the
  /// reference implementation and must stay accepted here.
  #[test]
  fn an_unusual_but_valid_query_is_not_refused() {
    // A width below zero is a number the merge reads like any other.
    assert!(!refuses_query("@media (min-width: -100px)"));
    // A unitless number is not a length, so the merge declines to read it and
    // the query passes through with its negation printed.
    assert!(!refuses_query("@media (min-width: 100)"));
    // An escaped character in a feature name, and a name outside the basic
    // multilingual plane.
    assert!(!refuses_query("@media (min-\\77 idth: 100px)"));
    assert!(!refuses_query(
      "@media (min-width: 100px) and (\u{1D400}: 1)"
    ));
  }

  /// A custom property is not a length, and a media feature has to resolve at
  /// media-evaluation time rather than at cascade time — so both compilers
  /// refuse this rather than emitting a query no browser could match.
  #[test]
  fn a_custom_property_in_a_value_position_is_refused() {
    assert!(refuses_query("@media (min-width: var(--breakpoint))"));
  }

  /// A key that is `@media` and nothing else, and one with a space the author
  /// did not mean to leave. Both are refused: the prefix check treats them as
  /// media keys, and neither parses to a query.
  #[test]
  fn a_key_that_is_only_the_at_rule_is_refused() {
    assert!(refuses_query("@media "));
    assert!(refuses_query("@media (min-width: 100px) "));
  }

  /// Nesting is walked once per level rather than searched, so a depth that
  /// makes the reference implementation backtrack for minutes is answered here
  /// in milliseconds — twelve levels take it twenty seconds and sixteen do not
  /// finish in thirty.
  ///
  /// Sixty-four levels is the budget, so this is the deepest query that still
  /// compiles, and what it compiles to is the query with the wrapping gone.
  #[test]
  fn nesting_up_to_the_budget_is_read_without_backtracking() {
    let deep = format!(
      "@media {}min-width: 100px{}",
      "(".repeat(64),
      ")".repeat(64)
    );

    assert_eq!(
      transformed_keys(json!({ "color": { "default": "black", deep: "red" } })),
      vec!["default", "@media (min-width: 100px)"]
    );
  }

  /// One level further is refused rather than compiled.
  ///
  /// Parsing recurses once per level and a stack overflow aborts the process
  /// instead of panicking, so nothing downstream could turn it into a
  /// diagnostic — which is why the depth is measured before the parse rather
  /// than caught during it. The number is far above any query an author writes
  /// and far below where the stack actually gives out: two thousand levels take
  /// 10 ms here, five thousand abort.
  #[test]
  fn nesting_past_the_budget_is_refused_rather_than_fatal() {
    let past = format!(
      "@media {}min-width: 100px{}",
      "(".repeat(65),
      ")".repeat(65)
    );
    assert!(refuses_query(&past));

    let far_past = format!(
      "@media {}min-width: 100px{}",
      "(".repeat(5000),
      ")".repeat(5000)
    );
    assert!(refuses_query(&far_past));
  }

  /// A chain of `not` keywords is refused, and it is refused by the budget
  /// rather than by luck.
  ///
  /// This is the nesting that costs no parenthesis: the operand of a bare `not`
  /// is a whole rule, so `not not (…)` recurses once per keyword while a scan
  /// for parentheses sees depth one. Before the parser charged its own frames,
  /// twenty thousand of these took the process down with a segfault and no
  /// diagnostic -- a stack overflow is not unwindable, so neither
  /// `catch_unwind` around compilation ever saw it.
  #[test]
  fn a_chain_of_not_keywords_is_refused_by_the_budget() {
    let past = format!("@media {}(min-width: 1px)", "not ".repeat(64));
    assert!(refuses_query(&past));

    let far_past = format!("@media {}(min-width: 1px)", "not ".repeat(20_000));
    assert!(refuses_query(&far_past));
  }

  /// The escaped spelling costs the same frame as the plain one.
  ///
  /// `n\6ft` is decoded by the tokenizer to `Ident("not")` and recurses
  /// identically, so a guard that read the raw text would have to
  /// over-approximate every escape to catch it. Charging the frame where it is
  /// entered needs to know nothing about how the keyword was spelled.
  #[test]
  fn an_escaped_not_chain_is_refused_like_a_plain_one() {
    let past = format!("@media {}(min-width: 1px)", "n\\6ft ".repeat(64));

    assert!(refuses_query(&past));
  }

  /// A *wide* run of negations is not a deep one, and stays accepted.
  ///
  /// `(not (a)) and (not (b)) and …` is parsed by a loop rather than by
  /// recursion, so each negation costs its own frame and releases it before the
  /// next is read -- the run is one level deep however long it gets. This is
  /// the case a guard that counted `not` keywords in the raw text would have
  /// refused, having no way to tell a run from a chain.
  ///
  /// The negations are parenthesized because a bare `not` combined with `and`
  /// is refused for its own reasons, matching the reference implementation.
  #[test]
  fn a_wide_run_of_negations_is_not_a_deep_one() {
    let clauses = (0..80)
      .map(|index| format!("(not (min-width: {}px))", index + 1))
      .collect::<Vec<_>>()
      .join(" and ");

    assert!(!refuses_query(&format!("@media {clauses}")));
  }
}

/// Queries that look wrong and are not.
///
/// Every expectation is a row of the same comparison the refusals above came
/// from: the input compiled through `@stylexjs/babel-plugin` 0.19.0 and through
/// this compiler, with the emitted `@media` preludes read back. All fifteen
/// agreed, and the point of pinning them here is that they go on agreeing.
#[cfg(test)]
mod unusual_but_valid_queries {
  use super::*;

  /// A vendor-prefixed feature is not one the range merge reads, so it blocks
  /// the interval merge and the negation prints beside it rather than folding
  /// into a bound.
  #[test]
  fn a_vendor_prefixed_feature_blocks_the_merge_rather_than_the_query() {
    assert_eq!(
      transformed_keys(json!({
        "color": {
          "default": "black",
          "@media (-webkit-min-device-pixel-ratio: 2)": "red",
          "@media (max-width: 50px)": "blue"
        }
      })),
      vec![
        "default",
        "@media (-webkit-min-device-pixel-ratio: 2) and (not (max-width: 50px))",
        "@media (max-width: 50px)",
      ]
    );

    assert_eq!(
      transformed_keys(json!({
        "color": {
          "default": "black",
          "@media (-moz-device-pixel-ratio: 2)": "red",
          "@media (max-width: 50px)": "blue"
        }
      })),
      vec![
        "default",
        "@media (-moz-device-pixel-ratio: 2) and (not (max-width: 50px))",
        "@media (max-width: 50px)",
      ]
    );
  }

  /// The same prefix beside a width the merge *can* read. The widths are still
  /// left alone, because one unreadable rule in the list stops the whole merge
  /// rather than only its own dimension.
  #[test]
  fn a_prefixed_feature_beside_a_width_stops_the_width_merging_too() {
    assert_eq!(
      transformed_keys(json!({
        "color": {
          "default": "black",
          "@media (-webkit-min-device-pixel-ratio: 2) and (min-width: 200px)": "red",
          "@media (min-width: 100px)": "blue"
        }
      })),
      vec![
        "default",
        "@media (-webkit-min-device-pixel-ratio: 2) and (min-width: 200px) and (not (min-width: 100px))",
        "@media (min-width: 100px)",
      ]
    );
  }

  /// Characters outside the basic multilingual plane, letters carrying
  /// combining accents, and a CSS escape that resolves to a character the
  /// tokenizer would otherwise treat as syntax. Each survives the round trip
  /// through the parser and the printer as the author wrote it.
  #[test]
  fn unicode_and_escapes_reach_the_stylesheet_unharmed() {
    assert_eq!(
      transformed_keys(json!({
        "color": {
          "default": "black",
          "@media (\u{1F600}: 1)": "red",
          "@media (max-width: 50px)": "blue"
        }
      }))[1],
      "@media (\u{1F600}: 1) and (not (max-width: 50px))"
    );

    assert_eq!(
      transformed_keys(json!({
        "color": {
          "default": "black",
          "@media (mín-width: 100px)": "red",
          "@media (max-width: 50px)": "blue"
        }
      }))[1],
      "@media (mín-width: 100px) and (not (max-width: 50px))"
    );

    // `\@foo` is an escaped at-sign, which prints as the bare character.
    assert_eq!(
      transformed_keys(json!({
        "color": {
          "default": "black",
          "@media (min-width: 100px) and (\\@foo: 1)": "red",
          "@media (max-width: 50px)": "blue"
        }
      }))[1],
      "@media (min-width: 100px) and (@foo: 1) and (not (max-width: 50px))"
    );
  }

  /// A comma-separated query is a disjunction, so each disjunct takes the
  /// negation separately — and one of the two collapses here while the other
  /// merges into a bound.
  #[test]
  fn each_disjunct_of_a_comma_query_is_negated_on_its_own() {
    assert_eq!(
      transformed_keys(json!({
        "color": {
          "default": "black",
          "@media (min-width: 200px), (max-width: 100px)": "red",
          "@media (min-width: 100px)": "blue"
        }
      })),
      vec![
        "default",
        "@media not all, (max-width: 99.98px)",
        "@media (min-width: 100px)",
      ]
    );
  }

  /// A media type in the list is parenthesized on the way out and, like any
  /// rule the merge cannot read, keeps the widths beside it from merging.
  #[test]
  fn a_media_type_is_parenthesized_and_blocks_the_merge() {
    assert_eq!(
      transformed_keys(json!({
        "color": {
          "default": "black",
          "@media screen and (min-width: 200px)": "red",
          "@media (min-width: 100px)": "blue"
        }
      }))[1],
      "@media (screen) and (min-width: 200px) and (not (min-width: 100px))"
    );
  }

  /// Lengths at the ends of what a double can hold. The larger one is finite
  /// and survives as an exponent; the smaller is far enough below the nudged
  /// bound beside it that the intersection keeps the nudge.
  #[test]
  fn lengths_at_the_edge_of_double_precision_merge_like_any_other() {
    assert_eq!(
      transformed_keys(json!({
        "color": {
          "default": "black",
          "@media (min-width: 1e308px)": "red",
          "@media (max-width: 50px)": "blue"
        }
      }))[1],
      "@media (min-width: 1e+308px)"
    );

    assert_eq!(
      transformed_keys(json!({
        "color": {
          "default": "black",
          "@media (min-width: 0.0000000001px)": "red",
          "@media (max-width: 50px)": "blue"
        }
      }))[1],
      "@media (min-width: 50.01px)"
    );
  }

  /// Two `min-width` breakpoints closer together than the negation step leave
  /// an empty interval, so the narrower declaration collapses to `not all` and
  /// is dropped with nothing said.
  ///
  /// A later key wins, so an earlier `min-width: 100px` is rewritten as itself
  /// and not the later one. The step for a negated `min-width` in pixels is
  /// `0.02`, so `not (min-width: 100.01px)` is `max-width: 99.99px` — below the
  /// `100px` the earlier rule starts at. The band the author wrote, from
  /// `100px` up to `100.01px`, disappears. Three hundredths apart and the band
  /// survives.
  ///
  /// The reference compiler does the same, so this is the boundary rather than
  /// a defect — but nothing stated it, and the number that decides it is the
  /// one a device pixel ratio forced up from `0.01`.
  #[test]
  fn breakpoints_closer_than_the_negation_step_collapse() {
    assert_eq!(
      transformed_keys(json!({
        "color": {
          "default": "black",
          "@media (min-width: 100px)": "blue",
          "@media (min-width: 100.01px)": "red"
        }
      })),
      vec!["default", "@media not all", "@media (min-width: 100.01px)"]
    );

    // Three hundredths apart, and the band is no longer empty.
    assert_eq!(
      transformed_keys(json!({
        "color": {
          "default": "black",
          "@media (min-width: 100px)": "blue",
          "@media (min-width: 100.03px)": "red"
        }
      })),
      vec![
        "default",
        "@media (min-width: 100px) and (max-width: 100.01px)",
        "@media (min-width: 100.03px)",
      ]
    );

    // The wider step is a `width` rule in pixels only, so the same pair on
    // `height` keeps its band at one hundredth.
    assert_eq!(
      transformed_keys(json!({
        "color": {
          "default": "black",
          "@media (min-height: 100px)": "blue",
          "@media (min-height: 100.01px)": "red"
        }
      })),
      vec![
        "default",
        "@media (min-height: 100px) and (max-height: 100px)",
        "@media (min-height: 100.01px)",
      ]
    );
  }

  /// A conditional value map holding only `default` has no media key to
  /// rewrite, so the transform hands it back untouched rather than treating the
  /// absence as an empty rewrite.
  #[test]
  fn a_map_with_no_media_key_is_untouched() {
    assert_eq!(
      transformed_keys(json!({ "color": { "default": "black" } })),
      vec!["default"]
    );
  }

  /// A map with no `default` is still rewritten. The first key collapses to a
  /// contradiction, which is the ordinary outcome rather than a consequence of
  /// the missing default.
  #[test]
  fn a_map_with_no_default_is_rewritten_the_same_way() {
    assert_eq!(
      transformed_keys(json!({
        "color": {
          "@media (min-width: 200px)": "red",
          "@media (min-width: 100px)": "blue"
        }
      })),
      vec!["@media not all", "@media (min-width: 100px)"]
    );
  }
}

/// How a comma and an `or` divide a query between them.
///
/// Both mean disjunction and both end up in the same `Or` node, but `or` groups
/// inside one comma segment and the segments group above it. The transform
/// distributes its negations over whatever the top-level `Or` holds, so getting
/// that nesting wrong changes the emitted rule text and the class name with it —
/// which is what these three shapes did before the two were told apart.
///
/// Every expectation is quoted from a run of `@stylexjs/babel-plugin` 0.19.0.
#[cfg(test)]
mod a_comma_binds_more_loosely_than_or {
  use super::*;

  /// Two segments, the second holding an `or`. Flattening them into three
  /// disjuncts spread the negation over three, and printed three commas.
  #[test]
  fn a_segment_holding_an_or_stays_one_disjunct() {
    assert_eq!(
      rewritten_first_key(
        "@media (min-width: 1px) and (min-width: 2px), (min-width: 3px) or (min-width: 1px)"
      ),
      "@media (min-width: 50.01px), (min-width: 3px) or (min-width: 1px) and (not (max-width: 50px))"
    );
  }

  /// The same with the `or` in the first segment, so a reader can see the
  /// grouping move rather than the text.
  #[test]
  fn the_or_segment_may_come_first() {
    assert_eq!(
      rewritten_first_key(
        "@media (min-width: 1px) or (min-width: 2px), (min-width: 3px) and (min-width: 1px)"
      ),
      "@media (min-width: 1px) or (min-width: 2px) and (not (max-width: 50px)), (min-width: 50.01px)"
    );
  }

  /// A media type as its own segment, beside one holding an `or`. The media
  /// type blocks the merge on its side, which leaves both segments visible in
  /// the output at once.
  #[test]
  fn a_media_type_segment_sits_beside_an_or_segment() {
    assert_eq!(
      rewritten_first_key("@media screen, (min-width: 1px) or (min-width: 2px)"),
      "@media (screen) and (not (max-width: 50px)), \
       (min-width: 1px) or (min-width: 2px) and (not (max-width: 50px))"
    );
  }
}

/// A disjunction inside parentheses.
///
/// `( <media-condition> )` is a media query in its own right, and a condition
/// may hold an `or`, so `((a) or (b))` is valid CSS. Reading only `and` inside
/// parentheses refused all four of these — including the shape the reference
/// implementation's own wrapped output is written in.
///
/// Every expectation is quoted from a run of `@stylexjs/babel-plugin` 0.19.0,
/// and each is worth reading twice: the parentheses that made the query
/// parseable do not survive into the output. That is the reference
/// implementation's serialization, precedence loss and all, and matching it is
/// the point.
#[cfg(test)]
mod a_disjunction_inside_parentheses {
  use super::*;

  /// On its own, the wrapper is dropped and the disjunction becomes the query.
  #[test]
  fn a_parenthesized_or_is_a_query_of_its_own() {
    assert_eq!(
      rewritten_first_key("@media ((min-width: 1px) or (min-width: 2px))"),
      "@media (min-width: 50.01px), (min-width: 50.01px)"
    );
  }

  /// Beside an `and`, on either side. The printed result reads as though the
  /// `and` bound tighter than the `or`, which is not what was written — the
  /// parentheses are gone and nothing replaced them.
  #[test]
  fn a_parenthesized_or_beside_an_and_loses_its_parentheses() {
    assert_eq!(
      rewritten_first_key("@media ((min-width: 1px) or (min-width: 2px)) and (min-width: 3px)"),
      "@media (min-width: 1px) or (min-width: 2px) and (min-width: 3px) and (not (max-width: 50px))"
    );

    assert_eq!(
      rewritten_first_key("@media (min-width: 1px) and ((min-width: 2px) or (min-width: 3px))"),
      "@media (min-width: 1px) and (min-width: 2px) or (min-width: 3px) and (not (max-width: 50px))"
    );
  }

  /// Under a negation the parentheses do survive, because `not` prints its
  /// operand wrapped.
  #[test]
  fn a_negated_parenthesized_or_keeps_its_parentheses() {
    assert_eq!(
      rewritten_first_key("@media not ((min-width: 1px) or (min-width: 2px))"),
      "@media (not ((min-width: 1px) or (min-width: 2px))) and (not (max-width: 50px))"
    );
  }
}

/// The combinator spellings CSS does not define, and the ones it does.
///
/// `<media-condition> = <media-not> | <media-in-parens> [ <media-and>* |
/// <media-or>* ]` — one condition takes `and`s or `or`s, never both, and a bare
/// `not` is the whole condition rather than an operand in one. Accepting either
/// spelling meant inventing a precedence the language does not define and
/// emitting a query that means something the author did not write.
///
/// Every verdict here was compared against `@stylexjs/babel-plugin` 0.19.0,
/// which refuses each of the refused ones.
#[cfg(test)]
mod combinators_css_does_not_define {
  use super::*;

  /// Mixing the two combinators at one level, in either order.
  #[test]
  fn an_unparenthesized_mix_of_and_and_or_is_refused() {
    assert!(refuses_query(
      "@media (min-width: 1px) and (min-width: 2px) or (min-width: 3px)"
    ));
    assert!(refuses_query(
      "@media (min-width: 1px) or (min-width: 2px) and (min-width: 3px)"
    ));
  }

  /// A bare `not` beside anything, on either side of it — inside a condition,
  /// which is everywhere except the one position below.
  #[test]
  fn a_bare_negation_cannot_be_combined() {
    assert!(refuses_query(
      "@media not (min-width: 1px) and (min-width: 2px)"
    ));
    assert!(refuses_query(
      "@media not (min-width: 1px) or (min-width: 2px)"
    ));
    assert!(refuses_query(
      "@media (min-width: 2px) and not (min-width: 1px)"
    ));
    assert!(refuses_query(
      "@media (min-width: 2px) or not (min-width: 1px)"
    ));
    assert!(refuses_query(
      "@media not (min-width: 1px) or not (min-width: 2px)"
    ));
  }

  /// Parentheses are what make the same operands legal, and they are the whole
  /// difference — so each of these is the refused spelling with brackets added.
  #[test]
  fn parentheses_make_the_same_operands_legal() {
    assert!(!refuses_query(
      "@media ((min-width: 1px) and (min-width: 2px)) or (min-width: 3px)"
    ));
    assert!(!refuses_query(
      "@media (min-width: 1px) or ((min-width: 2px) and (min-width: 3px))"
    ));
    assert!(!refuses_query(
      "@media (not (min-width: 1px)) and (min-width: 2px)"
    ));
    assert!(!refuses_query(
      "@media (not (min-width: 1px)) or (not (min-width: 2px))"
    ));
  }

  /// Straight after a media type's `and` is the one place a bare `not` is a
  /// query: `<media-query> = [not | only]? <media-type>
  /// [ and <media-condition-without-or> ]?`, and a
  /// `<media-condition-without-or>` may be a `<media-not>`.
  ///
  /// Only immediately, though. One more operand and the tail is a condition
  /// again, where an operand has to be parenthesized.
  ///
  /// This is the second place the two compilers disagree by choice: the
  /// official compiler refuses all of these, and refusing valid CSS to match it
  /// would cost an author a query they are entitled to write.
  #[test]
  fn a_media_type_takes_one_bare_negation_after_its_and() {
    assert!(!refuses_query(
      "@media screen and not (orientation: portrait)"
    ));
    assert!(!refuses_query(
      "@media not screen and not (orientation: portrait)"
    ));

    assert!(refuses_query(
      "@media screen and (orientation: portrait) and not (monochrome)"
    ));
  }

  /// A leading `not` before a media type is a different construct — a media
  /// query rather than a condition — and it combines with `and` as it always
  /// has. The refusal above must not reach it.
  #[test]
  fn a_negated_media_type_still_takes_an_and() {
    // Every media type, not only `screen`: they reach the same leading-`not`
    // peek, and it is the rule that comes back -- a keyword rather than a
    // negation -- that tells them apart from `not (min-width: 1px)`.
    for media_type in ["screen", "print", "all"] {
      assert!(!refuses_query(&format!("@media not {media_type}")));
      assert!(!refuses_query(&format!(
        "@media not {media_type} and (min-width: 1px)"
      )));
      assert!(!refuses_query(&format!(
        "@media only {media_type} and (min-width: 1px)"
      )));
    }
  }

  /// Nesting parentheses around a single condition stays accepted, and this is
  /// the one shape where the two compilers still disagree — deliberately.
  /// `( <media-condition> )` is what CSS says a condition in parentheses is,
  /// and a condition may itself be one, so `((min-width: 1px))` is valid.
  /// Refusing it to match would mean rejecting correct CSS.
  #[test]
  fn nested_parentheses_around_one_condition_stay_accepted() {
    assert_eq!(
      transformed_keys(json!({
        "color": { "default": "black", "@media ((min-width: 1px))": "red" }
      })),
      vec!["default", "@media (min-width: 1px)"]
    );

    assert_eq!(
      transformed_keys(json!({
        "color": { "default": "black", "@media (((((min-width: 1px)))))": "red" }
      })),
      vec!["default", "@media (min-width: 1px)"]
    );
  }
}
