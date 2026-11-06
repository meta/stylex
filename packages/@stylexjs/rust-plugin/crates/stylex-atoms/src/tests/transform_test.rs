use rustc_hash::FxHashMap;
use swc_core::{
  common::{DUMMY_SP, GLOBALS, Globals, Mark, SyntaxContext},
  ecma::{
    ast::{
      ArrayLit, CallExpr, Callee, ComputedPropName, Expr, ExprOrSpread, Id, Ident, IdentName, Lit,
      MemberExpr, MemberProp, Number, ObjectLit, PrivateName, Str, Super,
    },
    visit::VisitMutWith,
  },
};

use crate::transform::{
  AtomCompileResult, AtomFlatValue, Compile, DynamicStyle, InjectedAtomStyle, StaticStyle,
  compile_dynamic_style, compile_static_style, create_utility_styles_visitor,
  get_dynamic_style_from_path, get_static_style_from_path, is_utility_styles_identifier,
  normalize_value,
};
use stylex_ast::ast::convertors::convert_member_prop_to_string;

fn ident(name: &str) -> Ident {
  ident_with_ctxt(name, SyntaxContext::empty())
}

fn ident_with_ctxt(name: &str, ctxt: SyntaxContext) -> Ident {
  Ident {
    span: DUMMY_SP,
    ctxt,
    sym: name.into(),
    optional: false,
  }
}

fn atom_id(name: &str) -> Id {
  ident(name).to_id()
}

fn ident_prop(name: &str) -> MemberProp {
  MemberProp::Ident(IdentName {
    span: DUMMY_SP,
    sym: name.into(),
  })
}

fn computed_str_prop(value: &str) -> MemberProp {
  MemberProp::Computed(ComputedPropName {
    span: DUMMY_SP,
    expr: Box::new(Expr::Lit(Lit::Str(Str {
      span: DUMMY_SP,
      value: value.into(),
      raw: None,
    }))),
  })
}

/// Builds `base.first.second` where `base` is an identifier.
fn two_level_member(base: &str, first: MemberProp, second: MemberProp) -> MemberExpr {
  MemberExpr {
    span: DUMMY_SP,
    obj: Box::new(Expr::Member(MemberExpr {
      span: DUMMY_SP,
      obj: Box::new(Expr::Ident(ident(base))),
      prop: first,
    })),
    prop: second,
  }
}

/// Builds `base.prop`.
fn one_level_member(base: &str, prop: MemberProp) -> MemberExpr {
  MemberExpr {
    span: DUMMY_SP,
    obj: Box::new(Expr::Ident(ident(base))),
    prop,
  }
}

fn default_imports() -> FxHashMap<Id, String> {
  let mut map = FxHashMap::default();
  map.insert(atom_id("css"), "*".to_string());
  map
}

/// Selects the shape of the flat compiled namespace returned by the mock, so a
/// single `Compile` implementation can exercise every branch of the prop-hash
/// search in `compile_dynamic_style` (keeping the generic functions to a single
/// monomorphization).
#[derive(Clone, Copy)]
enum FlatShape {
  /// `[prop-hash, $$css]` — a normal compiled namespace.
  Normal,
  /// `[$$css, devClass(key == value), prop-hash]` — the marker and dev class
  /// must both be skipped before the real property hash is found.
  DevClassLead,
  /// `[$$css, devClass(key == value)]` — no property hash at all.
  MarkerOnly,
  /// The style is not statically evaluable; `style_x_create_set` returns `None`.
  NonEvaluable,
}

struct MockCompiler {
  imports: FxHashMap<Id, String>,
  registered: Vec<Vec<InjectedAtomStyle>>,
  hoist_count: usize,
  shape: FlatShape,
}

impl MockCompiler {
  fn new(imports: FxHashMap<Id, String>) -> Self {
    Self::with_shape(imports, FlatShape::Normal)
  }

  fn with_shape(imports: FxHashMap<Id, String>, shape: FlatShape) -> Self {
    Self {
      imports,
      registered: vec![],
      hoist_count: 0,
      shape,
    }
  }
}

impl Compile for MockCompiler {
  fn atom_imports(&self) -> &FxHashMap<Id, String> {
    &self.imports
  }

