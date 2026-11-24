//! Dynamic styles: `stylex.create({ root: (color) => ({ color }) })`.
//! Ports `parse-stylex-create-arg.js` and the dynamic half of `visitors/stylex-create.js`.

use crate::fx::IndexMap;
use oxc_ast::ast::*;
use oxc_span::GetSpan;
use oxc_syntax::operator::{BinaryOperator, LogicalOperator, UnaryOperator};
use stylex_constants::constants::{length_units::LENGTH_UNITS, number_properties::NUMBER_PROPERTY_SUFFIXIES};
use stylex_css::css::common::get_number_suffix;
use stylex_utils::hash::create_hash;

use stylex_css::order::structures::legacy_expand_shorthands_order::LegacyExpandShorthandsOrder;

use crate::css::{ClassPaths, Compiled, Injectable, Order, TRawValue};
use crate::diagnostics::{Res, bail};
use crate::eval::Eval;
use crate::value::{Obj, Val, js_key, js_string};

pub const ONLY_NAMED_PARAMETERS: &str = "Only named parameters are allowed in Dynamic Style functions. Destructuring, spreading or default values are not allowed.";

pub struct InlineStyle<'b, 'a> {
  pub var_name: String,
  pub path: Vec<String>,
  pub expr: &'b Expression<'a>,
}

pub struct DynFn<'b, 'a> {
  pub params_text: String,
  pub inline: Vec<InlineStyle<'b, 'a>>,
}

pub type DynFns<'b, 'a> = IndexMap<String, DynFn<'b, 'a>>;

/// `evaluateStyleXCreateArg`: the plain object plus the dynamic functions found in it.
pub fn evaluate_create_arg<'b, 'a>(
  ev: &mut Eval<'_, '_, 'a>,
  arg: &'b Expression<'a>,
  code: &str,
) -> Res<Option<(Obj, DynFns<'b, 'a>)>> {
  let Expression::ObjectExpression(o) = arg.without_parentheses() else {
    return Ok(match ev.eval(arg)? {
      Some(Val::Obj(v)) => Some((std::rc::Rc::unwrap_or_clone(v), DynFns::default())),
      Some(_) => bail(&crate::diagnostics::non_style_object("create"))?,
      None => None,
    });
  };
  let mut value = Obj::default();
  let mut fns = DynFns::default();
  for p in &o.properties {
    // Babel's ObjectMethod is not an ObjectProperty: evaluate the whole argument.
    let ObjectPropertyKind::ObjectProperty(p) = p else {
      return Ok(match ev.eval(arg)? {
        Some(Val::Obj(v)) => Some((std::rc::Rc::unwrap_or_clone(v), DynFns::default())),
        _ => None,
      });
    };
    if p.method || p.kind != PropertyKind::Init {
      return Ok(match ev.eval(arg)? {
        Some(Val::Obj(v)) => Some((std::rc::Rc::unwrap_or_clone(v), DynFns::default())),
        _ => None,
      });
    }
    let Some(key) = ev.key_of(&p.key, p.computed)? else {
      return Ok(None);
    };
    let Expression::ArrowFunctionExpression(f) = p.value.without_parentheses() else {
      match ev.eval(&p.value)? {
        Some(v) => {
          value.insert(key, v);
        },
        None => return Ok(None),
      }
      continue;
    };
    let mut names = vec![];
    // host: Babel's code frame points at the arrow function.
    let at_fn = (f.span.start, f.span.end);
    if f.params.rest.is_some() {
      crate::diagnostics::bail_at(ONLY_NAMED_PARAMETERS, at_fn)?;
    }
    for param in &f.params.items {
      match (&param.pattern, &param.initializer) {
        (BindingPattern::BindingIdentifier(id), None) => names.push(id.name.to_string()),
        _ => crate::diagnostics::bail_at(ONLY_NAMED_PARAMETERS, at_fn)?,
      }
    }
    let Some(Expression::ObjectExpression(body)) =
      f.get_expression().map(|e| e.without_parentheses())
    else {
      // Babel evaluates the whole argument, so the evaluator gives the reason.
      return Ok(match ev.eval(arg)? {
        Some(Val::Obj(v)) => Some((std::rc::Rc::unwrap_or_clone(v), DynFns::default())),
        _ => None,
      });
    };
    let saved = std::mem::take(&mut ev.shadowed);
    ev.shadowed = saved.iter().cloned().chain(names.iter().cloned()).collect();
    let r = evaluate_partial(ev, body, &[]);
    ev.shadowed = saved;
    let Some((v, inline)) = r? else {
      return Ok(None);
    };
    value.insert(key.clone(), Val::Obj(v.into()));
    let params_text = f
      .params
      .items
      .iter()
      .map(|p| &code[p.span.start as usize..p.span.end as usize])
      .collect::<Vec<_>>()
      .join(", ");
    fns.insert(
      key,
      DynFn {
        params_text,
        inline,
      },
    );
  }
  Ok(Some((crate::value::js_order(value), fns)))
}

