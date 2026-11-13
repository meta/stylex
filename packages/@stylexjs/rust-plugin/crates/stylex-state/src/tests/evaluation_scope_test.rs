use super::*;

use std::rc::Rc;
use std::sync::Arc;
use swc_core::{
  common::{DUMMY_SP, FileName, SourceMap, SyntaxContext},
  ecma::ast::{
    BindingIdent, CallExpr, Callee, IdentName, ImportDecl, ImportNamedSpecifier, ImportPhase,
    ImportSpecifier, Lit, MemberExpr, ModuleDecl, ModuleItem, Str,
  },
};

use stylex_enums::declaration_type::DeclarationType;
use stylex_structures::fold_ceilings::MAX_FOLDED_ENTRIES_LIMIT;
use stylex_structures::named_import_source::ImportSources;
use stylex_structures::stylex_env::{HostKind, HostRef};
use stylex_structures::stylex_state_options::StyleXStateOptions;
use stylex_types::enums::data_structures::injectable_style::InjectableStyleKind;
use stylex_types::structures::injectable_style::InjectableStyle;

fn state() -> StateManager {
  StateManager::for_test(None, StyleXStateOptions::default())
}

fn ident(name: &str) -> Ident {
  Ident::new(name.into(), DUMMY_SP, SyntaxContext::empty())
}

fn scoped(name: &str, scope: u32) -> Ident {
  Ident::new(name.into(), DUMMY_SP, SyntaxContext::from_u32(scope))
}

fn declarator(name: &Ident, init: Option<Expr>) -> VarDeclarator {
  VarDeclarator {
    span: DUMMY_SP,
    name: Pat::Ident(BindingIdent {
      id: name.clone(),
      type_ann: None,
    }),
    init: init.map(Box::new),
    definite: false,
  }
}

fn number(value: f64) -> Expr {
  Expr::Lit(Lit::Num(swc_core::ecma::ast::Number {
    span: DUMMY_SP,
    value,
    raw: None,
  }))
}

fn dotted(object: &str, property: &str) -> Expr {
  Expr::Member(MemberExpr {
    span: DUMMY_SP,
    obj: Box::new(Expr::Ident(ident(object))),
    prop: MemberProp::Ident(IdentName {
      span: DUMMY_SP,
      sym: property.into(),
    }),
  })
}

