//! `defineVars`, `createTheme` and the nested variants. `stylex.types` lands here too.

use crate::fx::IndexMap;
use oxc_ast::ast::*;

use crate::api::first_arg;
use crate::css::*;
use crate::diagnostics::{self as err, Res, bail};
use crate::eval::{Eval, Scope};
use crate::options::Opts;
use crate::transform::Cx;
use crate::value::{Obj, Val, to_js};
use stylex_utils::hash::create_hash;

pub(crate) const SPLIT: &str = "__$$__";

pub(crate) fn collect_vars_by_at_rule(
  key: &str,
  name_hash: &str,
  value: &Val,
  coll: &mut IndexMap<String, Vec<String>>,
  at_rules: &[String],
) -> Res<()> {
  match value {
    Val::Str(_) | Val::Num(_) => {
      let v = value.to_js_string().unwrap();
      let k = if at_rules.is_empty() {
        "default".to_string()
      } else {
        let mut s = at_rules.to_vec();
        s.sort();
        s.join(SPLIT)
      };
      coll
        .entry(k)
        .or_default()
        .push(format!("--{name_hash}:{v};"));
    },
    Val::Null | Val::Undefined => {},
    Val::Arr(_) => bail("Array is not supported in defineVars")?,
    Val::Obj(o) => {
      if !o.contains_key("default") {
        bail(&format!("Default value is not defined for {key} variable."))?;
      }
      for (at, v) in o.iter() {
        let mut next = at_rules.to_vec();
        if at != "default" {
          next.push(at.clone());
        }
        collect_vars_by_at_rule(key, name_hash, v, coll, &next)?;
      }
    },
    _ => bail("Invalid value in defineVars")?,
  }
  Ok(())
}

pub(crate) fn wrap_with_at_rules(ltr: &str, at_rule: &str) -> String {
  at_rule
    .split(SPLIT)
    .fold(ltr.to_string(), |acc, a| format!("{a}{{{acc}}}"))
}

pub(crate) fn priority_for_at_rule(at_rule: &str) -> f64 {
  if at_rule == "default" {
    1.0
  } else {
    1.0 + at_rule.split(SPLIT).count() as f64
  }
}

pub fn var_name_hash(prefix: &str, export_id: &str, key: &str) -> String {
  if let Some(stripped) = key.strip_prefix("--") {
    stripped.to_string()
  } else {
    format!("{prefix}{}", create_hash(&format!("{export_id}.{key}")))
  }
}

/// Returns the vars object and the injected rules.
pub fn stylex_define_vars(vars: &Obj, export_id: &str, opts: &Opts) -> Res<(Obj, Vec<Rule>)> {
  let group = format!("{}{}", opts.prefix, create_hash(export_id));
  let mut out = Obj::default();
  let mut by_at: IndexMap<String, Vec<String>> = IndexMap::default();
  let mut rules = vec![];
  for (key, value) in vars {
    let nh = var_name_hash(&opts.prefix, export_id, key);
    out.insert(key.clone(), Val::Str(format!("var(--{nh})")));
    let value = match as_typed(value) {
      Some((syntax, v)) => {
        let iv = default_value(v)?;
        let ltr = format!(
          "@property --{nh} {{ syntax: \"{syntax}\"; inherits: true;{} }}",
          iv.map(|iv| format!(" initial-value: {iv}"))
            .unwrap_or_default()
        );
        rules.push((
          nh.clone(),
          Injectable {
            ltr,
            rtl: None,
            priority: 0.0,
            ..Default::default()
          },
        ));
        v
      },
      None => value,
    };
    collect_vars_by_at_rule(key, &nh, value, &mut by_at, &[])?;
  }
  out.insert("__varGroupHash__".into(), Val::Str(group.clone()));
  for (at, decls) in by_at {
    let suffix = if at == "default" {
      String::new()
    } else {
      format!("-{}", create_hash(&at))
    };
    let mut ltr = format!(":root, .{group}{{{}}}", decls.join(""));
    if at != "default" {
      ltr = wrap_with_at_rules(&ltr, &at);
    }
    rules.push((
      format!("{group}{suffix}"),
      Injectable {
        ltr,
        rtl: None,
        priority: priority_for_at_rule(&at) / 10.0,
        ..Default::default()
      },
    ));
  }
  Ok((out, rules))
}

