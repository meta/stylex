//! `viewTransitionClass`.

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
use stylex_utils::string::dashify;

pub fn stylex_view_transition_class(styles: &Obj, opts: &Opts) -> Res<(String, Injectable)> {
  let mut strings: IndexMap<String, String> = IndexMap::default();
  for (k, style) in styles {
    // `Object.entries` of a string lists its characters; other primitives have no entries.
    let chars: Obj;
    let style = match style {
      Val::Obj(o) => o,
      Val::Str(s) => {
        chars = s
          .encode_utf16()
          .enumerate()
          .map(|(i, u)| (i.to_string(), Val::Str(String::from_utf16_lossy(&[u]))))
          .collect();
        &chars
      },
      Val::Num(_) | Val::Bool(_) => {
        chars = Obj::default();
        &chars
      },
      _ => bail("Cannot convert undefined or null to object")?,
    };
    let pre = preprocess_flat(style, opts, false)?;
    let s: String = pre.iter().map(|(k, v)| format!("{k}:{v};")).collect();
    strings.insert(format!("::view-transition-{}", dashify(k)), s);
  }
  let joined: String = strings.iter().map(|(k, v)| format!("{k}:{v};")).collect();
  let class = format!("{}{}", opts.prefix, create_hash(&joined));
  let ltr: String = strings
    .iter()
    .map(|(k, v)| format!("{k}(*.{class}){{{v}}}"))
    .collect();
  Ok((
    class,
    Injectable {
      ltr,
      rtl: None,
      priority: 1.0,
      ..Default::default()
    },
  ))
}

impl<'c> Cx<'c> {
  pub(crate) fn compile_view_transition_class<'p, 'a>(
    &self,
    scope: &Scope<'p, 'a>,
    c: &CallExpression<'a>,
    api: &str,
  ) -> Res<(String, Vec<Rule>)> {
    let mut ev = Eval::new(scope, self.opts);
    let arg = first_arg(c, api)?;
    let style = match ev.eval(arg)? {
      // Babel's viewTransitionClass knows keyframes and firstThatWorks, not positionTry.
      _ if ev.side.iter().any(|(n, _)| n.starts_with("--")) => bail(&err::non_static(api))?,
      Some(Val::Obj(v)) => v,
      Some(_) => bail(&err::non_style_object(api))?,
      None => bail(&err::non_static(api))?,
    };
    if style
      .keys()
      .any(|k| !matches!(k.as_str(), "group" | "imagePair" | "old" | "new"))
    {
      bail(INVALID_PROPERTY)?;
    }
    let mut rules = std::mem::take(&mut ev.side);
    let (n, inj) = stylex_view_transition_class(&style, self.opts)?;
    rules.push((n.clone(), inj));
    Ok((n, rules))
  }

  pub(crate) fn view_transition_class<'p, 'a>(
    &mut self,
    scope: &mut Scope<'p, 'a>,
    c: &CallExpression<'a>,
    name: String,
    stmt_start: u32,
    api: &str,
  ) -> Res<()> {
    let (n, rules) = self.compile_view_transition_class(scope, c, api)?;
    self.register(rules, stmt_start);
    self.edits.replace(c.span, js_string(&n));
    scope.values.insert(name, Val::Str(n));
    Ok(())
  }
}

const INVALID_PROPERTY: &str = "Invalid property in `viewTransitionClass()` call. It may only contain group, imagePair, old, and new properties";