fn import_of(source: &str, local: &Ident) -> ImportDecl {
  ImportDecl {
    span: DUMMY_SP,
    specifiers: vec![ImportSpecifier::Named(ImportNamedSpecifier {
      span: DUMMY_SP,
      local: local.clone(),
      imported: None,
      is_type_only: false,
    })],
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

fn import_item(source: &str) -> ModuleItem {
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

fn a_call_on_a_member() -> Expr {
  Expr::Call(CallExpr {
    span: DUMMY_SP,
    callee: Callee::Expr(Box::new(dotted("stylex", "create"))),
    args: vec![],
    type_args: None,
    ctxt: SyntaxContext::empty(),
  })
}

fn call_and_member(call: &Expr) -> (&CallExpr, &MemberExpr) {
  let Expr::Call(call_expr) = call else {
    unreachable!("the builder writes a call")
  };
  let Callee::Expr(callee) = &call_expr.callee else {
    unreachable!("the builder writes an expression callee")
  };
  let Expr::Member(member) = &**callee else {
    unreachable!("the builder writes a member callee")
  };

  (call_expr, member)
}

fn rule(class_name: &str, css: &str) -> (RuleKey, Rc<InjectableStyleKind>) {
  (
    class_name.into(),
    Rc::new(InjectableStyleKind::Regular(InjectableStyle {
      ltr: css.to_string(),
      rtl: None,
      priority: Some(3000.0),
    })),
  )
}

fn carried(snapshot: &EvaluationSnapshot) -> Vec<String> {
  snapshot
    .declarations()
    .iter()
    .filter_map(|declarator| match &declarator.name {
      Pat::Ident(binding) => Some(binding.id.sym.to_string()),
      _ => None,
    })
    .collect()
}

#[test]
fn a_snapshot_carries_the_names_it_was_asked_for_and_no_others() {
  let mut module = state();

  module.push_declaration(declarator(&ident("wanted"), Some(number(1.0))));
  module.push_declaration(declarator(&ident("unwanted"), Some(number(2.0))));
  module.push_declaration(declarator(&ident("also_unwanted"), Some(number(3.0))));

  let snapshot = module.evaluation_snapshot_for([ident("wanted").to_id()]);

  assert_eq!(carried(&snapshot), ["wanted"]);
  assert!(!snapshot.is_truncated());
}

#[test]
fn a_snapshot_follows_what_a_carried_declaration_reads() {
  let mut module = state();

  module.push_declaration(declarator(&ident("a"), Some(Expr::Ident(ident("b")))));
  module.push_declaration(declarator(&ident("b"), Some(Expr::Ident(ident("c")))));
  module.push_declaration(declarator(&ident("c"), Some(number(1.0))));
  module.push_declaration(declarator(&ident("elsewhere"), Some(number(2.0))));

  let snapshot = module.evaluation_snapshot_for([ident("a").to_id()]);
  let mut names = carried(&snapshot);
  names.sort();

  assert_eq!(names, ["a", "b", "c"]);
}

#[test]
fn a_name_nothing_declares_is_answered_and_carries_nothing() {
  let module = state();

  let snapshot = module.evaluation_snapshot_for([ident("missing").to_id()]);

  assert!(carried(&snapshot).is_empty());
  assert!(snapshot.reached().contains(&ident("missing").to_id()));
}

#[test]
fn a_dotted_property_name_is_not_followed() {
  let mut module = state();

  module.push_declaration(declarator(&ident("a"), Some(dotted("x", "b"))));
  module.push_declaration(declarator(&ident("x"), Some(number(1.0))));
  module.push_declaration(declarator(&ident("b"), Some(number(2.0))));

  let snapshot = module.evaluation_snapshot_for([ident("a").to_id()]);
  let mut names = carried(&snapshot);
  names.sort();

  assert_eq!(names, ["a", "x"]);
}

#[test]
fn an_import_that_binds_a_reached_name_is_carried() {
  let mut module = state();
  let tokens = ident("tokens");

  module.push_top_import(import_of("./tokens.stylex.js", &tokens));
  module.push_top_import(import_of("./other.js", &ident("other")));

  let snapshot = module.evaluation_snapshot_for([tokens.to_id()]);
  let spawned = snapshot.spawn().expect("the walk completed");

  assert!(spawned.import_binding(&tokens).is_some());
  assert!(spawned.import_binding(&ident("other")).is_none());
}

#[test]
fn a_shadowing_binding_resolves_to_itself() {
  let mut module = state();
  let outer = ident("value");
  let inner = scoped("value", 1);

  module.push_declaration(declarator(&outer, Some(number(1.0))));
  module.push_declaration(declarator(&inner, Some(number(2.0))));

  let snapshot = module.evaluation_snapshot_for([inner.to_id()]);
  let spawned = snapshot.spawn().expect("the walk completed");

  assert!(spawned.declaration_of(&inner).is_some());
  assert!(
    spawned.declaration_of(&outer).is_none(),
    "the outer binding was carried for an inner name"
  );
}

#[test]
fn a_walk_that_hits_its_bound_is_terminal() {
  let mut module = state();
  let length = module
    .evaluation_snapshot_for(std::iter::empty())
    .bounds()
    .bindings
    + 32;

  for step in 0..length {
    let next = Expr::Ident(ident(&format!("n{}", step + 1)));
    module.push_declaration(declarator(&ident(&format!("n{}", step)), Some(next)));
  }

  let snapshot = module.evaluation_snapshot_for([ident("n0").to_id()]);

  assert!(snapshot.is_truncated());
  assert!(snapshot.spawn().is_none(), "a cut walk answered a state");
  assert!(
    !snapshot.omitted().is_empty(),
    "the walk was cut without saying what it dropped"
  );
}

#[test]
fn the_bounds_scale_with_the_module_the_snapshot_is_taken_from() {
  let small = snapshot_bounds(0);
  let large = snapshot_bounds(4 * 1024 * 1024);

  assert_eq!(small.bindings, MIN_SNAPSHOT_BINDINGS);
  assert_eq!(small.references, MIN_REFERENCES_PER_INITIALIZER);
  assert!(
    large.bindings > small.bindings,
    "a four-megabyte module admits no more bindings than an empty one"
  );
  assert!(large.references > small.references);
}

#[test]
fn a_chain_inside_the_bound_is_carried_whole() {
  let mut module = state();
  let length = MIN_SNAPSHOT_BINDINGS / 2;

  for step in 0..length {
    let next = Expr::Ident(ident(&format!("n{}", step + 1)));
    module.push_declaration(declarator(&ident(&format!("n{}", step)), Some(next)));
  }

  let snapshot = module.evaluation_snapshot_for([ident("n0").to_id()]);

  assert!(!snapshot.is_truncated());
  assert_eq!(snapshot.declarations().len(), length);
  assert!(snapshot.spawn().is_some());
}

#[test]
fn a_snapshot_leaves_the_modules_own_output_behind() {
  let mut module = state();
  let (key, style) = rule("x1abc", ".x1abc{color:red}");

  module.other_injected_css_rules.insert(key, style);
  module.note_nested_rule_filed();
  module.queue_theme_import_if_absent(import_item("./tokens.stylex.js"));
  module.seen.insert(
    crate::seen_value::SeenKey {
      subtree: crate::seen_value::MemoExpressionKey::SubtreeHash(7),
      context: crate::seen_value::MemoContext {
        inventory: crate::seen_value::StaticMemoInventory::new(Rc::new(
          crate::functions::FunctionMap::default(),
        ))
        .unwrap(),
        characters: 0,
        entries: 0,
        depth: 0,
        span: DUMMY_SP,
        execution_position: None,
        host_epoch: 0,
        host_revision: 0,
        model_epoch: 0,
        rule_epoch: 0,
        input_revision: 0,
      },
    },
    Rc::new(crate::seen_value::SeenValue {
      expression: None,
      value: None,
      resolved: true,
    }),
  );

  let spawned = module
    .evaluation_snapshot_for(std::iter::empty())
    .spawn()
    .expect("an empty walk cannot truncate");

  assert!(spawned.other_injected_css_rules.is_empty());
  assert_eq!(spawned.nested_rules_filed(), 0);
  assert!(spawned.pending_module_items.is_empty());
  assert!(spawned.seen.is_empty());
}

#[test]
fn a_snapshot_leaves_the_modules_call_record_behind() {
  let mut module = state();
  let call = a_call_on_a_member();
  let (call_expr, member) = call_and_member(&call);

  module.add_call_expression(call_expr);

  assert!(module.is_member_call_callee(member));

  let spawned = module
    .evaluation_snapshot_for(std::iter::empty())
    .spawn()
    .expect("an empty walk cannot truncate");

  assert!(
    !spawned.is_member_call_callee(member),
    "the module's call map was copied into the snapshot"
  );
}

#[test]
fn a_seeded_body_answers_for_its_own_calls() {
  let module = state();
  let call = a_call_on_a_member();
  let (_, member) = call_and_member(&call);
  let mut snapshot = module.evaluation_snapshot_for(std::iter::empty());

  snapshot.observe_calls_in(&call);

  let spawned = snapshot.spawn().expect("an empty walk cannot truncate");

  assert!(spawned.is_member_call_callee(member));
}

#[test]
fn a_snapshot_shares_the_safety_records() {
  let mut module = state();

  module.adopt_binding_writes(BindingWrites {
    reassignments: [ident("written").to_id()].into_iter().collect(),
    mutations: Default::default(),
    deep_mutations: Default::default(),
    declared: [ident("written").to_id()].into_iter().collect(),
  });

  let spawned = module
    .evaluation_snapshot_for(std::iter::empty())
    .spawn()
    .expect("an empty walk cannot truncate");

  assert!(Rc::ptr_eq(
    &module.binding_reassignments,
    &spawned.binding_reassignments
  ));
  assert!(Rc::ptr_eq(
    &module.declared_bindings,
    &spawned.declared_bindings
  ));
  assert!(spawned.has_binding_reassignment(&ident("written")));
}

#[test]
fn a_snapshot_shares_the_promotion_heap() {
  let module = state();
  let allocation: Rc<dyn std::any::Any> = Rc::new(7u8);
  let reference = HostRef {
    owner: 1,
    id: Rc::from("ref-1"),
    kind: HostKind::PlainObject,
  };

  module
    .evaluation_heap()
    .borrow_mut()
    .record(1, 0, &allocation, reference.clone());

  let spawned = module
    .evaluation_snapshot_for(std::iter::empty())
    .spawn()
    .expect("an empty walk cannot truncate");

  assert!(Rc::ptr_eq(
    &module.evaluation_heap(),
    &spawned.evaluation_heap()
  ));
  assert_eq!(
    spawned
      .evaluation_heap()
      .borrow()
      .reference(1, 0, &allocation)
      .map(|found| found.id),
    Some(reference.id)
  );
}

#[test]
fn a_snapshot_keeps_the_source_and_its_byte_bound() {
  let mut module = state();
  let source_map = SourceMap::default();
  let source = source_map.new_source_file(
    Arc::new(FileName::Custom("snapshot_fixture.js".to_owned())),
    "const a = 1;".to_owned(),
  );

  module.set_input_source_file(Arc::clone(&source));

  let spawned = module
    .evaluation_snapshot_for(std::iter::empty())
    .spawn()
    .expect("an empty walk cannot truncate");

  assert_eq!(
    spawned
      .input_source_file
      .as_ref()
      .map(|file| file.src.len()),
    Some(source.src.len())
  );
}

#[test]
fn a_snapshot_keeps_the_options() {
  let mut module = state();

  module.options.core.max_evaluation_depth = 7;

  let spawned = module
    .evaluation_snapshot_for(std::iter::empty())
    .spawn()
    .expect("an empty walk cannot truncate");

  assert_eq!(spawned.evaluation_ceiling(), 7);
}

#[test]
fn harvest_takes_what_the_invocation_recorded() {
  let mut spawned = state();
  let (first, first_style) = rule("x1first", ".x1first{color:red}");
  let (second, second_style) = rule("x2second", ".x2second{color:blue}");

  spawned.other_injected_css_rules.insert(first, first_style);
  spawned
    .other_injected_css_rules
    .insert(second, second_style);
  spawned.note_nested_rule_filed();
  spawned.note_nested_rule_filed();
  spawned.queue_theme_import_if_absent(import_item("./tokens.stylex.js"));

  let journal = EvaluationEffects::harvest(&mut spawned);

  assert_eq!(journal.len(), 5);
  assert!(matches!(
    journal.effects().first(),
    Some(EvaluationEffect::Rule { key, .. }) if key.0 == "x1first"
  ));
  assert!(matches!(
    journal.effects()[1],
    EvaluationEffect::Rule { ref key, .. } if key.0 == "x2second"
  ));
  assert_eq!(
    journal
      .effects()
      .iter()
      .filter(|effect| matches!(effect, EvaluationEffect::NestedRuleFiled))
      .count(),
    2
  );
}

#[test]
fn harvesting_twice_answers_nothing_the_second_time() {
  let mut spawned = state();
  let (key, style) = rule("x1once", ".x1once{color:red}");

  spawned.other_injected_css_rules.insert(key, style);
  spawned.note_nested_rule_filed();

  assert_eq!(EvaluationEffects::harvest(&mut spawned).len(), 2);
  assert!(EvaluationEffects::harvest(&mut spawned).is_empty());
}

#[test]
fn harvest_leaves_a_producers_own_placement_alone() {
  let mut spawned = state();

  spawned.queue_insertion(InsertionSlot::PrependImport, import_item("./runtime.js"));
  spawned.queue_theme_import_if_absent(import_item("./tokens.stylex.js"));

  let journal = EvaluationEffects::harvest(&mut spawned);

  assert_eq!(journal.len(), 1);
  assert_eq!(spawned.pending_module_items.len(), 1);
}

#[test]
fn a_journal_replays_into_the_state_it_is_given() {
  let mut spawned = state();
  let (key, style) = rule("x1replay", ".x1replay{color:red}");

  spawned.other_injected_css_rules.insert(key, style);
  spawned.note_nested_rule_filed();
  spawned.queue_theme_import_if_absent(import_item("./tokens.stylex.js"));

  let journal = EvaluationEffects::harvest(&mut spawned);
  let mut outer = state();

  journal.replay_into(&mut outer);

  assert_eq!(outer.other_injected_css_rules.len(), 1);
  assert_eq!(outer.nested_rules_filed(), 1);
  assert_eq!(outer.pending_module_items.len(), 1);
}

#[test]
fn a_journal_replays_the_rules_in_the_order_they_were_filed() {
  let mut spawned = state();

  for name in ["x1a", "x2b", "x3c"] {
    let (key, style) = rule(name, ".x{color:red}");
    spawned.other_injected_css_rules.insert(key, style);
  }

  let journal = EvaluationEffects::harvest(&mut spawned);
  let mut outer = state();

  journal.replay_into(&mut outer);

  let written: Vec<String> = outer
    .other_injected_css_rules
    .keys()
    .map(|key| key.0.clone())
    .collect();

  assert_eq!(written, ["x1a", "x2b", "x3c"]);
}

#[test]
fn replaying_the_same_theme_import_twice_queues_it_once() {
  let mut outer = state();

  for _ in 0..2 {
    let mut spawned = state();
    spawned.queue_theme_import_if_absent(import_item("./tokens.stylex.js"));
    EvaluationEffects::harvest(&mut spawned).replay_into(&mut outer);
  }

  assert_eq!(outer.pending_module_items.len(), 1);
}

#[test]
fn journals_compose_in_order() {
  let mut first = EvaluationEffects::new();
  let mut second = EvaluationEffects::new();
  let (early, early_style) = rule("x1early", ".x{color:red}");
  let (late, late_style) = rule("x2late", ".x{color:blue}");

  first.record_rule(early, early_style);
  second.record_rule(late, late_style);
  first.absorb(second);

  let mut outer = state();
  first.replay_into(&mut outer);

  let written: Vec<String> = outer
    .other_injected_css_rules
    .keys()
    .map(|key| key.0.clone())
    .collect();

  assert_eq!(written, ["x1early", "x2late"]);
}

#[test]
fn publishing_outside_a_scope_answers_false() {
  assert!(!effects_are_collected());
  assert!(!publish_effect(EvaluationEffect::NestedRuleFiled));
}

#[test]
fn a_scope_collects_what_is_published_into_it() {
  let scope = ScopedEffects::install();

  assert!(effects_are_collected());
  assert!(publish_effect(EvaluationEffect::NestedRuleFiled));
  assert!(publish_effect(EvaluationEffect::NestedRuleFiled));

  let journal = scope.finish();

  assert_eq!(journal.len(), 2);
  assert!(
    !effects_are_collected(),
    "the sink outlived the scope that installed it"
  );
}

#[test]
fn a_discarded_scope_replays_nothing() {
  let mut outer = state();

  {
    let scope = ScopedEffects::install();
    let (key, style) = rule("x1lost", ".x{color:red}");

    scope.absorb({
      let mut journal = EvaluationEffects::new();
      journal.record_rule(key, style);
      journal.record_nested_rule_filed();
      journal
    });

    assert_eq!(scope.len(), 2);
    scope.discard();
  }

  assert!(outer.other_injected_css_rules.is_empty());
  assert_eq!(outer.nested_rules_filed(), 0);

  let scope = ScopedEffects::install();
  let (key, style) = rule("x1kept", ".x{color:red}");
  let mut journal = EvaluationEffects::new();

  journal.record_rule(key, style);
  scope.absorb(journal);
  scope.finish().replay_into(&mut outer);

  assert_eq!(outer.other_injected_css_rules.len(), 1);
}

#[test]
fn a_scope_is_uninstalled_even_when_the_body_unwinds() {
  let outer = ScopedEffects::install();

  let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    let _inner = ScopedEffects::install();
    panic!("the callback threw");
  }));

  assert!(unwound.is_err());
  assert!(
    effects_are_collected(),
    "the outer sink was lost with the inner one"
  );

  publish_effect(EvaluationEffect::NestedRuleFiled);

  assert_eq!(outer.finish().len(), 1);
}