  fn style_x_create_set(&mut self, property: &str, _value: &str) -> Option<AtomCompileResult> {
    if matches!(self.shape, FlatShape::NonEvaluable) {
      return None;
    }

    let class_name = format!("x{}", property);
    let prop_hash = (
      format!("k{}", property),
      AtomFlatValue::String(class_name.clone()),
    );
    let marker = ("$$css".to_string(), AtomFlatValue::Bool(true));
    let dev_class = (
      "devClass".to_string(),
      AtomFlatValue::String("devClass".to_string()),
    );
    let compiled_flat = match self.shape {
      FlatShape::Normal => vec![prop_hash, marker],
      FlatShape::DevClassLead => vec![marker, dev_class, prop_hash],
      FlatShape::MarkerOnly => vec![marker, dev_class],
      FlatShape::NonEvaluable => unreachable!("handled above"),
    };
    Some(AtomCompileResult {
      compiled_ast: Expr::Object(ObjectLit {
        span: DUMMY_SP,
        props: vec![],
      }),
      compiled_flat,
      injected: vec![InjectedAtomStyle {
        class_name,
        priority: 3000.0,
        ltr: format!(".x{}{{{}:value}}", property, property),
        rtl: None,
      }],
    })
  }

  fn register_styles(&mut self, injected: &[InjectedAtomStyle]) {
    self.registered.push(injected.to_vec());
  }

  fn hoist_expression(&mut self, _expr: Expr) -> Expr {
    self.hoist_count += 1;
    Expr::Ident(ident("_temp"))
  }
}

#[test]
fn normalize_value_strips_single_leading_underscore() {
  assert_eq!(normalize_value("_16px"), "16px");
  assert_eq!(normalize_value("_1"), "1");
  assert_eq!(normalize_value("flex"), "flex");
  assert_eq!(normalize_value("__double"), "_double");
}

#[test]
fn convert_member_prop_to_string_handles_ident_and_computed() {
  assert_eq!(
    convert_member_prop_to_string(&ident_prop("flex")).as_deref(),
    Some("flex")
  );
  assert_eq!(
    convert_member_prop_to_string(&computed_str_prop("calc(100% - 20px)")).as_deref(),
    Some("calc(100% - 20px)")
  );
  let num = MemberProp::Computed(ComputedPropName {
    span: DUMMY_SP,
    expr: Box::new(Expr::Lit(Lit::Num(Number {
      span: DUMMY_SP,
      value: 16.0,
      raw: None,
    }))),
  });
  assert_eq!(convert_member_prop_to_string(&num).as_deref(), Some("16"));
}

#[test]
fn is_utility_styles_identifier_checks_imports() {
  let imports = default_imports();
  assert!(is_utility_styles_identifier(&ident("css"), &imports));
  assert!(!is_utility_styles_identifier(&ident("styles"), &imports));
}

#[test]
fn get_static_style_default_import_two_level() {
  let imports = default_imports();
  let member = two_level_member("css", ident_prop("display"), ident_prop("flex"));
  assert_eq!(
    get_static_style_from_path(&member, &imports),
    Some(StaticStyle {
      property: "display".to_string(),
      value: "flex".to_string()
    })
  );
}

#[test]
fn get_static_style_leading_underscore_value() {
  let imports = default_imports();
  let member = two_level_member("css", ident_prop("padding"), ident_prop("_16px"));
  assert_eq!(
    get_static_style_from_path(&member, &imports),
    Some(StaticStyle {
      property: "padding".to_string(),
      value: "16px".to_string()
    })
  );
}

#[test]
fn get_static_style_computed_key_syntax() {
  let imports = default_imports();
  let member = two_level_member(
    "css",
    ident_prop("width"),
    computed_str_prop("calc(100% - 20px)"),
  );
  assert_eq!(
    get_static_style_from_path(&member, &imports),
    Some(StaticStyle {
      property: "width".to_string(),
      value: "calc(100% - 20px)".to_string(),
    })
  );
}

#[test]
fn get_static_style_named_import_single_level() {
  let mut imports = FxHashMap::default();
  imports.insert(atom_id("color"), "color".to_string());
  let member = one_level_member("color", ident_prop("blue"));
  assert_eq!(
    get_static_style_from_path(&member, &imports),
    Some(StaticStyle {
      property: "color".to_string(),
      value: "blue".to_string()
    })
  );
}

#[test]
fn import_lookup_uses_syntax_context() {
  let globals = Globals::default();
  GLOBALS.set(&globals, || {
    let import_ctxt = SyntaxContext::empty();
    let shadow_ctxt = SyntaxContext::empty().apply_mark(Mark::fresh(Mark::root()));

    let mut imports = FxHashMap::default();
    imports.insert(ident_with_ctxt("css", import_ctxt).to_id(), "*".to_string());

    let shadowed_css = ident_with_ctxt("css", shadow_ctxt);
    assert!(!is_utility_styles_identifier(&shadowed_css, &imports));
  });
}

