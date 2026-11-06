//! Media-query syntax tree, following style-value-parser's at-queries.
use super::numbers::{dimension, js_number};
use indexmap::IndexMap;
use regex::Regex;
use std::sync::OnceLock;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Query {
    Type { key: String, not: bool, only: bool },
    Pair { key: String, value: String },
    Feature(String),
    Not(Box<Query>),
    And(Vec<Query>),
    Or(Vec<Query>),
}
fn always() -> Query {
    Query::Type {
        key: "all".to_owned(),
        not: false,
        only: false,
    }
}
fn never() -> Query {
    Query::Type {
        key: "all".to_owned(),
        not: true,
        only: false,
    }
}
fn is_never(q: &Query) -> bool {
    matches!(q,Query::Type{key,not:true,..} if key=="all")
}
fn is_always(q: &Query) -> bool {
    matches!(q,Query::Type{key,not:false,only:false} if key=="all")
}
fn contains_type(q: &Query) -> bool {
    match q {
        Query::Type { .. } => true,
        Query::Not(q) => contains_type(q),
        Query::And(q) | Query::Or(q) => q.iter().any(contains_type),
        _ => false,
    }
}

fn split_top<'a>(input: &'a str, separator: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut depth = 0;
    let mut quote = None;
    let mut escaped = false;
    for (i, c) in input.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' {
            escaped = true;
            continue;
        }
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            continue;
        }
        if c == '\'' || c == '"' {
            quote = Some(c);
            continue;
        }
        if c == '(' {
            depth += 1;
        } else if c == ')' {
            depth -= 1;
        }
        if depth == 0 && input[i..].starts_with(separator) {
            out.push(input[start..i].trim());
            start = i + separator.len();
        }
    }
    out.push(input[start..].trim());
    out
}
fn outer(input: &str) -> bool {
    if !input.starts_with('(') || !input.ends_with(')') {
        return false;
    }
    let mut depth = 0;
    for (i, c) in input.char_indices() {
        if c == '(' {
            depth += 1;
        } else if c == ')' {
            depth -= 1;
            if depth == 0 && i + 1 < input.len() {
                return false;
            }
        }
    }
    depth == 0
}