#[test]
fn a_nested_scope_does_not_take_the_outer_ones_effects() {
  let outer = ScopedEffects::install();

  publish_effect(EvaluationEffect::NestedRuleFiled);

  let inner = ScopedEffects::install();

  publish_effect(EvaluationEffect::NestedRuleFiled);
  publish_effect(EvaluationEffect::NestedRuleFiled);

  assert_eq!(inner.finish().len(), 2);

  publish_effect(EvaluationEffect::NestedRuleFiled);

  assert_eq!(outer.finish().len(), 2);
}

#[test]
fn a_replayed_journal_cannot_be_replayed_again() {
  let mut outer = state();
  let scope = ScopedEffects::install();
  let (key, style) = rule("x1single", ".x{color:red}");
  let mut journal = EvaluationEffects::new();

  journal.record_rule(key, style);
  scope.absorb(journal);

  let collected = scope.finish();

  collected.replay_into(&mut outer);

  let empty = ScopedEffects::install().finish();

  empty.replay_into(&mut outer);

  assert_eq!(outer.other_injected_css_rules.len(), 1);
  assert_eq!(outer.nested_rules_filed(), 0);
}

#[test]
fn only_a_reached_names_root_record_is_carried() {
  let mut module = state();
  let wanted = ident("wanted");
  let unwanted = ident("unwanted");

  module.push_declaration(declarator(&wanted, Some(number(1.0))));
  module.push_declaration(declarator(&unwanted, Some(number(2.0))));
  module.adopt_module_root_bindings([wanted.to_id(), unwanted.to_id()].into_iter().collect());

  let spawned = module
    .evaluation_snapshot_for([wanted.to_id()])
    .spawn()
    .expect("the walk completed");

  assert!(spawned.is_module_root_binding(&wanted.to_id()));
  assert!(
    !spawned.is_module_root_binding(&unwanted.to_id()),
    "the module's whole root-binding set was carried"
  );
}

