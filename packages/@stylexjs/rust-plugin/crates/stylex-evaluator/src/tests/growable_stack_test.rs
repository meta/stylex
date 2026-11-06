//! Tests per-level stack growth, whole-stage claims and bounded nesting counts.

use stylex_structures::evaluation_depth::MAX_EVALUATION_DEPTH_LIMIT;

use super::scaffolding::{LARGE_THREAD, SMALL_THREAD, nested_literal, on_a_thread_of, parse_expr};
use crate::growable_stack::{claim_for, grown_for_depth, grown_per_level, nesting_of};

const FRAME: usize = 16 * 1024;

#[derive(Clone, Copy)]
enum Asking {
  AtEveryLevel,
  Never,
}

fn descend(levels: usize, asking: Asking) -> usize {
  let mut frame = [0u8; FRAME];

  frame[0] = levels as u8;

  if levels == 0 {
    return std::hint::black_box(&frame)[0] as usize;
  }

  let deeper = match asking {
    Asking::AtEveryLevel => grown_per_level(|| descend(levels - 1, asking)),
    Asking::Never => descend(levels - 1, asking),
  };

  std::hint::black_box(&frame);

  deeper + 1
}

#[test]
fn asking_by_the_level_carries_a_walk_past_the_thread_it_started_on() {
  let levels = 8 * 1024 * 1024 / FRAME;

  assert_eq!(
    on_a_thread_of(SMALL_THREAD, move || descend(levels, Asking::AtEveryLevel)),
    levels
  );
}

#[test]
fn a_claim_carries_a_descent_that_never_asks_again() {
  let claimed = 128;
  let levels = claimed * 2;

  assert_eq!(
    on_a_thread_of(SMALL_THREAD, move || grown_for_depth(claimed, || descend(
      levels,
      Asking::Never
    ))),
    levels
  );
}

#[test]
fn the_largest_ceiling_a_project_can_configure_can_be_claimed() {
  let room = on_a_thread_of(SMALL_THREAD, || {
    grown_for_depth(MAX_EVALUATION_DEPTH_LIMIT, stacker::remaining_stack)
  });

  let claimed = claim_for(MAX_EVALUATION_DEPTH_LIMIT);

  // remaining_stack may be unavailable; when present it excludes platform entry and guard space.
  if let Some(bytes) = room {
    assert!(
      bytes >= claimed,
      "the largest claim left {} bytes underfoot rather than {}",
      bytes,
      claimed
    );
  }
}

#[test]
fn platform_stack_reserve_does_not_consume_the_claimed_payload() {
  for levels in [1, 8, 128] {
    let room = on_a_thread_of(SMALL_THREAD, move || {
      grown_for_depth(levels, stacker::remaining_stack)
    });
    if let Some(bytes) = room {
      assert!(
        bytes >= claim_for(levels),
        "{levels} levels left only {bytes} usable bytes"
      );
    }
  }
}

#[test]
fn a_claim_that_is_already_covered_allocates_nothing() {
  let ceiling = 8;

  let (before, inside) = on_a_thread_of(SMALL_THREAD, move || {
    let before = stacker::remaining_stack();
    let inside = grown_for_depth(ceiling, stacker::remaining_stack);

    (before, inside)
  });

  if let (Some(before), Some(inside)) = (before, inside) {
    assert!(
      inside <= before && before - inside < FRAME,
      "a covered claim moved the walk onto {} bytes from {}",
      inside,
      before
    );
  }
}

#[test]
fn a_panic_crosses_a_grown_stack_with_its_payload() {
  for way in ["per level", "claimed"] {
    let raised = std::panic::catch_unwind(|| match way {
      "per level" => grown_per_level(|| panic!("{}", way)),
      _ => grown_for_depth(MAX_EVALUATION_DEPTH_LIMIT, || panic!("{}", way)),
    });

    let payload = raised.expect_err("the panic did not cross the boundary");

    assert_eq!(
      payload.downcast_ref::<String>().map(String::as_str),
      Some(way)
    );
  }
}

#[test]
fn a_leaf_nests_one_level() {
  assert_eq!(nesting_of(&parse_expr("'x'")), 1);
  assert_eq!(nesting_of(&parse_expr("1")), 1);
}

#[test]
fn a_nested_literal_nests_once_per_level_and_once_for_its_leaf() {
  for levels in [1, 2, 17, 300] {
    assert_eq!(nesting_of(&parse_expr(&nested_literal(levels))), levels + 1);
  }
}

#[test]
fn the_deepest_branch_is_what_the_count_answers() {
  assert_eq!(nesting_of(&parse_expr("['x', [['y']], 'z']")), 4);
  assert_eq!(nesting_of(&parse_expr("[[['y']], 'x']")), 4);
}

#[test]
fn objects_calls_and_templates_all_nest() {
  assert_eq!(nesting_of(&parse_expr("({ a: { b: 'x' } })")), 4);
  assert_eq!(nesting_of(&parse_expr("f(g(h('x')))")), 4);
  assert_eq!(nesting_of(&parse_expr("`a${`b${'c'}`}`")), 3);
  assert_eq!(nesting_of(&parse_expr("(x) => [[x]]")), 4);
}

#[test]
fn statements_nest_as_expressions_do() {
  assert_eq!(nesting_of(&parse_expr("(() => { if(1){} })")), 4);
  assert_eq!(nesting_of(&parse_expr("(() => { if(1){ if(1){} } })")), 6);
}

#[test]
fn patterns_nest_as_expressions_do() {
  assert_eq!(nesting_of(&parse_expr("(([q]) => q)")), 4);
  assert_eq!(nesting_of(&parse_expr("(([[q]]) => q)")), 5);
}

#[test]
fn brackets_inside_a_string_nest_nothing() {
  let source = format!("'{}'", "[".repeat(10000));

  assert_eq!(nesting_of(&parse_expr(&source)), 1);
}

#[test]
fn a_tree_past_the_largest_claim_is_counted_no_further() {
  let answered = on_a_thread_of(LARGE_THREAD, || {
    let levels = MAX_EVALUATION_DEPTH_LIMIT * 2;
    // Core-only deep fixture: 24 KiB per level covers Linux O0 parser cost and arm64 headroom.
    super::scaffolding::on_a_stack_for(levels, 24 * 1024, || {
      nesting_of(&parse_expr(&nested_literal(levels)))
    })
  });

  assert_eq!(answered, MAX_EVALUATION_DEPTH_LIMIT + 1);
}

#[test]
fn the_count_carries_itself_past_the_thread_it_started_on() {
  let deep = on_a_thread_of(LARGE_THREAD, || parse_expr(&nested_literal(2000)));

  assert_eq!(
    on_a_thread_of(SMALL_THREAD, move || {
      let counted = nesting_of(&deep);

      // Drop this deeply recursive AST on the fixture stack.
      std::mem::forget(deep);

      counted
    }),
    2001
  );
}
