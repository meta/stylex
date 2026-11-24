//! `props`, `attrs`, legacy `stylex()`, the `sx` prop, and the bail-out bookkeeping.

use crate::fx::{HashMap, HashSet};

use crate::fx::IndexMap;
use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, walk};
use oxc_span::{GetSpan, Span};

use crate::diagnostics::Res;
use crate::emit::{Keep, StyleVar};
use crate::eval::{Eval, Scope};
use crate::options::{ImportSource, Opts};
use crate::transform::Pass2;
use crate::value::{Obj, Val, js_string, to_js};
use stylex_utils::number::to_js_string;

/// See `Opts::jsx_fold_marker`. `make_expression` output is parenthesized, so this is a call.
const JSX_FOLD_MARKER: &str = "__oxs_folded_props__";

pub(crate) enum Arg {
  Style(Option<Val>),
  Cond(String, Option<Val>, Option<Val>),
}

impl<'x, 'c, 's, 'p, 'a> Pass2<'x, 'c, 's, 'p, 'a> {
  /// The `sx` prop on a lowercase JSX element. Returns true when the caller must not walk on.
  pub(crate) fn sx_prop(&mut self, it: &JSXOpeningElement<'a>) -> bool {
    let tag = match &it.name {
      JSXElementName::Identifier(n) => Some(n.name.as_str()),
      JSXElementName::IdentifierReference(r) => Some(r.name.as_str()),
      _ => None,
    };
    // Babel: `name[0] === name[0].toLowerCase()`.
    if let Some(sx) = self.cx.opts.sx_prop_name.clone()
      && let Some(tag) = tag
      && tag.chars().next().is_some_and(|c| !c.is_uppercase())
    {
      for (attr_index, attr) in it.attributes.iter().enumerate() {
        let JSXAttributeItem::Attribute(a) = attr else {
          continue;
        };
        let JSXAttributeName::Identifier(an) = &a.name else {
          continue;
        };
        if an.name.as_str() != sx {
          continue;
        }
        let Some(JSXAttributeValue::ExpressionContainer(c)) = &a.value else {
          continue;
        };
        let Some(e) = c.expression.as_expression() else {
          continue;
        };
        // Babel adds the runtime binding before it knows whether props inlines.
        let ns = self.runtime_binding();
        let mut flat = vec![];
        flatten_arg(Some(e), &mut flat);
        let text = match self.resolve_flat(flat) {
          Err(err) => {
            self.error = Some(err);
            return true;
          },
          Ok(Ok(args)) if self.cx.opts.jsx_fold_marker => {
            format!("{{...{JSX_FOLD_MARKER}{}}}", make_expression("props", &args))
          },
          Ok(Ok(args)) => {
            if args.iter().all(|a| matches!(a, Arg::Style(_))) {
              let styles: Vec<Option<&Val>> = args
                .iter()
                .map(|a| {
                  if let Arg::Style(s) = a {
                    s.as_ref()
                  } else {
                    None
                  }
                })
                .collect();
              match props_val("props", &styles) {
                // Babel keeps an empty result as a spread.
                Val::Obj(o) if o.is_empty() => "{...{}}".to_string(),
                Val::Obj(o) => o
                  .iter()
                  .map(|(k, v)| {
                    format!(
                      "{k}={}",
                      crate::emit::jsx_attr(&v.to_js_string().unwrap_or_default())
                    )
                  })
                  .collect::<Vec<_>>()
                  .join(" "),
                _ => String::new(),
              }
            } else {
              format!("{{...{}}}", make_expression("props", &args))
            }
          },
          Ok(Err(bail_index)) => {
            self.record_bailed_exprs(&[Some(e)], bail_index);
            // Splice around the value so edits inside it (atoms, hoists) still apply.
            let es = e.span();
            self.cx.edits.replace(
              Span::new(a.span.start, es.start),
              format!("{{...{ns}.props("),
            );
            self
              .cx
              .edits
              .replace(Span::new(es.end, a.span.end), ")}".into());
            self.in_bail += 1;
            self.visit_expression(e);
            self.in_bail -= 1;
            self.walk_other_attrs(it, attr_index);
            return true;
          },
        };
        self.cx.edits.replace(a.span, text);
        self.walk_other_attrs(it, attr_index);
        return true;
      }
    }
    false
  }