pub fn stylex_create_theme(
  theme_vars: &Obj,
  overrides: &Obj,
  opts: &Opts,
) -> Res<(Obj, Vec<Rule>)> {
  let Some(Val::Str(group)) = theme_vars.get("__varGroupHash__") else {
    bail("Can only override variables theme created with defineVars().")?
  };
  let mut keys: Vec<&String> = overrides.keys().collect();
  keys.sort();
  let mut by_at: IndexMap<String, Vec<String>> = IndexMap::default();
  for key in keys {
    let Some(Val::Str(var_ref)) = theme_vars.get(key) else {
      // Babel reads `themeVars[key].slice(6, -1)` and crashes on a key the theme lacks.
      bail("Cannot read properties of undefined (reading 'slice')")?
    };
    let nh = if var_ref.len() > 7 {
      &var_ref[6..var_ref.len() - 1]
    } else {
      var_ref.as_str()
    };
    let v = &overrides[key];
    let v = as_typed(v).map(|(_, x)| x).unwrap_or(v);
    collect_vars_by_at_rule(key, nh, v, &mut by_at, &[])?;
  }
  let mut ats: Vec<String> = by_at.keys().cloned().collect();
  ats.sort_by(|a, b| {
    if a == "default" {
      std::cmp::Ordering::Less
    } else if b == "default" {
      std::cmp::Ordering::Greater
    } else {
      a.to_lowercase().cmp(&b.to_lowercase()).then(a.cmp(b))
    }
  });
  let for_hash: String = ats
    .iter()
    .map(|a| wrap_with_at_rules(&by_at[a].join(""), a))
    .collect();
  let override_class = format!("{}{}", opts.prefix, create_hash(&for_hash));
  let mut rules = vec![];
  for at in &ats {
    let decls = by_at[at].join("");
    let rule = format!(".{override_class}, .{override_class}:root{{{decls}}}");
    let priority = 0.4 + priority_for_at_rule(at) / 10.0;
    let suffix = if at == "default" {
      String::new()
    } else {
      format!("-{}", create_hash(at))
    };
    let ltr = if at == "default" {
      rule
    } else {
      wrap_with_at_rules(&rule, at)
    };
    rules.push((
      format!("{override_class}{suffix}"),
      Injectable {
        ltr,
        rtl: None,
        priority,
        ..Default::default()
      },
    ));
  }
  let mut out = Obj::default();
  out.insert(group.clone(), Val::Str(format!("{override_class} {group}")));
  out.insert("$$css".into(), Val::Bool(true));
  Ok((out, rules))
}

/// Babel's `test` and `dev` theme objects: a `<basename>__<name>` class first.
pub(crate) fn dev_theme_class(obj: Obj, name: &str, opts: &Opts) -> Obj {
  let file = opts.filename.as_deref().unwrap_or("UnknownFile");
  let base = std::path::Path::new(file)
    .file_name()
    .map(|f| f.to_string_lossy().into_owned())
    .unwrap_or_default();
  let base = base.split('.').next().unwrap_or("").to_string();
  let dev = format!("{base}__{name}");
  let mut out = Obj::default();
  out.insert(dev.clone(), Val::Str(dev));
  if opts.css.core.test {
    out.insert("$$css".into(), Val::Bool(true));
  } else {
    out.extend(obj);
  }
  out
}

pub(crate) fn is_vars_leaf(v: &Val) -> bool {
  if as_typed(v).is_some() {
    return true;
  }
  match v {
    Val::Str(_) => true,
    Val::Obj(o) => {
      o.contains_key("default") && o.keys().all(|k| k == "default" || k.starts_with('@'))
    },
    _ => false,
  }
}

pub(crate) fn to_vars_config_value(v: &Val) -> Val {
  if as_typed(v).is_some() {
    return v.clone();
  }
  match v {
    Val::Str(_) => v.clone(),
    Val::Obj(o) => {
      let mut r = Obj::default();
      r.insert("default".into(), Val::Str(String::new()));
      for (k, x) in o.iter() {
        r.insert(k.clone(), to_vars_config_value(x));
      }
      Val::Obj(r.into())
    },
    Val::Null | Val::Undefined => Val::Str(String::new()),
    v => Val::Str(v.to_js_string().unwrap_or_default()),
  }
}

pub fn flatten_nested(
  obj: &Obj,
  prefix: &str,
  out: &mut Obj,
  is_leaf: &dyn Fn(&Val) -> bool,
  leaf: &dyn Fn(&Val) -> Val,
) -> Res<()> {
  for (k, v) in obj {
    if k.contains('.') {
      bail(&format!(
        "Key \"{k}\" must not contain the \".\" character. Use nested objects instead of dots in key names. See: https://www.designtokens.org/tr/drafts/format/#character-restrictions"
      ))?;
    }
    let full = if prefix.is_empty() {
      k.clone()
    } else {
      format!("{prefix}.{k}")
    };
    if is_leaf(v) {
      out.insert(full, leaf(v));
    } else if let Val::Obj(inner) = v {
      flatten_nested(inner, &full, out, is_leaf, leaf)?;
    }
  }
  Ok(())
}

