/*!
"Last media query wins" transformation: at depth >= 1 of the style object, earlier media
queries are rewritten with negations of later ones so the last matching query wins.
*/

use super::media_query::{
  MediaAndRules, MediaNotRule, MediaOrRules, MediaQuery, MediaQueryRule, validate_media_query,
};
use stylex_macros::stylex_panic;
use stylex_utils::collections::{FxBuildHasher, FxIndexMap, IndexMapEntry};
use swc_core::{
  atoms::Wtf8Atom,
  common::DUMMY_SP,
  ecma::ast::{Expr, KeyValueProp, ObjectLit, Prop, PropName, PropOrSpread, Str},
};

/// The property's key as text; `None` for numeric, bigint or computed keys.
fn key_value_str(key_value: &KeyValueProp) -> Option<&str> {
  match &key_value.key {
    PropName::Str(s) => s.value.as_str(),
    PropName::Ident(id) => Some(id.sym.as_str()),
    _ => None,
  }
}

/// Entry point for the last-media-query-wins transform.
pub fn last_media_query_wins_transform(styles: &[KeyValueProp]) -> Vec<KeyValueProp> {
  dfs_process_queries(styles, 0)
}

/// Create an `ObjectLit` from key-value pairs.
fn create_object_from_key_values(key_values: Vec<KeyValueProp>) -> ObjectLit {
  let props = key_values
    .into_iter()
    .map(|kv| PropOrSpread::Prop(Box::new(Prop::KeyValue(kv))))
    .collect();

  ObjectLit {
    span: DUMMY_SP,
    props,
  }
}

/// DFS traversal with depth tracking (upstream `dfsProcessQueries`).
fn dfs_process_queries(obj: &[KeyValueProp], depth: u32) -> Vec<KeyValueProp> {
  let mut result = Vec::with_capacity(obj.len());

  for prop in obj {
    match &*prop.value {
      Expr::Object(obj_lit) => {
        // An object with spreads/shorthands/methods is preserved unchanged so StyleX validation
        // can still report the non-static value.
        let mut key_values = Vec::with_capacity(obj_lit.props.len());
        let mut only_key_values = true;
        for obj_prop in &obj_lit.props {
          if let PropOrSpread::Prop(p) = obj_prop
            && let Prop::KeyValue(kv) = &**p
          {
            key_values.push(kv.clone());
          } else {
            only_key_values = false;
            break;
          }
        }

        if !only_key_values {
          result.push(prop.clone());
          continue;
        }

        let processed_values = dfs_process_queries(&key_values, depth + 1);
        let transformed_obj = create_object_from_key_values(processed_values);

        result.push(KeyValueProp {
          key: prop.key.clone(),
          value: Box::new(Expr::Object(transformed_obj)),
        });
      },
      _ => {
        // Non-object values pass through, including `firstThatWorks` arrays.
        result.push(prop.clone());
      },
    }
  }

  if depth >= 1 {
    transform_media_queries_in_result(result)
  } else {
    result
  }
}

/// What identifies a property inside one object level: its name, or its position when the
/// name is unreadable. Unlike a JS object, integer-like keys keep source order.
#[derive(PartialEq, Eq, Hash)]
enum PropertyKey {
  /// A name this pass can read, which is what an author wrote.
  Named(String),
  /// A property whose name this pass cannot read, identified by its position.
  Positional(usize),
}

impl PropertyKey {
  /// The property's name, when it has one this pass can read.
  fn name(&self) -> Option<&str> {
    match self {
      PropertyKey::Named(name) => Some(name),
      PropertyKey::Positional(_) => None,
    }
  }
}

fn property_key(key_value: &KeyValueProp, index: usize) -> PropertyKey {
  match key_value_str(key_value) {
    Some(name) if !name.is_empty() => PropertyKey::Named(name.to_owned()),
    _ => PropertyKey::Positional(index),
  }
}

/// One `@media` key at this level, with everything its rewrite needs.
struct MediaEntry {
  /// The authored key, exactly as it appears in the map.
  key: String,
  /// That key, parsed.
  query: MediaQuery,
}

