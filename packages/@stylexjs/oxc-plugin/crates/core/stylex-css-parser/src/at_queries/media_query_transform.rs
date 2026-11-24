/*!
"Last media query wins" transformation: at depth >= 1 of the style object, earlier media
queries are rewritten with negations of later ones so the last matching query wins.
*/

use super::media_query::{
  MediaAndRules, MediaNotRule, MediaOrRules, MediaQuery, MediaQueryRule, validate_media_query,
};
use stylex_macros::stylex_panic;
use stylex_utils::collections::{FxBuildHasher, FxIndexMap, IndexMapEntry};
use stylex_value::Value;

/// One property of a style object, as the transform needs to see it. #2008 ran this pass
/// over SWC `KeyValueProp`; the carve makes it generic so any front end (SWC, oxc) or the
/// core's own [`Value`] tree can use it. The behaviour is unchanged.
pub trait MediaQueryProp: Sized + Clone {
  /// The property's key as text; `None` for numeric, bigint or computed keys.
  fn key_str(&self) -> Option<&str>;
  /// The value's properties when it is an object made only of plain key-value properties;
  /// `None` for any other value, including objects with spreads, shorthands or methods.
  fn object_children(&self) -> Option<Vec<Self>>;
  /// This property with its value replaced by an object of `children`.
  fn with_object_children(&self, children: Vec<Self>) -> Self;
  /// A property named by the string `key`, carrying this property's value.
  fn rekeyed(self, key: &str) -> Self;
  /// Replace this property's value with `other`'s, keeping this key.
  fn set_value_from(&mut self, other: Self);
}

/// The core's own style tree: `(key, value)` entries of a [`Value::Object`].
impl MediaQueryProp for (String, Value) {
  fn key_str(&self) -> Option<&str> {
    Some(&self.0)
  }

  fn object_children(&self) -> Option<Vec<Self>> {
    match &self.1 {
      Value::Object(obj) => Some(obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
      _ => None,
    }
  }

  fn with_object_children(&self, children: Vec<Self>) -> Self {
    // Children are rebuilt in order; entries are already unique per key.
    (self.0.clone(), Value::Object(children.into_iter().collect()))
  }

  fn rekeyed(self, key: &str) -> Self {
    (key.to_owned(), self.1)
  }

  fn set_value_from(&mut self, other: Self) {
    self.1 = other.1;
  }
}

fn key_value_str<P: MediaQueryProp>(key_value: &P) -> Option<&str> {
  key_value.key_str()
}

/// Entry point for the last-media-query-wins transform.
pub fn last_media_query_wins_transform<P: MediaQueryProp>(styles: &[P]) -> Vec<P> {
  dfs_process_queries(styles, 0)
}

/// [`last_media_query_wins_transform`] over one style object of the core's value tree.
pub fn last_media_query_wins_transform_value(styles: &stylex_value::Object) -> stylex_value::Object {
  let entries: Vec<(String, Value)> = styles.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
  last_media_query_wins_transform(&entries).into_iter().collect()
}

/// DFS traversal with depth tracking (upstream `dfsProcessQueries`).
fn dfs_process_queries<P: MediaQueryProp>(obj: &[P], depth: u32) -> Vec<P> {
  let mut result = Vec::with_capacity(obj.len());

  for prop in obj {
    // An object with spreads/shorthands/methods is preserved unchanged so StyleX validation
    // can still report the non-static value. Non-object values pass through, including
    // `firstThatWorks` arrays.
    match prop.object_children() {
      Some(key_values) => {
        let processed_values = dfs_process_queries(&key_values, depth + 1);
        result.push(prop.with_object_children(processed_values));
      },
      None => result.push(prop.clone()),
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

fn property_key<P: MediaQueryProp>(key_value: &P, index: usize) -> PropertyKey {
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
fn transform_media_queries_in_result<P: MediaQueryProp>(result: Vec<P>) -> Vec<P> {
  let is_media_key = |key: &str| key.starts_with("@media ");

  // Bail out before building the map so a level with no media key is returned as is.
  if !result
    .iter()
    .any(|kv| key_value_str(kv).is_some_and(is_media_key))
  {
    return result;
  }

  let mut entries: FxIndexMap<PropertyKey, P> =
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
        (combined_query.to_string(), current)
      });

    // Iterated rather than branched on, so the impossible `None` needs no arm.
    for (new_media_key, current) in rewritten.into_iter() {
      // The rekeyed property is built before the key moves into the map.
      let rekeyed = current.rekeyed(&new_media_key);

      match entries.entry(PropertyKey::Named(new_media_key)) {
        // Already present: keeps its position and takes this value.
        IndexMapEntry::Occupied(mut occupied) => occupied.get_mut().set_value_from(rekeyed),
        IndexMapEntry::Vacant(vacant) => {
          vacant.insert(rekeyed);
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
#[path = "../tests/at_queries/mini_swc_ast.rs"]
pub(crate) mod mini_swc_ast;

#[cfg(test)]
#[path = "../tests/at_queries/media_query_transform_test.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/at_queries/media_query_transform_coverage_test.rs"]
mod media_query_transform_coverage_test;
