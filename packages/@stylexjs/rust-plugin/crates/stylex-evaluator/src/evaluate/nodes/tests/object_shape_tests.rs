//! What an object literal folds to, key shape by key shape.
//!
//! Every key an object can be written with becomes a string, and the *same*
//! reader decides whether two keys collide -- so a spelling read one way here
//! and another there is how one property comes to be two in the stylesheet.
//! That is why a number key is rendered as JavaScript spells a number rather
//! than as Rust does: `{ 1e21: x }` names the property `"1e+21"`.
//!
//! Every case is read past a declined fold, because an object the engine can
//! read is folded there. `sx.missing ?? <object>` is that shape: the engine
//! declines the name it cannot carry, and the object behind the `??` is what
//! this evaluator then folds.

use std::rc::Rc;

use crate::evaluate::source_evaluation::*;
use stylex_constants::constants::evaluation_errors::OBJECT_METHOD;
use stylex_constants::constants::evaluation_errors::PATH_WITHOUT_NODE;
use stylex_constants::constants::evaluation_errors::UNDEFINED_CONST;
use stylex_constants::constants::messages::{
  ILLEGAL_PROP_ARRAY_VALUE, ILLEGAL_PROP_VALUE, KEY_HAS_NO_NAME, SPREAD_PROPERTIES_UNREADABLE,
};
use stylex_state::{
  functions::{FunctionMap, FunctionType},
  theme_ref::ThemeRef,
};

/// Every object this fold writes carries key-value properties under plain
/// names and nothing else, and so does every value below it.
///
/// The readers of an evaluator-written object rely on that and none of them
/// re-checks it: a reader passes over whatever is not a pair rather than
/// refusing it, so a spread or a getter written here becomes a property the
/// reader silently drops. That is why it is asserted of the fold.
///
/// Both routes, because two things write one: the engine writes the answer it
/// carries out, and this evaluator writes the object it folded itself, which
/// is what a declined fold leaves.
#[test]
fn every_written_object_carries_key_value_properties_only() {
  for object in [
    "{ a: 1, b: 'x' }",
    "{ a: [1, [2]], b: { c: 3 } }",
    // The three shapes that would write something else, each folded away
    // before the object is written: a spread is merged, a shorthand becomes a
    // pair, and a computed key becomes a name.
    "{ a: 1, ...{ b: 2 } }",
    "{ ...{ a: [1] } }",
    "{ ['a']: 1 }",
    // A key that is not a name in the source is one in the answer, which is
    // what lets a reader name a property without asking how it was written.
    "{ 'background-color': 1, 0: 2, 1e21: 3 }",
  ] {
    for source in [format!("({object})"), format!("sx.missing ?? ({object})")] {
      let value = match evaluated_against_a_function_fold(&source).value {
        Some(value) => value,
        None => panic!("expected `{}` to fold", source),
      };

      match value.as_expr() {
        Some(expr) => assert_written_form(expr, &source),
        None => panic!("expected `{}` to fold to an expression", source),
      }
    }
  }
}

/// The keys `object` folds to, joined, read past a declined fold.
#[track_caller]
fn keys_of(object: &str) -> String {
  let source = format!("Object.keys(sx.missing ?? ({object}))[0]");

  folded_text_of(evaluated_against_a_function_fold(&source), &source)
}

/// How many own keys `object` folds to.
#[track_caller]
fn key_count_of(object: &str) -> f64 {
  let source = format!("Object.keys(sx.missing ?? ({object})).length");

  match folded_value_of(evaluated_against_a_function_fold(&source), &source)
    .as_expr()
    .and_then(|expr| expr.as_lit())
  {
    Some(swc_core::ecma::ast::Lit::Num(number)) => number.value,
    other => panic!("expected `{}` to count, got {:?}", source, other),
  }
}

#[track_caller]
fn assert_object_refuses(object: &str, reason: &str) {
  let source = format!("Object.keys(sx.missing ?? ({object})).length");

  assert_refused_with(&evaluated_against_a_function_fold(&source), &source, reason);
}

/// The one property `source` folds to, as the expression it carries.
#[track_caller]
fn function_value_of(source: &str) -> swc_core::ecma::ast::Expr {
  let value = assert_folds_to_a_value(source);

  match value.as_expr().and_then(|expr| expr.as_object()) {
    Some(object) => match object.props.first().and_then(|prop| prop.as_prop()) {
      Some(prop) => match prop.as_key_value() {
        Some(key_value) => *key_value.value.clone(),
        None => panic!("expected `{}` to fold to one key and a value", source),
      },
      None => panic!("expected `{}` to fold to one property", source),
    },
    None => panic!(
      "expected `{}` to fold to an object, got {:?}",
      source, value
    ),
  }
}