  /// Walks every attribute but the one at `skip` (the sx prop, already handled).
  fn walk_other_attrs(&mut self, it: &JSXOpeningElement<'a>, skip: usize) {
    for (i, attr) in it.attributes.iter().enumerate() {
      if i != skip {
        self.visit_jsx_attribute_item(attr);
      }
    }
  }

  /// `{...stylex.props(...)}` in JSX, turned into plain attributes. Returns true when handled.
  pub(crate) fn props_spread(&mut self, it: &JSXSpreadAttribute<'a>) -> bool {
    if let Expression::CallExpression(c) = &it.argument
      && let Some(kind @ ("props" | "attrs")) = self.scope.imports.api(&c.callee)
    {
      let kind = kind.to_string();
      match self.resolve_args(c) {
        Err(e) => self.error = Some(e),
        Ok(Ok(args)) if self.cx.opts.jsx_fold_marker => {
          let text = format!("{{...{JSX_FOLD_MARKER}{}}}", make_expression(&kind, &args));
          self.cx.edits.replace(it.span, text);
          return true;
        },
        Ok(Ok(args)) if args.iter().all(|a| matches!(a, Arg::Style(_))) => {
          let styles: Vec<Option<&Val>> = args
            .iter()
            .map(|a| {
              if let Arg::Style(s) = a {
                s.as_ref()
              } else {
                None
              }
            })
            .collect();
          if let Val::Obj(o) = props_val(&kind, &styles)
            && !o.is_empty()
          {
            let attrs: Vec<String> = o
              .iter()
              .map(|(k, v)| {
                format!(
                  "{k}={}",
                  crate::emit::jsx_attr(&v.to_js_string().unwrap_or_default())
                )
              })
              .collect();
            self.cx.edits.replace(it.span, attrs.join(" "));
            return true;
          }
        },
        _ => {},
      }
    }
    false
  }

  /// `props`, `attrs` and legacy `stylex()` calls. Returns true when handled (walked or replaced).
  pub(crate) fn props_call(&mut self, c: &CallExpression<'a>) -> bool {
    if let Some(kind @ ("props" | "attrs" | "default")) = self.scope.imports.api(&c.callee) {
      let merge = kind == "default";
      if merge && !matches!(c.callee.without_parentheses(), Expression::Identifier(_)) {
        return false;
      }
      let kind = kind.to_string();
      let resolved = if merge {
        self.resolve_merge_args(c)
      } else {
        self.resolve_args(c)
      };
      match resolved {
        Err(e) => self.error = Some(e),
        Ok(Ok(args)) => {
          let text = make_expression(&kind, &args);
          self.cx.edits.replace(c.span, text);
          return true;
        },
        Ok(Err(bail_index)) => {
          self.record_bailed(c, bail_index);
          // Only `props`/`attrs` hoist `$$css` objects in their arguments.
          self.cx.atoms.bail_hoists.push(!merge);
          self.in_bail += 1;
          walk::walk_call_expression(self, c);
          self.in_bail -= 1;
          self.cx.atoms.bail_hoists.pop();
          return true;
        },
      }
    }
    false
  }

  /// Babel's `getStylexRuntimeBinding`: a value namespace import, or a new one.
  pub(crate) fn runtime_binding(&mut self) -> String {
    if let Some(n) = &self.cx.runtime_ns {
      return n.clone();
    }
    if let Some(n) = self.scope.imports.ns.iter().min() {
      return n.clone();
    }
    let existing = self.scope.imports.any_source.clone();
    let source = existing.clone().unwrap_or_else(|| {
      self
        .cx
        .opts
        .import_sources
        .get(2)
        .or(self.cx.opts.import_sources.first())
        .map(|s| match s {
          ImportSource::Plain(p) => p.clone(),
          ImportSource::Named { from, .. } => from.clone(),
        })
        .unwrap_or_default()
    });
    let stmt = self.stmt_start;
    self.cx.atoms.sx_after_inject = self.cx.atoms.main_inject_starts.iter().any(|s| *s <= stmt);
    let name = if existing.is_some() || self.scope.declared.contains("stylex") {
      "_stylex"
    } else {
      "stylex"
    }
    .to_string();
    self.cx.header.push_str(&format!(
      "import * as {name} from {};\n",
      js_string(&source)
    ));
    self.cx.runtime_ns = Some(name.clone());
    name
  }