pub fn unflatten(flat: &Obj) -> Obj {
  let mut result = Obj::default();
  for (k, v) in flat {
    if k == "__varGroupHash__" || k == "$$css" || !k.contains('.') {
      result.insert(k.clone(), v.clone());
      continue;
    }
    let parts: Vec<&str> = k.split('.').collect();
    let mut cur = &mut result;
    for p in &parts[..parts.len() - 1] {
      let e = cur
        .entry(p.to_string())
        .or_insert_with(|| Val::Obj(Obj::default().into()));
      if !matches!(e, Val::Obj(_)) {
        *e = Val::Obj(Obj::default().into());
      }
      let Val::Obj(next) = e else { unreachable!() };
      let next = std::rc::Rc::make_mut(next);
      cur = next;
    }
    cur.insert(parts[parts.len() - 1].to_string(), v.clone());
  }
  result
}

pub fn stylex_define_vars_nested(
  vars: &Obj,
  export_id: &str,
  opts: &Opts,
) -> Res<(Obj, Vec<Rule>)> {
  let mut flat = Obj::default();
  flatten_nested(vars, "", &mut flat, &is_vars_leaf, &to_vars_config_value)?;
  let (mut out, rules) = stylex_define_vars(&flat, export_id, opts)?;
  let group = out.shift_remove("__varGroupHash__").unwrap_or(Val::Null);
  let mut nested = unflatten(&out);
  nested.insert("__varGroupHash__".into(), group);
  Ok((nested, rules))
}

pub fn stylex_create_theme_nested(
  theme: &Obj,
  overrides: &Obj,
  opts: &Opts,
) -> Res<(Obj, Vec<Rule>)> {
  let mut refs = theme.clone();
  let group = refs.shift_remove("__varGroupHash__");
  let mut flat_theme = Obj::default();
  flatten_nested(
    &refs,
    "",
    &mut flat_theme,
    &|v| matches!(v, Val::Str(_)),
    &|v| v.clone(),
  )?;
  if let Some(g) = group {
    flat_theme.insert("__varGroupHash__".into(), g);
  }
  let mut flat_overrides = Obj::default();
  flatten_nested(
    overrides,
    "",
    &mut flat_overrides,
    &is_vars_leaf,
    &|v| match as_typed(v) {
      Some((_, x)) => to_vars_config_value(x),
      None => to_vars_config_value(v),
    },
  )?;
  stylex_create_theme(&flat_theme, &flat_overrides, opts)
}

impl<'c> Cx<'c> {
  pub(crate) fn define_vars<'p, 'a>(
    &mut self,
    scope: &mut Scope<'p, 'a>,
    c: &CallExpression<'a>,
    name: String,
    stmt_start: u32,
    file_hash_name: Option<&String>,
  ) -> Res<()> {
    let Some(file) = file_hash_name else {
      bail(
        "Unable to generate hash for defineVars(). Check that the file has a valid extension and that unstable_moduleResolution is configured.",
      )?
    };
    let arg = first_arg(c, "defineVars")?;
    let mut ev = Eval::new(&scope, self.opts);
    let pre = match pre_eval(&mut ev, arg)? {
      Some(Pre::O(m)) => m,
      Some(Pre::V(Val::Obj(o))) => std::rc::Rc::unwrap_or_clone(o).into_iter().map(|(k, v)| (k, Pre::V(v))).collect(),
      Some(_) => bail(&err::non_style_object("defineVars"))?,
      None => bail(&err::non_static("defineVars"))?,
    };
    let side = std::mem::take(&mut ev.side);
    drop(ev);
    // Function values see the group itself, as Babel's self-reference proxy.
    let prev = scope.values.insert(
      name.clone(),
      Val::VarsProxy(file.clone(), name.clone()),
    );
    let mut norm = Normalizer {
      scope: &*scope,
      opts: self.opts,
      self_name: name.clone(),
      side,
    };
    let mut vars = Obj::default();
    let mut deps: IndexMap<String, crate::fx::IndexSet<String>> = IndexMap::default();
    let mut result = Ok(());
    for (k, p) in pre {
      let mut d = crate::fx::IndexSet::default();
      match norm.normalize(p, &k, &mut d, true) {
        Ok(v) => {
          vars.insert(k.clone(), v);
        },
        Err(e) => {
          result = Err(e);
          break;
        },
      }
      deps.insert(k, d);
    }
    let side = std::mem::take(&mut norm.side);
    match prev {
      Some(v) => scope.values.insert(name.clone(), v),
      None => scope.values.remove(&name),
    };
    result?;
    for (k, ds) in &deps {
      for d in ds {
        if d != "__varGroupHash__" && !vars.contains_key(d) {
          bail(&format!(
            "Unknown same-group reference \"{d}\" found while resolving \"{k}\" in defineVars()."
          ))?;
        }
      }
    }
    for ds in deps.values_mut() {
      ds.shift_remove("__varGroupHash__");
    }
    assert_no_cycles(&deps)?;
    let (obj, mut rules) = stylex_define_vars(&vars, &format!("{file}//{name}"), self.opts)?;
    let mut all = side;
    all.append(&mut rules);
    self.register(all, stmt_start);
    let mut s = String::new();
    to_js(&Val::Obj(obj.clone().into()), &mut s);
    self.edits.replace(c.span, s);
    scope.values.insert(name, Val::Obj(obj.into()));
    Ok(())
  }

