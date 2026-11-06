#[cfg(test)]
mod state_manager {
  use swc_core::{
    atoms::Atom,
    common::{BytePos, DUMMY_SP, EqIgnoreSpan, Span, SyntaxContext},
    ecma::ast::{
      ArrayLit, CallExpr, Callee, Decl, Expr, ExprOrSpread, ExprStmt, Ident, ImportDecl,
      ImportDefaultSpecifier, ImportNamedSpecifier, ImportPhase, ImportSpecifier,
      ImportStarAsSpecifier, KeyValueProp, Lit, ModuleDecl, ModuleItem, ObjectLit, ObjectPat,
      ParenExpr, Pat, Prop, PropOrSpread, SpreadElement, Stmt, Str, VarDecl, VarDeclKind,
      VarDeclarator,
    },
  };

  use crate::call_positions::{CallPositions, Position};
  use crate::state_manager::{InsertionSlot, StateManager, flush_pending_insertions};
  use crate::tests::prelude::{
    ident, ident_at, make_var_declarator, make_var_declarator_no_init, string_expr,
  };
  use stylex_ast::ast::convertors::convert_string_to_prop_name;
  use stylex_enums::declaration_type::DeclarationType;
  use stylex_enums::top_level_expression::TopLevelExpressionKind;
  use stylex_structures::ceiling::Ceiling;
  use stylex_structures::evaluation_depth::MAX_EVALUATION_DEPTH;
  use stylex_structures::fold_ceilings::{MAX_FOLDED_CHARACTERS, MAX_FOLDED_ENTRIES};
  use stylex_structures::top_level_expression::TopLevelExpression;
  use stylex_utils::hash::stable_hash_unspanned;

  fn ident_expr(name: &str) -> Expr {
    Expr::Ident(ident(name))
  }

  fn expr_stmt(value: &str) -> ModuleItem {
    ModuleItem::Stmt(Stmt::Expr(ExprStmt {
      span: DUMMY_SP,
      expr: Box::new(string_expr(value)),
    }))
  }