  pub(crate) fn text(&self, span: Span) -> &str {
    &self.cx.code[span.start as usize..span.end as usize]
  }

  /// `transformStyleXMerge`'s argument loop for legacy `stylex(...)` calls: no array
  /// flattening, only member, conditional and `&&` arguments, stopping at the first bail.
  pub(crate) fn resolve_merge_args(
    &mut self,
    c: &CallExpression<'a>,
  ) -> Res<Result<Vec<Arg>, Option<usize>>> {
    let mut out = Vec::with_capacity(c.arguments.len());
    let mut conditional = 0;
    for (index, a) in c.arguments.iter().enumerate() {
      let Some(e) = a.as_expression() else {
        return Ok(Err(Some(index)));
      };
      let arg = match e.without_parentheses() {
        e @ (Expression::StaticMemberExpression(_) | Expression::ComputedMemberExpression(_)) => {
          self.merge_nullable(e).map(Arg::Style)
        },
        Expression::ConditionalExpression(ce) => {
          match (
            self.merge_nullable(&ce.consequent),
            self.merge_nullable(&ce.alternate),
          ) {
            (Some(p), Some(f)) => {
              conditional += 1;
              Some(Arg::Cond(self.text(ce.test.span()).to_string(), p, f))
            },
            _ => None,
          }
        },
        Expression::LogicalExpression(le)
          if le.operator == oxc_syntax::operator::LogicalOperator::And =>
        {
          match (
            self.merge_nullable(&le.left),
            self.merge_nullable(&le.right),
          ) {
            (None, Some(r)) => {
              conditional += 1;
              Some(Arg::Cond(self.text(le.left.span()).to_string(), r, None))
            },
            _ => None,
          }
        },
        _ => None,
      };
      match arg {
        Some(a) => out.push(a),
        None => return Ok(Err(Some(index))),
      }
      if conditional > 4 {
        return Ok(Err(None));
      }
    }
    if !self.cx.opts.inlined_conditional_merge && conditional > 0 {
      return Ok(Err(None));
    }
    Ok(Ok(out))
  }

  /// `parseNullableStyle` in `stylex-merge.js`: null, undefined, or a namespace of a
  /// top-level `create` (`styleMap`). `None` means "other".
  fn merge_nullable(&self, e: &Expression<'a>) -> Option<Option<Val>> {
    let e = e.without_parentheses();
    match e {
      Expression::NullLiteral(_) => return Some(None),
      Expression::Identifier(i) if i.name == "undefined" => return Some(None),
      _ => {},
    }
    let m = e.as_member_expression()?;
    let Expression::Identifier(o) = m.object() else {
      return None;
    };
    if !self.style_vars.contains_key(o.name.as_str()) {
      return None;
    }
    let prop = match m {
      MemberExpression::StaticMemberExpression(s) => s.property.name.to_string(),
      MemberExpression::ComputedMemberExpression(c) => match &c.expression {
        Expression::StringLiteral(s) => s.value.display().to_string(),
        Expression::NumericLiteral(n) => to_js_string(n.value),
        _ => return None,
      },
      MemberExpression::PrivateFieldExpression(_) => return None,
    };
    match self
      .scope
      .values
      .get(o.name.as_str())?
      .as_obj()?
      .get(&prop)?
    {
      Val::Null | Val::Undefined => None,
      v => Some(Some(v.clone())),
    }
  }

  /// `parseNullableStyle`: `Ok(None)` means "other" (not statically known).
  pub(crate) fn nullable(&mut self, e: &Expression<'a>) -> Res<Option<Option<Val>>> {
    let e = e.without_parentheses();
    if let Some(v) = self.atom_value(e) {
      return Ok(Some(Some(v)));
    }
    match e {
      Expression::NullLiteral(_) => return Ok(Some(None)),
      Expression::Identifier(i) if i.name == "undefined" => return Ok(Some(None)),
      Expression::CallExpression(c)
        if self.scope.imports.api(&c.callee) != Some("defaultMarker") =>
      {
        return Ok(None);
      },
      _ => {},
    }
    // Babel reads `styles.ns` of a top-level create straight from `styleMap`.
    if let Some(v) = self.merge_nullable(e)
      && e.as_member_expression().is_some()
    {
      return Ok(Some(v));
    }
    // Anything else goes through `evaluate`, which deopts on a binding used before its
    // declaration.
    if used_before_declaration(self.scope, e) {
      return Ok(None);
    }
    let mut ev = Eval::new(self.scope, self.cx.opts);
    ev.disable_imports = true;
    match ev.eval(e)? {
      Some(v @ (Val::Obj(_) | Val::Arr(_))) => Ok(Some(Some(v))),
      _ => Ok(None),
    }
  }