#[test]
fn get_static_style_ignores_non_atom_identifier() {
  let imports = default_imports();
  let member = two_level_member("styles", ident_prop("display"), ident_prop("flex"));
  assert_eq!(get_static_style_from_path(&member, &imports), None);
}

fn call_member(base: &str, method: MemberProp, arg: Expr) -> CallExpr {
  CallExpr {
    span: DUMMY_SP,
    ctxt: SyntaxContext::empty(),
    callee: Callee::Expr(Box::new(Expr::Member(one_level_member(base, method)))),
    args: vec![ExprOrSpread {
      spread: None,
      expr: Box::new(arg),
    }],
    type_args: None,
  }
}

#[test]
fn get_dynamic_style_detects_single_arg_call() {
  let imports = default_imports();
  let call = call_member("css", ident_prop("color"), Expr::Ident(ident("value")));
  let dynamic = get_dynamic_style_from_path(&call, &imports);
  assert!(matches!(dynamic, Some(DynamicStyle { ref property, .. }) if property == "color"));
}

#[test]
fn get_dynamic_style_requires_exactly_one_arg() {
  let imports = default_imports();
  let mut call = call_member("css", ident_prop("color"), Expr::Ident(ident("value")));
  call.args.clear();
  assert!(get_dynamic_style_from_path(&call, &imports).is_none());
}

#[test]
fn compile_static_style_registers_and_returns_object() {
  let mut compiler = MockCompiler::new(default_imports());
  let member = two_level_member("css", ident_prop("display"), ident_prop("flex"));
  let result = compile_static_style(&mut compiler, &member);
  assert!(matches!(result, Some(Expr::Object(_))));
  assert_eq!(compiler.registered.len(), 1);
  assert_eq!(compiler.registered[0][0].class_name, "xdisplay");
}

#[test]
fn compile_dynamic_style_hoists_and_appends_property_rule() {
  let mut compiler = MockCompiler::new(default_imports());
  let call = call_member("css", ident_prop("color"), Expr::Ident(ident("value")));
  let result = compile_dynamic_style(&mut compiler, &call);

  // It hoists exactly once and returns `_temp.color(value)` for identifier-safe
  // CSS property names.
  assert_eq!(compiler.hoist_count, 1);
  let call_expr = match result {
    Some(Expr::Call(call)) => call,
    other => panic!("expected call expression, got {:?}", other),
  };
  let callee = match call_expr.callee {
    Callee::Expr(expr) => expr,
    _ => panic!("expected expression callee"),
  };
  match callee.as_ref() {
    Expr::Member(member) => {
      assert!(matches!(member.obj.as_ref(), Expr::Ident(id) if id.sym == *"_temp"));
      assert!(matches!(&member.prop, MemberProp::Ident(id) if id.sym == *"color"));
    },
    other => panic!("expected member callee, got {:?}", other),
  }

  // The injected styles include the `@property` rule at priority 0.
  let injected = &compiler.registered[0];
  let property_rule = injected
    .iter()
    .find(|style| style.class_name == "--x-color")
    .expect("missing @property rule");
  assert_eq!(property_rule.priority, 0.0);
  assert!(property_rule.ltr.contains("@property --x-color"));
}

fn str_expr(value: &str) -> Expr {
  Expr::Lit(Lit::Str(Str {
    span: DUMMY_SP,
    value: value.into(),
    raw: None,
  }))
}

/// A computed member key whose expression is a non-literal (e.g. `css[dyn]`),
/// which `convert_member_prop_to_string` cannot resolve to a name.
fn computed_ident_prop(name: &str) -> MemberProp {
  MemberProp::Computed(ComputedPropName {
    span: DUMMY_SP,
    expr: Box::new(Expr::Ident(ident(name))),
  })
}

fn computed_num_prop(value: f64, raw: Option<&str>) -> MemberProp {
  MemberProp::Computed(ComputedPropName {
    span: DUMMY_SP,
    expr: Box::new(Expr::Lit(Lit::Num(Number {
      span: DUMMY_SP,
      value,
      raw: raw.map(|r| r.into()),
    }))),
  })
}

