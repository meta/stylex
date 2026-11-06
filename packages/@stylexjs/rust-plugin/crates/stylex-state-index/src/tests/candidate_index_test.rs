#[cfg(test)]
mod candidate_index {
  use std::cell::Cell;

  use swc_core::{
    atoms::Atom,
    common::{BytePos, DUMMY_SP, Span, SyntaxContext},
    ecma::ast::Id,
  };

  use crate::candidate_index::CandidateIndex;

  /// A key the tests can spell without building an expression, since the index
  /// never derives one itself.
  const KEY: u128 = 0x1234_5678_9abc_def0_1234_5678_9abc_def0;
  const OTHER_KEY: u128 = 1;

  #[test]
  fn hands_back_nothing_before_anything_is_recorded() {
    let index: CandidateIndex<u128, usize> = CandidateIndex::default();

    assert!(index.candidates(|| KEY).is_empty());
  }

  #[test]
  fn hands_back_the_handles_recorded_under_a_key() {
    let mut index = CandidateIndex::default();

    index.record(KEY, 7usize);
    index.record(KEY, 9);

    assert_eq!(index.candidates(|| KEY), [7, 9]);
  }

  #[test]
  fn keeps_the_order_handles_were_recorded_in() {
    let mut index = CandidateIndex::default();

    for handle in [9usize, 3, 5] {
      index.record(KEY, handle);
    }

    assert_eq!(index.candidates(|| KEY), [9, 3, 5]);
  }

  #[test]
  fn records_a_handle_once_however_often_it_is_offered() {
    let mut index = CandidateIndex::default();

    for _ in 0..100 {
      index.record(KEY, 4usize);
    }

    assert_eq!(index.candidates(|| KEY), [4]);
  }

  #[test]
  fn keeps_keys_apart() {
    let mut index = CandidateIndex::default();

    index.record(KEY, 1usize);
    index.record(OTHER_KEY, 2);

    assert_eq!(index.candidates(|| KEY), [1]);
    assert_eq!(index.candidates(|| OTHER_KEY), [2]);
  }

  #[test]
  fn forgetting_drops_only_the_handle_named() {
    let mut index = CandidateIndex::default();

    index.record(KEY, 1usize);
    index.record(KEY, 2);
    index.forget(&KEY, &1);

    assert_eq!(index.candidates(|| KEY), [2]);
  }

  #[test]
  fn forgetting_the_last_handle_empties_the_index() {
    let mut index = CandidateIndex::default();

    index.record(KEY, 1usize);
    index.forget(&KEY, &1);

    // Not merely absent from its bucket: an index that kept an empty bucket
    // would stop answering for free, which is the whole of what the empty case
    // buys.
    assert!(
      index
        .candidates(|| panic!("an empty index must not compute a key"))
        .is_empty()
    );
  }

  #[test]
  fn forgetting_what_was_never_recorded_changes_nothing() {
    let mut index = CandidateIndex::default();

    index.record(KEY, 1usize);
    index.forget(&KEY, &2);
    index.forget(&OTHER_KEY, &1);

    assert_eq!(index.candidates(|| KEY), [1]);
  }

  #[test]
  fn a_handle_can_be_recorded_again_after_being_forgotten() {
    let mut index = CandidateIndex::default();

    index.record(KEY, 1usize);
    index.forget(&KEY, &1);
    index.record(OTHER_KEY, 1);

    assert!(index.candidates(|| KEY).is_empty());
    assert_eq!(index.candidates(|| OTHER_KEY), [1]);
  }

  /// The bucket `forget` removed has to come back, not stay gone: a declarator
  /// whose initializer is replaced and then replaced with the call it first held
  /// takes exactly this path.
  /// `a_handle_can_be_recorded_again_after_being_forgotten` re-records under a
  /// *different* key, so it never exercises it.
  #[test]
  fn a_key_whose_bucket_was_removed_can_be_recorded_under_again() {
    let mut index = CandidateIndex::default();

    index.record(KEY, 1usize);
    index.forget(&KEY, &1);
    index.record(KEY, 1);

    assert_eq!(index.candidates(|| KEY), [1]);
  }

  /// `set_declaration_init` produces this shape whenever a non-call initializer
  /// is replaced by a call: a key to leave that never held the handle.
  #[test]
  fn moving_an_entry_that_was_never_under_the_key_it_leaves_still_joins_the_new_one() {
    let mut index = CandidateIndex::default();

    index.record(OTHER_KEY, 2usize);
    index.move_entry(Some(KEY), Some(OTHER_KEY), 1usize);

    assert_eq!(index.candidates(|| OTHER_KEY), [2, 1]);
  }