  pub(crate) fn resolve_args(
    &mut self,
    c: &CallExpression<'a>,
  ) -> Res<Result<Vec<Arg>, Option<usize>>> {
    let mut flat: Vec<Option<&Expression<'a>>> = Vec::with_capacity(c.arguments.len());
    for a in &c.arguments {
      flatten_arg(a.as_expression(), &mut flat);
    }
    self.resolve_flat(flat)
  }

  /// Arguments after Babel's one-level array flattening. `None` (a spread or a hole) bails.
  pub(crate) fn resolve_flat(
    &mut self,
    flat: Vec<Option<&Expression<'a>>>,
  ) -> Res<Result<Vec<Arg>, Option<usize>>> {
    self.resolve_opt(flat)
  }

  /// Babel's bail-out bookkeeping: which namespaces (and which of their nulls) a call that
  /// could not be inlined still needs.
  pub(crate) fn record_bailed(&mut self, c: &CallExpression<'a>, bail_index: Option<usize>) {
    // A spread argument is traversed like any non-member argument.
    let exprs: Vec<(Option<&Expression<'a>>, bool)> = c
      .arguments
      .iter()
      .map(|a| match a {
        Argument::SpreadElement(s) => (Some(&s.argument), true),
        a => (a.as_expression(), false),
      })
      .collect();
    self.record_bailed_args(&exprs, bail_index);
  }

  pub(crate) fn record_bailed_exprs(
    &mut self,
    exprs: &[Option<&Expression<'a>>],
    bail_index: Option<usize>,
  ) {
    let args: Vec<(Option<&Expression<'a>>, bool)> = exprs.iter().map(|e| (*e, false)).collect();
    self.record_bailed_args(&args, bail_index);
  }

  /// Arguments with a flag for spreads, whose argument is always traversed.
  fn record_bailed_args(
    &mut self,
    exprs: &[(Option<&Expression<'a>>, bool)],
    bail_index: Option<usize>,
  ) {
    let mut non_null: Option<Vec<String>> = Some(vec![]);
    for (index, (e, spread)) in exprs.iter().copied().enumerate() {
      let Some(e) = e else { continue };
      let mut col = BailCollector {
        atoms: &self.cx.atoms,
        scope: self.scope,
        opts: self.cx.opts,
        style_vars: self.style_vars,
        keep: &mut self.keep,
        non_null: &mut non_null,
        past_bail: bail_index.is_some_and(|b| index > b),
      };
      // Babel handles a member argument directly and traverses any other argument.
      match e.without_parentheses().as_member_expression() {
        Some(m) if !spread => col.member(m),
        _ => col.visit_expression(e),
      }
    }
  }

  pub(crate) fn resolve_exprs(&mut self, flat: Vec<&Expression<'a>>) -> Res<Option<Vec<Arg>>> {
    Ok(match self.resolve_exprs_at(flat)? {
      Ok(v) => Some(v),
      Err(_) => None,
    })
  }

  /// Resolved arguments, or `Err(bailOutIndex)` when the call cannot be inlined.
  pub(crate) fn resolve_exprs_at(
    &mut self,
    flat: Vec<&Expression<'a>>,
  ) -> Res<Result<Vec<Arg>, Option<usize>>> {
    self.resolve_opt(flat.into_iter().map(Some).collect())
  }

  /// Babel's argument loop: the first bail sets `bailOutIndex`, except a `||`/`??` argument,
  /// which overrides it and stops the loop. `None` entries (spreads, holes) bail.
  fn resolve_opt(
    &mut self,
    flat: Vec<Option<&Expression<'a>>>,
  ) -> Res<Result<Vec<Arg>, Option<usize>>> {
    let mut out = Vec::with_capacity(flat.len());
    let mut conditional = 0;
    let mut first_bail: Option<usize> = None;
    let mut too_many = false;
    for (index, e) in flat.into_iter().enumerate() {
      let Some(e) = e else {
        first_bail.get_or_insert(index);
        continue;
      };
      if let Expression::LogicalExpression(le) = e.without_parentheses()
        && le.operator != oxc_syntax::operator::LogicalOperator::And
      {
        first_bail = Some(index);
        break;
      }
      if first_bail.is_some() {
        continue;
      }
      match self.resolve_one(e, &mut conditional)? {
        Some(a) => out.push(a),
        None => first_bail = Some(index),
      }
      if conditional > 4 {
        too_many = true;
      }
    }
    if let Some(i) = first_bail {
      return Ok(Err(Some(i)));
    }
    if too_many || (!self.cx.opts.inlined_conditional_merge && conditional > 0) {
      return Ok(Err(None));
    }
    Ok(Ok(out))
  }

  pub(crate) fn resolve_one(
    &mut self,
    e: &Expression<'a>,
    conditional: &mut usize,
  ) -> Res<Option<Arg>> {
    let inner = e.without_parentheses();
    Ok(Some(match inner {
      Expression::ConditionalExpression(ce) => {
        let (Some(p), Some(f)) = (
          self.nullable(&ce.consequent)?,
          self.nullable(&ce.alternate)?,
        ) else {
          return Ok(None);
        };
        *conditional += 1;
        Arg::Cond(self.text(ce.test.span()).to_string(), p, f)
      },
      Expression::LogicalExpression(le) => {
        if le.operator != oxc_syntax::operator::LogicalOperator::And
          || self.nullable(&le.left)?.is_some()
        {
          return Ok(None);
        }
        let Some(r) = self.nullable(&le.right)? else {
          return Ok(None);
        };
        *conditional += 1;
        Arg::Cond(self.text(le.left.span()).to_string(), r, None)
      },
      Expression::ObjectExpression(_)
      | Expression::Identifier(_)
      | Expression::StaticMemberExpression(_)
      | Expression::ComputedMemberExpression(_)
      | Expression::PrivateFieldExpression(_)
      | Expression::CallExpression(_) => match self.nullable(inner)? {
        Some(s) => Arg::Style(s),
        None => return Ok(None),
      },
      _ => return Ok(None),
    }))
  }
}

/// Whether `e` reads a top-level constant before its declaration ends (Babel's
/// `USED_BEFORE_DECLARATION` deopt). Function bodies inside `e` are not evaluated, so skipped.
pub(crate) fn used_before_declaration(scope: &Scope, e: &Expression) -> bool {
  struct V<'s, 'p, 'a> {
    scope: &'s Scope<'p, 'a>,
    hit: bool,
  }
  impl<'s, 'p, 'a, 'b> Visit<'b> for V<'s, 'p, 'a> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'b>) {
      if let Some(init) = self.scope.consts.get(it.name.as_str())
        && it.span.start < init.span().end
      {
        self.hit = true;
      }
    }
    fn visit_function(&mut self, _: &Function<'b>, _: oxc_syntax::scope::ScopeFlags) {}
    fn visit_arrow_function_expression(&mut self, _: &ArrowFunctionExpression<'b>) {}
  }
  if scope.consts.is_empty() {
    return false;
  }
  let mut v = V { scope, hit: false };
  v.visit_expression(e);
  v.hit
}

/// Babel flattens array arguments one level deep (`argPath.get('elements')`).
fn flatten_arg<'b, 'a>(e: Option<&'b Expression<'a>>, out: &mut Vec<Option<&'b Expression<'a>>>) {
  match e.map(|e| (e, e.without_parentheses())) {
    Some((_, Expression::ArrayExpression(arr))) => {
      for el in &arr.elements {
        out.push(el.as_expression());
      }
    },
    Some((e, _)) => out.push(Some(e)),
    None => out.push(None),
  }
}

/// `styleq`: merges compiled style objects, later arguments winning.
pub(crate) fn styleq(styles: &[&Obj]) -> (String, String) {
  let mut debug = String::new();
  for style in styles.iter().rev() {
    if let Some(Val::Str(d)) = style.get("$$css") {
      debug = if debug.is_empty() {
        d.clone()
      } else {
        format!("{d}; {debug}")
      };
    }
  }
  let mut class_name = String::new();
  // One style object has unique keys, so it needs no set. Otherwise size the set once.
  let single = styles.len() <= 1;
  let mut defined: HashSet<&str> = if single {
    HashSet::default()
  } else {
    HashSet::with_capacity_and_hasher(styles.iter().map(|s| s.len()).sum(), Default::default())
  };
  for style in styles.iter().rev() {
    let mut chunk = String::new();
    for (prop, value) in style.iter() {
      if prop == "$$css" {
        continue;
      }
      if matches!(value, Val::Str(_) | Val::Null) && (single || defined.insert(prop.as_str())) {
        if let Val::Str(v) = value {
          if !chunk.is_empty() {
            chunk.push(' ');
          }
          chunk.push_str(v);
        }
      }
    }
    if !chunk.is_empty() {
      class_name = if class_name.is_empty() {
        chunk
      } else {
        format!("{chunk} {class_name}")
      };
    }
  }
  // styleq does not dedupe class names: two keys can compile to the same class.
  (class_name, debug)
}

pub(crate) fn flatten_styles<'v>(v: &'v Val, out: &mut Vec<&'v Obj>) {
  match v {
    Val::Obj(o) => out.push(o),
    Val::Arr(a) => a.iter().for_each(|x| flatten_styles(x, out)),
    _ => {},
  }
}

pub(crate) fn props_val(kind: &str, styles: &[Option<&Val>]) -> Val {
  let mut s: Vec<&Obj> = vec![];
  for v in styles.iter().flatten() {
    flatten_styles(v, &mut s);
  }
  let (cn, debug) = styleq(&s);
  match kind {
    "default" => Val::Str(cn),
    _ => {
      let mut o = Obj::default();
      if !cn.is_empty() {
        o.insert(
          if kind == "attrs" {
            "class".into()
          } else {
            "className".into()
          },
          Val::Str(cn),
        );
      }
      if !debug.is_empty() {
        o.insert("data-style-src".into(), Val::Str(debug));
      }
      Val::Obj(o.into())
    },
  }
}

pub(crate) fn make_expression(kind: &str, args: &[Arg]) -> String {
  let tests: Vec<&str> = args
    .iter()
    .filter_map(|a| {
      if let Arg::Cond(t, ..) = a {
        Some(t.as_str())
      } else {
        None
      }
    })
    .collect();
  let mut s = String::new();
  if tests.is_empty() {
    let styles: Vec<Option<&Val>> = args
      .iter()
      .map(|a| {
        if let Arg::Style(s) = a {
          s.as_ref()
        } else {
          None
        }
      })
      .collect();
    let v = props_val(kind, &styles);
    s.push('(');
    to_js(&v, &mut s);
    s.push(')');
    return s;
  }
  let n = tests.len();
  s.push_str("({");
  for i in 0..(1u32 << n) {
    let perm: Vec<bool> = (0..n).map(|j| i & (1 << j) != 0).collect();
    let mut k = 0;
    let styles: Vec<Option<&Val>> = args
      .iter()
      .map(|a| match a {
        Arg::Style(s) => s.as_ref(),
        Arg::Cond(_, p, f) => {
          let r = if perm[k] { p.as_ref() } else { f.as_ref() };
          k += 1;
          r
        },
      })
      .collect();
    let key = perm.iter().fold(0u32, |acc, b| (acc << 1) | (*b as u32));
    if i > 0 {
      s.push(',');
    }
    s.push_str(&format!("\n{key}: "));
    to_js(&props_val(kind, &styles), &mut s);
  }
  s.push_str("\n})[");
  for (i, t) in tests.iter().enumerate() {
    if i > 0 {
      s.push_str(" | ");
    }
    s.push_str(&format!("!!({t}) << {}", n - i - 1));
  }
  s.push(']');
  s
}

pub(crate) struct BailCollector<'k, 's, 'p, 'a> {
  atoms: &'k crate::api::atoms::Atoms,
  scope: &'s Scope<'p, 'a>,
  opts: &'s Opts,
  style_vars: &'s IndexMap<String, StyleVar>,
  keep: &'k mut HashMap<String, Keep>,
  non_null: &'k mut Option<Vec<String>>,
  past_bail: bool,
}

impl<'k, 's, 'p, 'a> Visit<'a> for BailCollector<'k, 's, 'p, 'a> {
  fn visit_member_expression(&mut self, it: &MemberExpression<'a>) {
    // Babel sees a static atom as an object literal, not a member expression.
    if self.is_static_atom(it) {
      return;
    }
    self.member(it);
    walk::walk_member_expression(self, it);
  }

  // Babel's MemberExpression visitor skips OptionalMemberExpression links of a chain.
  fn visit_chain_expression(&mut self, it: &ChainExpression<'a>) {
    fn walk_link<'a>(c: &mut BailCollector<'_, '_, '_, 'a>, e: &Expression<'a>) {
      match e {
        Expression::StaticMemberExpression(m) => walk_link(c, &m.object),
        Expression::ComputedMemberExpression(m) => {
          walk_link(c, &m.object);
          c.visit_expression(&m.expression);
        },
        Expression::PrivateFieldExpression(m) => walk_link(c, &m.object),
        Expression::CallExpression(call) => {
          walk_link(c, &call.callee);
          for a in &call.arguments {
            c.visit_argument(a);
          }
        },
        e => c.visit_expression(e),
      }
    }
    match &it.expression {
      ChainElement::CallExpression(call) => {
        walk_link(self, &call.callee);
        for a in &call.arguments {
          self.visit_argument(a);
        }
      },
      ChainElement::TSNonNullExpression(t) => self.visit_expression(&t.expression),
      e => {
        if let Some(m) = e.as_member_expression() {
          walk_link(self, m.object());
          if let MemberExpression::ComputedMemberExpression(cm) = m {
            self.visit_expression(&cm.expression);
          }
        }
      },
    }
  }
}

impl<'k, 's, 'p, 'a> BailCollector<'k, 's, 'p, 'a> {
  fn is_static_atom(&self, it: &MemberExpression<'a>) -> bool {
    !self.atoms.statics.is_empty()
      && self
        .atoms
        .statics
        .contains_key(&(it.span().start, it.span().end))
  }

