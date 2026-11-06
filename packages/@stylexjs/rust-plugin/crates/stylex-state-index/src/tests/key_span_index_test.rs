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

  /// Parses into a source map the caller owns, so a case can put more than one
  /// file in it. Every other case here gets a fresh map, whose first byte is
  /// `BytePos(1)` -- which is the one arrangement where an offset into the file
  /// and a `BytePos` are interchangeable.
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
      // Held constant across these comparisons: the callee field outranks every
      // other, so a case varying it would be testing only that.
      callee_match: true,
      namespace_value_overlap,
      sibling_overlap,
      distance_from_target: Reverse(distance_from_target),
    }
  }

  /// A lookup for `key`, spelled out rather than read off a compiled call, so a
  /// test says which signals it is giving the index.
  ///
  /// The sibling keys are shared rather than owned by the query -- they belong to
  /// the *call*, and production builds them once per call and hands each
  /// namespace a handle. A case builds its own, since it is testing one lookup.
  fn query<'a>(
    key: &'a str,
    siblings: &[&str],
    value_keys: &[&str],
    target_offset: Option<u32>,
  ) -> NamespaceKeyQuery<'a> {
    // Stated directly: these fixtures parse into a source map of their own,
    // whose first byte is `BytePos(1)`, so a byte index into the fixture text is
    // already the offset production would compute against the module's base.
    NamespaceKeyQuery {
      namespace_key: key,
      sibling_keys: std::rc::Rc::new(keys(siblings)),
      namespace_value_keys: keys(value_keys),
      target_offset: target_offset.map(FileOffset::at),
      // Spelled-out queries name no callee, so every candidate ties on that
      // field and the cases here stay about the signals they vary. The
      // callee-aware path is `a_call_to_another_function_does_not_answer_...`.
      callee: None,
    }
  }

  /// The line `source` holds `key` on, 1-based, so a resolved span can be
  /// asserted as a position an author would recognise rather than as a byte
  /// offset.
  fn line_of(source: &str, needle: &str) -> usize {
    let offset = match source.find(needle) {
      Some(offset) => offset,
      None => panic!("the fixture does not contain {needle}"),
    };

    source[..offset].matches('\n').count() + 1
  }

  /// The 1-based line of `span` within `source`.
  ///
  /// Each fixture is parsed into a source map of its own, whose first byte is
  /// `BytePos(1)`, so a span's offset into the file is one less than its `lo`.
  fn line_at(source: &str, span: Span) -> usize {
    assert!(!span.is_dummy(), "the lookup resolved no position");

    let offset = span.lo.0 as usize - 1;

    source[..offset].matches('\n').count() + 1
  }

  /// Every `stylex.create(...)` call in `module`, in source order, so a case can
  /// name one by position.
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

    // The two calls are indistinguishable by keys alone -- above, without a
    // target position, this same fixture refuses. Pointing the target at one
    // call's object argument is what tells them apart.
    // An offset into the file, not a `BytePos`: the query side and the index
    // side are positioned in different source maps in production, so only
    // offsets compare. This fixture's module starts at the first byte, so the
    // two happen to coincide here -- `a_second_file_in_one_source_map_...`
    // is the case where they do not.
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
      // The second of the two `root:` lines.
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

    // Two candidates would tie and refuse; one resolves, and it is the property
    // a runtime object literal keeps.
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

    // The first call spells `root` twice and shares two sibling keys with the
    // query, so it would win on sibling overlap alone. It loses because the
    // occurrence that survives -- the last one, as at runtime -- has no value
    // object to overlap with, and value overlap outranks siblings.
    assert_eq!(
      resolved_line(source, "root", &["root"], &["color"]),
      line_of(source, "root: { color: 'red' },\n});\n"),
      "a duplicate key must be ranked by the value of its last occurrence"
    );
  }

  /// A call at `call_lo` whose object argument carries no position of its own
  /// and binds `key` at `key_span` -- the shape only a synthesized module has,
  /// and the one the position fallback exists for.
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

    // The first call's object holds no namespace key at all, so it contributes
    // no candidate and cannot tie with, or outrank, the one that does.
    assert_eq!(
      resolved_line(source, "root", &["root"], &["color"]),
      line_of(source, "root: { color: 'red' }")
    );
  }

  /// Every other multi-candidate case here presents its candidates in improving
  /// order, so a loop that simply kept the last one it looked at would pass all
  /// of them. This is the half that says the incumbent is kept.
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

  /// And that a strict improvement *clears* an earlier tie rather than merely
  /// outranking it, which is `resolve`'s `ambiguous = false` on the improvement
  /// arm. Without it the two equal candidates below would refuse the lookup and
  /// the better third one would never be reached.
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

  /// The call half of a span cache key. `cached_span` returns on the key alone --
  /// no structural confirm -- so two calls that share one entry are two styles
  /// where the second is annotated with the first's `file:line`.
  ///
  /// Two calls spelled identically are separated here by *position*, and by more
  /// than one part of the digest: the call and object spans are hashed
  /// explicitly, and `callee` is hashed through `stable_hash_wide`, which keeps
  /// spans. So removing either alone still passes -- what this pins is the
  /// property, not any one term. The property is what the cache depends on.
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

  /// The arrangement production actually runs in, which every other case here
  /// misses by parsing into a source map of its own.
  ///
  /// A `SourceMap` gives each file a start position after the previous file's
  /// end, so from the second file onward a `BytePos` is nowhere near an offset
  /// into that file. The index is built from a module re-parsed into the code
  /// frame's shared, process-global map, while the query is read off the
  /// compiled call in the compiler's per-transform one -- where the same file is
  /// usually the first, and so starts near zero.
  ///
  /// Comparing the two as raw positions adds a constant to every candidate's
  /// distance, which does not cancel: `argmin |base + c - t|` over a large
  /// `base` is just `argmin c`, so the tie-break silently became "the earliest
  /// candidate in the file" and the second call below resolved to the first
  /// call's `root`.
  #[test]
  fn a_module_that_does_not_start_the_source_map_is_still_measured_from_itself() {
    let source_map: Lrc<SourceMap> = Default::default();

    // Registered only to push the module under test off the start of the map,
    // which is what a process that has already compiled one file has done.
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

    // The two candidates are indistinguishable by keys, so the proximity
    // tie-break is the only thing that can separate them -- which is the point.
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

  /// An object handed to some other function is not a worse answer than the
  /// right one, it is not an answer at all.
  ///
  /// The index walks every call with an object first argument, because narrowing
  /// it to StyleX would mean teaching it the module's import bindings. So an
  /// application module full of `useMemo` and `createSlice` puts candidates here
  /// that can never be right, and one of them spelling a style namespace is only
  /// a matter of time.
  ///
  /// Where that bites is a call with no position of its own -- one an earlier
  /// transform synthesized, which is the case this index's value-matching
  /// fallback exists for. Every distance is then unmeasured, every candidate
  /// ties, and a tie is refused: the lookup falls back to the whole-module value
  /// walk this index replaced. Ranking the callee first separates them without a
  /// position.
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

  /// `CallLookup` reads its anchor off the object argument, and falls back to
  /// the call's own position when the object has none. Every other case builds
  /// its query by hand, so neither the fallback nor the `is_dummy` guard beside
  /// it was ever taken.
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

  /// `stylex.create(someVar)` has no object argument to read sibling keys from,
  /// so the lookup offers nothing and the index answers nothing. Reached through
  /// `CallLookup` rather than a hand-built query, which is the path production
  /// takes.
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

  /// A module with the whole program on one line, which is what a bundler hands
  /// a downstream transform. Nothing else here feeds one, and it is the shape
  /// where the line number carries no information at all -- so the column, and
  /// with it the proximity tie-break, is the only thing separating two
  /// candidates.
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

  /// A lookup built without a base offers no proximity signal, rather than
  /// measuring from zero.
  ///
  /// The difference between `Option<ModuleBase>` and a base defaulting to zero,
  /// which `ModuleBase`'s own documentation argues. Not hypothetical: the base
  /// used to be recorded only on the configurations that also memoize the
  /// module, while the lookup is reachable on the ones that do not.
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

  /// A property the index cannot read a name off -- a spread here -- is skipped,
  /// while the properties beside it are still indexed.
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

  /// The compiled call spells the key but binds it to a reference rather than to
  /// an object literal, so the namespace contributes no value keys to rank on.
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

  /// The wrapped call is a deep clone, so it is built once and handed back to
  /// every later caller rather than cloned again.
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
    // Namespace-value overlap dominates every other signal.
    assert!(rank(2, 0, Some(100)) > rank(1, 9, Some(0)));

    // Sibling-key overlap breaks value-overlap ties.
    assert!(rank(1, 3, Some(100)) > rank(1, 2, Some(0)));

    // Smaller distance to the target wins a full overlap tie.
    assert!(rank(1, 3, Some(5)) > rank(1, 3, Some(6)));

    // Deliberately not asserted here: that `Reverse(None)` outranks every
    // measured distance, and that identical signals rank equal. The first is an
    // ordering no input reaches, because the `Option` comes from the query and
    // is uniform across one `resolve`; the second is `x == x` under a derived
    // `PartialEq`. What a lookup does with a tie is
    // `two_equally_good_candidates_resolve_to_nothing`.
  }

  /// A module whose first byte is not the first byte of the source map, which
  /// is the arrangement every file after the first is in.
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

  /// The subtraction the type exists for: a position is recorded as the
  /// distance into its own file, not as the raw source-map position.
  #[test]
  fn an_offset_measures_from_the_base_of_its_own_module() {
    let module = module_based_at(4_096);

    assert_eq!(
      FileOffset::of(BytePos(4_126), ModuleBase::of(&module)),
      FileOffset::at(30)
    );
  }

  /// The base itself is offset zero, and the largest position a `BytePos` can
  /// hold does not overflow the subtraction.
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

  /// A release build clamps a position that precedes its own base instead of
  /// wrapping, because a code frame must not stop the build.
  ///
  /// The clamp is what a release build ships, because the assertion in
  /// [`FileOffset::of`] compiles out of that profile. The case reads the
  /// arithmetic directly, so every profile runs it.
  #[test]
  fn a_position_before_its_own_base_clamps_to_zero() {
    let module = module_based_at(4_096);

    assert_eq!(
      FileOffset::clamped(BytePos(0), ModuleBase::of(&module)),
      FileOffset::at(0)
    );
  }

  /// Splitting the arithmetic out of the assertion must not have changed the
  /// answer, so the two agree everywhere the assertion holds -- at the base,
  /// one past it, and at the top of the range.
  ///
  /// This is the whole risk the split carries. The case below it is what the
  /// split makes reachable; this one is what it must leave alone.
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

  /// And a test build is loud about it, because clamping every candidate to
  /// zero is the "rank by earliest in the file" failure the type prevents.
  #[test]
  #[cfg(debug_assertions)]
  #[should_panic(expected = "precedes the base of the module holding it")]
  fn a_position_before_its_own_base_is_loud_in_a_test_build() {
    let module = module_based_at(4_096);

    let _ = FileOffset::of(BytePos(0), ModuleBase::of(&module));
  }
}