#[test]
fn a_pinned_theme_import_replays_into_its_own_slot() {
  let mut spawned = state();
  let declaration = import_of("./tokens.stylex.js", &ident("tokens"));

  spawned.queue_theme_import_at(&declaration);

  let journal = EvaluationEffects::harvest(&mut spawned);

  assert!(matches!(
    journal.effects().first(),
    Some(EvaluationEffect::ThemeImport { at: Some(_), .. })
  ));

  let mut outer = state();

  journal.replay_into(&mut outer);

  assert_eq!(outer.pending_module_items.len(), 1);
  assert!(matches!(
    outer.pending_module_items.first().map(|queued| queued.slot),
    Some(InsertionSlot::ThemeImportAt(span)) if span == declaration.span
  ));
}

#[test]
fn an_invocation_against_a_snapshot_reports_only_its_own_output() {
  let mut module = state();
  let (existing, existing_style) = rule("x1existing", ".x{color:red}");

  module.push_declaration(declarator(&ident("token"), Some(number(1.0))));
  module
    .other_injected_css_rules
    .insert(existing, existing_style);

  let snapshot = module.evaluation_snapshot_for([ident("token").to_id()]);

  let mut later = state();
  let mut spawned = snapshot.spawn().expect("the walk completed");
  let (filed, filed_style) = rule("x2filed", ".x{color:blue}");

  spawned.other_injected_css_rules.insert(filed, filed_style);
  spawned.note_nested_rule_filed();

  EvaluationEffects::harvest(&mut spawned).replay_into(&mut later);

  let written: Vec<String> = later
    .other_injected_css_rules
    .keys()
    .map(|key| key.0.clone())
    .collect();

  assert_eq!(written, ["x2filed"], "the module's own rule was replayed");
  assert_eq!(later.nested_rules_filed(), 1);
  assert!(spawned.declaration_of(&ident("token")).is_some());
}