fn evaluate_partial<'b, 'a>(
  ev: &mut Eval<'_, '_, 'a>,
  o: &'b ObjectExpression<'a>,
  key_path: &[String],
) -> Res<Option<(Obj, Vec<InlineStyle<'b, 'a>>)>> {
  let mut obj = Obj::default();
  let mut inline = vec![];
  for p in &o.properties {
    match p {
      ObjectPropertyKind::SpreadProperty(s) => match ev.eval(&s.argument)? {
        Some(Val::Obj(m)) => obj.extend(std::rc::Rc::unwrap_or_clone(m)),
        _ => return Ok(None),
      },
      ObjectPropertyKind::ObjectProperty(p) => {
        if p.method {
          return Ok(None);
        }
        let Some(mut key) = ev.key_of(&p.key, p.computed)? else {
          return Ok(None);
        };
        if key.starts_with("var(") && key.ends_with(')') && key_path.is_empty() {
          key = key[4..key.len() - 1].to_string();
        }
        let mut path = key_path.to_vec();
        path.push(key.clone());
        if let Expression::ObjectExpression(inner) = p.value.without_parentheses() {
          let Some((v, styles)) = evaluate_partial(ev, inner, &path)? else {
            return Ok(None);
          };
          obj.insert(key, Val::Obj(v.into()));
          inline.extend(styles);
        } else {
          match ev.eval(&p.value)? {
            Some(v) => {
              obj.insert(key, v);
            },
            None => {
              let var_name = if key_path.is_empty() {
                format!("--x-{key}")
              } else {
                format!("--x-{}", create_hash(&path.join("_")))
              };
              obj.insert(key, Val::Str(format!("var({var_name})")));
              inline.push(InlineStyle {
                var_name,
                path,
                expr: &p.value,
              });
            },
          }
        }
      },
    }
  }
  Ok(Some((obj, inline)))
}

fn safe_to_skip_null_check(e: &Expression) -> bool {
  match e.without_parentheses() {
    Expression::TemplateLiteral(_)
    | Expression::StringLiteral(_)
    | Expression::NumericLiteral(_)
    | Expression::BooleanLiteral(_) => true,
    Expression::BinaryExpression(b) => matches!(
      b.operator,
      BinaryOperator::Addition
        | BinaryOperator::Subtraction
        | BinaryOperator::Multiplication
        | BinaryOperator::Division
        | BinaryOperator::Remainder
        | BinaryOperator::Exponential
    ),
    Expression::UnaryExpression(u) => matches!(
      u.operator,
      UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus
    ),
    Expression::ConditionalExpression(c) => {
      safe_to_skip_null_check(&c.consequent) && safe_to_skip_null_check(&c.alternate)
    },
    Expression::LogicalExpression(l) => match l.operator {
      LogicalOperator::Coalesce | LogicalOperator::Or => {
        safe_to_skip_null_check(&l.left) || safe_to_skip_null_check(&l.right)
      },
      LogicalOperator::And => safe_to_skip_null_check(&l.left) && safe_to_skip_null_check(&l.right),
    },
    _ => false,
  }
}