/// A number key is rendered as JavaScript spells a number. Rust's own
/// formatting would name `1e21` as its full digits, which is a different
/// property from the one the source wrote.
#[test]
fn a_number_key_is_spelled_as_the_language_spells_it() {
  assert_eq!(keys_of("{ 1e21: 'a' }"), "1e+21");
  assert_eq!(keys_of("{ 1: 'a' }"), "1");
  assert_eq!(keys_of("{ 0.5: 'a' }"), "0.5");
}

/// A bigint key is its digits, which is what the language names the property.
#[test]
fn a_bigint_key_is_its_digits() {
  assert_eq!(keys_of("{ 1n: 'a' }"), "1");
}

/// A computed key is the string its expression folds to, so a key built by
/// concatenation names the property the concatenation spells.
#[test]
fn a_computed_key_is_what_its_expression_folds_to() {
  assert_eq!(keys_of("{ ['a' + 'b']: 'c' }"), "ab");
  assert_eq!(keys_of("{ [1 + 1]: 'c' }"), "2");
}

/// A computed key whose expression resolves to nothing refuses for that
/// expression's own reason, and one that folds to a value with no string at all
/// refuses as a key.
///
/// An entry of the function fold is such a value: `String` of a function is its
/// source text, which this evaluator does not keep. It is the one value left
/// that has no name, now that a boolean, `null` and an object each name the
/// property the language names.
#[test]
fn a_computed_key_that_names_no_string_refuses() {
  assert_object_refuses("{ [unknownName]: 'c' }", UNDEFINED_CONST);
  assert_object_refuses("{ [own]: 'c' }", KEY_HAS_NO_NAME);
}

/// A value that resolves to nothing refuses, and the sentence names the key it
/// was written under -- which is the half of the object an author has to find.
#[test]
fn a_value_that_resolves_to_nothing_names_the_key_it_sits_under() {
  assert_object_refuses(
    "{ a: unknownName }",
    "a > Referenced constant is not defined.",
  );
}

/// An array value holding something with no compile-time form refuses, and
/// says what a style array may hold.
#[test]
fn an_array_value_the_evaluator_cannot_write_down_refuses() {
  assert_object_refuses("{ a: [own] }", ILLEGAL_PROP_ARRAY_VALUE);
}

/// One of the compiler's own functions written as a value folds to the object
/// it stands for, rather than refusing: it is an object upstream, and a
/// declaration reading a key off it has to see the same keys.
#[test]
fn one_of_the_compilers_functions_written_as_a_value_folds_to_its_object() {
  assert_eq!(key_count_of("{ a: own }"), 1.0);
  assert_eq!(key_count_of("{ a: sx }"), 1.0);
}

/// A spread contributes the operand's own keys, whatever kind the operand is:
/// the compiler's own fold, a string, an array, and a value with no own keys at
/// all.
#[test]
fn a_spread_contributes_the_operands_own_keys() {
  assert_eq!(key_count_of("{ ...sx }"), 1.0);
  assert_eq!(key_count_of("{ ...own }"), 1.0);
  assert_eq!(key_count_of("{ ...'ab' }"), 2.0);
  assert_eq!(key_count_of("{ ...[1, 2] }"), 2.0);
  assert_eq!(key_count_of("{ ...1 }"), 0.0);
}

/// An array reaches the spread in either of the two shapes an array can have --
/// the evaluator's own list, and the literal a fold or a property read hands
/// back -- and both contribute the same indices.
#[test]
fn a_spread_reads_an_array_in_both_of_its_shapes() {
  assert_eq!(key_count_of("{ ...[[1, 2], [3]] }"), 2.0);
  assert_eq!(key_count_of("{ ...({ a: [1, 2] }).a }"), 2.0);
}

/// An array of arrays keeps its nesting when it is written into a property, so
/// a value the author wrote two levels deep reaches the stylesheet two levels
/// deep. Read back through both levels, because a flattened array and an empty
/// one both carry the same one key.
#[test]
fn an_array_value_keeps_the_arrays_inside_it() {
  assert_eq!(key_count_of("{ a: [[1, 2]] }"), 1.0);
  assert_folds_to_number("({ a: [[1, 2]] }).a[0][1]", 2.0);
}

/// An array holding a value with no element form refuses the whole property,
/// rather than writing a shorter array than the source describes. An arrow is
/// such a value: it folds to a callback, which no array element can spell.
#[test]
fn an_array_value_holding_a_shape_with_no_element_form_refuses() {
  assert_object_refuses("{ a: [() => 1] }", ILLEGAL_PROP_ARRAY_VALUE);
}

/// A dynamic style's function is kept as the arrow it was written as, and the
/// parentheses an author may put around it are not part of it: the reference
/// implementation's tree has no node for them, so both spellings have to fold
/// to the same value. Reading the bare node refused the parenthesized one,
/// where the same function written without parentheses folded.
#[test]
fn parentheses_around_a_function_value_are_not_part_of_it() {
  for source in [
    "({ a: () => 1 })",
    "({ a: (() => 1) })",
    "({ a: ((() => 1)) })",
  ] {
    assert!(
      matches!(
        function_value_of(source),
        swc_core::ecma::ast::Expr::Arrow(_)
      ),
      "expected `{}` to keep its arrow",
      source
    );
  }
}