pub(crate) fn parse(input: &str) -> Result<Query, String> {
    let input = input
        .strip_prefix("@media ")
        .ok_or_else(|| "Invalid media query syntax.".to_owned())?;
    let mut depth = 0;
    for c in input.chars() {
        if c == '(' {
            depth += 1;
        } else if c == ')' {
            depth -= 1;
            if depth < 0 {
                return Err("Invalid media query syntax.".to_owned());
            }
        }
    }
    if depth != 0 {
        return Err("Invalid media query syntax.".to_owned());
    }
    parse_expr(input).map(|q| resolve_types(normalize(q)))
}
fn parse_expr(input: &str) -> Result<Query, String> {
    let input = input.trim();
    let fail = || "Invalid media query syntax.".to_owned();
    for sep in [",", " or ", " and "] {
        let parts = split_top(input, sep);
        if parts.len() > 1 {
            let queries = parts
                .into_iter()
                .map(parse_expr)
                .collect::<Result<Vec<_>, _>>()?;
            return Ok(if sep == " and " {
                Query::And(queries)
            } else {
                Query::Or(queries)
            });
        }
    }
    if let Some(rest) = input.strip_prefix("not ") {
        if matches!(rest, "screen" | "print" | "all") {
            return Ok(Query::Type {
                key: rest.to_owned(),
                not: true,
                only: false,
            });
        }
        return Ok(Query::Not(Box::new(parse_expr(rest)?)));
    }
    if let Some(rest) = input.strip_prefix("only ") {
        if matches!(rest, "screen" | "print" | "all") {
            return Ok(Query::Type {
                key: rest.to_owned(),
                not: false,
                only: true,
            });
        }
        return Err(fail());
    }
    if matches!(input, "screen" | "print" | "all") {
        return Ok(Query::Type {
            key: input.to_owned(),
            not: false,
            only: false,
        });
    }
    if outer(input) {
        let inside = input[1..input.len() - 1].trim();
        if inside.contains(" and ")
            || inside.contains(" or ")
            || inside.starts_with("not ")
            || matches!(inside, "screen" | "print" | "all")
        {
            return parse_expr(inside);
        }
        let pair = split_top(inside, ":");
        if pair.len() == 2 && !pair[0].is_empty() && !pair[1].is_empty() {
            return Ok(Query::Pair {
                key: pair[0].to_owned(),
                value: super::media_value::normalize(pair[1])?,
            });
        }
        static RANGE: OnceLock<Regex> = OnceLock::new();
        let re = RANGE.get_or_init(|| {
            Regex::new(r"^(?:(.+?)\s*(<=|>=|<|>)\s*)?(width|height)\s*(<=|>=|<|>)\s*(.+?)$")
                .unwrap()
        });
        if let Some(c) = re.captures(inside) {
            let key = &c[3];
            let mut out = Vec::new();
            if let (Some(value), Some(op)) = (c.get(1), c.get(2)) {
                out.push(range_pair(key, value.as_str(), op.as_str(), true)?);
            }
            out.push(range_pair(key, &c[5], &c[4], false)?);
            return Ok(if out.len() == 1 {
                out.remove(0)
            } else {
                Query::And(out)
            });
        }
        static REVERSED: OnceLock<Regex> = OnceLock::new();
        if let Some(c) = REVERSED
            .get_or_init(|| Regex::new(r"^(.+?)\s*(<=|>=|<|>)\s*(width|height)$").unwrap())
            .captures(inside)
        {
            return range_pair(&c[3], &c[1], &c[2], true);
        }
        if matches!(inside, "color" | "monochrome" | "grid" | "color-index") {
            return Ok(Query::Feature(inside.to_owned()));
        }
    }
    Err(fail())
}
fn range_pair(key: &str, value: &str, op: &str, reversed: bool) -> Result<Query, String> {
    let (number, unit) =
        dimension(value.trim()).ok_or_else(|| "Invalid media query syntax.".to_owned())?;
    if unit.is_empty() {
        return Err("Invalid media query syntax.".to_owned());
    }
    let mut value = number
        .parse::<f64>()
        .map_err(|_| "Invalid media query syntax.".to_owned())?;
    let min = (op.starts_with('>')) != reversed;
    if !op.ends_with('=') {
        value += if min { 0.01 } else { -0.01 };
    }
    Ok(Query::Pair {
        key: format!("{}-{key}", if min { "min" } else { "max" }),
        value: format!("{}{unit}", js_number(value)),
    })
}

