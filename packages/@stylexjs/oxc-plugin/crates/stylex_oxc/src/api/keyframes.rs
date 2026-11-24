//! `keyframes` (top level; inline keyframes are evaluated in `eval`).

use crate::fx::IndexMap;
use oxc_ast::ast::*;

use crate::api::first_arg;
use crate::css::*;
use crate::diagnostics::{self as err, Res, bail};
use crate::eval::{Eval, Scope};
use crate::options::Opts;
use crate::transform::{Cx, Pass2};
use crate::value::{Obj, Val, js_string};
use stylex_utils::hash::create_hash;
use stylex_utils::string::dashify;

pub fn stylex_keyframes(frames: &Obj, opts: &Opts) -> Res<(String, Injectable)> {
  let mut expanded: IndexMap<String, Vec<(String, String)>> = IndexMap::default();
  for (frame_key, frame) in frames {
    let Val::Obj(frame) = frame else {
      bail("Every frame within a keyframes() call must be an object.")?
    };
    let mut pairs: IndexMap<String, String> = IndexMap::default();
    for (k, v) in frame.iter() {
      // Babel expands a null value to nulls (splitValue(null) is [null]) and then drops them.
      // The carved core expands an absent value to empty strings instead.
      if matches!(v, Val::Null | Val::Undefined) {
        // Babel still runs the expansion, so an unsupported shorthand warns or throws per
        // propertyValidationMode (fuzz 1q3lgjr, 1bnrzh3). The expanded nulls are dropped.
        expand(k, v, opts)?;
        continue;
      }
      for (prop, val) in expand(k, v, opts)? {
        if matches!(val, Val::Str(_) | Val::Num(_)) {
          let t = transform_value(&prop, &val, opts)?;
          pairs.insert(dashify(&prop).into_owned(), t);
        }
      }
    }
    expanded.insert(frame_key.clone(), pairs.into_iter().collect());
  }
  // Babel calls generateLtr without options for the hash, so it sees the default options.
  let mut stable_opts = opts.css.clone();
  stable_opts.core.enable_logical_styles_polyfill = false;
  stable_opts.core.enable_legacy_value_flipping = false;
  stable_opts.core.style_resolution =
    stylex_enums::style_resolution::StyleResolution::ApplicationOrder;
  let build = |f: &dyn Fn(&Pair) -> (String, String)| -> String {
    let mut s = String::new();
    for (k, frame) in &expanded {
      s.push_str(k);
      s.push('{');
      // Babel builds an object from the mapped entries, so keys that map to the same name
      // collapse: first position, last value (fuzz 1kmvx43).
      let mut mapped: IndexMap<String, String> = IndexMap::default();
      for (pk, pv) in frame {
        let (a, b) = f(&Pair::new(pk.as_str(), pv.as_str()));
        mapped.insert(a, b);
      }
      for (a, b) in &mapped {
        s.push_str(&format!("{a}:{b};"));
      }
      s.push('}');
    }
    s
  };
  let ltr_s = build(&|p| {
    let r = generate_ltr(p, &opts.css);
    (r.key.into_owned(), r.value.into_owned())
  });
  let rtl_s = build(&|p| match generate_rtl(p, &opts.css) {
    Some(r) => (r.key.into_owned(), r.value.into_owned()),
    None => (p.key.clone(), p.value.clone()),
  });
  let stable_s = build(&|p| {
    let r = generate_ltr(p, &stable_opts);
    (r.key.into_owned(), r.value.into_owned())
  });
  let name = format!("{}{}-B", opts.prefix, create_hash(&format!("<>{stable_s}")));
  let ltr = format!("@keyframes {name}{{{ltr_s}}}");
  let rtl = if ltr_s == rtl_s {
    None
  } else {
    Some(format!("@keyframes {name}{{{rtl_s}}}"))
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
  /// Compiles a `keyframes()` declaration: the animation name and the rules to register.
  pub(crate) fn compile_keyframes<'p, 'a>(
    &self,
    scope: &Scope<'p, 'a>,
    c: &CallExpression<'a>,
  ) -> Res<(String, Vec<Rule>)> {
    let mut ev = Eval::new(scope, self.opts);
    let arg = first_arg(c, "keyframes")?;
    // Babel only takes an object literal here (a local constant is rejected).
    if !matches!(arg, Expression::ObjectExpression(_)) {
      bail(&err::non_style_object("keyframes"))?;
    }
    let frames = match ev.eval(arg)? {
      // Babel's top-level keyframes knows firstThatWorks only.
      _ if !ev.side.is_empty() => bail(&err::non_static("keyframes"))?,
      Some(Val::Obj(v)) => v,
      Some(_) => bail(&err::non_style_object("keyframes"))?,
      // Babel ignores the deopt reason here and always reports a non-static value.
      None => bail(&err::non_static("keyframes"))?,
    };
    let (anim, inj) = stylex_keyframes(&frames, self.opts)?;
    Ok((anim.clone(), vec![(anim, inj)]))
  }

  pub(crate) fn keyframes<'p, 'a>(
    &mut self,
    scope: &mut Scope<'p, 'a>,
    c: &CallExpression<'a>,
    name: String,
    stmt_start: u32,
  ) -> Res<()> {
    let (anim, rules) = self.compile_keyframes(scope, c)?;
    self.register(rules, stmt_start);
    self.edits.replace(c.span, js_string(&anim));
    scope.values.insert(name, Val::Str(anim));
    Ok(())
  }
}

impl<'x, 'c, 's, 'p, 'a> Pass2<'x, 'c, 's, 'p, 'a> {
  /// `keyframes`, `positionTry` and `viewTransitionClass` bound to a variable below the top
  /// level. Babel compiles these declarators wherever they are.
  pub(crate) fn nested_named_api(&mut self, d: &VariableDeclarator<'a>) -> bool {
    let (BindingPattern::BindingIdentifier(_), Some(Expression::CallExpression(c))) =
      (&d.id, &d.init)
    else {
      return false;
    };
    if self.cx.handled.contains(&c.span.start) {
      return false;
    }
    let api = self.scope.imports.api(&c.callee);
    if let Some(
      api @ ("defineVars" | "defineConsts" | "defineMarker" | "unstable_defineVarsNested"
      | "unstable_defineConstsNested"),
    ) = api
    {
      // Bound below the top level, so never a named export.
      if self.error.is_none() {
        self.error = Some(crate::diagnostics::StxErr {
          msg: err::non_export(api),
          span: Some((c.span.start, c.span.end)),
        });
      }
      return true;
    }
    if let Some(api @ ("createTheme" | "unstable_createThemeNested")) = api {
      let BindingPattern::BindingIdentifier(id) = &d.id else {
        return false;
      };
      let r = if api == "createTheme" {
        self.cx.compile_create_theme(self.scope, c, id.name.as_str())
      } else {
        self.cx.compile_create_theme_nested(self.scope, c, id.name.as_str(), api)
      };
      match r {
        Ok((obj, rules)) => {
          self.cx.register(rules, self.stmt_start);
          let mut s = String::new();
          crate::value::to_js(&Val::Obj(obj.into()), &mut s);
          self.cx.edits.replace(c.span, s);
        },
        Err(e) => {
          if self.error.is_none() {
            self.error = Some(e);
          }
        },
      }
      return true;
    }
    let r = match api {
      Some("keyframes") => self.cx.compile_keyframes(self.scope, c),
      Some(api @ "positionTry") => self.cx.compile_position_try(self.scope, c, api),
      Some(api @ "viewTransitionClass") => self.cx.compile_view_transition_class(self.scope, c, api),
      _ => return false,
    };
    match r {
      Ok((name, rules)) => {
        self.cx.register(rules, self.stmt_start);
        self.cx.edits.replace(c.span, js_string(&name));
      },
      Err(e) => {
        if self.error.is_none() {
          self.error = Some(e);
        }
      },
    }
    true
  }
}