fn has_explicit_nullish_fallback(e: &Expression) -> bool {
  match e.without_parentheses() {
    Expression::NullLiteral(_) => true,
    Expression::Identifier(i) => i.name == "undefined",
    Expression::UnaryExpression(u) => u.operator == UnaryOperator::Void,
    Expression::ConditionalExpression(c) => {
      has_explicit_nullish_fallback(&c.consequent) || has_explicit_nullish_fallback(&c.alternate)
    },
    Expression::LogicalExpression(l) => {
      has_explicit_nullish_fallback(&l.left) || has_explicit_nullish_fallback(&l.right)
    },
    _ => false,
  }
}

/// Every `--x-` variable a rule reads, in order.
fn rule_vars(rule: &str) -> Vec<String> {
  let mut out = vec![];
  let mut rest = rule;
  while let Some(i) = rest.find("var(--x-") {
    let after = &rest[i + 4..];
    let end = after.find([',', ')']).unwrap_or(after.len());
    out.push(after[..end].to_string());
    rest = &after[end..];
  }
  out
}

/// The `@property` rules dynamic variables need.
pub fn inherit_rules(fns: &DynFns) -> Vec<(String, Injectable)> {
  let mut out = vec![];
  for f in fns.values() {
    for s in &f.inline {
      let pe = s.path.iter().any(|p| p.starts_with("::"));
      out.push((
        s.var_name.clone(),
        Injectable {
          ltr: format!(
            "@property {} {{ syntax: \"*\"; inherits: {};}}",
            s.var_name,
            if pe { "true" } else { "false" }
          ),
          rtl: None,
          priority: 0.0,
          ..Default::default()
        },
      ));
    }
  }
  out
}

struct DynStyle<'b, 'a> {
  expr: &'b Expression<'a>,
  var_name: &'b str,
  path: String,
}

/// Babel's `dynamicStyles` list, with `legacyExpandShorthands` applied in that mode.
fn dynamic_styles<'b, 'a>(
  inline: &'b [InlineStyle<'b, 'a>],
  legacy: bool,
) -> Vec<DynStyle<'b, 'a>> {
  let mut out = Vec::with_capacity(inline.len());
  for s in inline {
    let path = s.path.join("_");
    if !legacy {
      out.push(DynStyle {
        expr: s.expr,
        var_name: &s.var_name,
        path,
      });
      continue;
    }
    let end = s
      .path
      .iter()
      .position(|p| !p.starts_with(':') && !p.starts_with('@'))
      .map_or(0, |i| i + 1);
    let key = s.path[..end].join("_");
    let bare = if key.starts_with("var(") && key.ends_with(')') {
      &key[4..key.len() - 1]
    } else {
      key.as_str()
    };
    let expanded: Vec<String> = match LegacyExpandShorthandsOrder::get_expansion_fn(bare) {
      None => vec![bare.to_string()],
      Some(f) => match f(Some(TRawValue::String("p".into()))) {
        Ok(pairs) => pairs
          .into_iter()
          .filter(|p| matches!(p.1, Some(TRawValue::String(_))))
          .map(|p| p.0.into_owned())
          .collect(),
        Err(_) => vec![],
      },
    };
    for k in expanded {
      let p = if path == key {
        k.clone()
      } else if path.contains(&format!("{key}_")) {
        path.replacen(&format!("{key}_"), &format!("{k}_"), 1)
      } else {
        path.replacen(&format!("_{key}"), &format!("_{k}"), 1)
      };
      out.push(DynStyle {
        expr: s.expr,
        var_name: &s.var_name,
        path: p,
      });
    }
  }
  out
}