fn normalize(q: Query) -> Query {
    match q {
        Query::And(queries) => {
            let mut flat = Vec::new();
            for q in queries {
                match normalize(q) {
                    Query::And(q) => flat.extend(q),
                    q => flat.push(q),
                }
            }
            let merged = merge_intervals(flat);
            if merged.is_empty() {
                never()
            } else {
                Query::And(merged)
            }
        }
        Query::Or(q) => Query::Or(q.into_iter().map(normalize).collect()),
        Query::Not(q) => match normalize(*q) {
            Query::Not(q) => normalize(*q),
            q if is_never(&q) => always(),
            q => Query::Not(Box::new(q)),
        },
        q => q,
    }
}
/// style-value-parser's `mergeIntervalsForAnd`; empty when the ranges cannot intersect.
fn merge_intervals(q: Vec<Query>) -> Vec<Query> {
    for (i, rule) in q.iter().enumerate() {
        if let Query::Not(inner) = rule {
            if let Query::And(parts) = &**inner {
                if parts.len() == 2 {
                    let branches = parts
                        .iter()
                        .map(|part| {
                            let mut branch: Vec<_> = q
                                .iter()
                                .enumerate()
                                .filter(|(j, _)| *j != i)
                                .map(|(_, q)| q.clone())
                                .collect();
                            branch.push(Query::Not(Box::new(part.clone())));
                            merge_intervals(branch)
                        })
                        .filter(|branch| !branch.is_empty())
                        .map(|mut branch| {
                            if branch.len() == 1 {
                                branch.remove(0)
                            } else {
                                Query::And(branch)
                            }
                        })
                        .collect();
                    return vec![Query::Or(branches)];
                }
            }
        }
    }
    let mut ranges: IndexMap<String, (String, f64, f64)> = IndexMap::new();
    for rule in &q {
        let (key, value, negative) = match rule {
            Query::Pair { key, value } => (key, value, false),
            Query::Not(inner) => match &**inner {
                Query::Pair { key, value } => (key, value, true),
                _ => return q.clone(),
            },
            _ => return q.clone(),
        };
        let Some((number, unit)) = dimension(value) else {
            return q.clone();
        };
        let Ok(value) = number.parse::<f64>() else {
            return q.clone();
        };
        if unit.is_empty() {
            return q.clone();
        }
        let (dim, min) = match key.as_str() {
            "min-width" => ("width", true),
            "max-width" => ("width", false),
            "min-height" => ("height", true),
            "max-height" => ("height", false),
            _ => return q.clone(),
        };
        let range = ranges.entry(dim.to_owned()).or_insert((
            unit.to_owned(),
            f64::NEG_INFINITY,
            f64::INFINITY,
        ));
        if range.0 != unit {
            return q.clone();
        }
        if negative {
            if min {
                range.2 = range.2.min(
                    value
                        - if dim == "width" && unit.eq_ignore_ascii_case("px") {
                            0.02
                        } else {
                            0.01
                        },
                );
            } else {
                range.1 = range.1.max(value + 0.01);
            }
        } else if min {
            range.1 = range.1.max(value);
        } else {
            range.2 = range.2.min(value);
        }
    }
    let mut out = Vec::new();
    for (dim, (unit, min, max)) in ranges {
        if min > max {
            return Vec::new();
        }
        if min.is_finite() {
            out.push(Query::Pair {
                key: format!("min-{dim}"),
                value: format!("{}{unit}", js_number(min)),
            });
        }
        if max.is_finite() {
            out.push(Query::Pair {
                key: format!("max-{dim}"),
                value: format!("{}{unit}", js_number(max)),
            });
        }
    }
    if out.is_empty() {
        q
    } else {
        out
    }
}
fn negate(q: Query) -> Query {
    match q {
        Query::Type { key, not, only } => Query::Type {
            key,
            not: !not,
            only,
        },
        Query::Not(q) => *q,
        Query::And(q) => Query::Or(q.into_iter().map(negate).collect()),
        Query::Or(q) => Query::And(q.into_iter().map(negate).collect()),
        q => Query::Not(Box::new(q)),
    }
}
fn requirement(q: &Query) -> Option<&str> {
    match q {
        Query::Type {
            key, not: false, ..
        } => Some(key),
        Query::Type { key, not: true, .. } => Some(if key == "screen" {
            "print"
        } else if key == "print" {
            "screen"
        } else {
            "never"
        }),
        Query::Not(q) => match &**q {
            Query::Type {
                key, not: false, ..
            } => Some(if key == "screen" {
                "print"
            } else if key == "print" {
                "screen"
            } else {
                "never"
            }),
            _ => None,
        },
        _ => None,
    }
}
fn resolve_types(q: Query) -> Query {
    if !contains_type(&q) {
        return q;
    }
    match q {
        Query::Or(queries) => {
            let mut flat = Vec::new();
            for q in queries {
                match resolve_types(q) {
                    Query::Or(q) => flat.extend(q),
                    q => flat.push(q),
                }
            }
            flat.retain(|q| !is_never(q));
            if flat.iter().any(is_always) {
                always()
            } else if flat.is_empty() {
                never()
            } else if flat.len() == 1 {
                flat.remove(0)
            } else {
                Query::Or(flat)
            }
        }
        Query::And(queries) => {
            let resolved: Vec<_> = queries.into_iter().map(resolve_types).collect();
            if let Some(i) = resolved.iter().position(|q| matches!(q, Query::Or(_))) {
                let Query::Or(branches) = &resolved[i] else {
                    unreachable!()
                };
                return resolve_types(Query::Or(
                    branches
                        .iter()
                        .map(|q| {
                            Query::And(
                                std::iter::once(q.clone())
                                    .chain(
                                        resolved
                                            .iter()
                                            .enumerate()
                                            .filter(|(j, _)| *j != i)
                                            .map(|(_, q)| q.clone()),
                                    )
                                    .collect(),
                            )
                        })
                        .collect(),
                ));
            }
            let mut ty = "all".to_owned();
            let mut anchor = None;
            let mut rest = Vec::new();
            for q in resolved {
                if let Query::Type {
                    key, not: false, ..
                } = &q
                {
                    if key != "all" {
                        if ty == "all" {
                            ty = key.clone();
                            anchor = Some(q.clone());
                        } else if ty != *key {
                            return never();
                        }
                    }
                    continue;
                }
                if let Some(req) = requirement(&q) {
                    if req == "never" {
                        return never();
                    }
                    if req != "all" {
                        if ty == "all" {
                            ty = req.to_owned();
                        } else if ty != req {
                            return never();
                        }
                    }
                } else {
                    rest.push(q);
                }
            }
            if ty != "all" {
                rest.insert(
                    0,
                    anchor.unwrap_or(Query::Type {
                        key: ty,
                        not: false,
                        only: false,
                    }),
                );
            }
            match rest.len() {
                0 => always(),
                1 => rest.remove(0),
                _ => Query::And(rest),
            }
        }
        Query::Not(inner) => {
            let inner = resolve_types(*inner);
            match inner {
                Query::Type { key, not, only } => Query::Type {
                    key,
                    not: !not,
                    only,
                },
                Query::And(_) | Query::Or(_) => resolve_types(negate(inner)),
                q => Query::Not(Box::new(q)),
            }
        }
        q => q,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WidthKey {
    pub context: String,
    pub min: bool,
    pub value: f64,
}
fn bounds(q: &Query) -> Option<Vec<(bool, f64)>> {
    match q {
        Query::Pair { key, value } => {
            let key = key.to_ascii_lowercase();
            let min = match key.as_str() {
                "min-width" => true,
                "max-width" => false,
                _ => return Some(Vec::new()),
            };
            let (number, unit) = dimension(value)?;
            let value = number.parse::<f64>().ok()?;
            if value == 0.0 || unit.eq_ignore_ascii_case("px") {
                Some(vec![(min, value)])
            } else {
                None
            }
        }
        Query::And(q) => {
            let mut out = Vec::new();
            for q in q {
                out.extend(bounds(q)?);
            }
            Some(out)
        }
        Query::Type { not: false, .. } => Some(Vec::new()),
        _ => None,
    }
}
pub(crate) fn width_chain_key(rule: &str) -> String {
    let mut remaining = rule;
    let mut preludes = Vec::new();
    while remaining.starts_with('@') {
        let Some(brace) = remaining.find('{') else {
            break;
        };
        preludes.push(remaining[..brace].trim_end());
        remaining = &remaining[brace + 1..];
    }
    preludes.join("{")
}

pub(crate) fn width_key(rule: &str) -> Option<WidthKey> {
    let mut remaining = rule;
    let mut preludes = Vec::new();
    while remaining.starts_with('@') {
        let brace = remaining.find('{')?;
        preludes.push(remaining[..brace].trim_end().to_owned());
        remaining = &remaining[brace + 1..];
    }
    let media: Vec<_> = preludes
        .iter()
        .enumerate()
        .filter(|(_, p)| p.starts_with("@media "))
        .map(|(i, _)| i)
        .collect();
    if media.len() != 1 {
        return None;
    }
    let i = media[0];
    // As in Babel, widths are read from the parsed, normalized query.
    let parsed = parse(&preludes[i]).ok()?;
    let found = bounds(&parsed)?;
    if found.len() != 1 {
        return None;
    }
    let (min, value) = found[0];
    preludes[i] = "@media".to_owned();
    Some(WidthKey {
        context: preludes.join("{"),
        min,
        value,
    })
}
