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

/// The value a CSS-typed declaration carries, and the type declaration itself
/// where the value is one.
///
/// `defineVars` accepts either a plain value or a `{ syntax, value }` object
/// that states the value's CSS type. This reads the second shape apart into the
/// value the stylesheet takes and the type declaration `@property` needs, and
/// hands a plain value straight back with no type.
pub fn get_css_value(key_value: KeyValueProp) -> (Box<Expr>, Option<BaseCSSType>) {
  let Some(obj) = key_value.value.as_object() else {
    return (key_value.value, None);
  };

  // A typed declaration is the one that names a syntax and gives a value.
  // Short of either name the object is an ordinary value, and it is handed
  // back whole.
  let Some(value) = value_named(obj, "syntax").and_then(|_| value_named(obj, "value")) else {
    return (key_value.value, None);
  };

  (Box::new(value.into_owned()), Some(obj.clone().into()))
}

/// The value written under `name` in `object`, read where it lies.
///
/// The key is named through the reader the rest of the workspace names a key
/// with, so every way of writing one is read. The four that can write `syntax`
/// or `value` are bare, quoted, computed from static text, and the shorthand:
/// `{ syntax: x }`, `{ "syntax": x }`, `{ ["syntax"]: x }` and `{ syntax }` all
/// name `syntax`. A number and a big integer are named too, and neither can
/// spell either word.
///
/// The value is borrowed from the object, except for a shorthand, which holds
/// only the name and so has no value node to borrow.
///
/// The match and the answer are one step on purpose. They used to be two, and
/// the two readings disagreed for exactly one spelling: a property was matched
/// through the pair a shorthand stands for, so `{ value }` matched, and the
/// value was then taken from the property as the object holds it, where a
/// shorthand has no pair to take. It matched and then could not be read.
///
/// The spellings are read through the same helper the rest of the workspace
/// names a key with. Before that, only a bare key was read, while the reader
/// beside this one -- which builds the type from the same object -- read all
/// of them. The two never disagreed where anyone could see it, because this
/// one guards the door: a quoted or computed name was not recognised, so the
/// object was handed back as an ordinary value and the other reader never
/// ran on a shape it would have accepted.
///
/// Asked twice for one object, which is why it is one function rather than a
/// walk spelled out at each reader.
///
/// **A shorthand answers an identifier, and an identifier is not a value the
/// caller can finish with.** [`BaseCSSType`] needs a string for `syntax` and a
/// literal or an object for `value`, so `{ syntax: '<color>', value }` is
/// still refused -- with the sentence the written-out pair is refused with,
/// which is the point. Reading the value is what stops the two spellings
/// disagreeing; it is not what makes the declaration compile.
///
/// No source reaches any spelling but the bare one. Both callers of
/// [`get_css_value`] are given a declaration the evaluator rebuilt, and the
/// rebuild writes every key as a bare name and expands every shorthand. The
/// other three spellings are therefore held by the suite alone. They are read
/// so that one object has one meaning here, not because a build depends on
/// it.
fn value_named<'object>(object: &'object ObjectLit, name: &str) -> Option<Cow<'object, Expr>> {
  object.props.iter().find_map(|prop| match prop {
    PropOrSpread::Spread(_) => stylex_unimplemented!("{}", SPREAD_NOT_SUPPORTED),
    PropOrSpread::Prop(prop) => match prop.as_ref() {
      Prop::Shorthand(ident) => {
        (ident.sym == *name).then(|| Cow::Owned(Expr::Ident(ident.clone())))
      },
      // Every way of writing a key is read, because a bare name, a quoted
      // name and a computed one name the same property in JavaScript. A key
      // with no static name, unreadable text included, is not the name this
      // reader was asked for.
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