/// Builds `base.first.second(arg)`.
fn call_two_level(base: &str, first: MemberProp, second: MemberProp, arg: Expr) -> CallExpr {
  CallExpr {
    span: DUMMY_SP,
    ctxt: SyntaxContext::empty(),
    callee: Callee::Expr(Box::new(Expr::Member(two_level_member(
      base, first, second,
    )))),
    args: vec![ExprOrSpread {
      spread: None,
      expr: Box::new(arg),
    }],
    type_args: None,
  }
}

#[test]
fn convert_member_prop_to_string_returns_none_for_computed_non_literal_and_private_name() {
  // A computed key whose expression is not a literal (e.g. `css[dynamic]`).
  let computed_ident = MemberProp::Computed(ComputedPropName {
    span: DUMMY_SP,
    expr: Box::new(Expr::Ident(ident("dynamic"))),
  });
  assert_eq!(convert_member_prop_to_string(&computed_ident), None);

  // A private name (`css.#secret`).
  let private = MemberProp::PrivateName(PrivateName {
    span: DUMMY_SP,
    name: "secret".into(),
  });
  assert_eq!(convert_member_prop_to_string(&private), None);
}

#[test]
fn convert_member_prop_to_string_numeric_renders_value_ignoring_raw() {
  // The numeric *value* is rendered, never the raw source token (matching
  // `String(prop.value)`): `css[0x10]` keys on "16", not "0x10".
  assert_eq!(
    convert_member_prop_to_string(&computed_num_prop(16.0, Some("0x10"))).as_deref(),
    Some("16")
  );
  // A whole number renders as an integer.
  assert_eq!(
    convert_member_prop_to_string(&computed_num_prop(16.0, None)).as_deref(),
    Some("16")
  );
  // A fractional number renders as a float.
  assert_eq!(
    convert_member_prop_to_string(&computed_num_prop(1.5, None)).as_deref(),
    Some("1.5")
  );
}

#[test]
fn get_static_style_namespace_single_level() {
  // `css.flex` where `css` is a namespace/default import (`*`): the property is
  // taken from the value key itself.
  let imports = default_imports();
  let member = one_level_member("css", ident_prop("flex"));
  assert_eq!(
    get_static_style_from_path(&member, &imports),
    Some(StaticStyle {
      property: "flex".to_string(),
      value: "flex".to_string()
    })
  );
}

#[test]
fn get_static_style_single_level_non_atom_returns_none() {
  let imports = default_imports();
  let member = one_level_member("styles", ident_prop("flex"));
  assert_eq!(get_static_style_from_path(&member, &imports), None);
}

#[test]
fn get_static_style_non_member_non_ident_object_returns_none() {
  // `"literal".flex` — the object is neither a member nor an identifier.
  let imports = default_imports();
  let member = MemberExpr {
    span: DUMMY_SP,
    obj: Box::new(str_expr("literal")),
    prop: ident_prop("flex"),
  };
  assert_eq!(get_static_style_from_path(&member, &imports), None);
}

#[test]
fn get_dynamic_style_returns_none_for_super_callee() {
  let imports = default_imports();
  let call = CallExpr {
    span: DUMMY_SP,
    ctxt: SyntaxContext::empty(),
    callee: Callee::Super(Super { span: DUMMY_SP }),
    args: vec![],
    type_args: None,
  };
  assert!(get_dynamic_style_from_path(&call, &imports).is_none());
}

#[test]
fn get_dynamic_style_returns_none_for_non_member_callee() {
  // `foo(value)` — the callee is a plain identifier, not a member expression.
  let imports = default_imports();
  let call = CallExpr {
    span: DUMMY_SP,
    ctxt: SyntaxContext::empty(),
    callee: Callee::Expr(Box::new(Expr::Ident(ident("foo")))),
    args: vec![ExprOrSpread {
      spread: None,
      expr: Box::new(Expr::Ident(ident("value"))),
    }],
    type_args: None,
  };
  assert!(get_dynamic_style_from_path(&call, &imports).is_none());
}

#[test]
fn get_dynamic_style_returns_none_for_spread_arg() {
  // `css.color(...value)` — a spread argument is not a single value.
  let imports = default_imports();
  let mut call = call_member("css", ident_prop("color"), Expr::Ident(ident("value")));
  call.args[0].spread = Some(DUMMY_SP);
  assert!(get_dynamic_style_from_path(&call, &imports).is_none());
}

#[test]
fn get_dynamic_style_two_level_member() {
  // `css.padding.top(value)` — the property is the inner member key.
  let imports = default_imports();
  let call = call_two_level(
    "css",
    ident_prop("padding"),
    ident_prop("top"),
    Expr::Ident(ident("value")),
  );
  let dynamic = get_dynamic_style_from_path(&call, &imports);
  assert!(matches!(dynamic, Some(DynamicStyle { ref property, .. }) if property == "padding"));
}

