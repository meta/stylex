use crate::at_queries::media_query_transform::last_media_query_wins_transform;
use serde_json::{Value, json};
use swc_core::{
  atoms::Wtf8Atom,
  common::DUMMY_SP,
  ecma::ast::{Expr, KeyValueProp, ObjectLit, Prop, PropName, PropOrSpread, Str},
};

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

fn convert_atom_to_string(atom: &Wtf8Atom) -> String {
  atom
    .as_str()
    .expect("Failed to convert Wtf8Atom to &str")
    .to_string()
}

/// Convert KeyValueProps back to JSON.
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
        "@media screen and (not (max-width: 500px)), (min-width: 500.01px) and (max-width: 800px)": "80%",
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
        "@media screen and (min-width: 900px), screen and (min-width: 900px) and (not (max-width: 500px))": "80%",
        "@media print and (max-width: 500px)": "50%"
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

/// Every `@media` key is re-added at the end of the object, even a lone one, and that order
/// reaches the emitted CSS.
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

// Computed bounds at double precision: https://github.com/Dwlad90/stylex-swc-plugin/issues/1267

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
fn transformed_styles(styles: Value) -> Value {
  key_value_prop_to_json(&last_media_query_wins_transform(&props_of(styles)))
}

/// The single property's entries in order, as pairs, since order is part of what is asserted.
fn transformed_entries(styles: Value) -> Vec<(String, Value)> {
  match transformed_styles(styles) {
    Value::Object(obj) => match obj.into_iter().next() {
      Some((_, Value::Object(inner))) => inner.into_iter().collect(),
      other => panic!("expected one property holding an object, got {other:?}"),
    },
    other => panic!("expected an object, got {other:?}"),
  }
}

/// The rewritten text of one `@media` key, with a later key after it so the rewrite runs.
fn rewritten_first_key(query: &str) -> String {
  transformed_keys(json!({
    "color": { "default": "black", query: "red", "@media (max-width: 50px)": "blue" }
  }))[1]
    .clone()
}

/// Whether the transform refuses `query`, judged by the refusal's panic message.
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

/// The keys of the single property in the transformed `styles`, in order.
fn transformed_keys(styles: Value) -> Vec<String> {
  transformed_entries(styles)
    .into_iter()
    .map(|(key, _)| key)
    .collect()
}

#[cfg(test)]
mod computed_bounds_carry_the_authored_digits {
  use super::*;

  /// Issue #1267: each upper bound is `next - 0.01` in double precision, as babel-plugin 0.19.0
  /// emits; a rounder `28.8rem` is the bug.
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

  /// Five fractional breakpoints, so passing cannot be explained by values surviving `f32`.
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

  /// A fractional aspect-ratio survives reprinting (`16.5/9` once became `16 / 9`), and numerators
  /// past `i32::MAX` no longer saturate.
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

  /// Round breakpoints print identically at either width.
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

/// Regression coverage for https://github.com/Dwlad90/stylex-swc-plugin/issues/1268;
/// expectations quoted from `@stylexjs/babel-plugin@0.19.0`.
#[cfg(test)]
mod a_ladder_of_exclusive_breakpoints {
  use super::*;

  /// Contradicting branches are kept as `not all` rather than pruned, because the key text is
  /// hashed into the class name.
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

  /// Two entries canonicalizing to one query leave one: the later value takes the earlier
  /// position, as babel-plugin 0.19.0 does.
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

  /// `rungs` non-touching exclusive width rungs, widest first, so every distributed branch
  /// contradicts.
  fn ladder(rungs: usize) -> Value {
    // Signed: a long ladder walks widths past zero, which is still a query the merge reads.
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

  /// Past the bound the rules come back as authored: the first key is its query plus one
  /// negation per later rung. 21 rungs is the shortest ladder past the bound.
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
        // `not` parenthesizes a compound operand; a single condition already has its pair.
        match query.contains(" and ") {
          true => format!(" and (not ({query}))"),
          false => format!(" and (not {query})"),
        }
      })
      .collect::<String>();

    let keys = transformed_keys(input);

    // Nothing dropped or collapsed, and the last rung is untouched.
    assert_eq!(keys.len(), rungs + 1);
    assert_eq!(keys[1], format!("{}{negations}", authored[0]));
    assert!(!keys[1].contains("not all"));
    assert_eq!(keys[rungs], authored[rungs - 1]);
  }
}

