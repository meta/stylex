//! `defineConsts` and `unstable_defineConstsNested` (the nested call dispatches through
//! `api::vars::define_vars_nested`).

use oxc_ast::ast::*;

use crate::api::first_arg;
use crate::api::vars::{flatten_nested, unflatten};
use crate::css::*;
use crate::diagnostics::{self as err, Res, bail};
use crate::eval::{Eval, Scope};
use crate::options::Opts;
use crate::transform::Cx;
use crate::value::{Obj, Val};
use stylex_utils::hash::create_hash;

pub fn stylex_define_consts(consts: &Obj, export_id: &str, opts: &Opts) -> (Obj, Vec<Rule>) {
  let mut rules = vec![];
  for (key, value) in consts {
    let const_key = match key.strip_prefix("--") {
      Some(k) => k.to_string(),
      None => format!(
        "{}{}",
        opts.prefix,
        create_hash(&format!("{export_id}.{key}"))
      ),
    };
    rules.push((
      const_key.clone(),
      Injectable {
        ltr: String::new(),
        rtl: None,
        priority: 0.0,
        const_key: Some(const_key),
        const_val: Some(value.clone()),
      },
    ));
  }
  (consts.clone(), rules)
}

pub fn stylex_define_consts_nested(
  consts: &Obj,
  export_id: &str,
  opts: &Opts,
) -> Res<(Obj, Vec<Rule>)> {
  let mut flat = Obj::default();
  flatten_nested(
    consts,
    "",
    &mut flat,
    &|v| matches!(v, Val::Str(_) | Val::Num(_)),
    &|v| v.clone(),
  )?;
  let (out, rules) = stylex_define_consts(&flat, export_id, opts);
  Ok((unflatten(&out), rules))
}

/// Babel evaluates defineConsts with `disableImports` and only env functions, so a read of an
/// imported binding, or any inline keyframes or positionTry, is not static.
pub(crate) fn uses_import_or_side<'a>(scope: &Scope<'_, 'a>, e: &Expression<'a>) -> bool {
  struct V<'s, 'p, 'a> {
    scope: &'s Scope<'p, 'a>,
    found: bool,
    seen: crate::fx::HashSet<String>,
  }
  impl<'s, 'p, 'a> oxc_ast_visit::Visit<'a> for V<'s, 'p, 'a> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
      let n = it.name.as_str();
      if self.scope.imports.vars.contains_key(n) || self.scope.imports.files.contains_key(n) {
        self.found = true;
      } else if !self.scope.values.contains_key(n)
        && let Some(init) = self.scope.consts.get(n).copied()
        && self.seen.insert(n.to_string())
      {
        // A local constant is evaluated through, so look through it too.
        self.visit_expression(init);
      }
    }
    fn visit_call_expression(&mut self, c: &CallExpression<'a>) {
      if matches!(
        self.scope.imports.api(&c.callee),
        Some("keyframes" | "positionTry" | "firstThatWorks" | "defaultMarker")
      ) {
        self.found = true;
      }
      oxc_ast_visit::walk::walk_call_expression(self, c);
    }
  }
  let mut v = V {
    scope,
    found: false,
    seen: Default::default(),
  };
  oxc_ast_visit::Visit::visit_expression(&mut v, e);
  v.found
}

impl<'c> Cx<'c> {
  pub(crate) fn define_consts<'p, 'a>(
    &mut self,
    scope: &mut Scope<'p, 'a>,
    c: &CallExpression<'a>,
    name: String,
    stmt_start: u32,
    file_hash_name: Option<&String>,
  ) -> Res<()> {
    let arg = first_arg(c, "defineConsts")?;
    let mut ev = Eval::for_api(&scope, self.opts, "defineConsts");
    let consts = match ev.eval(arg)? {
      _ if uses_import_or_side(scope, arg) => bail(&err::non_static("defineConsts"))?,
      Some(Val::Obj(v)) => v,
      Some(_) => bail(&err::non_style_object("defineConsts"))?,
      None => bail(&err::non_static("defineConsts"))?,
    };
    // Babel evaluates before it checks the file name.
    let Some(file) = file_hash_name else {
      bail(
        "Unable to generate hash for defineConsts(). Check that the file has a valid extension and that unstable_moduleResolution is configured.",
      )?
    };
    let (obj, rules) = stylex_define_consts(&consts, &format!("{file}//{name}"), self.opts);
    self.register(rules, stmt_start);
    let mut s = String::new();
    crate::api::create::convert_object_to_ast(&Val::Obj(obj.clone().into()), &mut s);
    self.edits.replace(c.span, s);
    scope.values.insert(name, Val::Obj(obj.into()));
    Ok(())
  }
}