#[test]
fn get_dynamic_style_two_level_non_atom_returns_none() {
  let imports = default_imports();
  let call = call_two_level(
    "styles",
    ident_prop("padding"),
    ident_prop("top"),
    Expr::Ident(ident("value")),
  );
  assert!(get_dynamic_style_from_path(&call, &imports).is_none());
}

#[test]
fn get_dynamic_style_non_ident_non_member_object_returns_none() {
  // `"literal".color(value)` — the callee object is neither a member nor an ident.
  let imports = default_imports();
  let callee_member = MemberExpr {
    span: DUMMY_SP,
    obj: Box::new(str_expr("literal")),
    prop: ident_prop("color"),
  };
  let call = CallExpr {
    span: DUMMY_SP,
    ctxt: SyntaxContext::empty(),
    callee: Callee::Expr(Box::new(Expr::Member(callee_member))),
    args: vec![ExprOrSpread {
      spread: None,
      expr: Box::new(Expr::Ident(ident("value"))),
    }],
    type_args: None,
  };
  assert!(get_dynamic_style_from_path(&call, &imports).is_none());
}

#[test]
fn compile_dynamic_style_skips_marker_and_dev_class_keys() {
  // The flat namespace leads with `$$css` and a dev class name (key == value),
  // both of which must be skipped before the real property hash is found.
  let mut compiler = MockCompiler::with_shape(default_imports(), FlatShape::DevClassLead);
  let call = call_member("css", ident_prop("color"), Expr::Ident(ident("value")));
  let result = compile_dynamic_style(&mut compiler, &call);
  assert!(matches!(result, Some(Expr::Call(_))));
  assert_eq!(compiler.hoist_count, 1);
}

#[test]
fn visitor_compiles_dynamic_atom_call() {
  let mut compiler = MockCompiler::new(default_imports());
  let mut expr = Expr::Call(call_member(
    "css",
    ident_prop("color"),
    Expr::Ident(ident("value")),
  ));
  let mut visitor = create_utility_styles_visitor(&mut compiler);
  expr.visit_mut_with(&mut visitor);

  // Replaced with the hoisted `_temp.color(value)` call.
  assert_eq!(compiler.hoist_count, 1);
  assert!(matches!(&expr, Expr::Call(call) if matches!(call.callee, Callee::Expr(_))));
}

#[test]
fn visitor_compiles_static_atom_member() {
  let mut compiler = MockCompiler::new(default_imports());
  let mut expr = Expr::Member(two_level_member(
    "css",
    ident_prop("display"),
    ident_prop("flex"),
  ));
  let mut visitor = create_utility_styles_visitor(&mut compiler);
  expr.visit_mut_with(&mut visitor);

  // Replaced in place with the compiled style object.
  assert!(matches!(&expr, Expr::Object(_)));
  assert_eq!(compiler.registered.len(), 1);
}

#[test]
fn visitor_leaves_non_atom_member_untouched() {
  let mut compiler = MockCompiler::new(default_imports());
  // `styles.foo` is not an atom import: `compile_static_style` returns `None`,
  // so the member is left as-is and its children are still traversed.
  let mut expr = Expr::Member(one_level_member("styles", ident_prop("foo")));
  let mut visitor = create_utility_styles_visitor(&mut compiler);
  expr.visit_mut_with(&mut visitor);

  assert!(
    matches!(&expr, Expr::Member(member) if matches!(member.obj.as_ref(), Expr::Ident(id) if id.sym == *"styles"))
  );
  assert_eq!(compiler.registered.len(), 0);
}