/// Queries the transform refuses and ones it must accept; checked against babel-plugin 0.19.0.
#[cfg(test)]
mod malformed_queries {
  use super::*;

  /// The tokenizer synthesizes a missing `)` at end of input; only the balance check refuses these.
  #[test]
  fn an_unbalanced_parenthesis_is_refused() {
    assert!(refuses_query("@media (min-width: 100px"));
    assert!(refuses_query("@media ((min-width: 100px)"));
    assert!(refuses_query("@media (width: calc(100px)"));
    assert!(refuses_query("@media min-width: 100px)"));
    assert!(refuses_query("@media (min-width: 100px))"));
  }

  /// An unclosed string swallows the closing parenthesis, so it is unbalanced.
  #[test]
  fn an_unclosed_quote_is_refused() {
    assert!(refuses_query("@media (min-width: \"100px)"));
    assert!(refuses_query("@media (min-width: '100px)"));
  }

  /// A parenthesis that is a character, not syntax, does not count toward the balance, matching
  /// upstream.
  #[test]
  fn a_parenthesis_that_is_not_syntax_does_not_count() {
    // An escaped open parenthesis, which prints as the bare character.
    assert!(!refuses_query("@media (min-width: 100px) and (\\(: 1)"));
    // Inside a closed string the grammar refuses, as upstream does.
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

  /// Accepted upstream, so they must stay accepted.
  #[test]
  fn an_unusual_but_valid_query_is_not_refused() {
    // A width below zero is a number the merge reads like any other.
    assert!(!refuses_query("@media (min-width: -100px)"));
    // A unitless number is not a length, so the query passes through with its negation printed.
    assert!(!refuses_query("@media (min-width: 100)"));
    // An escaped feature-name character, and a name outside the basic multilingual plane.
    assert!(!refuses_query("@media (min-\\77 idth: 100px)"));
    assert!(!refuses_query(
      "@media (min-width: 100px) and (\u{1D400}: 1)"
    ));
  }

  /// A custom property cannot resolve at media-evaluation time, so both compilers refuse it.
  #[test]
  fn a_custom_property_in_a_value_position_is_refused() {
    assert!(refuses_query("@media (min-width: var(--breakpoint))"));
  }

  /// A bare `@media` key, and one with a stray space, are media keys that parse to no query.
  #[test]
  fn a_key_that_is_only_the_at_rule_is_refused() {
    assert!(refuses_query("@media "));
    assert!(refuses_query("@media (min-width: 100px) "));
  }

  /// Sixty-four levels, the budget, still compiles (quickly, where upstream backtracks for
  /// minutes) and drops the wrapping.
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

  /// One level past the budget is refused before parsing: a stack overflow cannot become a
  /// diagnostic.
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

  /// A chain of bare `not`s recurses per keyword without parentheses; the parser's frame budget
  /// refuses it.
  #[test]
  fn a_chain_of_not_keywords_is_refused_by_the_budget() {
    let past = format!("@media {}(min-width: 1px)", "not ".repeat(64));
    assert!(refuses_query(&past));

    let far_past = format!("@media {}(min-width: 1px)", "not ".repeat(20_000));
    assert!(refuses_query(&far_past));
  }

  /// The escaped `n\6ft` decodes to `not` and costs the same frame.
  #[test]
  fn an_escaped_not_chain_is_refused_like_a_plain_one() {
    let past = format!("@media {}(min-width: 1px)", "n\\6ft ".repeat(64));

    assert!(refuses_query(&past));
  }

  /// A wide run of parenthesized negations is parsed by a loop, one level deep, so it stays
  /// accepted.
  #[test]
  fn a_wide_run_of_negations_is_not_a_deep_one() {
    let clauses = (0..80)
      .map(|index| format!("(not (min-width: {}px))", index + 1))
      .collect::<Vec<_>>()
      .join(" and ");

    assert!(!refuses_query(&format!("@media {clauses}")));
  }
}

/// Queries that look wrong and are not; all agreed with babel-plugin 0.19.0.
#[cfg(test)]
mod unusual_but_valid_queries {
  use super::*;

  /// A vendor-prefixed feature blocks the interval merge, so the negation prints beside it.
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

  /// One unreadable rule stops the whole merge, so readable widths beside it stay unmerged.
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

  /// Non-BMP characters, combining accents and a CSS escape round-trip as authored.
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