  #[test]
  fn computes_no_key_while_the_index_holds_nothing() {
    let index: CandidateIndex<u128, usize> = CandidateIndex::default();
    let computed = Cell::new(0);

    index.candidates(|| {
      computed.set(computed.get() + 1);
      KEY
    });

    assert_eq!(computed.get(), 0);
  }

  #[test]
  fn computes_the_key_once_when_the_index_holds_something() {
    let mut index = CandidateIndex::default();
    let computed = Cell::new(0);

    index.record(OTHER_KEY, 1usize);
    index.candidates(|| {
      computed.set(computed.get() + 1);
      KEY
    });

    assert_eq!(computed.get(), 1);
  }

  #[test]
  fn holds_a_name_as_readily_as_a_position() {
    let mut index = CandidateIndex::default();

    index.record(KEY, String::from("styles"));
    index.record(KEY, String::from("other"));
    index.forget(&KEY, &String::from("styles"));

    assert_eq!(index.candidates(|| KEY), [String::from("other")]);
  }

  /// A key that is not a hash at all. It stands for no production index by
  /// itself -- the three that are keyed by something other than a structural
  /// hash are a `Span`, an `Atom` and an `Id`, each exercised below -- and is
  /// kept because a borrowed key has to narrow as an owned one does.
  #[test]
  fn narrows_on_a_key_that_is_not_a_hash() {
    let mut index: CandidateIndex<&str, usize> = CandidateIndex::default();

    index.record("styles", 0);
    index.record("styles", 4);
    index.record("theme", 1);

    assert_eq!(index.candidates(|| "styles"), [0, 4]);
    assert_eq!(index.candidates(|| "theme"), [1]);
    assert!(index.candidates(|| "absent").is_empty());
  }

  #[test]
  fn moving_an_entry_forgets_the_key_it_left_and_records_the_one_it_joined() {
    let mut index = CandidateIndex::default();

    index.record(KEY, 1usize);
    index.move_entry(Some(KEY), Some(OTHER_KEY), 1);

    assert!(index.candidates(|| KEY).is_empty());
    assert_eq!(index.candidates(|| OTHER_KEY), [1]);
  }

  /// The ordering the method exists to encode: forgetting runs first, so an
  /// entry that moves to the key it already held keeps its record.
  #[test]
  fn moving_an_entry_to_the_key_it_already_held_keeps_it() {
    let mut index = CandidateIndex::default();

    index.record(KEY, 1usize);
    index.move_entry(Some(KEY), Some(KEY), 1);

    assert_eq!(index.candidates(|| KEY), [1]);
  }

  #[test]
  fn moving_an_entry_from_or_to_nothing_touches_only_the_side_given() {
    let mut index = CandidateIndex::default();

    // An entry that had no key joins one.
    index.move_entry(None, Some(KEY), 1usize);
    assert_eq!(index.candidates(|| KEY), [1]);

    // And one that gains no key leaves the one it had.
    index.move_entry(Some(KEY), None, 1);
    assert!(index.candidates(|| KEY).is_empty());

    // Neither side is a no-op rather than a panic.
    index.move_entry(None, None, 1);
    assert!(index.candidates(|| KEY).is_empty());
  }

  /// One lookup site asked before and after anything is recorded, so the same
  /// closure walks both the empty short-circuit and the bucket behind it.
  ///
  /// Written as one site on purpose: the key is taken by a closure, so every
  /// call site is its own instantiation, and a site that only ever meets an
  /// empty index leaves the lookup below it unexercised in that instantiation.
  #[test]
  fn one_lookup_site_answers_before_and_after_anything_is_recorded() {
    fn look(index: &CandidateIndex<u128, usize>) -> Vec<usize> {
      index.candidates(|| KEY).to_vec()
    }

    let mut index = CandidateIndex::default();

    assert!(look(&index).is_empty());

    index.record(KEY, 7usize);

    assert_eq!(look(&index), [7]);
  }

  /// Far past any module, to show the bucket that answers is the one the key
  /// names rather than a walk of everything recorded.
  #[test]
  fn stays_exact_across_a_hundred_thousand_keys() {
    let mut index = CandidateIndex::default();

    for handle in 0..100_000u128 {
      index.record(handle, handle as usize);
    }

    assert_eq!(index.candidates(|| 0), [0]);
    assert_eq!(index.candidates(|| 99_999), [99_999]);
    assert!(index.candidates(|| 100_000).is_empty());
  }

