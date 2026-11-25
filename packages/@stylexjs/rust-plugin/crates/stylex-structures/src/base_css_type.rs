use indexmap::IndexMap;
use std::borrow::Cow;
use stylex_ast::ast::convertors::{
  convert_key_value_to_str, convert_lit_to_string, get_key_values_from_object, key_value_name,
};
use stylex_ast::ast::factories::{
  create_key_value_prop, create_object_expression, create_object_lit, create_string_key_value_prop,
};
use stylex_ast::ast::keys::try_namespace_name_from_prop_key;
use stylex_constants::constants::messages::{SPREAD_NOT_SUPPORTED, VALUE_MUST_BE_STRING};
use stylex_enums::{css_syntax::CSSSyntax, value_with_default::ValueWithDefault};
use stylex_macros::{stylex_panic, stylex_unimplemented};
use stylex_utils::swc::get_expr_node_kind;
use swc_core::ecma::ast::{Expr, KeyValueProp, ObjectLit, Prop, PropOrSpread};

impl From<BaseCSSType> for Expr {
  fn from(instance: BaseCSSType) -> Self {
    let syntax_prop =
      create_string_key_value_prop("syntax", format!("{}", instance.syntax).as_str());

    let mut props = vec![syntax_prop];

    props.extend(BaseCSSType::value_to_props(instance.value, None));

    create_object_expression(props)
  }
}

#[derive(Debug, PartialEq, Clone, Hash)]
pub struct BaseCSSType {
  pub value: ValueWithDefault,
  pub syntax: CSSSyntax,
}

impl BaseCSSType {
  pub fn value_to_props(value: ValueWithDefault, top_key: Option<String>) -> Vec<PropOrSpread> {
    match value {
      ValueWithDefault::Number(n) => {
        let value_prop = create_string_key_value_prop(
          top_key.unwrap_or(String::from("value")).as_str(),
          n.to_string().as_str(),
        );
        vec![value_prop]
      },
      ValueWithDefault::String(s) => {
        let value_prop = create_string_key_value_prop(
          top_key.unwrap_or(String::from("value")).as_str(),
          s.as_str(),
        );
        vec![value_prop]
      },
      ValueWithDefault::Map(map) => {
        let mut local_props = Vec::with_capacity(map.len());

        for (key, val) in map {
          let props_to_extend = BaseCSSType::value_to_props(val, Some(key));
          local_props.extend(props_to_extend);
        }

        let object_expr = create_object_expression(local_props);
        let prop =
          create_key_value_prop(top_key.unwrap_or("value".to_string()).as_str(), object_expr);

        vec![prop]
      },
    }
  }
}

/// Splits a `defineVars` `{ syntax, value }` object into its value and `@property` type;
/// a plain value comes back with no type.
pub fn get_css_value(key_value: KeyValueProp) -> (Box<Expr>, Option<BaseCSSType>) {
  let Some(obj) = key_value.value.as_object() else {
    return (key_value.value, None);
  };

  // Without both `syntax` and `value` the object is an ordinary value.
  let Some(value) = value_named(obj, "syntax").and_then(|_| value_named(obj, "value")) else {
    return (key_value.value, None);
  };

  (Box::new(value.into_owned()), Some(obj.clone().into()))
}

/// The value under `name` in `object`, for every key spelling (bare, quoted, computed,
/// shorthand). A shorthand yields an identifier, which [`BaseCSSType`] still refuses.
fn value_named<'object>(object: &'object ObjectLit, name: &str) -> Option<Cow<'object, Expr>> {
  object.props.iter().find_map(|prop| match prop {
    PropOrSpread::Spread(_) => stylex_unimplemented!("{}", SPREAD_NOT_SUPPORTED),
    PropOrSpread::Prop(prop) => match prop.as_ref() {
      Prop::Shorthand(ident) => {
        (ident.sym == *name).then(|| Cow::Owned(Expr::Ident(ident.clone())))
      },
      // Bare, quoted and computed keys name the same property; non-static keys never match.
      Prop::KeyValue(key_value) => try_namespace_name_from_prop_key(&key_value.key)
        .is_some_and(|key| key == *name)
        .then(|| Cow::Borrowed(key_value.value.as_ref())),
      _ => stylex_unimplemented!("Unsupported prop type in CSS value"),
    },
  })
}

#[cfg(test)]
#[path = "tests/base_css_type_test.rs"]
mod tests;

impl From<ObjectLit> for BaseCSSType {
  fn from(obj: ObjectLit) -> BaseCSSType {
    let key_values = get_key_values_from_object(&obj);
    let mut syntax: Option<CSSSyntax> = None;

    let mut values: IndexMap<String, ValueWithDefault> = IndexMap::new();

    for key_value in key_values {
      let key = key_value_name(&key_value);

      match key.as_ref() {
        "syntax" => {
          syntax = key_value
            .value
            .as_lit()
            .and_then(convert_lit_to_string)
            .map(|str_val| str_val.into())
        },
        "value" => {
          let obj_value = match key_value.value.as_ref() {
            Expr::Object(obj) => obj,
            Expr::Lit(obj) => {
              let value = match convert_lit_to_string(obj) {
                Some(v) => v,
                None => stylex_panic!("{}", VALUE_MUST_BE_STRING),
              };

              let prop = create_string_key_value_prop("default", value.as_str());

              &create_object_lit(vec![prop])
            },
            _ => stylex_panic!(
              "Value must be an object or string, but got: {}",
              get_expr_node_kind(&key_value.value)
            ),
          };

          for key_value in get_key_values_from_object(obj_value) {
            let key = convert_key_value_to_str(&key_value);

            match key_value.value.as_ref() {
              Expr::Object(obj) => {
                let mut obj_map = IndexMap::new();

                let key_values = get_key_values_from_object(obj);

                for key_value in key_values {
                  let key = convert_key_value_to_str(&key_value);

                  match key_value.value.as_ref() {
                    Expr::Lit(lit) => {
                      let value = match convert_lit_to_string(lit) {
                        Some(v) => v,
                        None => stylex_panic!("{}", VALUE_MUST_BE_STRING),
                      };

                      obj_map.insert(key, ValueWithDefault::String(value));
                    },
                    _ => stylex_panic!(
                      "Value must be a string, but got: {}",
                      get_expr_node_kind(&key_value.value)
                    ),
                  }
                }

                let value = ValueWithDefault::Map(obj_map);

                values.insert(key, value);
              },
              Expr::Lit(lit) => {
                let value = match convert_lit_to_string(lit) {
                  Some(v) => v,
                  None => stylex_panic!("{}", VALUE_MUST_BE_STRING),
                };

                values.insert(key, ValueWithDefault::String(value));
              },
              _ => stylex_panic!(
                "Value must be a string or object, but got: {}",
                get_expr_node_kind(&key_value.value)
              ),
            }
          }
        },
        _ => {
          stylex_panic!(r#"Key "{}" not support by BaseCSSType"#, key)
        },
      }
    }

    assert!(!values.is_empty(), "Invalid value in defineVars");

    assert!(
      values.contains_key("default"),
      "CSS type requires a default value but none was provided."
    );

    BaseCSSType {
      value: ValueWithDefault::Map(values),
      syntax: match syntax {
        Some(s) => s,
        None => stylex_panic!("CSS syntax definition is required for this type."),
      },
    }
  }
}
