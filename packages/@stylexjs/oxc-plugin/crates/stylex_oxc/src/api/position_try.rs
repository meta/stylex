//! `positionTry` (top level; inline `positionTry` in `create` is evaluated in `eval`).

use crate::fx::IndexMap;
use oxc_ast::ast::*;

use crate::api::first_arg;
use crate::css::*;
use crate::diagnostics::{self as err, Res, bail};
use crate::eval::{Eval, Scope};
use crate::options::Opts;
use crate::transform::Cx;
use crate::value::{Obj, Val, js_string};
use stylex_utils::hash::create_hash;

pub fn stylex_position_try(style: &Obj, opts: &Opts) -> Res<(String, Injectable)> {
  let expanded = preprocess_flat(style, opts, true)?;
  let mut ltr_pairs: Vec<(String, String)> = vec![];
  let mut rtl_pairs: Vec<(String, String)> = vec![];
  for (k, v) in &expanded {
    let p = Pair::new(k.as_str(), v.as_str());
    let l = generate_ltr(&p, &opts.css);
    ltr_pairs.push((l.key.into_owned(), l.value.into_owned()));
    match generate_rtl(&p, &opts.css) {
      Some(r) => rtl_pairs.push((r.key.into_owned(), r.value.into_owned())),
      None => rtl_pairs.push((k.clone(), v.clone())),
    }
  }
  let build = |mut pairs: Vec<(String, String)>| -> String {
    let mut m: IndexMap<String, String> = IndexMap::default();
    for (k, v) in pairs.drain(..) {
      m.insert(k, v);
    }
    let mut keys: Vec<&String> = m.keys().collect();
    keys.sort();
    keys.iter().map(|k| format!("{k}:{};", m[*k])).collect()
  };
  let ltr_s = build(ltr_pairs);
  let rtl_s = build(rtl_pairs);
  let name = format!("--{}{}", opts.prefix, create_hash(&ltr_s));
  let ltr = format!("@position-try {name} {{{ltr_s}}}");
  let rtl = if ltr_s == rtl_s {
    None
  } else {
    Some(format!("@position-try {name} {{{rtl_s}}}"))
  };
  Ok((
    name,
    Injectable {
      ltr,
      rtl,
      priority: 0.0,
      ..Default::default()
    },
  ))
}

impl<'c> Cx<'c> {
  pub(crate) fn compile_position_try<'p, 'a>(
    &self,
    scope: &Scope<'p, 'a>,
    c: &CallExpression<'a>,
    api: &str,
  ) -> Res<(String, Vec<Rule>)> {
    let mut ev = Eval::new(scope, self.opts);
    let arg = first_arg(c, api)?;
    let style = match ev.eval(arg)? {
      // Babel's top-level positionTry only knows firstThatWorks, not inline keyframes.
      _ if !ev.side.is_empty() => bail(&err::non_static(api))?,
      Some(Val::Obj(v)) => v,
      Some(_) => bail(&err::non_style_object(api))?,
      None => bail(&err::non_static(api))?,
    };
    if style.keys().any(|k| !VALID_PROPERTIES.contains(&k.as_str())) {
      bail(POSITION_TRY_INVALID_PROPERTY)?;
    }
    let (n, inj) = stylex_position_try(&style, self.opts)?;
    Ok((n.clone(), vec![(n, inj)]))
  }

  pub(crate) fn position_try<'p, 'a>(
    &mut self,
    scope: &mut Scope<'p, 'a>,
    c: &CallExpression<'a>,
    name: String,
    stmt_start: u32,
    api: &str,
  ) -> Res<()> {
    let (n, rules) = self.compile_position_try(scope, c, api)?;
    self.register(rules, stmt_start);
    self.edits.replace(c.span, js_string(&n));
    scope.values.insert(name, Val::Str(n));
    Ok(())
  }
}

pub(crate) const POSITION_TRY_INVALID_PROPERTY: &str = "Invalid property in `positionTry()` call. It may only contain, positionAnchor, positionArea, inset properties (top, left, insetInline etc.), margin properties, size properties (height, inlineSize, etc.), and self-alignment properties (alignSelf, justifySelf, placeSelf)";

/// The properties a top-level `positionTry()` accepts.
const VALID_PROPERTIES: [&str; 40] = [
  "anchorName",
  "positionAnchor",
  "positionArea",
  "top",
  "right",
  "bottom",
  "left",
  "inset",
  "insetBlock",
  "insetBlockEnd",
  "insetBlockStart",
  "insetInline",
  "insetInlineEnd",
  "insetInlineStart",
  "margin",
  "marginBlock",
  "marginBlockEnd",
  "marginBlockStart",
  "marginInline",
  "marginInlineEnd",
  "marginInlineStart",
  "marginTop",
  "marginBottom",
  "marginLeft",
  "marginRight",
  "width",
  "height",
  "minWidth",
  "minHeight",
  "maxWidth",
  "maxHeight",
  "blockSize",
  "inlineSize",
  "minBlockSize",
  "minInlineSize",
  "maxBlockSize",
  "maxInlineSize",
  "alignSelf",
  "justifySelf",
  "placeSelf",
];