/// Rewrite the `@media` keys of one object level so that a later query wins.
///
/// Matches upstream `dfsProcessQueries` (`delete result[old]; result[new] = value` on a JS
/// object): rewritten keys move to the end in declaration order; a key canonicalizing to an
/// existing one keeps the earlier position with the later value (a ported upstream defect); the
/// value is read at rewrite time; duplicate authored keys collapse first-position, last-value.
///
/// TODO(upstream-report): record the facebook/stylex issue number here and in
/// the ADR once the report drafted for it is filed.
fn transform_media_queries_in_result(result: Vec<KeyValueProp>) -> Vec<KeyValueProp> {
  let is_media_key = |key: &str| key.starts_with("@media ");

  // Bail out before building the map so a level with no media key is returned as is.
  if !result
    .iter()
    .any(|kv| key_value_str(kv).is_some_and(is_media_key))
  {
    return result;
  }

  let mut entries: FxIndexMap<PropertyKey, KeyValueProp> =
    FxIndexMap::with_capacity_and_hasher(result.len(), FxBuildHasher);
  for (index, kv) in result.into_iter().enumerate() {
    entries.insert(property_key(&kv, index), kv);
  }

  let media_keys = entries
    .keys()
    .filter_map(PropertyKey::name)
    .filter(|name| is_media_key(name))
    .map(str::to_owned)
    .collect::<Vec<_>>();

  let mut media_entries: Vec<MediaEntry> = Vec::with_capacity(media_keys.len());
  for key in media_keys {
    // Validated, not just parsed: the tokenizer synthesizes a closing paren at end of input.
    match validate_media_query(&key) {
      Ok(query) => media_entries.push(MediaEntry { key, query }),
      Err(_) => {
        // Hard error: no later phase rejects it. The caller reports invalid media query syntax.
        stylex_panic!("Invalid media query: {}", key);
      },
    }
  }

  // Each key negates the queries declared after it: suffixes of one list, sliced when consumed.
  let declared_queries: Vec<MediaQuery> = media_entries
    .iter()
    .map(|entry| entry.query.clone())
    .collect();

  // `shift_remove` keeps order at a quadratic cost; fine at style-object sizes.
  for (index, entry) in media_entries.into_iter().enumerate() {
    // The lookup cannot miss. The value is read now, so an earlier rewrite that landed on a
    // later key is what that key carries.
    let rewritten = entries
      .shift_remove(&PropertyKey::Named(entry.key))
      .map(|current| {
        let later_queries = declared_queries[index + 1..].to_vec();
        let combined_query = combine_media_query_with_negations(entry.query, later_queries);
        (combined_query.to_string(), current.value)
      });

    // Iterated rather than branched on, so the impossible `None` needs no arm.
    for (new_media_key, value) in rewritten.into_iter() {
      // Atom taken before the key moves into the map, avoiding a second copy of a long name.
      let key_atom = Wtf8Atom::from(new_media_key.as_str());

      match entries.entry(PropertyKey::Named(new_media_key)) {
        // Already present: keeps its position and takes this value.
        IndexMapEntry::Occupied(mut occupied) => occupied.get_mut().value = value,
        IndexMapEntry::Vacant(vacant) => {
          vacant.insert(KeyValueProp {
            key: PropName::Str(Str {
              span: DUMMY_SP,
              value: key_atom,
              raw: None,
            }),
            value,
          });
        },
      }
    }
  }

  entries.into_values().collect()
}

/// Combine a media query with the negations of every query that follows it.
fn combine_media_query_with_negations(
  current: MediaQuery,
  negations: Vec<MediaQuery>,
) -> MediaQuery {
  if negations.is_empty() {
    return current;
  }

  let not_rules: Vec<MediaQueryRule> = negations
    .into_iter()
    .map(|mq| MediaQueryRule::Not(MediaNotRule::new(mq.queries)))
    .collect();

  let combined_ast = match current.queries {
    MediaQueryRule::Or(or_rules) => {
      let new_rules = or_rules
        .rules
        .into_iter()
        .map(|rule| {
          let mut and_rules = vec![rule];
          and_rules.extend(not_rules.clone());
          MediaQueryRule::And(MediaAndRules::new(and_rules))
        })
        .collect();
      MediaQueryRule::Or(MediaOrRules::new(new_rules))
    },
    other => {
      let mut rules = vec![other];
      rules.extend(not_rules);
      MediaQueryRule::And(MediaAndRules::new(rules))
    },
  };

  MediaQuery::new(combined_ast)
}

#[cfg(test)]
#[path = "../tests/at_queries/media_query_transform_test.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/at_queries/media_query_transform_coverage_test.rs"]
mod media_query_transform_coverage_test;