  /// Compiles a `createTheme()` call bound to `name`: the theme object and its rules.
  pub(crate) fn compile_create_theme<'p, 'a>(
    &self,
    scope: &Scope<'p, 'a>,
    c: &CallExpression<'a>,
    name: &str,
  ) -> Res<(Obj, Vec<Rule>)> {
    let mut ev = Eval::new(scope, self.opts);
    if c.arguments.len() != 2 {
      bail(&err::arg_len("createTheme", 2))?;
    }
    let Some(theme) = ev.eval(c.arguments[0].as_expression().unwrap())? else {
      bail(&err::non_static("createTheme"))?
    };
    let overrides = match eval_typed(&mut ev, c.arguments[1].as_expression().unwrap())? {
      Some(Val::Obj(o)) => o,
      Some(_) => bail(&err::non_style_object("createTheme"))?,
      None => bail(&err::non_static("createTheme"))?,
    };
    let theme_obj = match theme {
      Val::Obj(o) => std::rc::Rc::unwrap_or_clone(o),
      p @ Val::VarsProxy(..) => {
        let mut o = Obj::default();
        for k in overrides
          .keys()
          .chain(std::iter::once(&"__varGroupHash__".to_string()))
        {
          if let Some(v) = ev.get_member(&p, k) {
            o.insert(k.clone(), v);
          }
        }
        o
      },
      _ => bail("Can only override variables theme created with defineVars().")?,
    };
    let side = std::mem::take(&mut ev.side);
    let (mut obj, mut rules) = stylex_create_theme(&theme_obj, &overrides, self.opts)?;
    if self.opts.css.core.test || self.opts.dev {
      obj = dev_theme_class(obj, name, self.opts);
    }
    let mut all = side;
    all.append(&mut rules);
    Ok((obj, all))
  }

  pub(crate) fn create_theme<'p, 'a>(
    &mut self,
    scope: &mut Scope<'p, 'a>,
    c: &CallExpression<'a>,
    name: String,
    stmt_start: u32,
  ) -> Res<()> {
    let (obj, rules) = self.compile_create_theme(scope, c, &name)?;
    self.register(rules, stmt_start);
    let mut s = String::new();
    to_js(&Val::Obj(obj.clone().into()), &mut s);
    self.edits.replace(c.span, s);
    scope.values.insert(name, Val::Obj(obj.into()));
    Ok(())
  }

  pub(crate) fn define_vars_nested<'p, 'a>(
    &mut self,
    scope: &mut Scope<'p, 'a>,
    c: &CallExpression<'a>,
    name: String,
    stmt_start: u32,
    file_hash_name: Option<&String>,
    api: &str,
  ) -> Res<()> {
    let mut ev = Eval::new(&scope, self.opts);
    let arg = first_arg(c, &api)?;
    let consts = api != "unstable_defineVarsNested";
    let v = if consts {
      ev.eval(arg)?
    } else {
      eval_typed(&mut ev, arg)?
    };
    let v = match v {
      _ if consts && crate::api::consts::uses_import_or_side(scope, arg) => {
        bail(&err::non_static(&api))?
      },
      Some(Val::Obj(v)) => v,
      Some(_) => bail(&err::non_style_object(&api))?,
      None => bail(&err::non_static(&api))?,
    };
    // Babel evaluates before it checks the file name.
    let Some(file) = file_hash_name else {
      bail(&format!(
        "Unable to generate hash for {api}(). Check that the file has a valid extension and that unstable_moduleResolution is configured."
      ))?
    };
    let mut rules = std::mem::take(&mut ev.side);
    let export_id = format!("{file}//{name}");
    let (obj, more) = if api == "unstable_defineVarsNested" {
      stylex_define_vars_nested(&v, &export_id, self.opts)?
    } else {
      crate::api::consts::stylex_define_consts_nested(&v, &export_id, self.opts)?
    };
    rules.extend(more);
    self.register(rules, stmt_start);
    let mut s = String::new();
    to_js(&Val::Obj(obj.clone().into()), &mut s);
    self.edits.replace(c.span, s);
    scope.values.insert(name, Val::Obj(obj.into()));
    Ok(())
  }

