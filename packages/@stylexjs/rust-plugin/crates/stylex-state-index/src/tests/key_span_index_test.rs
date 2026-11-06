#[cfg(test)]
mod key_span_index {
  use std::cmp::Reverse;

  use rustc_hash::FxHashSet;
  use swc_core::{
    atoms::Atom,
    common::{
      BytePos, DUMMY_SP, EqIgnoreSpan, FileName, SourceMap, Span, SyntaxContext, sync::Lrc,
    },
    ecma::{
      ast::{
        CallExpr, Callee, Decl, EsVersion, Expr, ExprStmt, Ident, IdentName, KeyValueProp, Lit,
        Module, ModuleItem, Number, ObjectLit, Prop, PropName, PropOrSpread, Stmt,
      },
      parser::{Parser, StringInput, Syntax, TsSyntax, lexer::Lexer},
    },
  };

  use crate::key_span_index::{
    CallLookup, CandidateRank, FileOffset, KeySpanIndex, ModuleBase, NamespaceKeyQuery,
  };

  fn parse(source: &str) -> Module {
    parse_into(&Default::default(), FileName::Anon, source)
  }

  /// Parses into the caller's source map, allowing fixtures with nonzero module bases.
  fn parse_into(source_map: &Lrc<SourceMap>, name: FileName, source: &str) -> Module {
    let source_file = source_map.new_source_file(name.into(), source.to_owned());
    let lexer = Lexer::new(
      Syntax::Typescript(TsSyntax {
        tsx: true,
        ..Default::default()
      }),
      EsVersion::EsNext,
      StringInput::from(&*source_file),
      None,
    );

    match Parser::new_from(lexer).parse_module() {
      Ok(module) => module,
      Err(error) => panic!("failed to parse the fixture: {:?}", error),
    }
  }

  fn keys(names: &[&str]) -> FxHashSet<Atom> {
    names.iter().map(|name| Atom::from(*name)).collect()
  }

  fn rank(
    namespace_value_overlap: usize,
    sibling_overlap: usize,
    distance_from_target: Option<u32>,
  ) -> CandidateRank {
    CandidateRank {
      callee_match: true,
      namespace_value_overlap,
      sibling_overlap,
      distance_from_target: Reverse(distance_from_target),
    }
  }