#[test]
fn visitor_visits_nested_args_without_compiling_the_callee() {
  let mut compiler = MockCompiler::new(default_imports());
  // `foo([css.display.flex])` — `foo` is not an atom, so the callee is skipped
  // while the static atom nested inside the array argument is compiled.
  let array = Expr::Array(ArrayLit {
    span: DUMMY_SP,
    elems: vec![Some(ExprOrSpread {
      spread: None,
      expr: Box::new(Expr::Member(two_level_member(
        "css",
        ident_prop("display"),
        ident_prop("flex"),
      ))),
    })],
  });
  let mut expr = Expr::Call(CallExpr {
    span: DUMMY_SP,
    ctxt: SyntaxContext::empty(),
    callee: Callee::Expr(Box::new(Expr::Ident(ident("foo")))),
    args: vec![ExprOrSpread {
      spread: None,
      expr: Box::new(array),
    }],
    type_args: None,
  });
  let mut visitor = create_utility_styles_visitor(&mut compiler);
  expr.visit_mut_with(&mut visitor);

  let call = match &expr {
    Expr::Call(call) => call,
    other => panic!("expected call expression, got {:?}", other),
  };
  // The callee identifier is untouched.
  assert!(
    matches!(&call.callee, Callee::Expr(callee) if matches!(callee.as_ref(), Expr::Ident(id) if id.sym == *"foo"))
  );
  // The nested atom inside the array argument was compiled to an object.
  match call.args[0].expr.as_ref() {
    Expr::Array(arr) => assert!(matches!(
      arr.elems[0].as_ref().unwrap().expr.as_ref(),
      Expr::Object(_)
    )),
    other => panic!("expected array argument, got {:?}", other),
  }
  assert_eq!(compiler.registered.len(), 1);
}

#[test]
fn visitor_leaves_non_expression_callee_untouched() {
  let mut compiler = MockCompiler::new(default_imports());
  let mut expr = Expr::Call(CallExpr {
    span: DUMMY_SP,
    ctxt: SyntaxContext::empty(),
    callee: Callee::Super(Super { span: DUMMY_SP }),
    args: vec![],
    type_args: None,
  });

  let mut visitor = create_utility_styles_visitor(&mut compiler);
  expr.visit_mut_with(&mut visitor);

  let call = match &expr {
    Expr::Call(call) => call,
    other => panic!("expected call expression, got {:?}", other),
  };
  assert!(matches!(&call.callee, Callee::Super(_)));
  assert_eq!(compiler.registered.len(), 0);
  assert_eq!(compiler.hoist_count, 0);
}

#[test]
fn visitor_leaves_bad_arity_two_level_atom_callee_untouched() {
  let mut compiler = MockCompiler::new(default_imports());
  let mut call = call_two_level(
    "css",
    ident_prop("display"),
    ident_prop("flex"),
    Expr::Ident(ident("first")),
  );
  call.args.push(ExprOrSpread {
    spread: None,
    expr: Box::new(Expr::Ident(ident("second"))),
  });
  let mut expr = Expr::Call(call);

  let mut visitor = create_utility_styles_visitor(&mut compiler);
  expr.visit_mut_with(&mut visitor);

  let call = match &expr {
    Expr::Call(call) => call,
    other => panic!("expected call expression, got {:?}", other),
  };
  let callee = match &call.callee {
    Callee::Expr(callee) => callee,
    other => panic!("expected expression callee, got {:?}", other),
  };

  match callee.as_ref() {
    Expr::Member(member) => match member.obj.as_ref() {
      Expr::Member(parent) => {
        assert!(matches!(parent.obj.as_ref(), Expr::Ident(id) if id.sym == *"css"));
        assert!(matches!(&parent.prop, MemberProp::Ident(id) if id.sym == *"display"));
        assert!(matches!(&member.prop, MemberProp::Ident(id) if id.sym == *"flex"));
      },
      other => panic!("expected nested member object, got {:?}", other),
    },
    other => panic!("expected member callee, got {:?}", other),
  }
  assert_eq!(compiler.registered.len(), 0);
  assert_eq!(compiler.hoist_count, 0);
}

#[test]
fn visitor_compiles_atoms_nested_in_computed_callee_keys() {
  let mut compiler = MockCompiler::new(default_imports());
  let computed_key = Expr::Member(two_level_member(
    "css",
    ident_prop("display"),
    ident_prop("flex"),
  ));
  let mut expr = Expr::Call(CallExpr {
    span: DUMMY_SP,
    ctxt: SyntaxContext::empty(),
    callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
      span: DUMMY_SP,
      obj: Box::new(Expr::Ident(ident("handlers"))),
      prop: MemberProp::Computed(ComputedPropName {
        span: DUMMY_SP,
        expr: Box::new(computed_key),
      }),
    }))),
    args: vec![],
    type_args: None,
  });

  let mut visitor = create_utility_styles_visitor(&mut compiler);
  expr.visit_mut_with(&mut visitor);

  let call = match &expr {
    Expr::Call(call) => call,
    other => panic!("expected call expression, got {:?}", other),
  };
  let callee = match &call.callee {
    Callee::Expr(callee) => callee,
    other => panic!("expected expression callee, got {:?}", other),
  };

  match callee.as_ref() {
    Expr::Member(member) => match &member.prop {
      MemberProp::Computed(computed) => assert!(matches!(computed.expr.as_ref(), Expr::Object(_))),
      other => panic!("expected computed property, got {:?}", other),
    },
    other => panic!("expected member callee, got {:?}", other),
  }
  assert_eq!(compiler.registered.len(), 1);
}