#[test]
fn a_thousand_unreached_declarations_populate_neither_captured_map() {
  let mut module = state();
  let reached = ident("reached");

  module.add_function_name_declaration(reached.clone());
  module.push_declaration(declarator(&reached, Some(number(1.0))));

  for index in 0..1_000 {
    module.add_function_name_declaration(ident(&format!("helper{index}")));
    module.add_class_name_declaration(ident(&format!("Widget{index}")));
  }

  assert_eq!(
    module.declarations_state.class_names.len() + module.declarations_state.function_names.len(),
    2_001,
    "the module itself holds every record"
  );

  let snapshot = module.evaluation_snapshot_for([reached.to_id()]);

  assert_eq!(
    snapshot.declaration_records(),
    1,
    "only the reached name's own record may travel"
  );

  let spawned = snapshot.spawn().expect("the walk completed");

  assert!(
    matches!(
      spawned.declared_as(&reached),
      Some(DeclarationType::Function)
    ),
    "the reached name lost the kind its declaration gave it"
  );

  for index in [0usize, 999] {
    assert!(
      spawned
        .declared_as(&ident(&format!("helper{index}")))
        .is_none(),
      "an unreached helper was carried"
    );
  }
}

#[test]
fn opaque_fault_snapshots_keep_only_reached_ids_and_original_spans() {
  use crate::state_manager::{OpaqueDeclarationFault, OpaqueDeclarationKind};
  use swc_core::common::{BytePos, Span};

  let mut module = state();
  let wanted = ident("value");
  let shadowed = scoped("value", 1);
  let fault = OpaqueDeclarationFault {
    kind: OpaqueDeclarationKind::VariableDeclarator,
    span: Span::new(BytePos(31), BytePos(52)),
  };
  let mut faults = FxHashMap::default();
  faults.insert(wanted.to_id(), fault);
  faults.insert(
    shadowed.to_id(),
    OpaqueDeclarationFault {
      kind: OpaqueDeclarationKind::Identifier,
      span: Span::new(BytePos(71), BytePos(76)),
    },
  );
  for index in 0..1_000 {
    faults.insert(ident(&format!("unrelated{index}")).to_id(), fault);
  }
  module.adopt_opaque_declaration_faults(faults);

  let snapshot = module.evaluation_snapshot_for([wanted.to_id()]);
  assert!(!snapshot.is_truncated());
  assert_eq!(snapshot.declaration_records(), 1);
  let spawned = snapshot
    .spawn()
    .expect("the reached binding fits the bound");
  assert_eq!(spawned.opaque_declaration_fault(&wanted), Some(fault));
  assert_eq!(spawned.opaque_declaration_fault(&shadowed), None);
  assert_eq!(
    spawned.opaque_declaration_fault(&ident("unrelated999")),
    None
  );
}

#[test]
fn a_thousand_unreached_imports_carry_none_of_their_paths() {
  let mut module = state();
  let tokens = ident("tokens");

  module.insert_stylex_import(ImportSources::Regular("@stylexjs/stylex".to_owned()));
  module.insert_import_path("@stylexjs/stylex".to_owned());
  module.push_top_import(import_of("./tokens.stylex.js", &tokens));
  module.insert_import_path("./tokens.stylex.js".to_owned());

  for index in 0..1_000 {
    module.insert_import_path(format!("./unrelated/{index}.js"));
  }

  assert_eq!(module.imports.import_paths.len(), 1_002);

  let snapshot = module.evaluation_snapshot_for([tokens.to_id()]);

  assert_eq!(snapshot.import_paths(), 2);
  assert!(
    snapshot.retains_import_path("./tokens.stylex.js"),
    "the reached import's own source was dropped"
  );
  assert!(
    snapshot.retains_import_path("@stylexjs/stylex"),
    "the StyleX import source was dropped"
  );

  for index in [0usize, 999] {
    assert!(
      !snapshot.retains_import_path(&format!("./unrelated/{index}.js")),
      "an unreached import's source was carried"
    );
  }

  let spawned = snapshot.spawn().expect("the walk completed");

  assert!(spawned.import_binding(&tokens).is_some());
  assert!(spawned.has_import_paths());
}