  /// Builds a query with explicit ranking signals.
  fn query<'a>(
    key: &'a str,
    siblings: &[&str],
    value_keys: &[&str],
    target_offset: Option<u32>,
  ) -> NamespaceKeyQuery<'a> {
    // Fresh fixture maps start at BytePos(1), so target text indices are file offsets.
    NamespaceKeyQuery {
      namespace_key: key,
      sibling_keys: std::rc::Rc::new(keys(siblings)),
      namespace_value_keys: keys(value_keys),
      target_offset: target_offset.map(FileOffset::at),
      callee: None,
    }
  }

  /// Returns the 1-based line containing `needle`.
  fn line_of(source: &str, needle: &str) -> usize {
    let offset = match source.find(needle) {
      Some(offset) => offset,
      None => panic!("the fixture does not contain {needle}"),
    };

    source[..offset].matches('\n').count() + 1
  }

  /// Returns the 1-based span line; fixture maps start at BytePos(1).
  fn line_at(source: &str, span: Span) -> usize {
    assert!(!span.is_dummy(), "the lookup resolved no position");

    let offset = span.lo.0 as usize - 1;

    source[..offset].matches('\n').count() + 1
  }

  /// Collects stylex.create calls in source order.
  fn create_calls(module: &Module) -> Vec<CallExpr> {
    module
      .body
      .iter()
      .filter_map(|item| match item {
        ModuleItem::Stmt(Stmt::Decl(Decl::Var(variable))) => variable.decls.first(),
        _ => None,
      })
      .filter_map(|declarator| match declarator.init.as_deref() {
        Some(Expr::Call(call)) => Some(call.clone()),
        _ => None,
      })
      .collect()
  }

  fn resolved_line(source: &str, key: &str, siblings: &[&str], value_keys: &[&str]) -> usize {
    let module = parse(source);
    let span = KeySpanIndex::build(&module).resolve(&query(key, siblings, value_keys, None));

    line_at(source, span)
  }

  #[test]
  fn resolves_a_namespace_key_to_the_line_it_is_written_on() {
    let source = "\
const styles = stylex.create({
  root: { color: 'red' },
  hovered: { color: 'blue' },
});
";

    assert_eq!(
      resolved_line(source, "hovered", &["root", "hovered"], &["color"]),
      line_of(source, "hovered:")
    );
  }

  #[test]
  fn resolves_a_key_written_inside_a_typescript_namespace() {
    let source = "\
import * as stylex from '@stylexjs/stylex';
export namespace Demo {
  const styles = stylex.create({
    color: (value: string) => ({ color: value }),
    base: { display: 'flex' },
  });
  export function render() {
    return stylex.props(styles.base, styles.color('red'));
  }
}
";

    assert_eq!(
      resolved_line(source, "base", &["color", "base"], &["display"]),
      line_of(source, "base:")
    );
  }

  #[test]
  fn a_key_no_object_argument_spells_resolves_to_nothing() {
    let module = parse("const styles = stylex.create({ root: { color: 'red' } });");

    let span = KeySpanIndex::build(&module).resolve(&query("missing", &["root"], &["color"], None));

    assert_eq!(span, DUMMY_SP, "a key nothing spells must not resolve");
  }

  #[test]
  fn the_namespace_value_keys_pick_between_two_calls_spelling_the_same_key() {
    let source = "\
const first = stylex.create({
  root: { color: 'red' },
});
const second = stylex.create({
  root: { display: 'flex', flexGrow: 1 },
});
";

    assert_eq!(
      resolved_line(source, "root", &["root"], &["display", "flexGrow"]),
      line_of(source, "root: { display")
    );
  }

  #[test]
  fn two_equally_good_candidates_resolve_to_nothing() {
    let source = "\
const first = stylex.create({
  root: { color: 'red' },
});
const second = stylex.create({
  root: { color: 'red' },
});
";
    let module = parse(source);

    let span = KeySpanIndex::build(&module).resolve(&query("root", &["root"], &["color"], None));

    assert_eq!(
      span, DUMMY_SP,
      "a tie must be refused rather than guessed: a wrong file:line is worse than none"
    );
  }

  #[test]
  fn the_position_nearest_the_compiled_call_breaks_a_tie() {
    let source = "\
const first = stylex.create({
  root: { color: 'red' },
});
const second = stylex.create({
  root: { color: 'red' },
});
";
    let module = parse(source);
    let index = KeySpanIndex::build(&module);

    let second_object_offset = match source.rfind("({") {
      Some(offset) => offset as u32 + 2,
      None => panic!("the fixture no longer holds a call"),
    };

    let span = index.resolve(&query(
      "root",
      &["root"],
      &["color"],
      Some(second_object_offset),
    ));

    assert_eq!(
      line_at(source, span),
      line_of(source, "root:") + 3,
      "the candidate nearest the compiled call must win a tie the keys cannot break"
    );
  }

  #[test]
  fn a_key_written_twice_in_one_object_is_one_candidate_not_two() {
    let source = "\
const styles = stylex.create({
  root: { color: 'red' },
  root: { color: 'blue' },
});
";

    assert_eq!(
      resolved_line(source, "root", &["root"], &["color"]),
      line_of(source, "root: { color: 'blue' }")
    );
  }

  #[test]
  fn a_duplicate_key_is_ranked_by_the_value_that_survives_it() {
    let source = "\
const first = stylex.create({
  root: { color: 'red' },
  root: someVar,
});
const second = stylex.create({
  root: { color: 'red' },
});
";

    assert_eq!(
      resolved_line(source, "root", &["root"], &["color"]),
      line_of(source, "root: { color: 'red' },\n});\n"),
      "a duplicate key must be ranked by the value of its last occurrence"
    );
  }

  /// Constructs a call with a positionless object argument.
  fn call_with_positionless_object(call_lo: u32, key: &str, key_span: Span) -> Stmt {
    let prop = KeyValueProp {
      key: PropName::Ident(IdentName::new(key.into(), key_span)),
      value: Box::new(Expr::Lit(Lit::Num(Number {
        span: DUMMY_SP,
        value: 1.0,
        raw: None,
      }))),
    };

    Stmt::Expr(ExprStmt {
      span: DUMMY_SP,
      expr: Box::new(Expr::Call(CallExpr {
        span: Span::new(BytePos(call_lo), BytePos(call_lo + 1)),
        callee: Callee::Expr(Box::new(Expr::Ident(Ident::new(
          "create".into(),
          DUMMY_SP,
          SyntaxContext::empty(),
        )))),
        args: vec![
          Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(prop)))],
          })
          .into(),
        ],
        type_args: None,
        ctxt: SyntaxContext::empty(),
      })),
    })
  }

  #[test]
  fn a_positionless_object_is_placed_by_its_call() {
    let near_key = Span::new(BytePos(20), BytePos(24));
    let module = Module {
      span: DUMMY_SP,
      shebang: None,
      body: vec![
        ModuleItem::Stmt(call_with_positionless_object(
          100,
          "root",
          Span::new(BytePos(10), BytePos(14)),
        )),
        ModuleItem::Stmt(call_with_positionless_object(200, "root", near_key)),
      ],
    };

    let span = KeySpanIndex::build(&module).resolve(&query("root", &["root"], &[], Some(190)));

    assert_eq!(
      span, near_key,
      "with no object position to measure from, the candidate must be placed by \
       its call: the second one is written nearer the target"
    );
  }

  #[test]
  fn an_object_that_names_nothing_is_not_a_candidate() {
    let source = "\
const spread = stylex.create({
  ...base,
});
const styles = stylex.create({
  root: { color: 'red' },
});
";

    assert_eq!(
      resolved_line(source, "root", &["root"], &["color"]),
      line_of(source, "root: { color: 'red' }")
    );
  }

  #[test]
  fn a_later_candidate_that_ranks_lower_does_not_displace_the_best() {
    let source = "\
const first = stylex.create({
  root: { display: 'flex', flexGrow: 1 },
});
const second = stylex.create({
  root: { color: 'red' },
});
";

    assert_eq!(
      resolved_line(source, "root", &["root"], &["display", "flexGrow"]),
      line_of(source, "root: { display")
    );
  }

  #[test]
  fn a_strict_improvement_clears_an_earlier_tie() {
    let source = "\
const first = stylex.create({ root: { color: 'red' } });
const second = stylex.create({ root: { color: 'red' } });
const third = stylex.create({
  root: { display: 'flex', flexGrow: 1 },
});
";

    assert_eq!(
      resolved_line(source, "root", &["root"], &["display", "flexGrow"]),
      line_of(source, "root: { display")
    );
  }

  #[test]
  fn the_call_digest_separates_two_calls_and_is_stable_for_one() {
    let module = parse(
      "\
const first = stylex.create({ root: { color: 'red' } });
const second = stylex.create({ root: { color: 'red' } });
",
    );

    let calls = create_calls(&module);

    assert_ne!(
      CallLookup::new(&calls[0], Some(ModuleBase::of(&module))).digest(),
      CallLookup::new(&calls[1], Some(ModuleBase::of(&module))).digest(),
      "two calls written at different positions must key apart"
    );
    assert_eq!(
      CallLookup::new(&calls[0], Some(ModuleBase::of(&module))).digest(),
      CallLookup::new(&calls[0], Some(ModuleBase::of(&module))).digest(),
      "one call must digest the same however often it is asked"
    );
  }

  #[test]
  fn a_module_that_does_not_start_the_source_map_is_still_measured_from_itself() {
    let source_map: Lrc<SourceMap> = Default::default();

    // Push this module past the first file in the shared SourceMap.
    let earlier = "const filler = 1;\n".repeat(8);
    let _ = parse_into(
      &source_map,
      FileName::Custom("earlier.ts".to_owned()),
      &earlier,
    );

    let source = "\
const a = stylex.create({ root: { color: 'red' } });
const b = stylex.create({ root: { color: 'red' } });
";
    let module = parse_into(
      &source_map,
      FileName::Custom("under-test.ts".to_owned()),
      source,
    );

    let second_object_offset = match source.rfind("({") {
      Some(offset) => offset as u32 + 2,
      None => panic!("the fixture no longer holds two calls"),
    };

    let span = KeySpanIndex::build(&module).resolve(&query(
      "root",
      &["root"],
      &["color"],
      Some(second_object_offset),
    ));

    assert!(!span.is_dummy(), "the lookup resolved no position");
    assert_eq!(
      source_map.lookup_char_pos(span.lo).line,
      2,
      "the target names the second call's object, so the second call's `root` is \
       the nearer candidate -- measured within the file, which is the only frame \
       the query's offset is expressed in"
    );
  }

  #[test]
  fn a_call_to_another_function_does_not_answer_for_a_create_call() {
    let source = "\
const cfg = createSlice({ root: { color: 'red' } });
const styles = stylex.create({ root: { color: 'red' } });
";
    let module = parse(source);
    let calls = create_calls(&module);

    let create_call = match calls.last() {
      Some(call) => call,
      None => panic!("the fixture no longer holds a call"),
    };

    let mut lookup = query("root", &["root"], &["color"], None);

    lookup.callee = Some(&create_call.callee);

    let span = KeySpanIndex::build(&module).resolve(&lookup);

    assert!(
      !span.is_dummy(),
      "the two candidates tie on every signal a positionless call offers, so a \
       ranking that ignores the callee refuses instead of answering"
    );
    assert_eq!(
      line_at(source, span),
      line_of(source, "const styles"),
      "the `stylex.create` candidate is the one the query is about"
    );
  }

  #[test]
  fn a_lookup_takes_its_anchor_from_the_object_then_from_the_call() {
    let source = "const styles = stylex.create({ root: { color: 'red' } });\n";
    let module = parse(source);
    let calls = create_calls(&module);

    let call = match calls.first() {
      Some(call) => call,
      None => panic!("the fixture no longer holds a call"),
    };

    let from_object = CallLookup::new(call, Some(ModuleBase::of(&module)))
      .query("root")
      .target_offset;

    let mut positionless = call.clone();
    positionless.span = DUMMY_SP;
    positionless.args.clear();

    let from_nothing = CallLookup::new(&positionless, Some(ModuleBase::of(&module)))
      .query("root")
      .target_offset;

    assert!(
      from_object.is_some_and(|offset| offset != FileOffset::at(0)),
      "an object argument's own position is the anchor"
    );
    assert_eq!(
      from_nothing, None,
      "a call with neither an object argument nor a position of its own anchors \
       nothing, rather than anchoring at byte zero -- which would sort before \
       every authored position instead of meaning \"unknown\""
    );
  }

  #[test]
  fn a_call_with_no_object_argument_resolves_to_nothing() {
    let module = parse("const styles = stylex.create(someVar);\n");
    let calls = create_calls(&module);

    let call = match calls.first() {
      Some(call) => call,
      None => panic!("the fixture no longer holds a call"),
    };

    let lookup = CallLookup::new(call, Some(ModuleBase::of(&module)));
    let span = KeySpanIndex::build(&module).resolve(&lookup.query("root"));

    assert_eq!(span, DUMMY_SP);
  }

  #[test]
  fn a_single_line_module_still_separates_its_candidates() {
    let source = "import*as stylex from'x';const a=stylex.create({root:{color:'red'}}),\
b=stylex.create({root:{color:'blue'}});";
    let module = parse(source);

    let second_object_offset = match source.rfind("({") {
      Some(offset) => offset as u32 + 2,
      None => panic!("the fixture no longer holds two calls"),
    };

    let index = KeySpanIndex::build(&module);

    let second = index.resolve(&query(
      "root",
      &["root"],
      &["color"],
      Some(second_object_offset),
    ));
    let first = index.resolve(&query("root", &["root"], &["color"], Some(30)));

    assert!(!first.is_dummy() && !second.is_dummy(), "both must resolve");
    assert_ne!(
      first, second,
      "the two candidates share a line, so only the anchor tells them apart"
    );
    assert!(
      second.lo > first.lo,
      "the anchor nearer the end of the line resolves to the later candidate"
    );
  }

  #[test]
  fn a_lookup_without_a_base_offers_no_proximity_signal() {
    let module = parse("const styles = stylex.create({ root: { color: 'red' } });\n");
    let calls = create_calls(&module);

    let call = match calls.first() {
      Some(call) => call,
      None => panic!("the fixture no longer holds a call"),
    };

    assert_eq!(
      CallLookup::new(call, None).query("root").target_offset,
      None,
      "no base means no measurable distance, not a distance measured from zero"
    );
    assert!(
      CallLookup::new(call, Some(ModuleBase::of(&module)))
        .query("root")
        .target_offset
        .is_some(),
      "and a base is what makes one measurable"
    );
  }

  #[test]
  fn a_property_with_no_readable_name_is_skipped() {
    let source = "\
const styles = stylex.create({
  ...shared,
  root: { color: 'red' },
});
";

    assert_eq!(
      resolved_line(source, "root", &["root"], &["color"]),
      line_of(source, "root:")
    );
  }

  #[test]
  fn a_namespace_bound_to_a_reference_offers_no_value_keys() {
    let module = parse("const styles = stylex.create({ root: someVar });\n");
    let calls = create_calls(&module);

    let call = match calls.first() {
      Some(call) => call,
      None => panic!("the fixture no longer holds a call"),
    };

    assert!(
      CallLookup::new(call, Some(ModuleBase::of(&module)))
        .query("root")
        .namespace_value_keys
        .is_empty(),
      "a namespace that is not an object literal spells no value keys"
    );
  }

  #[test]
  fn the_wrapped_call_is_built_once_and_reused() {
    let module = parse("const styles = stylex.create({ root: { color: 'red' } });\n");
    let calls = create_calls(&module);

    let call = match calls.first() {
      Some(call) => call,
      None => panic!("the fixture no longer holds a call"),
    };

    let lookup = CallLookup::new(call, Some(ModuleBase::of(&module)));
    let first = lookup.wrapped();

    match first {
      Expr::Call(wrapped) => assert!(
        wrapped.eq_ignore_span(call),
        "the wrapper must carry the call it was built from"
      ),
      other => panic!("the wrapper is not a call: {other:?}"),
    }

    assert!(
      std::ptr::eq(first, lookup.wrapped()),
      "a second reader must get the wrapper the first one built"
    );
  }

  #[test]
  fn rank_prefers_value_overlap_then_sibling_overlap_then_proximity() {
    assert!(rank(2, 0, Some(100)) > rank(1, 9, Some(0)));

    assert!(rank(1, 3, Some(100)) > rank(1, 2, Some(0)));

    assert!(rank(1, 3, Some(5)) > rank(1, 3, Some(6)));
  }

  fn module_based_at(base: u32) -> Module {
    Module {
      span: Span {
        lo: BytePos(base),
        hi: BytePos(base + 1_000),
      },
      body: vec![],
      shebang: None,
    }
  }

  #[test]
  fn an_offset_measures_from_the_base_of_its_own_module() {
    let module = module_based_at(4_096);

    assert_eq!(
      FileOffset::of(BytePos(4_126), ModuleBase::of(&module)),
      FileOffset::at(30)
    );
  }

  #[test]
  fn an_offset_holds_at_both_ends_of_the_range() {
    let module = module_based_at(4_096);
    let base = ModuleBase::of(&module);

    assert_eq!(FileOffset::of(BytePos(4_096), base), FileOffset::at(0));
    assert_eq!(
      FileOffset::of(BytePos(u32::MAX), base),
      FileOffset::at(u32::MAX - 4_096)
    );
  }

  #[test]
  fn a_position_before_its_own_base_clamps_to_zero() {
    let module = module_based_at(4_096);

    assert_eq!(
      FileOffset::clamped(BytePos(0), ModuleBase::of(&module)),
      FileOffset::at(0)
    );
  }

  #[test]
  fn the_clamp_answers_what_the_asserted_reader_answers() {
    let module = module_based_at(4_096);
    let base = ModuleBase::of(&module);

    for position in [4_096, 4_097, 4_126, u32::MAX] {
      assert_eq!(
        FileOffset::clamped(BytePos(position), base),
        FileOffset::of(BytePos(position), base),
        "the two readers disagree at {position}"
      );
    }
  }

  #[test]
  #[cfg(debug_assertions)]
  #[should_panic(expected = "precedes the base of the module holding it")]
  fn a_position_before_its_own_base_is_loud_in_a_test_build() {
    let module = module_based_at(4_096);

    let _ = FileOffset::of(BytePos(0), ModuleBase::of(&module));
  }
}
