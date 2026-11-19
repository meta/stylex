//! Width sort keys for `processStylexRules`, read from the engine's media-query parser as Babel
//! reads them from style-value-parser (`mediaQueryWidthSortKey`). The engine is more lenient than
//! style-value-parser for a few spellings Babel refuses to compile, such as `((min-width: 1px))`;
//! those sort by their bound here, as the compiler accepts them.
use stylex_css_parser::at_queries::media_query::{
    validate_media_query, MediaQueryRule, MediaRuleValue,
};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WidthKey {
    pub context: String,
    pub min: bool,
    pub value: f64,
}

/// Babel's `collectWidthBounds`: the `min-width`/`max-width` px bounds a query imposes, or `None`
/// when it has none to sort by (`not`, `or`, and non-px bounds other than zero).
fn bounds(rule: &MediaQueryRule) -> Option<Vec<(bool, f64)>> {
    match rule {
        MediaQueryRule::Pair(pair) => {
            let min = match pair.key.to_ascii_lowercase().as_str() {
                "min-width" => true,
                "max-width" => false,
                _ => return Some(Vec::new()),
            };
            match &pair.value {
                MediaRuleValue::Number(value) if *value == 0.0 => Some(vec![(min, 0.0)]),
                MediaRuleValue::Length(length)
                    if length.value == 0.0 || length.unit.eq_ignore_ascii_case("px") =>
                {
                    Some(vec![(min, length.value)])
                }
                _ => None,
            }
        }
        MediaQueryRule::And(and) => {
            let mut out = Vec::new();
            for rule in &and.rules {
                out.extend(bounds(rule)?);
            }
            Some(out)
        }
        MediaQueryRule::MediaKeyword(keyword) if !keyword.not => Some(Vec::new()),
        _ => None,
    }
}

/// The leading at-rule preludes of a rule, outermost first.
fn preludes(rule: &str) -> Vec<&str> {
    let mut remaining = rule;
    let mut preludes = Vec::new();
    while remaining.starts_with('@') {
        let Some(brace) = remaining.find('{') else {
            break;
        };
        preludes.push(remaining[..brace].trim_end());
        remaining = &remaining[brace + 1..];
    }
    preludes
}

/// The rule's at-rule chain, the cache key for its width key.
pub(crate) fn width_chain_key(rule: &str) -> String {
    preludes(rule).join("{")
}

/// Babel's `widthSortKeyForChain`: the single `@media` bound a rule sorts by, with the
/// surrounding at-rules as its context.
pub(crate) fn width_key(rule: &str) -> Option<WidthKey> {
    let preludes = preludes(rule);
    let mut media = preludes
        .iter()
        .enumerate()
        .filter(|(_, prelude)| prelude.starts_with("@media "));
    let (index, prelude) = media.next()?;
    if media.next().is_some() {
        return None;
    }
    // The engine's validator: the same parse the compiler uses, cached, with unbalanced and too
    // deeply nested input refused before tokenizing.
    let parsed = validate_media_query(prelude).ok()?;
    let found = bounds(&parsed.queries)?;
    let [(min, value)] = found[..] else {
        return None;
    };
    let context = preludes
        .iter()
        .enumerate()
        .map(|(i, prelude)| if i == index { "@media" } else { prelude })
        .collect::<Vec<_>>()
        .join("{");
    Some(WidthKey {
        context,
        min,
        value,
    })
}