  /// Babel's bail-out `MemberExpression` handler for one member expression.
  fn member(&mut self, it: &MemberExpression<'a>) {
    if self.is_static_atom(it) {
      return;
    }
    let obj = match it.object() {
      Expression::Identifier(o) if self.style_vars.contains_key(o.name.as_str()) => {
        Some(o.name.to_string())
      },
      _ => None,
    };
    let prop = match it {
      MemberExpression::StaticMemberExpression(m) => Some(m.property.name.to_string()),
      MemberExpression::ComputedMemberExpression(m) => match &m.expression {
        Expression::StringLiteral(s) => s.value.as_str().map(String::from),
        Expression::NumericLiteral(n) => Some(to_js_string(n.value)),
        _ => None,
      },
      _ => None,
    };
    if self.past_bail {
      *self.non_null = None;
    }
    // Babel cannot evaluate a dynamic namespace (`styles.fn`) here, nor a styles object read
    // before its declaration (`USED_BEFORE_DECLARATION`).
    if let Some(o) = &obj
      && let Some(sv) = self.style_vars.get(o.as_str())
      && (it.span().start < sv.call.end
        || prop
          .as_ref()
          .is_some_and(|p| sv.created.dyn_text.contains_key(p.as_str())))
    {
      *self.non_null = None;
    }
    if let Expression::Identifier(o) = it.object()
      && self
        .scope
        .consts
        .get(o.name.as_str())
        .is_some_and(|init| o.span.start < init.span().end)
    {
      *self.non_null = None;
    }
    let style_nn: Option<Vec<String>> = match self.non_null.as_mut() {
      None => None,
      Some(nn) => {
        let mut ev = Eval::new(self.scope, self.opts);
        ev.disable_imports = true;
        match ev.eval_member(it) {
          Ok(Some(Val::Obj(o))) => {
            let before = nn.clone();
            nn.extend(
              o.iter()
                .filter(|(_, v)| **v != Val::Null)
                .map(|(k, _)| k.clone()),
            );
            Some(before)
          },
          _ => {
            *self.non_null = None;
            None
          },
        }
      },
    };
    if let Some(obj) = obj {
      let k = self.keep.entry(obj).or_default();
      match prop {
        Some(p) => k.add(p, style_nn.map(|v| v.into_iter().collect())),
        None => k.all = true,
      }
    }
  }
}