#[test]
fn a_published_rule_reaches_the_collector_once_and_stays_in_the_cache() {
  let scope = ScopedEffects::install();
  let publication = CallbackPublication::install();
  let mut local = state();
  let (key, style) = rule("x1pub", ".x{color:red}");

  local.file_evaluation_rule(key, style);

  assert_eq!(scope.len(), 1, "the rule did not reach the collector");
  assert_eq!(publication.rules_published(), 1);
  assert_eq!(
    local.other_injected_css_rules.len(),
    1,
    "the local cache lost the rule it filed"
  );

  assert!(
    publication.harvest(&mut local).is_empty(),
    "a published rule was journalled a second time"
  );
}

#[test]
fn an_unpublished_rule_is_carried_by_the_harvest_as_before() {
  let mut local = state();
  let (key, style) = rule("x1plain", ".x{color:red}");

  local.file_evaluation_rule(key, style);

  assert!(!effects_are_published());
  assert_eq!(EvaluationEffects::harvest(&mut local).len(), 1);
}

#[test]
fn a_nested_calls_effect_lands_where_it_happened_rather_than_at_the_end() {
  let outer_scope = ScopedEffects::install();
  let outer_publication = CallbackPublication::install();
  let mut outer_local = state();

  let (first, first_style) = rule("K1", ".x{color:red}");
  outer_local.file_evaluation_rule(first, first_style);

  {
    let inner_scope = ScopedEffects::install();
    let inner_publication = CallbackPublication::install();
    let mut inner_local = state();
    let (nested, nested_style) = rule("K3", ".x{color:green}");

    inner_local.file_evaluation_rule(nested, nested_style);

    assert!(inner_publication.harvest(&mut inner_local).is_empty());

    let carried = inner_scope.finish();

    assert_eq!(carried.len(), 1, "the nested call collected nothing");

    drop(inner_publication);
    carried.replay_into(&mut outer_local);
  }

  let (second, second_style) = rule("K2", ".x{color:blue}");
  outer_local.file_evaluation_rule(second, second_style);

  assert!(
    outer_publication.harvest(&mut outer_local).is_empty(),
    "the harvest re-journalled an effect the collector already had"
  );

  let sequence: Vec<String> = outer_scope
    .finish()
    .effects()
    .iter()
    .filter_map(|effect| match effect {
      EvaluationEffect::Rule { key, .. } => Some(key.0.clone()),
      _ => None,
    })
    .collect();

  assert_eq!(sequence, ["K1", "K3", "K2"]);
}

#[test]
fn a_filing_is_published_once_and_only_the_unpublished_are_harvested() {
  let scope = ScopedEffects::install();
  let publication = CallbackPublication::install();
  let mut local = state();

  local.note_evaluation_rule_filed();
  local.note_evaluation_rule_filed();

  local.note_nested_rule_filed();

  assert_eq!(scope.len(), 2);
  assert_eq!(publication.filings_published(), 2);
  assert_eq!(local.nested_rules_filed(), 3, "the cache lost a filing");

  let journal = publication.harvest(&mut local);

  assert_eq!(
    journal.len(),
    1,
    "the harvest owes exactly the unpublished one"
  );
  assert_eq!(scope.finish().len(), 2);
}

#[test]
fn repeated_invocations_publish_against_distinct_ledgers() {
  let (key, style) = rule("x1shared", ".x{color:red}");
  let mut sequences = Vec::new();

  for _ in 0..2 {
    let scope = ScopedEffects::install();
    let publication = CallbackPublication::install();
    let mut local = state();

    local.file_evaluation_rule(key.clone(), Rc::clone(&style));
    local.note_evaluation_rule_filed();

    assert_eq!(publication.rules_published(), 1);
    assert_eq!(publication.filings_published(), 1);
    assert!(
      publication.harvest(&mut local).is_empty(),
      "a fresh ledger harvested something it had already published"
    );

    sequences.push(scope.finish().len());
  }

  assert_eq!(
    sequences,
    [2, 2],
    "the second invocation was filtered by the first invocation's ledger"
  );
}

#[test]
fn the_same_key_with_a_different_style_is_published_again() {
  let scope = ScopedEffects::install();
  let _publication = CallbackPublication::install();
  let mut local = state();
  let (key, first) = rule("x1same", ".x{color:red}");
  let (_, second) = rule("x1same", ".x{color:blue}");

  local.file_evaluation_rule(key.clone(), first);
  local.file_evaluation_rule(key, second);

  assert_eq!(scope.len(), 2);
}

#[test]
fn a_caught_callback_failure_publishes_nothing() {
  let mut outer = state();

  {
    let scope = ScopedEffects::install();
    let _publication = CallbackPublication::install();
    let mut local = state();
    let (key, style) = rule("x1lost", ".x{color:red}");

    local.file_evaluation_rule(key, style);
    local.note_evaluation_rule_filed();

    assert_eq!(scope.len(), 2);

    scope.discard();
  }

  assert!(outer.other_injected_css_rules.is_empty());
  assert_eq!(outer.nested_rules_filed(), 0);

  let scope = ScopedEffects::install();
  let _publication = CallbackPublication::install();
  let mut local = state();
  let (key, style) = rule("x1kept", ".x{color:red}");

  local.file_evaluation_rule(key, style);
  scope.finish().replay_into(&mut outer);

  assert_eq!(outer.other_injected_css_rules.len(), 1);
}