  /// Each comma disjunct takes the negation separately: one collapses, the other merges.
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
        "@media (max-width: 99.98px)",
        "@media (min-width: 100px)",
      ]
    );
  }

  /// A media type is parenthesized on output and blocks the widths beside it from merging.
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
      "@media screen and (min-width: 200px) and (not (min-width: 100px))"
    );
  }

  /// Lengths at the ends of the double range: the larger prints as an exponent, the smaller
  /// keeps the nudge.
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

  /// Two `min-width` breakpoints closer than the 0.02px negation step leave an empty band, which
  /// is dropped silently, as upstream does.
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

    // The wider step applies to pixel `width` only, so `height` keeps its band at 0.01.
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

  /// A value map with only `default` has no media key, so it is returned untouched.
  #[test]
  fn a_map_with_no_media_key_is_untouched() {
    assert_eq!(
      transformed_keys(json!({ "color": { "default": "black" } })),
      vec!["default"]
    );
  }

  /// A map without `default` is still rewritten.
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

/// `or` groups inside one comma segment and segments group above it; an `or` under an `and`
/// keeps its parentheses (upstream #2001).
#[cfg(test)]
mod a_comma_binds_more_loosely_than_or {
  use super::*;

  /// The `or` in the second segment; flattening once spread the negation over three disjuncts.
  #[test]
  fn a_segment_holding_an_or_stays_one_disjunct() {
    assert_eq!(
      rewritten_first_key(
        "@media (min-width: 1px) and (min-width: 2px), (min-width: 3px) or (min-width: 1px)"
      ),
      "@media (min-width: 50.01px), ((min-width: 3px) or (min-width: 1px)) and (not (max-width: 50px))"
    );
  }

  /// The same with the `or` in the first segment.
  #[test]
  fn the_or_segment_may_come_first() {
    assert_eq!(
      rewritten_first_key(
        "@media (min-width: 1px) or (min-width: 2px), (min-width: 3px) and (min-width: 1px)"
      ),
      "@media ((min-width: 1px) or (min-width: 2px)) and (not (max-width: 50px)), (min-width: 50.01px)"
    );
  }

  /// A media-type segment beside one holding an `or`; the type blocks the merge on its side.
  #[test]
  fn a_media_type_segment_sits_beside_an_or_segment() {
    assert_eq!(
      rewritten_first_key("@media screen, (min-width: 1px) or (min-width: 2px)"),
      "@media screen and (not (max-width: 50px)), (min-width: 50.01px), (min-width: 50.01px)"
    );
  }
}

/// `((a) or (b))` is valid CSS. Alone, upstream drops the wrapping parentheses on output;
/// beside an `and` it keeps them (#2001), and so do we.
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

  /// Beside an `and`, on either side, the parentheses survive.
  #[test]
  fn a_parenthesized_or_beside_an_and_keeps_its_parentheses() {
    assert_eq!(
      rewritten_first_key("@media ((min-width: 1px) or (min-width: 2px)) and (min-width: 3px)"),
      "@media ((min-width: 1px) or (min-width: 2px)) and (min-width: 3px) and (not (max-width: 50px))"
    );

    assert_eq!(
      rewritten_first_key("@media (min-width: 1px) and ((min-width: 2px) or (min-width: 3px))"),
      "@media (min-width: 1px) and ((min-width: 2px) or (min-width: 3px)) and (not (max-width: 50px))"
    );
  }

  /// Under a negation the parentheses survive, because `not` wraps its operand.
  #[test]
  fn a_negated_parenthesized_or_keeps_its_parentheses() {
    assert_eq!(
      rewritten_first_key("@media not ((min-width: 1px) or (min-width: 2px))"),
      "@media (not ((min-width: 1px) or (min-width: 2px))) and (not (max-width: 50px))"
    );
  }
}