  /// One key that every entry shares -- the worst a collision or a module of
  /// identical calls can do. The index still answers, and still answers with
  /// everything, because narrowing is all it promises: equality is the caller's.
  #[test]
  fn a_bucket_every_handle_shares_still_hands_back_all_of_them() {
    let mut index = CandidateIndex::default();

    for handle in 0..1_000usize {
      index.record(KEY, handle);
    }

    assert_eq!(index.candidates(|| KEY).len(), 1_000);

    for handle in 0..1_000usize {
      index.forget(&KEY, &handle);
    }

    assert!(index.candidates(|| KEY).is_empty());
  }

  // ── The five shapes production instantiates ────────────────────────
  //
  // The state manager builds six indexes, in five distinct instantiations --
  // the declaration and top-level *call* indexes are both
  // `CandidateIndex<u128, usize>`. Coverage reports the generic as covered as
  // soon as any one of them runs, so each shape needs a case of its own: a
  // structural hash over a position and over a name, a `Span`, an `Atom` and
  // an `Id`. The cases above cover `<u128, usize>` and `<u128, String>`;
  // these cover the other three.

  /// `declaration_span_index`: where a declarator was written, keyed by the
  /// span itself.
  #[test]
  fn narrows_on_the_span_a_declarator_was_written_at() {
    let mut index: CandidateIndex<Span, usize> = CandidateIndex::default();
    let first = span(10, 20);
    let second = span(30, 40);

    index.record(first, 0);
    index.record(second, 1);

    assert_eq!(index.candidates(|| first), [0]);
    assert_eq!(index.candidates(|| second), [1]);
    assert!(index.candidates(|| span(10, 21)).is_empty());
  }

  /// Every synthesized node carries the same dummy span, so a span key cannot
  /// tell two of them apart. The bucket therefore holds both, and the caller's
  /// `eq_ignore_span` decides -- which is the contract, not a defect.
  #[test]
  fn dummy_spans_share_one_bucket() {
    let mut index: CandidateIndex<Span, usize> = CandidateIndex::default();

    index.record(DUMMY_SP, 0);
    index.record(DUMMY_SP, 1);

    assert_eq!(index.candidates(|| DUMMY_SP), [0, 1]);
  }

  /// `top_level_name_index`: the positions a name binds, keyed by the name.
  #[test]
  fn narrows_on_the_name_a_top_level_declarator_binds() {
    let mut index: CandidateIndex<Atom, usize> = CandidateIndex::default();
    let styles = Atom::from("styles");

    // `var` permits redeclaration, so one name can bind more than one position.
    index.record(styles.clone(), 0);
    index.record(styles.clone(), 3);
    index.record(Atom::from("theme"), 1);

    assert_eq!(index.candidates(|| styles.clone()), [0, 3]);
    assert_eq!(index.candidates(|| Atom::from("theme")), [1]);
    assert!(index.candidates(|| Atom::from("absent")).is_empty());
  }

  /// `top_import_index`: the import declaration and specifier a binding names,
  /// keyed by the binding rather than by the name.
  #[test]
  fn narrows_on_an_import_binding() {
    let mut index: CandidateIndex<Id, (usize, usize)> = CandidateIndex::default();
    let stylex = binding("stylex", SyntaxContext::empty());

    index.record(stylex.clone(), (0, 1));
    index.record(binding("css", SyntaxContext::empty()), (2, 0));

    assert_eq!(index.candidates(|| stylex.clone()), [(0, 1)]);
    assert!(
      index
        .candidates(|| binding("absent", SyntaxContext::empty()))
        .is_empty()
    );
  }

  /// The reason the import index is keyed by a binding: two scopes may both
  /// declare `styles`, and only the syntax context tells the imports apart. A
  /// key that was the name alone would hand a reference the other scope's
  /// import.
  #[test]
  fn keeps_one_name_under_two_syntax_contexts_apart() {
    let mut index: CandidateIndex<Id, (usize, usize)> = CandidateIndex::default();
    let outer = binding("styles", SyntaxContext::empty());
    // Spelled from a raw value rather than through a `Mark`, which needs the
    // global interner a unit test has no reason to install.
    let inner = binding("styles", SyntaxContext::from_u32(1));

    assert_ne!(outer, inner);

    index.record(outer.clone(), (0, 0));
    index.record(inner.clone(), (1, 0));

    assert_eq!(index.candidates(|| outer.clone()), [(0, 0)]);
    assert_eq!(index.candidates(|| inner.clone()), [(1, 0)]);
  }

  fn span(lo: u32, hi: u32) -> Span {
    Span {
      lo: BytePos(lo),
      hi: BytePos(hi),
    }
  }

  fn binding(name: &str, ctxt: SyntaxContext) -> Id {
    (Atom::from(name), ctxt)
  }
}