#[test]
fn an_isolated_job_does_not_reach_the_collector_around_it() {
  let outer = ScopedEffects::install();
  let _outer_publication = CallbackPublication::install();
  let mut outer_local = state();
  let (before, before_style) = rule("x1before", ".x{color:red}");

  outer_local.file_evaluation_rule(before, before_style);

  with_isolated_evaluation_effects(|| {
    assert!(
      !effects_are_collected(),
      "the collector leaked into the job"
    );
    assert!(!effects_are_published(), "the ledger leaked into the job");

    let mut inner_local = state();
    let (inside, inside_style) = rule("x1inside", ".x{color:green}");

    inner_local.file_evaluation_rule(inside, inside_style);

    let job_scope = ScopedEffects::install();
    let _job_publication = CallbackPublication::install();
    let mut job_local = state();
    let (owned, owned_style) = rule("x1job", ".x{color:blue}");

    job_local.file_evaluation_rule(owned, owned_style);

    assert_eq!(job_scope.len(), 1);
  });

  assert!(effects_are_collected());
  assert!(effects_are_published());

  let (after, after_style) = rule("x1after", ".x{color:blue}");

  outer_local.file_evaluation_rule(after, after_style);

  let sequence: Vec<String> = outer
    .finish()
    .effects()
    .iter()
    .filter_map(|effect| match effect {
      EvaluationEffect::Rule { key, .. } => Some(key.0.clone()),
      _ => None,
    })
    .collect();

  assert_eq!(sequence, ["x1before", "x1after"]);
}

#[test]
fn an_isolated_job_restores_the_scopes_it_took_when_it_unwinds() {
  let outer = ScopedEffects::install();
  let _publication = CallbackPublication::install();

  let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    with_isolated_evaluation_effects(|| panic!("the job threw"));
  }));

  assert!(unwound.is_err());
  assert!(
    effects_are_collected(),
    "the collector was lost with the job"
  );
  assert!(effects_are_published(), "the ledger was lost with the job");

  let mut local = state();
  let (key, style) = rule("x1after", ".x{color:red}");

  local.file_evaluation_rule(key, style);

  assert_eq!(outer.finish().len(), 1);
}

#[test]
fn an_invocation_that_registered_a_producer_is_reported_as_unreplayable() {
  let mut spawned = state();

  spawned.queue_insertion(InsertionSlot::PrependImport, import_item("./runtime.js"));

  let journal = EvaluationEffects::harvest(&mut spawned);

  assert!(journal.is_empty());

  let left = unreplayable_effects(&spawned).expect("the placement must be reported");

  assert_eq!(left.placements, 1);
  assert_eq!(left.total(), 1);
}

#[test]
fn an_invocation_with_only_journalled_effects_reports_nothing() {
  let scope = ScopedEffects::install();
  let publication = CallbackPublication::install();
  let mut spawned = state();
  let (key, style) = rule("x1fine", ".x{color:red}");

  spawned.file_evaluation_rule(key, style);
  spawned.note_evaluation_rule_filed();
  spawned.queue_theme_import_if_absent(import_item("./tokens.stylex.js"));

  let journal = publication.harvest(&mut spawned);

  assert_eq!(journal.len(), 1);
  assert_eq!(
    unreplayable_effects(&spawned),
    None,
    "nothing unreplayable was produced"
  );
  assert_eq!(scope.finish().len(), 2);
}

#[test]
fn an_effect_queued_by_an_immutable_operation_is_published_before_the_next_write() {
  let scope = ScopedEffects::install();
  let publication = CallbackPublication::install();
  let mut local = state();

  let (first, first_style) = rule("K1", ".x{color:red}");
  local.file_evaluation_rule(first, first_style);

  {
    let reader: &StateManager = &local;
    let mut produced = EvaluationEffects::new();
    let (nested, nested_style) = rule("K3", ".x{color:green}");

    produced.record_rule(nested, nested_style);
    reader.queue_evaluation_effects(produced);
  }

  assert_eq!(local.pending_evaluation_effects(), 1, "nothing was queued");
  assert_eq!(
    scope.len(),
    1,
    "a queued effect must not publish on its own"
  );

  let (second, second_style) = rule("K2", ".x{color:blue}");
  local.file_evaluation_rule(second, second_style);

  assert_eq!(
    local.pending_evaluation_effects(),
    0,
    "the queue did not drain"
  );
  assert!(publication.harvest(&mut local).is_empty());

  let sequence: Vec<String> = scope
    .finish()
    .effects()
    .iter()
    .filter_map(|effect| match effect {
      EvaluationEffect::Rule { key, .. } => Some(key.0.clone()),
      _ => None,
    })
    .collect();

  assert_eq!(sequence, ["K1", "K3", "K2"]);
}

#[test]
fn the_harvest_flushes_a_queue_no_writer_followed() {
  let scope = ScopedEffects::install();
  let publication = CallbackPublication::install();
  let mut local = state();
  let mut produced = EvaluationEffects::new();
  let (key, style) = rule("x1last", ".x{color:red}");

  produced.record_rule(key, style);
  local.queue_evaluation_effects(produced);

  assert_eq!(scope.len(), 0);

  assert!(publication.harvest(&mut local).is_empty());
  assert_eq!(scope.finish().len(), 1, "the queued effect was lost");
}

#[test]
fn a_queued_effect_is_replayed_once_however_often_the_queue_is_flushed() {
  let mut local = state();
  let mut produced = EvaluationEffects::new();

  let (key, style) = rule("x1once", ".x{color:red}");

  produced.record_rule(key, style);
  produced.record_nested_rule_filed();
  local.queue_evaluation_effects(produced);

  local.replay_queued_evaluation_effects();

  assert_eq!(local.other_injected_css_rules.len(), 1);
  assert_eq!(local.nested_rules_filed(), 1);

  for _ in 0..3 {
    local.replay_queued_evaluation_effects();
  }

  assert_eq!(
    local.other_injected_css_rules.len(),
    1,
    "a rule was replayed twice"
  );
  assert_eq!(local.nested_rules_filed(), 1, "a filing was counted twice");
}

#[test]
fn a_heap_queued_directly_is_bounded_by_the_floor() {
  let mut heap = EvaluationHeap::default();
  let mut fits = EvaluationEffects::new();

  for _ in 0..MIN_PENDING_EFFECTS {
    fits.record_nested_rule_filed();
  }

  heap.queue_effects_at_the_floor(fits);

  assert_eq!(heap.pending_effects(), MIN_PENDING_EFFECTS);
  assert!(!heap.pending_overflowed(), "the floor itself must fit");

  let mut overflowing = EvaluationEffects::new();

  overflowing.record_nested_rule_filed();
  heap.queue_effects_at_the_floor(overflowing);

  assert_eq!(
    heap.pending_effects(),
    MIN_PENDING_EFFECTS,
    "the queue took part of a batch it could not take whole"
  );
  assert!(heap.pending_overflowed());
}

