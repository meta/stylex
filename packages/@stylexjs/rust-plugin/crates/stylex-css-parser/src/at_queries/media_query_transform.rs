/*!
Media query transformation functionality.

Implements the "last media query wins" transformation logic. This ensures proper
specificity handling when multiple media queries target the same properties.

This implementation provides media query transformation:
1. DFS traversal of the style object
2. At depth >= 1, apply negation-based media query transformation
3. Use pure AST manipulation, not range-based logic
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

/// The property's key as text, when this pass can read it.
///
/// Borrowed rather than owned, because the media-key bail-out below asks this of
/// every property of every style object the compiler sees and throws the answer
/// away. Both readable key kinds already hold their text. `None` is a name this
/// pass cannot read -- numeric, bigint, computed -- which is the same answer the
/// empty string used to stand for, and costs nothing to give.
fn key_value_str(key_value: &KeyValueProp) -> Option<&str> {
  match &key_value.key {
    PropName::Str(s) => s.value.as_str(),
    PropName::Ident(id) => Some(id.sym.as_str()),
    _ => None,
  }
}

/// Main entry point for the last-media-query-wins transform
pub fn last_media_query_wins_transform(styles: &[KeyValueProp]) -> Vec<KeyValueProp> {
  dfs_process_queries(styles, 0)
}

/// Helper function to create ObjectLit from key-value pairs
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

/// DFS traversal with depth tracking, mirroring the reference
/// implementation's `dfsProcessQueries`.
fn dfs_process_queries(obj: &[KeyValueProp], depth: u32) -> Vec<KeyValueProp> {
  // Every arm of the loop below pushes exactly once, so the length is known.
  let mut result = Vec::with_capacity(obj.len());

  for prop in obj {
    match &*prop.value {
      Expr::Object(obj_lit) => {
        // Extract key-value pairs from the object. If the object contains
        // spreads/shorthands/methods, preserve it unchanged; silently dropping
        // those props would mutate user AST before the main StyleX validation
        // can report the unsupported non-static value.
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

        // Recursively process the object at depth + 1
        let processed_values = dfs_process_queries(&key_values, depth + 1);
        let transformed_obj = create_object_from_key_values(processed_values);

        result.push(KeyValueProp {
          key: prop.key.clone(),
          value: Box::new(Expr::Object(transformed_obj)),
        });
      },
      _ => {
        // Non-object values pass through unchanged, including the `firstThatWorks`
        // arrays this transform deliberately ignores.
        result.push(prop.clone());
      },
    }
  }

  // Apply media query transformation if at depth >= 1
  if depth >= 1 {
    transform_media_queries_in_result(result)
  } else {
    result
  }
}

/// What identifies a property inside one object level.
///
/// The rewritten keys are held in a map, so this has to tell two properties
/// apart the way a JavaScript object's own key does. A name this pass cannot
/// read -- numeric, bigint, computed -- has no text to key on, and letting every
/// one of those share the empty string would merge properties that are
/// genuinely distinct; they are identified by where they sit instead. Two
/// variants rather than one string with a reserved prefix, so that "a position
/// can never collide with something an author wrote" holds by construction
/// rather than by nothing else ever using a NUL byte.
///
/// One respect in which this is *not* a JavaScript object: an object enumerates
/// integer-like keys first and in ascending order, whatever the source order,
/// while this keeps every key where it was written. No `@media` key is
/// integer-like, so the rewrite this map exists for cannot reach the
/// difference, and matching it would mean reproducing a rule of the language
/// rather than of the transform.
#[derive(PartialEq, Eq, Hash)]
enum PropertyKey {
  /// A name this pass can read, which is what an author wrote.
  Named(String),
  /// A property whose name this pass cannot read, identified by its position so
  /// that two of them stay distinct.
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
///
/// These travelled as parallel vectors indexed in lockstep, which is one
/// off-by-one away from combining a key with another key's negations -- a
/// mistake that would not fail to compile and would emit a plausible query.
struct MediaEntry {
  /// The authored key, exactly as it appears in the map.
  key: String,
  /// That key, parsed.
  query: MediaQuery,
}

/// Rewrite the `@media` keys of one object level so that a later query wins.
///
/// The keys live in an insertion-ordered map rather than a list, because the
/// reference implementation's `dfsProcessQueries` holds them in a plain
/// JavaScript object and rewrites each one with `delete result[old]` followed
/// by `result[new] = value`. Four consequences follow, and all four are
/// contract:
///
/// - deleting and re-adding moves a key to the end, so the rewritten media keys
///   end up after every other property, in their own declaration order
/// - assigning a key that is already present keeps that key's position and
///   replaces only its value, so two entries canonicalizing to one query text
///   leave one rule, at the earlier position, holding the later value
/// - the value is read from the map at the moment its key is rewritten, not
///   collected beforehand, so an earlier rewrite that landed on a later key is
///   what that later key then carries
/// - building the map is itself an assignment per property, so two properties
///   an author wrote under one name collapse the same way, before any rewrite
///   runs -- the first position, the last value. A duplicate key is legal
///   JavaScript and an object keeps one of it; this keeps one of it too
///
/// The second is the one an author notices, and the fourth looks like it from
/// the outside: one of their declarations is absent from the output. The fourth
/// is not a defect -- it is what the language does with a duplicate key, and
/// upstream's object does it too. The second is. That is faithful rather than accidental, and nothing is
/// reported for it, because the reference implementation reports nothing. It is
/// a **ported upstream defect, not a design** -- see
/// [ADR 0001](../../docs/adr/0001-the-official-compilers-output-wins.md). When
/// the upstream report is resolved this follows it.
///
/// TODO(upstream-report): record the facebook/stylex issue number here and in
/// the ADR once the report drafted for it is filed.
//
// JS-parity: insertion order is observable here, so this is an `FxIndexMap`
// rather than an `FxHashMap` -- it stands in for the plain JavaScript object
// `dfsProcessQueries` builds in `@stylexjs/babel-plugin` 0.19.0.
fn transform_media_queries_in_result(result: Vec<KeyValueProp>) -> Vec<KeyValueProp> {
  let is_media_key = |key: &str| key.starts_with("@media ");

  // Bail out before building the map so that a level with no media key is
  // handed back exactly as it arrived.
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
    // Validated rather than merely parsed, because the tokenizer synthesizes a
    // closing parenthesis at end of input: `(min-width: 100px` parses cleanly
    // here and would reach the stylesheet as a query the author never wrote.
    // The reference implementation's tokenizer synthesizes nothing, so its
    // parse fails outright on the same input -- the balanced-parenthesis check
    // is how the two arrive at the same refusal.
    match validate_media_query(&key) {
      Ok(query) => media_entries.push(MediaEntry { key, query }),
      Err(_) => {
        // An unparseable query is a hard error, not something to pass through:
        // no later phase rejects it, so returning here emitted the broken query
        // verbatim into the stylesheet. The caller catches this and reports it
        // as invalid media query syntax.
        stylex_panic!("Invalid media query: {}", key);
      },
    }
  }

  // What each key negates is the run of queries declared after it, so the runs
  // are the suffixes of one list. Held as that list and sliced where it is
  // consumed rather than materialized per entry: the combined query embeds its
  // negations by value, so the same number of clones is paid either way, but
  // only one entry's suffix is alive at a time instead of all of them at once.
  let declared_queries: Vec<MediaQuery> = media_entries
    .iter()
    .map(|entry| entry.query.clone())
    .collect();

  // `shift_remove` keeps the surviving entries in order, which is the whole
  // point, and costs a shift each time -- so this loop is quadratic in the
  // number of properties at one level. That is the right trade at the sizes a
  // style object reaches; the expensive thing here is the expansion below, not
  // the bookkeeping.
  for (index, entry) in media_entries.into_iter().enumerate() {
    // Every key here came from this map and this line is the only thing that
    // removes one, so the lookup cannot miss. The rewrite is computed through
    // the option rather than branched on, so the impossible case needs no arm
    // of its own -- an arm no test could ever reach.
    //
    // The value is read here, at the moment this key is rewritten, rather than
    // collected up front: an earlier rewrite that landed on a later key is what
    // that later key then carries.
    let rewritten = entries
      .shift_remove(&PropertyKey::Named(entry.key))
      .map(|current| {
        // Consumed rather than cloned: an entry is read once and dead
        // afterwards.
        let later_queries = declared_queries[index + 1..].to_vec();
        let combined_query = combine_media_query_with_negations(entry.query, later_queries);
        (combined_query.to_string(), current.value)
      });

    // Iterated rather than branched on, which is what keeps the impossible
    // `None` from becoming an arm no test can reach -- see the note above the
    // lookup. A one-or-zero-iteration loop is the price of not having an
    // uncoverable branch here.
    for (new_media_key, value) in rewritten.into_iter() {
      // The atom is taken off the text before the key is moved into the map, so
      // the rewritten name is copied once rather than twice. On a distributed
      // ladder one of these names is hundreds of kilobytes, which is not a copy
      // to make for nothing.
      let key_atom = Wtf8Atom::from(new_media_key.as_str());

      match entries.entry(PropertyKey::Named(new_media_key)) {
        // The key is already there: it keeps its position and takes this value,
        // and the declaration that put it there is gone from the output.
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

/// Combine a media query with the negations of every query that follows it
fn combine_media_query_with_negations(
  current: MediaQuery,
  negations: Vec<MediaQuery>,
) -> MediaQuery {
  if negations.is_empty() {
    return current;
  }

  // Wrap each negated query in a `not` rule
  let not_rules: Vec<MediaQueryRule> = negations
    .into_iter()
    .map(|mq| MediaQueryRule::Not(MediaNotRule::new(mq.queries)))
    .collect();

  // Combine media query with negations
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