/// An array written with a hole does not fold, so a spread of one refuses
/// rather than dropping the hole and shifting every key after it: `{ ...[, 1] }`
/// names the property `1`, where an array read one element short would name
/// `0`. The refusal is the array's own and travels out through the spread,
/// which is what makes a second check here unnecessary.
#[test]
fn a_spread_of_an_array_with_a_hole_refuses() {
  assert_object_refuses("{ ...[, 1] }", PATH_WITHOUT_NODE);
}

/// A string spreads as its characters, one key per UTF-16 code unit. A string
/// holding a lone surrogate has a unit no Rust string can carry, so the spread
/// refuses rather than writing a replacement character the source never had.
#[test]
fn a_spread_of_a_string_the_evaluator_cannot_carry_refuses() {
  assert_object_refuses(r"{ ...'\uD800' }", SPREAD_PROPERTIES_UNREADABLE);
}

/// A `defineVars` group has no keys this compiler can name -- its members live
/// in another file -- so a spread of one is refused rather than answered empty.
/// Answering empty would spread nothing and compile an object the author did
/// not write.
#[test]
fn a_spread_of_a_group_refuses_rather_than_contributing_nothing() {
  let source = "Object.keys(sx.missing ?? ({ ...colors })).length";

  assert_refused_with(
    &evaluated_against(&a_fold_and_a_group(), source),
    source,
    SPREAD_PROPERTIES_UNREADABLE,
  );
}

/// A group written as a style value is not a style value: it stands for the
/// whole group rather than for one of its tokens, and a member read is what
/// names a token. Refused rather than dropped, because a property that vanished
/// would compile a rule the source does not describe.
#[test]
fn a_group_written_as_a_style_value_refuses() {
  let source = "Object.keys(sx.missing ?? ({ a: colors })).length";

  assert_refused_with(
    &evaluated_against(&a_fold_and_a_group(), source),
    source,
    ILLEGAL_PROP_VALUE,
  );
}

/// A dynamic style's function is kept only where the value *is* the arrow. One
/// reached through anything else -- a conditional choosing between two of them
/// -- folds to a function this compiler cannot write back down, so the property
/// refuses rather than compiling a value the source does not describe.
#[test]
fn a_function_reached_through_another_expression_is_not_kept() {
  assert_object_refuses("{ a: true ? () => 1 : () => 2 }", ILLEGAL_PROP_VALUE);
}

/// The fold's own namespace and a group under one map, for the two cases that
/// need both: the namespace makes the engine decline, and the group is the
/// value being written.
fn a_fold_and_a_group() -> FunctionMap {
  let theme = ThemeRef::new("vars.stylex.js", "vars", "x");
  let mut fns = a_function_fold();

  fns.identifiers.insert(
    "colors".into(),
    Box::new(folded_entry(
      FunctionType::ThemeRefMapper(Rc::new(move || theme.clone())),
      false,
    )),
  );

  fns
}

/// A property value that answered nothing while the walk stayed confident is
/// named by its key and its shape. Neither the value nor the state has a
/// sentence of its own there, so a reader would otherwise be told only that
/// something went wrong.
///
/// The memo is what produces such a value, by the route the computed-key case
/// in `member_lookup_tests` describes.
#[test]
fn a_property_value_that_answered_nothing_names_its_key_and_shape() {
  let result = evaluated_after(UNRESOLVED_MEMO_WARM, "({ k: (() => 1) + 1 })");

  assert_refused(&result, "a value the memo answers nothing for");

  let reason = result.reason.unwrap_or_default();

  assert!(
    reason.contains("Value of key 'k' has no compile-time value"),
    "expected the key and the shape to be named, got {:?}",
    reason
  );
}

/// A shorthand method and an accessor are both refused, and the two are refused
/// by different arms of the fold.
///
/// A method is a property whose value is not one, and it is read off the
/// property's own kind before the key is asked for; an accessor falls through
/// to the arm below every readable shape. Every case beside this uses an
/// accessor, so the first arm was entered by nothing.
#[test]
fn a_method_and_an_accessor_are_both_refused() {
  for object in [
    "{ a() { return 1; } }",
    "{ *a() { return 1; } }",
    "{ async a() { return 1; } }",
    "{ get a() { return 1; } }",
    "{ set a(value) {} }",
  ] {
    assert_object_refuses(object, OBJECT_METHOD);
  }
}

/// A key written as a text with no `str` refuses as a key. No Rust string can
/// hold a lone surrogate, so there is no property name to write down.
///
/// It aborted the build from inside a converter before, which reported neither
/// the key nor the object it sits in.
#[test]
fn a_written_key_with_no_name_refuses_as_a_key() {
  assert_object_refuses(r"{ '\ud800': 1 }", KEY_HAS_NO_NAME);
}