#[test]
fn a_states_queue_is_sized_from_the_module_rather_than_capped() {
  let local = state();

  assert_eq!(
    local.pending_effects_limit(),
    local.entry_ceiling() * 4,
    "the entry reading is what sizes a default module"
  );
  assert!(
    local.pending_effects_limit() > MIN_PENDING_EFFECTS,
    "the floor must not be the cap"
  );
}

#[test]
fn a_default_module_can_queue_an_event_pair_for_every_entry_it_allows() {
  let local = state();
  let entries = local.entry_ceiling();
  let mut produced = EvaluationEffects::new();

  for index in 0..entries {
    let (key, style) = rule(&format!("x1k{index}"), ".x{color:red}");

    produced.record_rule(key, style);
    produced.record_nested_rule_filed();
  }

  assert_eq!(produced.len(), entries * 2);

  local.queue_evaluation_effects(produced);

  assert_eq!(
    local.pending_evaluation_effects(),
    entries * 2,
    "a module filing one pair per allowed entry was refused"
  );
  assert!(!local.pending_effects_overflowed());
}

#[test]
fn a_tiny_ceiling_is_still_bounded_and_still_gets_the_floor() {
  let mut local = state();

  local.options.max_folded_entries = 1;

  assert_eq!(local.pending_effects_limit(), MIN_PENDING_EFFECTS);

  let mut fits = EvaluationEffects::new();

  for _ in 0..MIN_PENDING_EFFECTS {
    fits.record_nested_rule_filed();
  }

  local.queue_evaluation_effects(fits);

  assert_eq!(local.pending_evaluation_effects(), MIN_PENDING_EFFECTS);
  assert!(!local.pending_effects_overflowed());

  let mut overflowing = EvaluationEffects::new();

  overflowing.record_nested_rule_filed();
  local.queue_evaluation_effects(overflowing);

  assert!(
    local.pending_effects_overflowed(),
    "a tiny ceiling must still be bounded"
  );
}

#[test]
fn the_queue_never_exceeds_the_absolute_ceiling() {
  let mut local = state();

  local.options.max_folded_entries = MAX_FOLDED_ENTRIES_LIMIT;

  assert_eq!(local.pending_effects_limit(), MAX_PENDING_EFFECTS_LIMIT);
}

#[test]
fn draining_the_queue_does_not_forget_that_it_overflowed() {
  let mut local = state();
  let limit = local.pending_effects_limit();
  let mut fits = EvaluationEffects::new();

  fits.record_nested_rule_filed();
  local.queue_evaluation_effects(fits);

  let mut overflowing = EvaluationEffects::new();

  for _ in 0..=limit {
    overflowing.record_nested_rule_filed();
  }

  local.queue_evaluation_effects(overflowing);

  assert_eq!(local.pending_evaluation_effects(), 1);
  assert!(local.pending_effects_overflowed());

  local.replay_queued_evaluation_effects();

  assert_eq!(local.pending_evaluation_effects(), 0);
  assert_eq!(local.nested_rules_filed(), 1, "the queued filing was lost");
  assert!(
    local.pending_effects_overflowed(),
    "a drained queue still lost what it lost"
  );
}

#[test]
fn a_queue_flushed_inside_an_isolated_job_stays_inside_it() {
  let outer = ScopedEffects::install();
  let _outer_publication = CallbackPublication::install();
  let mut local = state();

  let (before, before_style) = rule("x1before", ".x{color:red}");
  local.file_evaluation_rule(before, before_style);

  let mut produced = EvaluationEffects::new();
  let (queued, queued_style) = rule("x1queued", ".x{color:green}");

  produced.record_rule(queued, queued_style);
  local.queue_evaluation_effects(produced);

  with_isolated_evaluation_effects(|| {
    assert!(!effects_are_collected());

    local.replay_queued_evaluation_effects();
  });

  assert_eq!(
    local.pending_evaluation_effects(),
    0,
    "the queue did not drain"
  );
  assert_eq!(
    local.other_injected_css_rules.len(),
    2,
    "the flush must still reach the state it was queued against"
  );

  let sequence: Vec<String> = outer
    .finish()
    .effects()
    .iter()
    .filter_map(|effect| match effect {
      EvaluationEffect::Rule { key, .. } => Some(key.0.clone()),
      _ => None,
    })
    .collect();

  assert_eq!(
    sequence,
    ["x1before"],
    "an effect flushed inside an isolated job leaked to the collector around it"
  );
}

#[test]
fn a_spawned_state_queues_onto_the_heap_it_shares_with_its_module() {
  let mut module = state();
  let reached = ident("reached");

  module.push_declaration(declarator(&reached, Some(number(1.0))));

  let snapshot = module.evaluation_snapshot_for([reached.to_id()]);
  let spawned = snapshot.spawn().expect("the walk completed");

  let mut produced = EvaluationEffects::new();
  let (key, style) = rule("x1shared", ".x{color:red}");

  produced.record_rule(key, style);
  spawned.queue_evaluation_effects(produced);

  assert_eq!(
    module.pending_evaluation_effects(),
    1,
    "the spawned state queued onto a heap of its own"
  );

  let (own, own_style) = rule("x1module", ".x{color:blue}");
  module.file_evaluation_rule(own, own_style);

  assert_eq!(module.other_injected_css_rules.len(), 2);
  assert_eq!(module.pending_evaluation_effects(), 0);
}