  pub(crate) fn compile_create_theme_nested<'p, 'a>(
    &self,
    scope: &Scope<'p, 'a>,
    c: &CallExpression<'a>,
    name: &str,
    api: &str,
  ) -> Res<(Obj, Vec<Rule>)> {
    if c.arguments.len() != 2 {
      bail(&err::arg_len(api, 2))?;
    }
    let mut ev = Eval::new(scope, self.opts);
    let theme = match ev.eval(c.arguments[0].as_expression().unwrap())? {
      Some(Val::Obj(o)) => o,
      _ => bail(&err::non_static(api))?,
    };
    if !matches!(theme.get("__varGroupHash__"), Some(Val::Str(s)) if !s.is_empty()) {
      bail("Can only override variables theme created with unstable_defineVarsNested().")?;
    }
    let overrides = match eval_typed(&mut ev, c.arguments[1].as_expression().unwrap())? {
      Some(Val::Obj(o)) => o,
      Some(_) => bail(&err::non_style_object(api))?,
      None => bail(&err::non_static(api))?,
    };
    let mut rules = std::mem::take(&mut ev.side);
    let (mut obj, more) = stylex_create_theme_nested(&theme, &overrides, self.opts)?;
    if self.opts.css.core.test || self.opts.dev {
      obj = dev_theme_class(obj, name, self.opts);
    }
    rules.extend(more);
    Ok((obj, rules))
  }

  pub(crate) fn create_theme_nested<'p, 'a>(
    &mut self,
    scope: &mut Scope<'p, 'a>,
    c: &CallExpression<'a>,
    name: String,
    stmt_start: u32,
    api: &str,
  ) -> Res<()> {
    let (obj, rules) = self.compile_create_theme_nested(scope, c, &name, api)?;
    self.register(rules, stmt_start);
    let mut s = String::new();
    to_js(&Val::Obj(obj.clone().into()), &mut s);
    self.edits.replace(c.span, s);
    scope.values.insert(name, Val::Obj(obj.into()));
    Ok(())
  }
}

// ---- `stylex.types` values and defineVars function values ----

/// Marks a `stylex.types.*` value. The value model has no CSS type variant, so a typed value is
/// an object with this key holding the syntax and `value` holding the converted value.
pub(crate) const TYPE_KEY: &str = "\u{0}stylex.cssType";

pub(crate) fn typed(syntax: &str, value: Val) -> Val {
  let mut o = Obj::default();
  o.insert(TYPE_KEY.into(), Val::Str(syntax.into()));
  o.insert("value".into(), value);
  Val::Obj(o.into())
}

/// `(syntax, value)` when `v` is a `stylex.types.*` value (Babel's `isCSSType`).
pub(crate) fn as_typed(v: &Val) -> Option<(&str, &Val)> {
  let Val::Obj(o) = v else { return None };
  match (o.get(TYPE_KEY), o.get("value")) {
    (Some(Val::Str(s)), Some(val)) if !matches!(val, Val::Null | Val::Undefined) => {
      Some((s.as_str(), val))
    },
    _ => None,
  }
}

fn convert_numbers(v: &Val, f: &dyn Fn(f64) -> String) -> Val {
  match v {
    Val::Num(n) => Val::Str(f(*n)),
    Val::Obj(o) => Val::Obj(
      o.iter()
        .map(|(k, x)| (k.clone(), convert_numbers(x, f)))
        .collect::<Obj>().into(),
    ),
    v => v.clone(),
  }
}

/// `stylex.types.<name>(value)`.
pub(crate) fn make_type(name: &str, value: &Val) -> Option<Val> {
  let num = |n: f64| stylex_utils::number::to_js_string(n);
  let (syntax, v) = match name {
    "angle" => ("<angle>", value.clone()),
    "color" => ("<color>", value.clone()),
    "url" => ("<url>", value.clone()),
    "image" => ("<image>", value.clone()),
    "resolution" => ("<resolution>", value.clone()),
    "time" => ("<time>", value.clone()),
    "transformFunction" => ("<transform-function>", value.clone()),
    "transformList" => ("<transform-list>", value.clone()),
    "integer" => ("<integer>", convert_numbers(value, &num)),
    "number" => ("<number>", convert_numbers(value, &num)),
    "length" | "lengthPercentage" => (
      if name == "length" {
        "<length>"
      } else {
        "<length-percentage>"
      },
      convert_numbers(value, &|n| {
        if n == 0.0 {
          "0".into()
        } else {
          format!("{}px", num(n))
        }
      }),
    ),
    "percentage" => (
      "<percentage>",
      convert_numbers(value, &|n| {
        if n == 0.0 {
          "0".into()
        } else {
          format!("{}%", num(n * 100.0))
        }
      }),
    ),
    _ => return None,
  };
  Some(typed(syntax, v))
}