  fn import_stmt(source: &str) -> ModuleItem {
    ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
      span: DUMMY_SP,
      specifiers: vec![],
      src: Box::new(Str {
        span: DUMMY_SP,
        value: source.into(),
        raw: None,
      }),
      type_only: false,
      with: None,
      phase: ImportPhase::Evaluation,
    }))
  }

  fn var_decl_item(name: &str, init: Expr) -> ModuleItem {
    ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      kind: VarDeclKind::Const,
      declare: false,
      decls: vec![make_var_declarator(name, init)],
    }))))
  }

  fn item_label(item: &ModuleItem) -> String {
    match item {
      ModuleItem::Stmt(Stmt::Expr(expr_stmt)) => match expr_stmt.expr.as_lit() {
        Some(Lit::Str(value)) => value.value.as_str().unwrap_or("").to_string(),
        _ => "expr".to_string(),
      },
      ModuleItem::Stmt(Stmt::Decl(Decl::Var(var_decl))) => {
        let decl = &var_decl.decls[0];
        match decl.name.as_ident() {
          Some(binding) => format!("var:{}", binding.id.sym),
          None => "var:pattern".to_string(),
        }
      },
      ModuleItem::ModuleDecl(ModuleDecl::Import(import)) => {
        format!("import:{}", import.src.value.as_str().unwrap_or(""))
      },
      _ => "other".to_string(),
    }
  }

  fn item_labels(items: &[ModuleItem]) -> Vec<String> {
    items.iter().map(item_label).collect()
  }

  #[test]
  fn flush_pending_insertions_places_each_slot_in_emit_order() {
    let mut state = StateManager::default();
    let styles_init = Expr::Object(ObjectLit {
      span: DUMMY_SP,
      props: vec![],
    });
    let before_decl_hash = stable_hash_unspanned(&styles_init);
    let mut body = vec![
      expr_stmt("use strict"),
      import_stmt("existing"),
      var_decl_item("styles", styles_init),
    ];

    state.queue_insertion(InsertionSlot::AfterImports, expr_stmt("after_imports"));
    state.queue_insertion(
      InsertionSlot::BeforeDecl(before_decl_hash),
      expr_stmt("before_decl"),
    );
    state.queue_insertion(InsertionSlot::ThemeImports, import_stmt("theme"));
    state.queue_insertion(InsertionSlot::BeforeImports, expr_stmt("before_imports"));

    flush_pending_insertions(&mut state, &mut body, true);

    assert_eq!(
      item_labels(&body),
      vec![
        "use strict",
        "before_imports",
        "import:theme",
        "import:existing",
        "after_imports",
        "before_decl",
        "var:styles",
      ]
    );
  }

  #[test]
  fn flush_pending_insertions_drops_runtime_gated_slots_when_runtime_injection_is_disabled() {
    let mut state = StateManager::default();
    let styles_init = Expr::Object(ObjectLit {
      span: DUMMY_SP,
      props: vec![],
    });
    let before_decl_hash = stable_hash_unspanned(&styles_init);
    let mut body = vec![
      expr_stmt("use strict"),
      import_stmt("existing"),
      var_decl_item("styles", styles_init),
    ];

    state.queue_insertion(InsertionSlot::AfterImports, expr_stmt("after_imports"));
    state.queue_insertion(
      InsertionSlot::BeforeDecl(before_decl_hash),
      expr_stmt("before_decl"),
    );
    state.queue_insertion(InsertionSlot::ThemeImports, import_stmt("theme"));
    state.queue_insertion(InsertionSlot::BeforeImports, expr_stmt("before_imports"));

    flush_pending_insertions(&mut state, &mut body, false);

    assert_eq!(
      item_labels(&body),
      vec![
        "use strict",
        "import:existing",
        "after_imports",
        "var:styles",
      ]
    );
  }

  /// An array literal holding `elements`, the shape a module writes when it
  /// binds every style it declares to one name.
  fn array_expr(elements: Vec<Expr>) -> Expr {
    Expr::Array(ArrayLit {
      span: DUMMY_SP,
      elems: elements
        .into_iter()
        .map(|expr| {
          Some(ExprOrSpread {
            spread: None,
            expr: Box::new(expr),
          })
        })
        .collect(),
    })
  }

  fn paren_expr(inner: Expr) -> Expr {
    Expr::Paren(ParenExpr {
      span: DUMMY_SP,
      expr: Box::new(inner),
    })
  }

  fn empty_object() -> Expr {
    Expr::Object(ObjectLit {
      span: DUMMY_SP,
      props: vec![],
    })
  }

  /// The styles a `create` inside a top-level array leaves behind sit one level
  /// down from the declarator, so the injection call is keyed to the object and
  /// the initializer is the array. It still belongs before the statement.
  #[test]
  fn flush_pending_insertions_looks_through_an_array_initializer() {
    let mut state = StateManager::default();
    let styles_init = empty_object();
    let before_decl_hash = stable_hash_unspanned(&styles_init);
    let mut body = vec![var_decl_item("styles", array_expr(vec![styles_init]))];

    state.queue_insertion(
      InsertionSlot::BeforeDecl(before_decl_hash),
      expr_stmt("before_decl"),
    );

    flush_pending_insertions(&mut state, &mut body, true);

    assert_eq!(item_labels(&body), vec!["before_decl", "var:styles"]);
  }

  /// Arrays nest, and a parenthesis is not a different initializer, so both are
  /// read through at every level.
  #[test]
  fn flush_pending_insertions_looks_through_nested_arrays_and_parentheses() {
    let mut state = StateManager::default();
    let styles_init = empty_object();
    let before_decl_hash = stable_hash_unspanned(&styles_init);
    let initializer = paren_expr(array_expr(vec![array_expr(vec![paren_expr(styles_init)])]));
    let mut body = vec![var_decl_item("styles", initializer)];

    state.queue_insertion(
      InsertionSlot::BeforeDecl(before_decl_hash),
      expr_stmt("before_decl"),
    );

    flush_pending_insertions(&mut state, &mut body, true);

    assert_eq!(item_labels(&body), vec!["before_decl", "var:styles"]);
  }

  /// An array holds what the author put in it. A value that can hold no styles
  /// is stepped over, and a string -- the shape a compiled `keyframes` leaves --
  /// is read like any other initializer.
  #[test]
  fn flush_pending_insertions_reads_every_kind_of_array_element() {
    let mut state = StateManager::default();
    let name_init = string_expr("x1e2nbdu-B");
    let before_decl_hash = stable_hash_unspanned(&name_init);
    let initializer = array_expr(vec![ident_expr("label"), empty_object(), name_init]);
    let mut body = vec![var_decl_item("styles", initializer)];

    state.queue_insertion(
      InsertionSlot::BeforeDecl(before_decl_hash),
      expr_stmt("before_decl"),
    );

    flush_pending_insertions(&mut state, &mut body, true);

    assert_eq!(item_labels(&body), vec!["before_decl", "var:styles"]);
  }

  /// An object literal holding `entries`, the shape an author writes to group
  /// the styles a module declares.
  fn object_holding(entries: Vec<PropOrSpread>) -> Expr {
    Expr::Object(ObjectLit {
      span: DUMMY_SP,
      props: entries,
    })
  }

  fn named_prop(key: &str, value: Expr) -> PropOrSpread {
    PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
      key: convert_string_to_prop_name(key),
      value: Box::new(value),
    })))
  }

  fn spread_prop(value: Expr) -> PropOrSpread {
    PropOrSpread::Spread(SpreadElement {
      dot3_token: DUMMY_SP,
      expr: Box::new(value),
    })
  }

  fn shorthand_prop(name: &str) -> PropOrSpread {
    PropOrSpread::Prop(Box::new(Prop::Shorthand(ident(name))))
  }

  /// An array can hold the styles inside an object the author wrote, so the
  /// walk looks inside an object a container holds.
  #[test]
  fn flush_pending_insertions_looks_inside_an_object_an_array_holds() {
    let mut state = StateManager::default();
    let styles_init = empty_object();
    let before_decl_hash = stable_hash_unspanned(&styles_init);
    let initializer = array_expr(vec![object_holding(vec![named_prop("s", styles_init)])]);
    let mut body = vec![var_decl_item("styles", initializer)];

    state.queue_insertion(
      InsertionSlot::BeforeDecl(before_decl_hash),
      expr_stmt("before_decl"),
    );

    flush_pending_insertions(&mut state, &mut body, true);

    assert_eq!(item_labels(&body), vec!["before_decl", "var:styles"]);
  }

  /// A spread is read like the value it stands for, and a parenthesis around
  /// that value is not a different value.
  ///
  /// The shorthand beside them carries a name rather than an expression. It
  /// cannot change the answer, because a name is not a candidate wherever the
  /// walk meets it -- it is here to show the walk steps over what it cannot
  /// read instead of stopping.
  #[test]
  fn flush_pending_insertions_reads_a_spread_and_steps_over_a_shorthand() {
    let mut state = StateManager::default();
    let styles_init = string_expr("x1e2nbdu-B");
    let before_decl_hash = stable_hash_unspanned(&styles_init);
    let held = object_holding(vec![
      shorthand_prop("label"),
      spread_prop(paren_expr(object_holding(vec![named_prop(
        "name",
        styles_init,
      )]))),
    ]);
    let mut body = vec![var_decl_item("styles", array_expr(vec![held]))];

    state.queue_insertion(
      InsertionSlot::BeforeDecl(before_decl_hash),
      expr_stmt("before_decl"),
    );

    flush_pending_insertions(&mut state, &mut body, true);

    assert_eq!(item_labels(&body), vec!["before_decl", "var:styles"]);
  }

  /// An initializer that is itself an object is read like any other container:
  /// an author can group the styles a module declares under one object, and the
  /// rules still belong to the statement.
  #[test]
  fn flush_pending_insertions_looks_inside_an_object_initializer() {
    let mut state = StateManager::default();
    let styles_init = empty_object();
    let before_decl_hash = stable_hash_unspanned(&styles_init);
    let mut body = vec![var_decl_item(
      "styles",
      object_holding(vec![named_prop("s", styles_init)]),
    )];

    state.queue_insertion(
      InsertionSlot::BeforeDecl(before_decl_hash),
      expr_stmt("before_decl"),
    );

    flush_pending_insertions(&mut state, &mut body, true);

    assert_eq!(item_labels(&body), vec!["before_decl", "var:styles"]);
  }

  /// The walk stops at the object it matched. Nothing below a registered object
  /// is another registered object -- what is below one is the namespaces of the
  /// styles that matched -- so a large top-level object keeps the one hash it
  /// has always cost. The inner queue below is a shape no producer writes, and
  /// it is here to hold that boundary.
  #[test]
  fn flush_pending_insertions_stops_at_the_object_it_matched() {
    let mut state = StateManager::default();
    let inner = empty_object();
    let outer = object_holding(vec![named_prop("s", inner.clone())]);
    let mut body = vec![var_decl_item("styles", outer.clone())];

    state.queue_insertion(
      InsertionSlot::BeforeDecl(stable_hash_unspanned(&outer)),
      expr_stmt("outer"),
    );
    state.queue_insertion(
      InsertionSlot::BeforeDecl(stable_hash_unspanned(&inner)),
      expr_stmt("inner"),
    );

    flush_pending_insertions(&mut state, &mut body, true);

    assert_eq!(item_labels(&body), vec!["outer", "var:styles"]);
  }

  /// A declarator can carry no initializer at all. It names no declaration the
  /// injection call can land before, and the walk steps over it.
  #[test]
  fn flush_pending_insertions_steps_over_a_declarator_with_no_initializer() {
    let mut state = StateManager::default();
    let styles_init = empty_object();
    let before_decl_hash = stable_hash_unspanned(&styles_init);
    let mut body = vec![
      ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        kind: VarDeclKind::Let,
        declare: false,
        decls: vec![make_var_declarator_no_init("bare")],
      })))),
      var_decl_item("styles", styles_init),
    ];

    state.queue_insertion(
      InsertionSlot::BeforeDecl(before_decl_hash),
      expr_stmt("before_decl"),
    );

    flush_pending_insertions(&mut state, &mut body, true);

    assert_eq!(
      item_labels(&body),
      vec!["var:bare", "before_decl", "var:styles"]
    );
  }

  /// A hole holds nothing, so the walk steps over it and the queued call lands
  /// before the statement on the strength of the element beside it.
  #[test]
  fn flush_pending_insertions_steps_over_an_array_hole() {
    let mut state = StateManager::default();
    let styles_init = empty_object();
    let before_decl_hash = stable_hash_unspanned(&styles_init);
    let initializer = Expr::Array(ArrayLit {
      span: DUMMY_SP,
      elems: vec![
        None,
        Some(ExprOrSpread {
          spread: None,
          expr: Box::new(styles_init),
        }),
      ],
    });
    let mut body = vec![var_decl_item("styles", initializer)];

    state.queue_insertion(
      InsertionSlot::BeforeDecl(before_decl_hash),
      expr_stmt("before_decl"),
    );

    flush_pending_insertions(&mut state, &mut body, true);

    assert_eq!(item_labels(&body), vec!["before_decl", "var:styles"]);
  }

  /// A spread stands for the elements it holds, so `[...[styles]]` names the
  /// same declaration `[styles]` does.
  #[test]
  fn flush_pending_insertions_reads_a_spread_like_what_it_spreads() {
    let mut state = StateManager::default();
    let styles_init = empty_object();
    let before_decl_hash = stable_hash_unspanned(&styles_init);
    let initializer = Expr::Array(ArrayLit {
      span: DUMMY_SP,
      elems: vec![Some(ExprOrSpread {
        spread: Some(DUMMY_SP),
        expr: Box::new(array_expr(vec![styles_init])),
      })],
    });
    let mut body = vec![var_decl_item("styles", initializer)];

    state.queue_insertion(
      InsertionSlot::BeforeDecl(before_decl_hash),
      expr_stmt("before_decl"),
    );

    flush_pending_insertions(&mut state, &mut body, true);

    assert_eq!(item_labels(&body), vec!["before_decl", "var:styles"]);
  }

  /// A zero-argument call, the shape every `defineMarker()` shares, carrying
  /// only the position that tells two of them apart.
  fn call_at(span: Span) -> CallExpr {
    CallExpr {
      span,
      ctxt: SyntaxContext::empty(),
      callee: Callee::Expr(Box::new(ident_expr("defineMarker"))),
      args: vec![],
      type_args: None,
    }
  }

  fn span_at(lo: u32, hi: u32) -> Span {
    Span {
      lo: BytePos(lo),
      hi: BytePos(hi),
    }
  }

  #[test]
  fn find_top_level_expr_by_span_pins_the_entry_recorded_from_that_call() {
    let mut state = StateManager::default();

    let first = call_at(span_at(1, 10));
    let second = call_at(span_at(20, 30));

    state.push_top_level_expression(TopLevelExpression(
      TopLevelExpressionKind::NamedExport,
      Expr::Call(first.clone()),
      Some("first".into()),
    ));
    state.push_top_level_expression(TopLevelExpression(
      TopLevelExpressionKind::Stmt,
      Expr::Call(second.clone()),
      Some("second".into()),
    ));

    // The two calls are equal ignoring spans, so only the span tells the
    // entries apart.
    assert_eq!(
      state.find_top_level_expr_by_span(&second).map(|tpe| &tpe.0),
      Some(&TopLevelExpressionKind::Stmt)
    );
    assert_eq!(
      state.find_top_level_expr_by_span(&first).map(|tpe| &tpe.0),
      Some(&TopLevelExpressionKind::NamedExport)
    );
    assert!(
      state
        .find_top_level_expr_by_span(&call_at(span_at(40, 50)))
        .is_none()
    );
  }

  #[test]
  fn find_top_level_expr_by_span_ignores_a_non_call_entry_at_the_same_position() {
    let mut state = StateManager::default();

    let call = call_at(span_at(1, 10));

    state.push_top_level_expression(TopLevelExpression(
      TopLevelExpressionKind::NamedExport,
      Expr::Ident(Ident {
        span: span_at(1, 10),
        sym: "notACall".into(),
        optional: false,
        ctxt: SyntaxContext::empty(),
      }),
      Some("notACall".into()),
    ));

    // The span is only ever consulted to tell two recorded *calls* apart, so a
    // position match on some other expression is not a match.
    assert!(state.find_top_level_expr_by_span(&call).is_none());
  }

  #[test]
  fn find_top_level_expr_by_span_never_matches_a_spanless_call() {
    let mut state = StateManager::default();

    state.push_top_level_expression(TopLevelExpression(
      TopLevelExpressionKind::NamedExport,
      Expr::Call(call_at(DUMMY_SP)),
      Some("synthesized".into()),
    ));

    assert!(
      state
        .find_top_level_expr_by_span(&call_at(DUMMY_SP))
        .is_none()
    );
  }

  /// A crate above reads the list through the accessor, and the accessor gives
  /// back what `push_declaration` wrote, in source order. The field itself is
  /// `pub(crate)`, so a crate above cannot grow the list past the writer that
  /// keeps `declaration_index` in step with it.
  #[test]
  fn declarations_gives_back_what_push_declaration_wrote() {
    let mut state = StateManager::default();

    assert!(state.declarations().is_empty());

    state.push_declaration(make_var_declarator("first", ident_expr("a")));
    state.push_declaration(make_var_declarator("second", ident_expr("b")));

    let names: Vec<String> = state
      .declarations()
      .iter()
      .filter_map(|declarator| declarator.name.as_ident())
      .map(|binding| binding.id.sym.to_string())
      .collect();

    assert_eq!(names, vec!["first", "second"]);
  }

  #[test]
  fn find_call_declaration_index_by_span_pins_the_declarator_that_call_initialises() {
    let mut state = StateManager::default();

    let first = call_at(span_at(1, 10));
    let second = call_at(span_at(20, 30));

    state
      .declarations
      .push(make_var_declarator("first", Expr::Call(first.clone())));
    state
      .declarations
      .push(make_var_declarator("second", Expr::Call(second.clone())));
    state
      .declarations
      .push(make_var_declarator("notACall", ident_expr("other")));

    assert_eq!(state.find_call_declaration_index_by_span(&second), Some(1));
    assert_eq!(state.find_call_declaration_index_by_span(&first), Some(0));
    assert_eq!(
      state.find_call_declaration_index_by_span(&call_at(span_at(40, 50))),
      None
    );
  }

  /// A parenthesis is not a different initializer, so a call wrapped in one is
  /// found by the same lookups the bare call is found by.
  ///
  /// Both indexes, because they are keyed differently and a paren blinded each:
  /// the structural one recorded the declarator under no key at all, and the
  /// span one matched no recorded expression. A producer call written
  /// `const fade = (stylex.keyframes({…}))` was left untransformed for it, with
  /// no error.
  #[test]
  fn a_parenthesised_initializer_is_found_by_the_call_it_holds() {
    let mut state = StateManager::default();

    let bare = call_at(span_at(1, 10));

    // Structurally different from `bare`, because the structural lookup below
    // answers the earliest declarator a call reads as, and two calls that
    // differ only in their span read alike.
    let mut wrapped = call_at(span_at(20, 30));
    wrapped.callee = Callee::Expr(Box::new(ident_expr("defineConsts")));

    state.push_declaration(make_var_declarator("bare", Expr::Call(bare.clone())));
    state.push_declaration(make_var_declarator(
      "wrapped",
      Expr::Paren(ParenExpr {
        span: span_at(19, 31),
        expr: Box::new(Expr::Call(wrapped.clone())),
      }),
    ));

    assert_eq!(state.find_call_declaration_index_by_span(&wrapped), Some(1));
    assert_eq!(state.find_call_declaration_index_by_span(&bare), Some(0));

    assert_eq!(
      state
        .find_call_declaration(&wrapped)
        .and_then(|decl| decl.name.as_ident())
        .map(|ident| ident.sym.to_string()),
      Some(String::from("wrapped"))
    );
  }

  #[test]
  fn find_call_declaration_by_span_reads_the_declarator_at_that_position() {
    let mut state = StateManager::default();

    let first = call_at(span_at(1, 10));
    let second = call_at(span_at(20, 30));

    state
      .declarations
      .push(make_var_declarator("first", Expr::Call(first)));
    state
      .declarations
      .push(make_var_declarator("second", Expr::Call(second.clone())));

    assert_eq!(
      bound_name(state.find_call_declaration_by_span(&second)),
      Some("second")
    );
    assert!(
      state
        .find_call_declaration_by_span(&call_at(span_at(40, 50)))
        .is_none()
    );
  }

  #[test]
  fn find_call_declaration_index_by_span_never_matches_a_spanless_call() {
    let mut state = StateManager::default();

    state.declarations.push(make_var_declarator(
      "synthesized",
      Expr::Call(call_at(DUMMY_SP)),
    ));

    assert_eq!(
      state.find_call_declaration_index_by_span(&call_at(DUMMY_SP)),
      None
    );
  }

  /// The options struct holds each ceiling as a bare `usize`, which a
  /// struct-update literal can set to anything -- so what the three accessors
  /// answer is bracketed again where it is read rather than trusted as held.
  /// Without that bracket a zero would refuse every fold the compiler runs to do
  /// its own work, and a `usize::MAX` would be no ceiling at all.
  ///
  /// Asserted through the accessors rather than through `Ceiling::clamped`,
  /// because the claim is that these three readings are wired to it: a `clamped`
  /// pinned only in its own crate stays green when a call site here drops it.
  #[test]
  fn a_ceiling_held_by_the_options_struct_is_read_back_inside_its_bracket() {
    /// One reading: the option a project writes, the field the options struct
    /// holds it in, the accessor that spends it, and the ceiling that brackets
    /// it. Named rather than a tuple, so the loop below reads as the claim.
    struct Reading {
      option: &'static str,
      hold: fn(&mut StateManager, usize),
      spend: fn(&StateManager) -> usize,
      ceiling: &'static Ceiling,
    }

    let readings = [
      Reading {
        option: "maxEvaluationDepth",
        hold: |state, held| state.options.max_evaluation_depth = held,
        spend: StateManager::evaluation_ceiling,
        ceiling: &MAX_EVALUATION_DEPTH,
      },
      Reading {
        option: "maxFoldedCharacters",
        hold: |state, held| state.options.max_folded_characters = held,
        spend: StateManager::character_ceiling,
        ceiling: &MAX_FOLDED_CHARACTERS,
      },
      Reading {
        option: "maxFoldedEntries",
        hold: |state, held| state.options.max_folded_entries = held,
        spend: StateManager::entry_ceiling,
        ceiling: &MAX_FOLDED_ENTRIES,
      },
    ];

    for reading in readings {
      let ceiling = reading.ceiling;

      for (held, expected) in [
        (0, 1),
        (usize::MAX, ceiling.limit),
        (ceiling.limit + 1, ceiling.limit),
        (ceiling.default, ceiling.default),
      ] {
        let mut state = StateManager::default();
        (reading.hold)(&mut state, held);

        assert_eq!(
          (reading.spend)(&state),
          expected,
          "{}: held {held}",
          reading.option
        );
      }
    }
  }

  /// A call carrying `argument`, so two calls can differ in content rather than
  /// only in position.
  fn call_of(name: &str, argument: &str) -> CallExpr {
    CallExpr {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      callee: Callee::Expr(Box::new(ident_expr(name))),
      args: vec![ExprOrSpread {
        spread: None,
        expr: Box::new(string_expr(argument)),
      }],
      type_args: None,
    }
  }

  /// A call that carries both content and a position, so the two lookups can be
  /// asked about the same node.
  fn call_of_at(name: &str, argument: &str, span: Span) -> CallExpr {
    CallExpr {
      span,
      ..call_of(name, argument)
    }
  }

  /// The name a found declarator binds, for assertions that only care about it.
  fn bound_name(declarator: Option<&VarDeclarator>) -> Option<&str> {
    declarator
      .and_then(|decl| decl.name.as_ident())
      .map(|ident| ident.sym.as_str())
  }

  fn declarator_at(name: &str, span: Span, init: Expr) -> VarDeclarator {
    VarDeclarator {
      span,
      ..make_var_declarator(name, init)
    }
  }

  #[test]
  fn find_call_declaration_pins_the_declarator_that_call_initialises() {
    let mut state = StateManager::default();

    let first = call_of("create", "first");
    let second = call_of("create", "second");

    state.push_declaration(make_var_declarator("first", Expr::Call(first)));
    state.push_declaration(make_var_declarator("second", Expr::Call(second.clone())));
    state.push_declaration(make_var_declarator("plain", string_expr("no call here")));

    assert_eq!(
      bound_name(state.find_call_declaration(&second)),
      Some("second")
    );
    assert!(
      state
        .find_call_declaration(&call_of("create", "absent"))
        .is_none()
    );
  }

  #[test]
  fn find_call_declaration_answers_with_the_earliest_of_equal_declarators() {
    let mut state = StateManager::default();

    let call = call_of("create", "shared");

    state.push_declaration(make_var_declarator("earlier", Expr::Call(call.clone())));
    state.push_declaration(make_var_declarator("later", Expr::Call(call.clone())));

    // Two declarators can hold structurally equal calls, and the walk this
    // replaced answered with whichever came first in the module.
    assert_eq!(
      bound_name(state.find_call_declaration(&call)),
      Some("earlier")
    );
  }

  /// The two lookups answer different questions and must keep disagreeing.
  ///
  /// `find_call_declaration` is keyed on what a call *is*, so two calls that
  /// differ only in position collapse onto the earliest declarator holding one.
  /// `find_call_declaration_by_span` is keyed on where a call was *written*, so
  /// the same pair resolves to one declarator each. Routing either through the
  /// other would look like a tidy-up and would silently break whichever
  /// question it stopped answering.
  #[test]
  fn a_call_is_found_by_content_collapsed_and_by_position_apart() {
    let mut state = StateManager::default();

    let earlier = call_of_at("create", "shared", span_at(1, 10));
    let later = call_of_at("create", "shared", span_at(20, 30));

    state.push_declaration(make_var_declarator("earlier", Expr::Call(earlier.clone())));
    state.push_declaration(make_var_declarator("later", Expr::Call(later.clone())));

    for call in [&earlier, &later] {
      assert_eq!(
        bound_name(state.find_call_declaration(call)),
        Some("earlier")
      );
    }

    assert_eq!(
      bound_name(state.find_call_declaration_by_span(&earlier)),
      Some("earlier")
    );
    assert_eq!(
      bound_name(state.find_call_declaration_by_span(&later)),
      Some("later")
    );
  }

  /// The same disagreement at a width no hand-written module reaches, so the
  /// index answering from a bucket and the walk answering from a position are
  /// both exercised past the point where a linear fallback would be visible.
  #[test]
  fn content_and_position_keep_disagreeing_across_five_thousand_equal_calls() {
    const DECLARATORS: usize = 5_000;

    let mut state = StateManager::default();

    let calls: Vec<CallExpr> = (0..DECLARATORS)
      .map(|index| {
        let start = (index as u32 + 1) * 100;
        call_of_at("create", "shared", span_at(start, start + 10))
      })
      .collect();

    for (index, call) in calls.iter().enumerate() {
      state.push_declaration(make_var_declarator(
        &format!("styles{index}"),
        Expr::Call(call.clone()),
      ));
    }

    for index in [0usize, 1, 2_499, DECLARATORS - 1] {
      assert_eq!(
        bound_name(state.find_call_declaration(&calls[index])),
        Some("styles0"),
        "content lookup at {index}"
      );
      assert_eq!(
        bound_name(state.find_call_declaration_by_span(&calls[index])),
        Some(format!("styles{index}").as_str()),
        "positional lookup at {index}"
      );
    }

    // A position no declarator was written at is not the last one seen.
    assert!(
      state
        .find_call_declaration_by_span(&call_of_at("create", "shared", span_at(7, 8)))
        .is_none()
    );
  }

  #[test]
  fn find_call_declaration_ignores_a_declarator_with_no_initializer() {
    let mut state = StateManager::default();

    state.push_declaration(VarDeclarator {
      init: None,
      ..make_var_declarator("bare", string_expr("unused"))
    });

    assert!(
      state
        .find_call_declaration(&call_of("create", "anything"))
        .is_none()
    );
  }

  #[test]
  fn replacing_an_initializer_moves_the_declarator_to_the_call_it_now_holds() {
    let mut state = StateManager::default();

    let original = call_of("create", "original");
    let replacement = call_of("create", "replacement");

    state.push_declaration(make_var_declarator("styles", Expr::Call(original.clone())));
    state.set_declaration_init(0, Expr::Call(replacement.clone()));

    // The key the declarator was recorded under stops being true, so the call
    // it no longer holds must stop finding it.
    assert!(state.find_call_declaration(&original).is_none());
    assert!(state.find_call_declaration(&replacement).is_some());
  }

  #[test]
  fn replacing_an_initializer_with_a_non_call_leaves_it_findable_by_nothing() {
    let mut state = StateManager::default();

    let original = call_of("defineMarker", "marker");

    state.push_declaration(make_var_declarator("marker", Expr::Call(original.clone())));
    state.set_declaration_init(0, string_expr("a compiled marker object"));

    assert!(state.find_call_declaration(&original).is_none());
  }

  #[test]
  fn replacing_an_initializer_with_the_same_call_keeps_it_findable() {
    let mut state = StateManager::default();

    let call = call_of("create", "unchanged");

    state.push_declaration(make_var_declarator("styles", Expr::Call(call.clone())));
    state.set_declaration_init(0, Expr::Call(call.clone()));

    // Forgetting and recording happen in that order, so a replacement that
    // reads the same must not leave the record dropped.
    assert!(state.find_call_declaration(&call).is_some());
  }

  #[test]
  fn replacing_an_initializer_out_of_range_changes_nothing() {
    let mut state = StateManager::default();

    let call = call_of("create", "only");

    state.push_declaration(make_var_declarator("styles", Expr::Call(call.clone())));
    state.set_declaration_init(99, string_expr("nowhere"));

    assert!(state.find_call_declaration(&call).is_some());
  }

  #[test]
  fn find_top_level_expr_pins_the_entry_that_is_that_call() {
    let mut state = StateManager::default();

    let create = call_of("create", "styles");
    let theme = call_of("createTheme", "theme");

    state.push_top_level_expression(TopLevelExpression(
      TopLevelExpressionKind::Stmt,
      Expr::Call(create),
      Some("styles".into()),
    ));
    state.push_top_level_expression(TopLevelExpression(
      TopLevelExpressionKind::NamedExport,
      Expr::Call(theme.clone()),
      Some("theme".into()),
    ));

    assert_eq!(
      state.find_top_level_expr(&theme).map(|tpe| tpe.0),
      Some(TopLevelExpressionKind::NamedExport)
    );
    assert!(
      state
        .find_top_level_expr(&call_of("create", "absent"))
        .is_none()
    );
  }

  #[test]
  fn find_top_level_expr_ignores_an_entry_that_merely_holds_the_call() {
    let mut state = StateManager::default();

    let call = call_of("create", "styles");

    state.push_top_level_expression(TopLevelExpression(
      TopLevelExpressionKind::Stmt,
      Expr::Array(ArrayLit {
        span: DUMMY_SP,
        elems: vec![Some(ExprOrSpread {
          spread: None,
          expr: Box::new(Expr::Call(call.clone())),
        })],
      }),
      Some("lotsOfStyles".into()),
    ));

    // The entry *is* the array, not the call inside it. Where the call stands
    // is answered by its position, not by the entry it sits in.
    assert!(state.find_top_level_expr(&call).is_none());
  }

  /// A record holding `span` in every position there is.
  fn positions_holding(span: Span) -> CallPositions {
    let mut positions = CallPositions::default();

    for position in [
      Position::ProgramLevel,
      Position::BareStatement,
      Position::TypeAsserted,
    ] {
      positions.record(span, position);
    }

    positions
  }

  /// A call the walk recorded is answered by its own position, and a call
  /// beside it is not. Two calls that read the same are two calls.
  #[test]
  fn a_call_position_answers_for_that_call_alone() {
    let mut state = StateManager::default();

    let recorded = call_of_at("create", "styles", span_at(12, 30));
    let beside = call_of_at("create", "styles", span_at(90, 99));

    state.record_call_positions(positions_holding(recorded.span));

    assert!(state.is_program_level_call(&recorded));
    assert!(state.is_bare_call_statement(&recorded));
    assert!(state.is_type_asserted_call(&recorded));

    assert!(!state.is_program_level_call(&beside));
    assert!(!state.is_bare_call_statement(&beside));
    assert!(!state.is_type_asserted_call(&beside));
  }

  /// A synthesized call was written nowhere. A dummy span reads as position
  /// zero, which a recorded position starting at zero would otherwise answer
  /// for.
  #[test]
  fn a_span_less_call_stands_in_no_position() {
    let mut state = StateManager::default();

    let synthesized = call_of("create", "styles");

    state.record_call_positions(positions_holding(synthesized.span));

    assert!(!state.is_program_level_call(&synthesized));
    assert!(!state.is_bare_call_statement(&synthesized));
    assert!(!state.is_type_asserted_call(&synthesized));
  }

  /// A bucket a lookup can stop early in is one nothing has moved. Rewriting an
  /// entry back to the expression it started with re-records it at the back, so
  /// the bucket stops being ascending -- and the answer has to stay the earliest
  /// entry the module writes, not the first the bucket holds.
  #[test]
  fn a_bucket_an_entry_moved_back_into_still_answers_with_the_earliest() {
    let mut state = StateManager::default();

    let shared = call_of("create", "shared");
    let other = call_of("create", "other");

    for name in ["first", "second", "third"] {
      state.push_top_level_expression(TopLevelExpression(
        TopLevelExpressionKind::Stmt,
        Expr::Call(shared.clone()),
        Some(name.into()),
      ));
    }

    // Out of the bucket and back into it, which leaves position 0 behind
    // positions 1 and 2 in it.
    state.set_top_level_expr(0, Expr::Call(other));
    state.set_top_level_expr(0, Expr::Call(shared.clone()));

    assert_eq!(
      state
        .find_top_level_expr(&shared)
        .and_then(|tpe| tpe.2.as_ref())
        .map(Atom::as_str),
      Some("first")
    );
  }

  #[test]
  fn replacing_a_top_level_expression_moves_it_to_the_call_it_now_is() {
    let mut state = StateManager::default();

    let original = call_of("create", "original");
    let replacement = call_of("create", "replacement");

    state.push_top_level_expression(TopLevelExpression(
      TopLevelExpressionKind::Stmt,
      Expr::Call(original.clone()),
      Some("styles".into()),
    ));
    state.set_top_level_expr(0, Expr::Call(replacement.clone()));

    assert!(state.find_top_level_expr(&original).is_none());
    assert!(state.find_top_level_expr(&replacement).is_some());
  }

  #[test]
  fn find_top_level_expr_named_answers_for_the_name_it_binds() {
    let mut state = StateManager::default();

    let shared = Expr::Call(call_of("create", "shared"));

    state.push_top_level_expression(TopLevelExpression(
      TopLevelExpressionKind::NamedExport,
      shared.clone(),
      Some("first".into()),
    ));
    state.push_top_level_expression(TopLevelExpression(
      TopLevelExpressionKind::Stmt,
      shared.clone(),
      Some("second".into()),
    ));

    // Both entries read the same, so only the name tells them apart -- where a
    // walk comparing expressions answered with whichever came first for either.
    assert_eq!(
      state
        .find_top_level_expr_named(&"second".into(), &shared)
        .map(|tpe| tpe.0),
      Some(TopLevelExpressionKind::Stmt)
    );
    assert_eq!(
      state
        .find_top_level_expr_named(&"first".into(), &shared)
        .map(|tpe| tpe.0),
      Some(TopLevelExpressionKind::NamedExport)
    );
  }

  #[test]
  fn find_top_level_expr_named_refuses_a_name_whose_expression_has_moved_on() {
    let mut state = StateManager::default();

    let recorded = Expr::Call(call_of("create", "recorded"));

    state.push_top_level_expression(TopLevelExpression(
      TopLevelExpressionKind::Stmt,
      recorded,
      Some("styles".into()),
    ));

    assert!(
      state
        .find_top_level_expr_named(&"styles".into(), &string_expr("something else"))
        .is_none()
    );
    assert!(
      state
        .find_top_level_expr_named(&"absent".into(), &string_expr("anything"))
        .is_none()
    );
  }

  #[test]
  fn holds_declaration_dedups_on_position_then_content() {
    let mut state = StateManager::default();

    let first = declarator_at("styles", span_at(1, 10), string_expr("value"));
    let same_position_other_content = declarator_at("styles", span_at(1, 10), string_expr("other"));
    let same_content_other_position =
      declarator_at("styles", span_at(20, 30), string_expr("value"));

    state.push_declaration(first.clone());

    assert!(state.holds_declaration(&first));
    assert!(!state.holds_declaration(&same_position_other_content));
    // Two declarations that merely read alike stay two entries, because a
    // lookup that pins a call to its declarator by span needs them to.
    assert!(!state.holds_declaration(&same_content_other_position));
  }

  #[test]
  fn holds_declaration_tells_synthesized_declarators_apart_by_content() {
    let mut state = StateManager::default();

    let first = declarator_at("a", DUMMY_SP, string_expr("first"));
    let second = declarator_at("b", DUMMY_SP, string_expr("second"));

    state.push_declaration(first.clone());

    // Every synthesized declarator shares `DUMMY_SP`, so content is all that
    // separates them.
    assert!(state.holds_declaration(&first));
    assert!(!state.holds_declaration(&second));
  }

  #[test]
  fn matching_style_var_answers_for_the_name_the_declarator_binds() {
    let mut state = StateManager::default();

    let declarator = make_var_declarator("styles", Expr::Call(call_of("create", "styles")));

    state.insert_style_var("styles".to_string(), declarator.clone());

    assert!(state.matching_style_var(&declarator).is_some());
    // A declarator of another name is another style variable, however it reads.
    assert!(
      state
        .matching_style_var(&make_var_declarator(
          "other",
          Expr::Call(call_of("create", "styles"))
        ))
        .is_none()
    );
    // And the recorded name having moved on is not a match either.
    assert!(
      state
        .matching_style_var(&make_var_declarator(
          "styles",
          string_expr("something else")
        ))
        .is_none()
    );
  }

  /// A declarator that holds no value is no style variable either: the two
  /// parts the lookup answers with are the name and the initializer, and this
  /// one has only the name.
  #[test]
  fn matching_style_var_refuses_a_declarator_with_no_initializer() {
    let mut state = StateManager::default();

    state.insert_style_var(
      "styles".to_string(),
      make_var_declarator("styles", Expr::Call(call_of("create", "styles"))),
    );

    assert!(
      state
        .matching_style_var(&make_var_declarator_no_init("styles"))
        .is_none()
    );
  }

  #[test]
  fn matching_style_var_refuses_a_declarator_bound_to_a_pattern() {
    let mut state = StateManager::default();

    let init = Expr::Call(call_of("create", "styles"));

    state.insert_style_var(
      "styles".to_string(),
      make_var_declarator("styles", init.clone()),
    );

    let pattern_bound = VarDeclarator {
      span: DUMMY_SP,
      name: Pat::Object(ObjectPat {
        span: DUMMY_SP,
        props: vec![],
        optional: false,
        type_ann: None,
      }),
      init: Some(Box::new(init)),
      definite: false,
    };

    assert!(state.matching_style_var(&pattern_bound).is_none());
  }

  #[test]
  fn replacing_a_style_var_initializer_moves_it_to_the_call_it_now_holds() {
    let mut state = StateManager::default();

    let original = call_of("create", "original");
    let replacement = call_of("create", "replacement");

    state.insert_style_var(
      "styles".to_string(),
      make_var_declarator("styles", Expr::Call(original)),
    );
    state.set_style_var_init("styles".to_string(), Expr::Call(replacement.clone()));

    assert_eq!(
      state.style_vars["styles"]
        .init
        .as_deref()
        .and_then(Expr::as_call)
        .map(|call| call.eq_ignore_span(&replacement)),
      Some(true)
    );
  }

  #[test]
  fn inserting_a_style_var_twice_forgets_the_call_the_first_one_held() {
    let mut state = StateManager::default();

    let first = call_of("create", "first");
    let second = call_of("create", "second");

    state.insert_style_var(
      "styles".to_string(),
      make_var_declarator("styles", Expr::Call(first)),
    );
    state.insert_style_var(
      "styles".to_string(),
      make_var_declarator("styles", Expr::Call(second.clone())),
    );

    assert!(
      state
        .matching_style_var(&make_var_declarator("styles", Expr::Call(second)))
        .is_some()
    );
  }

  #[test]
  fn declared_as_tells_a_class_from_a_function_and_keeps_the_first_position() {
    let mut state = StateManager::default();

    let class_name = Ident {
      span: span_at(1, 10),
      ..ident("Widget")
    };
    let later_spelling = Ident {
      span: span_at(50, 60),
      ..ident("Widget")
    };

    state.add_class_name_declaration(class_name);
    state.add_class_name_declaration(later_spelling);
    state.add_function_name_declaration(ident("render"));

    assert!(matches!(
      state.declared_as(&ident("Widget")),
      Some(DeclarationType::Class)
    ));
    assert!(matches!(
      state.declared_as(&ident("render")),
      Some(DeclarationType::Function)
    ));
    assert!(state.declared_as(&ident("unbound")).is_none());
    // First writer wins, which is the position a used-before-declaration
    // diagnostic is about.
    assert_eq!(
      state.class_name_declaration(&ident("Widget")),
      Some(span_at(1, 10))
    );
  }

  #[test]
  fn a_declaration_resolves_only_for_the_binding_its_own_context_names() {
    let mut state = StateManager::default();

    let outer = ident("Widget");
    // A context of its own, spelled directly rather than through a `Mark`,
    // which needs a `GLOBALS` scope this unit test has no other use for.
    let shadowed = Ident {
      ctxt: SyntaxContext::from_u32(1),
      ..ident("Widget")
    };

    state.add_class_name_declaration(outer.clone());

    assert!(matches!(
      state.declared_as(&outer),
      Some(DeclarationType::Class)
    ));
    // A name shadowing the class declares something else, and resolving it to
    // the declaration it shadows is what keying on the symbol alone would do.
    assert!(state.declared_as(&shadowed).is_none());
  }

  /// Far past any authored module, to show the lookups answer from a key rather
  /// than by walking what has been recorded.
  #[test]
  fn the_lookups_stay_exact_across_ten_thousand_declarations() {
    let mut state = StateManager::default();

    let calls: Vec<CallExpr> = (0..10_000)
      .map(|index| call_of("create", &format!("styles{index}")))
      .collect();

    for (index, call) in calls.iter().enumerate() {
      let name = format!("styles{index}");

      state.push_declaration(make_var_declarator(&name, Expr::Call(call.clone())));
      state.push_top_level_expression(TopLevelExpression(
        TopLevelExpressionKind::Stmt,
        Expr::Call(call.clone()),
        Some(name.as_str().into()),
      ));
    }

    for index in [0usize, 1, 4_999, 9_998, 9_999] {
      assert_eq!(
        bound_name(state.find_call_declaration(&calls[index])),
        Some(format!("styles{index}").as_str())
      );
      assert_eq!(
        state
          .find_top_level_expr(&calls[index])
          .and_then(|tpe| tpe.2.as_ref())
          .map(Atom::as_str),
        Some(format!("styles{index}").as_str())
      );
    }

    assert!(
      state
        .find_call_declaration(&call_of("create", "styles10000"))
        .is_none()
    );
  }

  #[test]
  fn find_top_level_expr_named_answers_for_a_redeclared_name() {
    let mut state = StateManager::default();

    let first = Expr::Call(call_of("create", "first"));
    let second = Expr::Call(call_of("create", "second"));

    // `var styles = ...; var styles = ...;` is legal, so one name can bind two
    // entries and the second must stay findable.
    state.push_top_level_expression(TopLevelExpression(
      TopLevelExpressionKind::NamedExport,
      first.clone(),
      Some("styles".into()),
    ));
    state.push_top_level_expression(TopLevelExpression(
      TopLevelExpressionKind::Stmt,
      second.clone(),
      Some("styles".into()),
    ));

    assert_eq!(
      state
        .find_top_level_expr_named(&"styles".into(), &second)
        .map(|tpe| tpe.0),
      Some(TopLevelExpressionKind::Stmt)
    );
    assert_eq!(
      state
        .find_top_level_expr_named(&"styles".into(), &first)
        .map(|tpe| tpe.0),
      Some(TopLevelExpressionKind::NamedExport)
    );
  }

  #[test]
  fn find_top_level_expr_named_ignores_an_entry_bound_to_another_name() {
    let mut state = StateManager::default();

    let shared = Expr::Call(call_of("create", "shared"));

    state.push_top_level_expression(TopLevelExpression(
      TopLevelExpressionKind::NamedExport,
      shared.clone(),
      Some("exported".into()),
    ));
    state.push_top_level_expression(TopLevelExpression(
      TopLevelExpressionKind::Stmt,
      shared.clone(),
      Some("styles".into()),
    ));

    // Both entries read the same, and the exported one comes first. The answer
    // is the entry this name binds, not whichever the module spells first --
    // the kind belongs to a declarator, and reading another's is what the walk
    // this replaced did.
    assert_eq!(
      state
        .find_top_level_expr_named(&"styles".into(), &shared)
        .map(|tpe| tpe.0),
      Some(TopLevelExpressionKind::Stmt)
    );
  }

  #[test]
  fn replacing_out_of_range_entries_changes_nothing() {
    let mut state = StateManager::default();

    let call = call_of("create", "only");

    state.push_top_level_expression(TopLevelExpression(
      TopLevelExpressionKind::Stmt,
      Expr::Call(call.clone()),
      Some("styles".into()),
    ));
    state.insert_style_var(
      "styles".to_string(),
      make_var_declarator("styles", Expr::Call(call.clone())),
    );

    state.set_top_level_expr(99, string_expr("nowhere"));
    state.set_style_var_init("absent".to_string(), string_expr("nowhere"));

    assert!(state.find_top_level_expr(&call).is_some());
    assert!(
      state
        .matching_style_var(&make_var_declarator("styles", Expr::Call(call)))
        .is_some()
    );
  }

  /// Which declaration binds a name, over the two indices the state keeps for
  /// it. Every case fixes the records first and then asks, because the answer is
  /// the index and the list agreeing.
  ///
  /// Four narrower cases that used to sit above the pass-through are folded in
  /// rather than carried: a plain hit and a plain miss, which
  /// [`answers_for_the_binding_a_specifier_declares`] asserts together, and a
  /// bare hit for each of the default and namespace arms, which
  /// [`answers_for_a_default_specifier`] and
  /// [`answers_for_a_namespace_specifier`] assert with the matching miss beside
  /// it.
  mod import_binding {
    use super::*;
    use swc_core::ecma::ast::ModuleExportName;

    /// One import declaration over the specifiers handed in. Every case here is
    /// some choice of source and specifier list, so they share one builder --
    /// otherwise a case that needs a shape no helper covers grows a fifteen-line
    /// literal of its own and the shapes stop being comparable at a glance.
    fn import_from(source: &str, specifiers: Vec<ImportSpecifier>) -> ImportDecl {
      ImportDecl {
        span: DUMMY_SP,
        specifiers,
        src: Box::new(Str {
          span: DUMMY_SP,
          value: source.into(),
          raw: None,
        }),
        type_only: false,
        with: None,
        phase: ImportPhase::Evaluation,
      }
    }

    /// `import { a, b }` at the parser's context, for the cases that only need a
    /// list of plain names.
    fn named_import(source: &str, names: &[&str]) -> ImportDecl {
      import_from(source, names.iter().map(|name| named(name, 0)).collect())
    }

    /// `import { local }` -- the specifier whose comparison #1266 was about.
    fn named(local: &str, ctxt: u32) -> ImportSpecifier {
      ImportSpecifier::Named(ImportNamedSpecifier {
        span: DUMMY_SP,
        local: ident_at(local, ctxt),
        imported: None,
        is_type_only: false,
      })
    }

    /// `import { imported as local }`.
    fn aliased(local: &str, imported: &str, ctxt: u32) -> ImportSpecifier {
      ImportSpecifier::Named(ImportNamedSpecifier {
        span: DUMMY_SP,
        local: ident_at(local, ctxt),
        imported: Some(ModuleExportName::Ident(ident_at(imported, ctxt))),
        is_type_only: false,
      })
    }

    /// `import { "imported" as local }`, whose imported name need not be a legal
    /// identifier.
    fn str_named(local: &str, imported: &str, ctxt: u32) -> ImportSpecifier {
      ImportSpecifier::Named(ImportNamedSpecifier {
        span: DUMMY_SP,
        local: ident_at(local, ctxt),
        imported: Some(ModuleExportName::Str(Str {
          span: DUMMY_SP,
          value: imported.into(),
          raw: None,
        })),
        is_type_only: false,
      })
    }

    /// `import local from` and `import * as local from`.
    fn default_of(local: &str, ctxt: u32) -> ImportSpecifier {
      ImportSpecifier::Default(ImportDefaultSpecifier {
        span: DUMMY_SP,
        local: ident_at(local, ctxt),
      })
    }

    fn namespace_of(local: &str, ctxt: u32) -> ImportSpecifier {
      ImportSpecifier::Namespace(ImportStarAsSpecifier {
        span: DUMMY_SP,
        local: ident_at(local, ctxt),
      })
    }

    /// The source a lookup answered with, as authored text. Reads the atom
    /// directly rather than comparing `Debug` renderings, which would pass for
    /// the wrong reason the moment the atom's formatting changes.
    fn source_of<'a>(found: Option<(&'a ImportDecl, &ImportSpecifier)>) -> Option<&'a str> {
      found.and_then(|(import, _)| import.src.value.as_str())
    }

    #[test]
    fn answers_for_the_binding_a_specifier_declares() {
      let mut state = StateManager::default();

      state.push_top_import(named_import("theme.stylex.js", &["spacing", "colors"]));
      state.push_top_import(named_import("other.stylex.js", &["radii"]));

      assert_eq!(
        source_of(state.import_binding(&ident_at("colors", 0))),
        Some("theme.stylex.js")
      );
      assert_eq!(
        source_of(state.import_binding(&ident_at("radii", 0))),
        Some("other.stylex.js")
      );
      assert!(state.import_binding(&ident_at("unbound", 0)).is_none());
    }

    /// A module that imports nothing has no index to probe, which is a step
    /// earlier than the miss the case above asks for.
    #[test]
    fn answers_for_nothing_where_the_module_imports_nothing() {
      let state = StateManager::default();

      assert!(state.import_binding(&ident_at("nonexistent", 0)).is_none());
    }

    /// `import * as stylex` binds through another arm of `local_binding_of` than
    /// the named form, and the namespace form is what every StyleX module
    /// actually writes.
    #[test]
    fn answers_for_a_namespace_specifier() {
      let mut state = StateManager::default();

      state.push_top_import(import_from(
        "@stylexjs/stylex",
        vec![namespace_of("stylex", 0)],
      ));

      assert!(state.import_binding(&ident_at("stylex", 0)).is_some());
      assert!(state.import_binding(&ident_at("other", 0)).is_none());
    }

    /// `import stylex from` is the third arm, and the only one whose binding is
    /// the declaration's own default.
    #[test]
    fn answers_for_a_default_specifier() {
      let mut state = StateManager::default();

      state.push_top_import(import_from(
        "@stylexjs/stylex",
        vec![default_of("stylex", 0)],
      ));

      assert!(state.import_binding(&ident_at("stylex", 0)).is_some());
      assert!(state.import_binding(&ident_at("other", 0)).is_none());
    }

    #[test]
    fn does_not_match_a_renamed_import_by_the_name_it_was_aliased_away_from() {
      let mut state = StateManager::default();
      state.push_top_import(import_from(
        "@stylexjs/stylex",
        vec![aliased("localName", "create", 0)],
      ));

      // `import { create as localName }` leaves `create` unbound in this module,
      // so a reference spelled that way names something else or nothing at all.
      // Answering the import for it resolved a binding no scope holds.
      assert!(state.import_binding(&ident_at("create", 0)).is_none());
    }

    #[test]
    fn finds_a_renamed_import_by_its_local_name() {
      let mut state = StateManager::default();
      state.push_top_import(import_from(
        "@stylexjs/stylex",
        vec![aliased("localName", "create", 0)],
      ));

      assert!(state.import_binding(&ident_at("localName", 0)).is_some());
    }

    #[test]
    fn does_not_match_a_string_named_specifier_by_its_imported_name() {
      let mut state = StateManager::default();
      state.push_top_import(import_from(
        "module",
        vec![str_named("localName", "strExport", 0)],
      ));

      // The same answer for the string-named spelling, and the one that was
      // reachable in practice: an imported name that *is* a legal identifier
      // matched a reference to it by symbol alone, across every scope.
      assert!(state.import_binding(&ident_at("strExport", 0)).is_none());

      // The local binding is what the declaration introduces, and it still
      // answers -- a lookup that said `None` to everything would pass the
      // assertion above on its own.
      assert!(state.import_binding(&ident_at("localName", 0)).is_some());
    }

    // ──────────────────────────────────────────────
    // Shadowing: the lookup answers about a binding, not a name (#1266)
    //
    // Every case here fixes two references with the same symbol at *different*
    // syntax contexts, which is what a shadowing binding looks like once the
    // resolver has run. Written against the lookup rather than the transform
    // because a context is the whole question and the transform has to build a
    // whole module to ask it.
    // ──────────────────────────────────────────────

    #[test]
    fn refuses_a_reference_to_a_named_import_from_another_scope() {
      let mut state = StateManager::default();

      state.push_top_import(named_import("theme.stylex.js", &["spacing"]));

      // The arrow parameter in `(spacing) => ({ spacing })`: same symbol, its
      // own context. Resolving it to the import it shadows is #1266.
      //
      // Both halves in one case on purpose: a fix that answered `None` for
      // everything would pass the refusal on its own.
      assert!(state.import_binding(&ident_at("spacing", 0)).is_some());
      assert!(state.import_binding(&ident_at("spacing", 1)).is_none());
    }

    #[test]
    fn refuses_a_reference_to_an_aliased_import_from_another_scope() {
      let mut state = StateManager::default();
      state.push_top_import(import_from(
        "zIndex.stylex.js",
        vec![aliased("zi", "zIndex", 1)],
      ));

      // `import { zIndex as zi }` shadowed by a parameter `zi`, which failed
      // identically to the unaliased shape.
      assert!(state.import_binding(&ident_at("zi", 2)).is_none());
      assert!(state.import_binding(&ident_at("zi", 1)).is_some());
    }

    #[test]
    fn matches_only_the_specifier_whose_context_agrees() {
      let mut state = StateManager::default();
      state.push_top_import(import_from("first.stylex.js", vec![named("shadowed", 1)]));
      state.push_top_import(import_from("second.stylex.js", vec![named("shadowed", 2)]));

      // Two declarations cannot bind one name in valid source, but the lookup
      // must not answer by position.
      assert_eq!(
        source_of(state.import_binding(&ident_at("shadowed", 2))),
        Some("second.stylex.js")
      );
    }

    #[test]
    fn answers_with_the_earliest_of_two_imports_of_one_name() {
      let mut state = StateManager::default();

      state.push_top_import(named_import("first.stylex.js", &["spacing"]));
      state.push_top_import(named_import("second.stylex.js", &["spacing"]));

      assert_eq!(
        source_of(state.import_binding(&ident_at("spacing", 0))),
        Some("first.stylex.js")
      );
    }

    #[test]
    fn matches_one_specifier_of_a_declaration_without_matching_its_siblings() {
      let mut state = StateManager::default();
      state.push_top_import(import_from(
        "tokens.stylex.js",
        vec![named("spacing", 1), named("zIndex", 1)],
      ));

      // The answer is per declaration, but the match is per specifier: a
      // declaration that binds a live `spacing` does not thereby answer for a
      // shadowed `zIndex`. Both are the same declaration, so a lookup that
      // stopped at "does this declaration mention the name" would say yes to
      // both.
      assert!(state.import_binding(&ident_at("spacing", 1)).is_some());
      assert!(state.import_binding(&ident_at("zIndex", 1)).is_some());
      assert!(state.import_binding(&ident_at("zIndex", 2)).is_none());
      assert!(state.import_binding(&ident_at("nothing", 1)).is_none());
    }

    #[test]
    fn a_shadowed_default_or_namespace_import_was_already_context_aware() {
      let mut state = StateManager::default();
      state.push_top_import(import_from("theme.stylex.js", vec![default_of("theme", 1)]));
      state.push_top_import(import_from(
        "tokens.stylex.js",
        vec![namespace_of("tokens", 1)],
      ));

      // The two arms the named one now matches. Pinned so a later edit cannot
      // regress all three to a name match at once.
      assert!(state.import_binding(&ident_at("theme", 2)).is_none());
      assert!(state.import_binding(&ident_at("tokens", 2)).is_none());
      assert!(state.import_binding(&ident_at("theme", 1)).is_some());
      assert!(state.import_binding(&ident_at("tokens", 1)).is_some());
    }

    #[test]
    fn a_string_named_specifier_answers_only_for_its_local_binding() {
      let mut state = StateManager::default();
      state.push_top_import(import_from(
        "tokens.stylex.js",
        vec![str_named("spacing", "spacing-lg", 1)],
      ));

      // The imported name is compared against nothing now, at any context.
      // `spacing-lg` is not a legal identifier, so the only way a reference
      // could ever have carried that symbol was through the specifier itself.
      assert!(state.import_binding(&ident_at("spacing-lg", 9)).is_none());
      assert!(state.import_binding(&ident_at("spacing-lg", 1)).is_none());

      // Its local binding still resolves, and a parameter shadowing that local
      // binding still does not.
      assert!(state.import_binding(&ident_at("spacing", 1)).is_some());
      assert!(state.import_binding(&ident_at("spacing", 2)).is_none());
    }

    #[test]
    fn a_non_ascii_local_name_matches_by_binding_too() {
      let mut state = StateManager::default();
      // `zÍndex` is a different identifier that merely looks like `zIndex`, and
      // the lookup compares interned atoms, so it must not match one for the
      // other. A unicode-escaped spelling needs no case of its own here: the
      // lexer folds a unicode escape to the atom it spells long before this, so at
      // this seam the two spellings are one value. The escape is exercised where
      // it can still differ -- as authored source, in the parity corpus.
      state.push_top_import(import_from("zIndex.stylex.js", vec![named("zIndex", 1)]));
      state.push_top_import(import_from("accented.stylex.js", vec![named("zÍndex", 1)]));

      assert!(state.import_binding(&ident_at("zIndex", 1)).is_some());
      assert!(state.import_binding(&ident_at("zÍndex", 1)).is_some());
      assert!(state.import_binding(&ident_at("zÍndex", 2)).is_none());
    }

    #[test]
    fn answers_with_the_specifier_that_bound_the_name() {
      let mut state = StateManager::default();
      state.push_top_import(import_from(
        "tokens.stylex.js",
        vec![default_of("theme", 1), named("spacing", 1)],
      ));

      // One declaration can bind both kinds, and the chain refuses a default
      // where it resolves a named one. Which specifier answered is therefore
      // part of the answer, not something the caller can re-derive from the
      // declaration -- searching it again by name is what could come back empty.
      let (_, default_specifier) = match state.import_binding(&ident_at("theme", 1)) {
        Some(found) => found,
        None => panic!("the default specifier binds `theme`"),
      };
      assert!(matches!(default_specifier, ImportSpecifier::Default(_)));

      let (_, named_specifier) = match state.import_binding(&ident_at("spacing", 1)) {
        Some(found) => found,
        None => panic!("the named specifier binds `spacing`"),
      };
      assert!(matches!(named_specifier, ImportSpecifier::Named(_)));
    }

    #[test]
    fn an_empty_import_declaration_answers_for_nothing() {
      let mut state = StateManager::default();
      state.push_top_import(import_from("side-effect.css", vec![]));

      // `import './side-effect.css'` binds no name at all. A walk over no
      // specifiers finds nothing, which is the answer -- pinned because a
      // refactor to `all` would make it `true` and resolve every reference to
      // it.
      assert!(state.import_binding(&ident_at("anything", 0)).is_none());
    }

    /// Far past any authored module, to show the lookup answers from a key
    /// rather than by walking every specifier the module imports.
    #[test]
    fn stays_exact_across_ten_thousand_specifiers() {
      let mut state = StateManager::default();

      let names: Vec<String> = (0..10_000).map(|index| format!("token{index}")).collect();

      for chunk in names.chunks(100) {
        let borrowed: Vec<&str> = chunk.iter().map(String::as_str).collect();

        state.push_top_import(named_import("tokens.stylex.js", &borrowed));
      }

      for index in [0usize, 1, 4_999, 9_999] {
        assert!(state.import_binding(&ident_at(&names[index], 0)).is_some());
      }

      assert!(state.import_binding(&ident_at("token10000", 0)).is_none());
    }
  }

  /// The declarator a name is bound by, over the position index
  /// [`StateManager::push_declaration`] keeps beside the list.
  mod declaration_of {
    use super::*;

    #[test]
    fn finds_the_declarator_that_binds_the_name() {
      let mut state = StateManager::default();

      state.push_declaration(make_var_declarator("x", string_expr("1")));

      assert!(state.declaration_of(&ident("x")).is_some());
    }

    #[test]
    fn answers_for_nothing_where_the_module_declares_nothing() {
      let state = StateManager::default();

      assert!(state.declaration_of(&ident("nonexistent")).is_none());
    }

    /// The same claim the import lookup makes, for the other index: far past any
    /// authored module, and answering from a key rather than by a walk.
    #[test]
    fn stays_exact_across_ten_thousand_declarators() {
      let mut state = StateManager::default();

      let names: Vec<String> = (0..10_000).map(|index| format!("token{index}")).collect();

      for name in &names {
        state.push_declaration(make_var_declarator(name, string_expr(name)));
      }

      for index in [0usize, 1, 4_999, 9_999] {
        assert!(state.declaration_of(&ident(&names[index])).is_some());
      }

      assert!(state.declaration_of(&ident("token10000")).is_none());
    }
  }

  mod css_property_seen_tests {
    use crate::state_manager::StateManager;

    #[test]
    fn reads_back_what_the_mutable_accessor_wrote() {
      let mut state = StateManager::default();

      assert!(state.css_property_seen().is_empty());

      state
        .css_property_seen_mut()
        .insert("marginInlineStart".to_string(), "10px".to_string());

      assert_eq!(
        state.css_property_seen().get("marginInlineStart"),
        Some(&"10px".to_string())
      );
    }
  }

  mod keeping_the_module_source {
    use super::StateManager;
    use stylex_diagnostics::state::DiagnosticState;
    use swc_core::{common::DUMMY_SP, ecma::ast::Module};

    /// Whether the state kept a copy after being asked for one.
    fn keeps(debug_build: bool, reads_source_from_disk: bool) -> bool {
      let mut state = StateManager::default();

      state.options.use_real_file_for_source = reads_source_from_disk;
      state.keep_module_source_copy(
        &Module {
          span: DUMMY_SP,
          body: vec![],
          shebang: None,
        },
        debug_build,
      );

      DiagnosticState::get_seen_module_source_code(&state).is_some()
    }

    /// A debug build keeps the copy whatever the option says: the assertions
    /// and the code frames that quote the source only run there.
    #[test]
    fn a_debug_build_keeps_the_copy_whatever_the_option_says() {
      assert!(keeps(true, true));
      assert!(keeps(true, false));
    }

    /// A release build keeps the copy only where no file can be read back,
    /// because the copy is a deep clone of the whole module. Asked from
    /// whichever build runs this, which is the half the walk cannot vary.
    #[test]
    fn a_release_build_keeps_the_copy_only_when_no_file_may_be_read() {
      assert!(!keeps(false, true));
      assert!(keeps(false, false));
    }
  }
}