/// The arrow function a dynamic namespace compiles to. `hoist` turns a static object into the
/// name of a hoisted top-level constant.
pub fn build_fn(
  ns: &Compiled,
  tag: &str,
  f: &DynFn,
  class_paths: &ClassPaths,
  ns_name: &str,
  injected: &IndexMap<String, Injectable>,
  code: &str,
  legacy: bool,
  hoist: &mut dyn FnMut(String) -> String,
) -> String {
  let text = |e: &Expression| {
    format!(
      "({})",
      &code[e.span().start as usize..e.span().end as usize]
    )
  };
  let empty = IndexMap::default();
  let paths = class_paths.get(ns_name).unwrap_or(&empty);
  let dyn_styles = dynamic_styles(&f.inline, legacy);
  let nullish: Vec<&DynStyle> = dyn_styles
    .iter()
    .filter(|s| has_explicit_nullish_fallback(s.expr))
    .collect();
  let mut static_props: Vec<String> = vec![];
  let mut conditional_props: Vec<String> = vec![];
  let mut guarded: Vec<String> = vec![];
  for (key, value) in ns {
    let k = js_key(key);
    let Some(value) = value else {
      static_props.push(format!("{k}: null"));
      continue;
    };
    let classes: Vec<&str> = value.split(' ').collect();
    let mut is_static = true;
    let mut controlling: Option<&Expression> = None;
    let mut pieces: Vec<String> = vec![];
    for (i, cls) in classes.iter().enumerate() {
      let orig = paths.get(*cls).map(|p| p.join("_"));
      let mut expr = orig
        .as_ref()
        .and_then(|o| dyn_styles.iter().find(|s| &s.path == o))
        .map(|s| s.expr);
      if expr.is_none() && !nullish.is_empty() {
        if let Some(inj) = injected.get(*cls) {
          for v in rule_vars(&inj.ltr) {
            if let Some(s) = nullish.iter().find(|s| s.var_name == v) {
              expr = Some(s.expr);
              break;
            }
          }
        }
      }
      let with_space = if i == classes.len() - 1 {
        cls.to_string()
      } else {
        format!("{cls} ")
      };
      match expr {
        Some(e) if !safe_to_skip_null_check(e) => {
          is_static = false;
          controlling = Some(e);
          pieces.push(format!(
            "({} != null ? {} : \"\")",
            text(e),
            js_string(&with_space)
          ));
        },
        _ => pieces.push(js_string(&with_space)),
      }
    }
    let joined = if pieces.is_empty() {
      "\"\"".to_string()
    } else {
      pieces.join(" + ")
    };
    if is_static {
      static_props.push(format!("{k}: {joined}"));
    } else if classes.len() == 1
      && let Some(e) = controlling
    {
      guarded.push(format!(
        "{} !== undefined && {{\n{k}: {joined},\n$$css: {tag}\n}}",
        text(e)
      ));
    } else {
      conditional_props.push(format!("{k}: {joined}"));
    }
  }
  let inline_obj = {
    let entries: Vec<String> = f
      .inline
      .iter()
      .map(|s| {
        let prop = s
          .path
          .iter()
          .find(|k| !k.starts_with(':') && !k.starts_with('@') && k.as_str() != "default")
          .cloned()
          .unwrap_or_else(|| s.path.last().cloned().unwrap_or_default());
        // `get_time_units()` is the key set of NUMBER_PROPERTY_SUFFIXIES, built afresh per call.
        let unit = if NUMBER_PROPERTY_SUFFIXIES.contains_key(prop.as_str()) || LENGTH_UNITS.contains(prop.as_str()) {
          get_number_suffix(&prop)
        } else {
          ""
        };
        let e = text(s.expr);
        let v = if !unit.is_empty() {
          format!(
            "(val => typeof val === \"number\" ? val + {} : val != null ? val : undefined)({e})",
            js_string(unit)
          )
        } else {
          format!("{e} != null ? {e} : undefined")
        };
        format!("{}: {v}", js_string(&s.var_name))
      })
      .collect();
    format!("{{\n{}\n}}", entries.join(",\n"))
  };
  let mut items: Vec<String> = vec![];
  if !static_props.is_empty() {
    static_props.push(format!("\"$$css\": {tag}"));
    items.push(hoist(format!("{{\n{}\n}}", static_props.join(",\n"))));
  }
  items.extend(guarded);
  if !conditional_props.is_empty() {
    conditional_props.push(format!("$$css: {tag}"));
    items.push(format!("{{\n{}\n}}", conditional_props.join(",\n")));
  }
  if items.is_empty() {
    format!("({}) => ({inline_obj})", f.params_text)
  } else {
    items.push(inline_obj);
    format!("({}) => [{}]", f.params_text, items.join(", "))
  }
}
