//! Current StyleX media-type resolution (#1888). Runs after interval normalization.
//! Source: meta/stylex style-value-parser/at-queries/media-query.js (3e33a25).
use super::media_query::{MediaAndRules, MediaKeyword, MediaNotRule, MediaOrRules, MediaQueryRule};

fn keyword(key: &str, not: bool) -> MediaQueryRule {
  MediaQueryRule::MediaKeyword(MediaKeyword::new(key, not, false))
}
fn never(rule: &MediaQueryRule) -> bool {
  matches!(rule, MediaQueryRule::MediaKeyword(value) if value.key == "all" && value.not)
}
fn always(rule: &MediaQueryRule) -> bool {
  matches!(rule, MediaQueryRule::MediaKeyword(value) if value.key == "all" && !value.not && !value.only)
}
fn contains_type(rule: &MediaQueryRule) -> bool {
  match rule {
    MediaQueryRule::MediaKeyword(_) => true,
    MediaQueryRule::Not(value) => contains_type(&value.rule),
    MediaQueryRule::And(value) => value.rules.iter().any(contains_type),
    MediaQueryRule::Or(value) => value.rules.iter().any(contains_type),
    _ => false,
  }
}
fn complement(key: &str) -> &'static str {
  match key {
    "screen" => "print",
    "print" => "screen",
    _ => "never",
  }
}
fn requirement(rule: &MediaQueryRule) -> Option<&str> {
  match rule {
    MediaQueryRule::MediaKeyword(value) => Some(if value.not {
      complement(&value.key)
    } else {
      &value.key
    }),
    MediaQueryRule::Not(value) => match value.rule.as_ref() {
      MediaQueryRule::MediaKeyword(value) if !value.not => Some(complement(&value.key)),
      _ => None,
    },
    _ => None,
  }
}
fn negate(rule: MediaQueryRule) -> MediaQueryRule {
  match rule {
    MediaQueryRule::MediaKeyword(mut value) => {
      value.not = !value.not;
      MediaQueryRule::MediaKeyword(value)
    },
    MediaQueryRule::Not(value) => *value.rule,
    MediaQueryRule::And(value) => MediaQueryRule::Or(MediaOrRules::new(
      value.rules.into_iter().map(negate).collect(),
    )),
    MediaQueryRule::Or(value) => MediaQueryRule::And(MediaAndRules::new(
      value.rules.into_iter().map(negate).collect(),
    )),
    value => MediaQueryRule::Not(MediaNotRule::new(value)),
  }
}

pub(super) fn resolve(rule: MediaQueryRule) -> MediaQueryRule {
  // Media-type-free ASTs retain their exact normalized shape and ordering.
  if !contains_type(&rule) {
    return rule;
  }
  match rule {
    MediaQueryRule::Or(value) => {
      let mut rules = Vec::new();
      for child in value.rules {
        match resolve(child) {
          MediaQueryRule::Or(value) => {
            rules.extend(value.rules.into_iter().filter(|rule| !never(rule)))
          },
          value if !never(&value) => rules.push(value),
          _ => {},
        }
      }
      if rules.iter().any(always) {
        return keyword("all", false);
      }
      match rules.len() {
        0 => keyword("all", true),
        1 => rules.remove(0),
        _ => MediaQueryRule::Or(MediaOrRules::new(rules)),
      }
    },
    MediaQueryRule::And(value) => {
      let mut resolved: Vec<_> = value.rules.into_iter().map(resolve).collect();
      if let Some(index) = resolved
        .iter()
        .position(|rule| matches!(rule, MediaQueryRule::Or(_)))
      {
        let MediaQueryRule::Or(branches) = resolved.remove(index) else {
          unreachable!()
        };
        return resolve(MediaQueryRule::Or(MediaOrRules::new(
          branches
            .rules
            .into_iter()
            .map(|branch| {
              MediaQueryRule::And(MediaAndRules::new(
                std::iter::once(branch)
                  .chain(resolved.iter().cloned())
                  .collect(),
              ))
            })
            .collect(),
        )));
      }
      let mut media_type = "all".to_owned();
      let mut anchor = None;
      let mut others = Vec::new();
      for rule in resolved {
        if let MediaQueryRule::MediaKeyword(value) = &rule
          && !value.not
        {
          if value.key != "all" {
            if media_type == "all" {
              media_type = value.key.clone();
              anchor = Some(rule.clone());
            } else if media_type != value.key {
              return keyword("all", true);
            } else if anchor.is_none() {
              anchor = Some(rule.clone());
            }
          }
          continue;
        }
        match requirement(&rule) {
          Some("never") => return keyword("all", true),
          Some("all") => {},
          Some(required) if media_type == "all" => media_type = required.to_owned(),
          Some(required) if media_type != required => return keyword("all", true),
          Some(_) => {},
          None => others.push(rule),
        }
      }
      if media_type != "all" {
        others.insert(0, anchor.unwrap_or_else(|| keyword(&media_type, false)));
      }
      match others.len() {
        0 => keyword("all", false),
        1 => others.remove(0),
        _ => MediaQueryRule::And(MediaAndRules::new(others)),
      }
    },
    MediaQueryRule::Not(value) => {
      let inner = resolve(*value.rule);
      match inner {
        MediaQueryRule::MediaKeyword(mut value) => {
          value.not = !value.not;
          resolve(MediaQueryRule::MediaKeyword(value))
        },
        MediaQueryRule::And(_) | MediaQueryRule::Or(_) => resolve(negate(inner)),
        value => MediaQueryRule::Not(MediaNotRule::new(value)),
      }
    },
    value => value,
  }
}