/// Babel's `getDefaultValue` for a typed var's `initial-value`.
fn default_value(v: &Val) -> Res<Option<String>> {
  match v {
    Val::Str(_) | Val::Num(_) => Ok(v.to_js_string()),
    Val::Null | Val::Undefined => Ok(None),
    Val::Arr(_) => bail("Array is not supported in defineVars"),
    Val::Obj(o) => match o.get("default") {
      None | Some(Val::Undefined) => bail("Default value is not defined for variable."),
      Some(d) => default_value(d),
    },
    _ => bail("Invalid value in defineVars"),
  }
}

/// The type name a callee names: `stylex.types.color` or `types.color` (named `types` import).
fn types_callee<'b>(callee: &'b Expression, scope: &Scope) -> Option<&'b str> {
  let Expression::StaticMemberExpression(m) = callee.without_parentheses() else {
    return None;
  };
  let is_types = match m.object.without_parentheses() {
    Expression::StaticMemberExpression(inner) => {
      matches!(&inner.object, Expression::Identifier(o) if scope.imports.ns.contains(o.name.as_str()))
        && inner.property.name == "types"
    },
    Expression::Identifier(o) => scope
      .imports
      .named
      .get(o.name.as_str())
      .is_some_and(|n| n == "types"),
    _ => false,
  };
  is_types.then(|| m.property.name.as_str())
}

/// A defineVars argument evaluated up to its function values (Babel evaluates the object first
/// and calls the functions while normalizing).
enum Pre<'b, 'a> {
  V(Val),
  Fn(&'b ArrowFunctionExpression<'a>),
  O(IndexMap<String, Pre<'b, 'a>>),
}

fn strip<'b, 'a>(mut e: &'b Expression<'a>) -> &'b Expression<'a> {
  loop {
    e = match e {
      Expression::ParenthesizedExpression(p) => &p.expression,
      Expression::TSAsExpression(p) => &p.expression,
      Expression::TSSatisfiesExpression(p) => &p.expression,
      Expression::TSNonNullExpression(p) => &p.expression,
      _ => return e,
    };
  }
}

/// Evaluates `e` like `Eval::eval`, but keeps arrow functions and understands `stylex.types`.
fn pre_eval<'b, 'p: 'b, 'a>(
  ev: &mut Eval<'_, 'p, 'a>,
  e: &'b Expression<'a>,
) -> Res<Option<Pre<'b, 'a>>> {
  pre_eval_at(ev, e, 0)
}

