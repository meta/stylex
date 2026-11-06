use rustc_hash::FxHashMap;
use swc_core::{
  common::{DUMMY_SP, SyntaxContext},
  ecma::{
    ast::{
      CallExpr, Callee, ComputedPropName, Expr, ExprOrSpread, Id, Ident, IdentName, Lit,
      MemberExpr, MemberProp, ObjectLit, Str, Super,
    },
    visit::VisitMutWith,
  },
};

use crate::transform::{
  AtomCompileResult, AtomFlatValue, Compile, DynamicStyle, InjectedAtomStyle,
  compile_dynamic_style, create_utility_styles_visitor, get_dynamic_style_from_path,
  get_static_style_from_path,
};

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

/// The flat namespace shape the mock returns, covering each branch of the prop-hash search.
#[derive(Clone, Copy)]
enum FlatShape {
  /// `[prop-hash, $$css]` — a normal compiled namespace.
  Normal,
  /// `[$$css, devClass(key == value), prop-hash]` — both must be skipped.
  DevClassLead,
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

fn str_expr(value: &str) -> Expr {
  Expr::Lit(Lit::Str(Str {
    span: DUMMY_SP,
    value: value.into(),
    raw: None,
  }))
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
fn compile_dynamic_style_skips_marker_and_dev_class_keys() {
  // `$$css` and a dev class lead the namespace and must be skipped.
  let mut compiler = MockCompiler::with_shape(default_imports(), FlatShape::DevClassLead);
  let call = call_member("css", ident_prop("color"), Expr::Ident(ident("value")));
  let result = compile_dynamic_style(&mut compiler, &call);
  assert!(matches!(result, Some(Expr::Call(_))));
  assert_eq!(compiler.hoist_count, 1);
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
fn visitor_leaves_non_evaluable_atom_untouched() {
  // A non-evaluable static atom is left as written for the runtime.
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
