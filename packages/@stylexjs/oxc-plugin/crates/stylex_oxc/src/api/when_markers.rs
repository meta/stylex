//! `defineMarker`, `defaultMarker` and the `when.*` selectors.

use oxc_ast::ast::*;

use crate::diagnostics::{self as err, Res, bail};
use crate::eval::{Eval, Scope};
use crate::transform::{Cx, Pass2};
use crate::value::{Obj, Val, to_js};
use stylex_types::traits::WhenMarkerValue;

/// A `when.*` marker argument.
pub struct Marker {
  literal: Option<String>,
  proxy: Option<String>,
  first_key: Option<String>,
}

impl Marker {
  pub(crate) fn from_val(v: &Val, ev: &Eval) -> Marker {
    match v {
      Val::Str(s) => Marker {
        literal: Some(s.clone()),
        proxy: None,
        first_key: None,
      },
      Val::VarsProxy(..) => Marker {
        literal: None,
        proxy: match ev.get_member(v, "__varGroupHash__") {
          Some(Val::Str(s)) => Some(s),
          _ => None,
        },
        first_key: None,
      },
      Val::Obj(o) => Marker {
        literal: None,
        proxy: None,
        first_key: o.keys().find(|k| *k != "$$css").cloned(),
      },
      _ => Marker {
        literal: None,
        proxy: None,
        first_key: None,
      },
    }
  }
}

impl WhenMarkerValue for Marker {
  fn as_str_value(&self) -> Option<&str> {
    self.literal.as_deref()
  }
  fn is_proxy(&self) -> bool {
    self.proxy.is_some()
  }
  fn as_proxy_string(&self) -> Option<String> {
    self.proxy.clone()
  }
  fn first_css_key(&self) -> Option<&str> {
    self.first_key.as_deref()
  }
  fn class_name_prefix(&self) -> Option<&str> {
    None
  }
}

pub(crate) fn when_call(
  f: &str,
  pseudo: &str,
  m: Option<&dyn WhenMarkerValue>,
) -> Result<String, String> {
  use stylex_css::utils::when;
  match f {
    "ancestor" => when::ancestor(pseudo, m),
    "descendant" => when::descendant(pseudo, m),
    "siblingBefore" => when::sibling_before(pseudo, m),
    "siblingAfter" => when::sibling_after(pseudo, m),
    "anySibling" => when::any_sibling(pseudo, m),
    _ => Err(format!("Unknown when function: {f}")),
  }
}

impl<'c> Cx<'c> {
  pub(crate) fn define_marker<'p, 'a>(
    &mut self,
    scope: &mut Scope<'p, 'a>,
    c: &CallExpression<'a>,
    name: String,
    _stmt_start: u32,
    file_hash_name: Option<&String>,
  ) -> Res<()> {
    if !c.arguments.is_empty() {
      bail(&err::arg_len("defineMarker", 0))?;
    }
    let Some(file) = file_hash_name else {
      bail(
        "Unable to generate hash for defineMarker(). Check that the file has a valid extension and that unstable_moduleResolution is configured.",
      )?
    };
    let id = format!(
      "{}{}",
      self.opts.prefix,
      stylex_utils::hash::create_hash(&format!("{file}//{name}"))
    );
    let mut o = Obj::default();
    o.insert(id.clone(), Val::Str(id));
    o.insert("$$css".into(), Val::Bool(true));
    let mut s = String::new();
    to_js(&Val::Obj(o.clone().into()), &mut s);
    self.edits.replace(c.span, s);
    scope.values.insert(name, Val::Obj(o.into()));
    Ok(())
  }
}

impl<'x, 'c, 's, 'p, 'a> Pass2<'x, 'c, 's, 'p, 'a> {
  /// `stylex.defaultMarker()` inlined. Returns true when handled.
  pub(crate) fn default_marker(&mut self, c: &CallExpression<'a>) -> bool {
    if self.scope.imports.api(&c.callee) == Some("defaultMarker") {
      let mut ev = Eval::new(self.scope, self.cx.opts);
      if let Ok(Some(v)) = ev.eval_call(c) {
        let mut t = String::new();
        to_js(&v, &mut t);
        self.cx.edits.replace(c.span, t);
        return true;
      }
    }
    false
  }
}