fn pre_eval_at<'b, 'p: 'b, 'a>(
  ev: &mut Eval<'_, 'p, 'a>,
  e: &'b Expression<'a>,
  depth: u32,
) -> Res<Option<Pre<'b, 'a>>> {
  let e = strip(e);
  match e {
    Expression::ArrowFunctionExpression(f) => {
      let simple = f
        .params
        .items
        .iter()
        .all(|p| matches!(p.pattern, BindingPattern::BindingIdentifier(_)) && p.initializer.is_none())
        && f.params.rest.is_none();
      if f.get_expression().is_some() && simple {
        return Ok(Some(Pre::Fn(f)));
      }
      Ok(None)
    },
    // A top-level constant holding an object or a function: evaluate its initializer the same way.
    Expression::Identifier(id)
      if !ev.scope.values.contains_key(id.name.as_str())
        && let Some(init) = ev.scope.consts.get(id.name.as_str()).copied()
        && matches!(
          strip(init),
          Expression::ObjectExpression(_) | Expression::ArrowFunctionExpression(_)
        ) =>
    {
      if depth > 32 {
        return Ok(None);
      }
      pre_eval_at(ev, init, depth + 1)
    },
    Expression::CallExpression(c) => {
      // Babel's defineVars and createTheme only know keyframes, positionTry, types,
      // unstable_conditional and env functions.
      if matches!(
        ev.scope.imports.api(&c.callee),
        Some("firstThatWorks" | "defaultMarker")
      ) {
        return Ok(None);
      }
      if let Some(name) = types_callee(&c.callee, ev.scope) {
        let Some(arg) = c.arguments.first().and_then(|a| a.as_expression()) else {
          return Ok(None);
        };
        let Some(v) = ev.eval(arg)? else {
          return Ok(None);
        };
        return Ok(make_type(name, &v).map(Pre::V));
      }
      Ok(ev.eval(e)?.map(Pre::V))
    },
    Expression::ObjectExpression(o) => {
      let mut out: IndexMap<String, Pre<'b, 'a>> = IndexMap::default();
      for p in &o.properties {
        match p {
          ObjectPropertyKind::ObjectProperty(p) => {
            if p.method || p.kind != PropertyKind::Init {
              return Ok(None);
            }
            let Some(k) = ev.key_of(&p.key, p.computed)? else {
              return Ok(None);
            };
            let Some(v) = pre_eval_at(ev, &p.value, depth)? else {
              return Ok(None);
            };
            out.insert(k, v);
          },
          ObjectPropertyKind::SpreadProperty(s) => match ev.eval(&s.argument)? {
            Some(Val::Obj(m)) => {
              for (k, v) in std::rc::Rc::unwrap_or_clone(m) {
                out.insert(k, Pre::V(v));
              }
            },
            Some(Val::Null | Val::Undefined) => {},
            _ => return Ok(None),
          },
        }
      }
      // JS enumeration order: array-index keys first. Without index keys the order stands.
      if !out.keys().any(|k| crate::value::is_index_key(k)) {
        return Ok(Some(Pre::O(out)));
      }
      let order: Vec<String> = crate::value::js_order(
        out.keys().map(|k| (k.clone(), Val::Null)).collect(),
      )
      .into_keys()
      .collect();
      let mut sorted = IndexMap::with_capacity_and_hasher(order.len(), Default::default());
      for k in order {
        // `out` is only drained here, so its order no longer matters: swap_remove is O(1).
        let v = out.swap_remove(&k).unwrap();
        sorted.insert(k, v);
      }
      Ok(Some(Pre::O(sorted)))
    },
    _ => Ok(ev.eval(e)?.map(Pre::V)),
  }
}

/// A `Pre` as a plain value. Babel's vars helpers skip function values, so they become
/// `undefined`, which every consumer here ignores the same way.
fn pre_to_val(p: Pre) -> Option<Val> {
  match p {
    Pre::V(v) => Some(v),
    Pre::Fn(_) => Some(Val::Undefined),
    Pre::O(m) => {
      let mut o = Obj::default();
      for (k, v) in m {
        o.insert(k, pre_to_val(v)?);
      }
      Some(Val::Obj(o.into()))
    },
  }
}

/// Evaluates a style-like argument that may hold `stylex.types` calls.
pub(crate) fn eval_typed<'a>(ev: &mut Eval<'_, '_, 'a>, e: &Expression<'a>) -> Res<Option<Val>> {
  Ok(pre_eval(ev, e)?.and_then(pre_to_val))
}

/// `name.key` reads inside a function body: its same-group dependencies. Babel records the
/// reads its proxy sees while the body runs, so branches that do not run are skipped.
struct SelfRefs<'n, 'e, 'p, 'a> {
  name: &'n str,
  ev: Eval<'e, 'p, 'a>,
  deps: crate::fx::IndexSet<String>,
}

impl<'a, 'n, 'e, 'p> oxc_ast_visit::Visit<'a> for SelfRefs<'n, 'e, 'p, 'a> {
  fn visit_member_expression(&mut self, it: &MemberExpression<'a>) {
    if let Expression::Identifier(o) = it.object().without_parentheses()
      && o.name == self.name
    {
      match it {
        MemberExpression::StaticMemberExpression(m) => {
          self.deps.insert(m.property.name.to_string());
        },
        MemberExpression::ComputedMemberExpression(m) => {
          if let Ok(Some(k)) = self.ev.eval(&m.expression).map(|v| v.and_then(|v| v.to_js_string())) {
            self.deps.insert(k);
          }
        },
        _ => {},
      }
    }
    oxc_ast_visit::walk::walk_member_expression(self, it);
  }

  fn visit_conditional_expression(&mut self, it: &ConditionalExpression<'a>) {
    self.visit_expression(&it.test);
    match self.ev.eval(&it.test) {
      Ok(Some(t)) if t.truthy() => self.visit_expression(&it.consequent),
      Ok(Some(_)) => self.visit_expression(&it.alternate),
      _ => {
        self.visit_expression(&it.consequent);
        self.visit_expression(&it.alternate);
      },
    }
  }

  fn visit_logical_expression(&mut self, it: &LogicalExpression<'a>) {
    use oxc_syntax::operator::LogicalOperator as L;
    self.visit_expression(&it.left);
    let skip = match self.ev.eval(&it.left) {
      Ok(Some(l)) => match it.operator {
        L::And => !l.truthy(),
        L::Or => l.truthy(),
        L::Coalesce => !matches!(l, Val::Null | Val::Undefined),
      },
      _ => false,
    };
    if !skip {
      self.visit_expression(&it.right);
    }
  }
}