/// One condition takes `and`s or `or`s, never both, and a bare `not` is a whole condition;
/// babel-plugin 0.19.0 refuses each refused spelling.
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

  /// A bare `not` beside anything inside a condition, on either side.
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

  /// The refused spellings with parentheses added, which makes them legal.
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

  /// Straight after a media type's `and`, a bare `not` is valid CSS: upstream refuses it, we
  /// accept it deliberately. One more operand and parentheses are required again.
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

  /// A leading `not` before a media type is a media query and combines with `and` as before.
  #[test]
  fn a_negated_media_type_still_takes_an_and() {
    // Every media type: the parsed rule (keyword, not negation) is what tells them apart.
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

  /// `((min-width: 1px))` is valid CSS and stays accepted, deliberately unlike upstream.
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

/// A leading `not` negates the whole query (upstream #2001, media-query-transform-test.js).
/// Upstream's snapshots list keys alphabetically, so these compare keys and values, not order.
#[cfg(test)]
mod a_leading_not {
  use super::*;

  #[test]
  fn a_leading_not_negates_the_whole_query() {
    assert_eq!(
      transformed_styles(json!({
        "filter": { "default": "grayscale(1)", "@media not all and (monochrome)": "none" },
        "color": { "default": "black", "@media not screen and (min-width: 100px)": "red" }
      })),
      json!({
        "filter": { "default": "grayscale(1)", "@media not all and (monochrome)": "none" },
        "color": { "default": "black", "@media not screen and (min-width: 100px)": "red" }
      })
    );
  }

  #[test]
  fn a_leading_not_followed_by_a_later_query() {
    assert_eq!(
      transformed_styles(json!({
        "color": {
          "default": "black",
          "@media not all and (monochrome)": "blue",
          "@media (min-width: 500px)": "red"
        }
      })),
      json!({
        "color": {
          "default": "black",
          "@media (not (min-width: 500px)) and (not (monochrome))": "blue",
          "@media (min-width: 500px)": "red"
        }
      })
    );
  }

  #[test]
  fn a_later_query_with_a_leading_not() {
    assert_eq!(
      transformed_styles(json!({
        "color": {
          "default": "black",
          "@media (min-width: 500px)": "red",
          "@media not all and (monochrome)": "blue"
        },
        "width": {
          "default": "100%",
          "@media (max-width: 900px)": "80%",
          "@media not print and (min-width: 600px)": "60%"
        }
      })),
      json!({
        "color": {
          "default": "black",
          "@media (min-width: 500px) and (monochrome)": "red",
          "@media not all and (monochrome)": "blue"
        },
        "width": {
          "default": "100%",
          "@media print and (max-width: 900px) and (min-width: 600px)": "80%",
          "@media not print and (min-width: 600px)": "60%"
        }
      })
    );
  }

  #[test]
  fn a_leading_not_over_an_or_condition_stays_linear_in_size() {
    let query = "@media not screen and ((min-width: 100px) or (min-width: 200px) or (min-width: 300px) or (min-width: 400px))";
    assert_eq!(
      transformed_styles(json!({
        "color": { "default": "black", "@media (min-width: 10px)": "red", query: "blue" }
      })),
      json!({
        "color": {
          "default": "black",
          "@media screen and (min-width: 100px) and (min-width: 10px), screen and (min-width: 200px) and (min-width: 10px), screen and (min-width: 300px) and (min-width: 10px), screen and (min-width: 400px) and (min-width: 10px)": "red",
          query: "blue"
        }
      })
    );
  }

  #[test]
  fn a_leading_not_over_mixed_and_or_conditions() {
    let query =
      "@media not all and (min-width: 500px) and ((max-width: 900px) or (max-height: 600px))";
    assert_eq!(
      transformed_styles(json!({
        "color": { "default": "black", query: "blue" },
        "width": { "default": "100%", "@media (min-width: 10px)": "80%", query: "60%" }
      })),
      json!({
        "color": { "default": "black", query: "blue" },
        "width": {
          "default": "100%",
          "@media (max-width: 900px) and (min-width: 10px) and (min-width: 500px), (max-height: 600px) and (min-width: 10px) and (min-width: 500px)": "80%",
          query: "60%"
        }
      })
    );
  }

  #[test]
  fn an_or_condition_inside_an_and_keeps_its_parentheses() {
    let query = "@media (min-width: 1px) and ((max-width: 200px) or (max-height: 600px))";
    assert_eq!(
      transformed_styles(json!({ "color": { "default": "black", query: "blue" } })),
      json!({ "color": { "default": "black", query: "blue" } })
    );
  }

  #[test]
  fn a_media_type_from_a_negated_later_query_is_moved_to_the_front() {
    assert_eq!(
      transformed_styles(json!({
        "color": {
          "default": "black",
          "@media (max-width: 900px)": "red",
          "@media not print, (max-width: 100px)": "blue"
        }
      })),
      json!({
        "color": {
          "default": "black",
          "@media print and (max-width: 900px) and (not (max-width: 100px))": "red",
          "@media not print, (max-width: 100px)": "blue"
        }
      })
    );
  }
}