#[test]
fn get_static_style_returns_none_for_unkeyable_value_prop() {
  // `css[dyn]` — the value key cannot be resolved, so detection bails out.
  let imports = default_imports();
  let member = one_level_member("css", computed_ident_prop("dyn"));
  assert_eq!(get_static_style_from_path(&member, &imports), None);
}

#[test]
fn get_static_style_returns_none_for_unkeyable_parent_prop() {
  // `css[dyn].flex` — the inner property key cannot be resolved.
  let imports = default_imports();
  let member = two_level_member("css", computed_ident_prop("dyn"), ident_prop("flex"));
  assert_eq!(get_static_style_from_path(&member, &imports), None);
}

#[test]
fn get_dynamic_style_returns_none_for_unkeyable_callee_prop() {
  // `css[dyn](value)` — the callee property key cannot be resolved.
  let imports = default_imports();
  let callee_member = one_level_member("css", computed_ident_prop("dyn"));
  let call = CallExpr {
    span: DUMMY_SP,
    ctxt: SyntaxContext::empty(),
    callee: Callee::Expr(Box::new(Expr::Member(callee_member))),
    args: vec![ExprOrSpread {
      spread: None,
      expr: Box::new(Expr::Ident(ident("value"))),
    }],
    type_args: None,
  };
  assert!(get_dynamic_style_from_path(&call, &imports).is_none());
}

#[test]
fn get_dynamic_style_returns_none_for_unkeyable_parent_prop() {
  // `css[dyn].b(value)` — the inner property key cannot be resolved.
  let imports = default_imports();
  let call = call_two_level(
    "css",
    computed_ident_prop("dyn"),
    ident_prop("b"),
    Expr::Ident(ident("value")),
  );
  assert!(get_dynamic_style_from_path(&call, &imports).is_none());
}

#[test]
fn compile_dynamic_style_returns_none_without_property_hash() {
  // The flat namespace has only the `$$css` marker and a dev class name, so the
  // prop-hash search finds nothing and the dynamic atom bails out.
  let mut compiler = MockCompiler::with_shape(default_imports(), FlatShape::MarkerOnly);
  let call = call_member("css", ident_prop("color"), Expr::Ident(ident("value")));
  assert!(compile_dynamic_style(&mut compiler, &call).is_none());
}

#[test]
fn compile_dynamic_style_does_not_register_styles_without_property_hash() {
  let mut compiler = MockCompiler::with_shape(default_imports(), FlatShape::MarkerOnly);
  let call = call_member("css", ident_prop("color"), Expr::Ident(ident("value")));

  assert!(compile_dynamic_style(&mut compiler, &call).is_none());
  assert!(compiler.registered.is_empty());
  assert_eq!(compiler.hoist_count, 0);
}

#[test]
fn compile_static_style_returns_none_when_not_evaluable() {
  // `style_x_create_set` returns `None` (the inline style is not statically
  // evaluable): `compile_static_style` must bail out instead of panicking.
  let mut compiler = MockCompiler::with_shape(default_imports(), FlatShape::NonEvaluable);
  let member = two_level_member("css", ident_prop("display"), ident_prop("flex"));
  assert!(compile_static_style(&mut compiler, &member).is_none());
  assert_eq!(compiler.registered.len(), 0);
}

#[test]
fn compile_dynamic_style_returns_none_when_not_evaluable() {
  let mut compiler = MockCompiler::with_shape(default_imports(), FlatShape::NonEvaluable);
  let call = call_member("css", ident_prop("color"), Expr::Ident(ident("value")));
  assert!(compile_dynamic_style(&mut compiler, &call).is_none());
  assert_eq!(compiler.hoist_count, 0);
}

#[test]
fn visitor_leaves_non_evaluable_atom_untouched() {
  // A non-evaluable static atom is left exactly as written, so it survives to
  // runtime rather than crashing the compiler.
  let mut compiler = MockCompiler::with_shape(default_imports(), FlatShape::NonEvaluable);
  let mut expr = Expr::Member(two_level_member(
    "css",
    ident_prop("display"),
    ident_prop("flex"),
  ));
  let mut visitor = create_utility_styles_visitor(&mut compiler);
  expr.visit_mut_with(&mut visitor);

  assert!(matches!(&expr, Expr::Member(_)));
  assert_eq!(compiler.registered.len(), 0);
}