struct Normalizer<'s, 'p, 'a> {
  scope: &'s Scope<'p, 'a>,
  opts: &'s Opts,
  self_name: String,
  side: Vec<Rule>,
}

impl<'s, 'p, 'a> Normalizer<'s, 'p, 'a> {
  fn call(
    &mut self,
    f: &ArrowFunctionExpression<'a>,
    root_key: &str,
    deps: &mut crate::fx::IndexSet<String>,
  ) -> Res<Val> {
    if !f.params.items.is_empty() {
      bail(&invalid_fn())?;
    }
    let Some(body) = f.get_expression() else {
      bail(&err::non_static("defineVars"))?
    };
    let mut refs = SelfRefs {
      name: &self.self_name,
      ev: Eval::new(self.scope, self.opts),
      deps: Default::default(),
    };
    oxc_ast_visit::Visit::visit_expression(&mut refs, body);
    deps.extend(refs.deps);
    let mut ev = Eval::new(self.scope, self.opts);
    let r = match pre_eval(&mut ev, body) {
      Ok(Some(r)) => r,
      _ => bail(&err::non_static("defineVars"))?,
    };
    self.side.append(&mut ev.side);
    match r {
      Pre::Fn(_) => bail(&invalid_fn()),
      Pre::V(v) if as_typed(&v).is_some() => Ok(v),
      r => self.normalize(r, root_key, deps, false),
    }
  }

  fn normalize(
    &mut self,
    p: Pre<'_, 'a>,
    root_key: &str,
    deps: &mut crate::fx::IndexSet<String>,
    allow_type: bool,
  ) -> Res<Val> {
    match p {
      Pre::Fn(f) => self.call(f, root_key, deps),
      Pre::V(v) => {
        if as_typed(&v).is_some() {
          return if allow_type { Ok(v) } else { bail(&invalid_fn()) };
        }
        match v {
          Val::Str(_) | Val::Num(_) | Val::Null => Ok(v),
          Val::Arr(_) => bail("Array is not supported in defineVars"),
          Val::Obj(o) => {
            if matches!(o.get("default"), None | Some(Val::Undefined)) {
              bail(&format!(
                "Default value is not defined for {root_key} variable."
              ))?;
            }
            let mut out = Obj::default();
            for (k, x) in std::rc::Rc::unwrap_or_clone(o) {
              out.insert(k, self.normalize(Pre::V(x), root_key, deps, false)?);
            }
            Ok(Val::Obj(out.into()))
          },
          _ => bail("Invalid value in defineVars"),
        }
      },
      Pre::O(m) => {
        if !m.contains_key("default") || matches!(m.get("default"), Some(Pre::V(Val::Undefined)))
        {
          bail(&format!(
            "Default value is not defined for {root_key} variable."
          ))?;
        }
        let mut out = Obj::default();
        for (k, x) in m {
          out.insert(k, self.normalize(x, root_key, deps, false)?);
        }
        Ok(Val::Obj(out.into()))
      },
    }
  }
}

fn invalid_fn() -> String {
  "Function values in defineVars() must be zero-argument and return a static value supported by defineVars().".into()
}

fn assert_no_cycles(deps: &IndexMap<String, crate::fx::IndexSet<String>>) -> Res<()> {
  fn visit(
    key: &str,
    deps: &IndexMap<String, crate::fx::IndexSet<String>>,
    visited: &mut crate::fx::HashSet<String>,
    stack: &mut Vec<String>,
  ) -> Res<()> {
    if let Some(i) = stack.iter().position(|k| k == key) {
      let mut cycle: Vec<String> = stack[i..].to_vec();
      cycle.push(key.to_string());
      bail(&format!(
        "Cyclic same-group references in defineVars() are not allowed: {}.",
        cycle.join(" -> ")
      ))?;
    }
    if !visited.insert(key.to_string()) {
      return Ok(());
    }
    stack.push(key.to_string());
    if let Some(ds) = deps.get(key) {
      for d in ds {
        if deps.contains_key(d) {
          visit(d, deps, visited, stack)?;
        }
      }
    }
    stack.pop();
    Ok(())
  }
  let mut visited = crate::fx::HashSet::default();
  let mut stack = vec![];
  for k in deps.keys() {
    visit(k, deps, &mut visited, &mut stack)?;
  }
  Ok(())
}