#[test]
fn compile_dynamic_style_uses_verbatim_custom_property_name() {
  let mut compiler = MockCompiler::new(default_imports());
  let call = call_member(
    "css",
    computed_str_prop("accent-color"),
    Expr::Ident(ident("value")),
  );
  let result = compile_dynamic_style(&mut compiler, &call);
  assert!(matches!(result, Some(Expr::Call(_))));

  let injected = &compiler.registered[0];
  let custom_property_name = "--x-accent-color";
  let property_rule = injected
    .iter()
    .find(|style| style.class_name == custom_property_name)
    .expect("missing @property rule");
  assert!(
    property_rule
      .ltr
      .contains(&format!("@property {custom_property_name}"))
  );

  let call_expr = match result {
    Some(Expr::Call(call)) => call,
    other => panic!("expected call expression, got {:?}", other),
  };
  let callee = match call_expr.callee {
    Callee::Expr(expr) => expr,
    _ => panic!("expected expression callee"),
  };
  match callee.as_ref() {
    Expr::Member(member) => assert!(matches!(&member.prop, MemberProp::Computed(_))),
    other => panic!("expected member callee, got {:?}", other),
  }
}

#[test]
fn compile_dynamic_style_rejects_unsafe_computed_property_name() {
  let mut compiler = MockCompiler::new(default_imports());
  let call = call_member(
    "css",
    computed_str_prop("color; } body {"),
    Expr::Ident(ident("value")),
  );

  assert!(compile_dynamic_style(&mut compiler, &call).is_none());
  assert!(compiler.registered.is_empty());
  assert_eq!(compiler.hoist_count, 0);
}

#[test]
fn compile_dynamic_style_emits_identifier_members_for_dollar_and_underscore_properties() {
  for property in ["_color", "$color"] {
    let mut compiler = MockCompiler::new(default_imports());
    let call = call_member(
      "css",
      computed_str_prop(property),
      Expr::Ident(ident("value")),
    );

    let result = compile_dynamic_style(&mut compiler, &call);
    let call_expr = match result {
      Some(Expr::Call(call)) => call,
      other => panic!("expected call expression, got {:?}", other),
    };
    let callee = match call_expr.callee {
      Callee::Expr(expr) => expr,
      _ => panic!("expected expression callee"),
    };

    match callee.as_ref() {
      Expr::Member(member) => {
        assert!(matches!(&member.prop, MemberProp::Ident(id) if id.sym == *property));
      },
      other => panic!("expected member callee, got {:?}", other),
    }
  }
}

#[test]
fn compile_dynamic_style_emits_computed_member_for_invalid_identifier_start() {
  let mut compiler = MockCompiler::new(default_imports());
  let call = call_member(
    "css",
    computed_str_prop("1color"),
    Expr::Ident(ident("value")),
  );

  let result = compile_dynamic_style(&mut compiler, &call);
  let call_expr = match result {
    Some(Expr::Call(call)) => call,
    other => panic!("expected call expression, got {:?}", other),
  };
  let callee = match call_expr.callee {
    Callee::Expr(expr) => expr,
    _ => panic!("expected expression callee"),
  };

  match callee.as_ref() {
    Expr::Member(member) => assert!(matches!(&member.prop, MemberProp::Computed(_))),
    other => panic!("expected member callee, got {:?}", other),
  }
}

#[test]
fn convert_member_prop_to_string_numeric_renders_js_edge_values() {
  // Non-finite numeric keys render exactly as JS `String(Number)` does, never
  // a saturated integer cast or Rust's `inf`/`NaN` lowercase spellings.
  assert_eq!(
    convert_member_prop_to_string(&computed_num_prop(f64::NAN, None)).as_deref(),
    Some("NaN")
  );
  assert_eq!(
    convert_member_prop_to_string(&computed_num_prop(f64::INFINITY, None)).as_deref(),
    Some("Infinity")
  );
  assert_eq!(
    convert_member_prop_to_string(&computed_num_prop(f64::NEG_INFINITY, None)).as_deref(),
    Some("-Infinity")
  );
  // A magnitude beyond the safe-integer range stays on the float path rather
  // than going through a saturating `as i64` cast, and renders in JS's
  // exponential spelling — the key the property is actually looked up by.
  assert_eq!(
    convert_member_prop_to_string(&computed_num_prop(1e21, None)).as_deref(),
    Some("1e+21")
  );
}
