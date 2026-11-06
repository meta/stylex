//! Tests for evaluation error message functions and static constants.

use crate::constants::{
  common::{VALID_CALLEES, VALUE_ONLY_GLOBALS},
  evaluation_errors::*,
};

#[test]
fn test_unsupported_operator() {
  assert_eq!(unsupported_operator("+"), "Unsupported operator: +\n\n");
  assert_eq!(unsupported_operator("**"), "Unsupported operator: **\n\n");
}

#[test]
fn test_unsupported_expression() {
  assert_eq!(
    unsupported_expression("AwaitExpression"),
    "Unsupported expression: AwaitExpression\n\n"
  );
}

#[test]
fn test_uncoercible_value() {
  assert_eq!(
    uncoercible_value("String"),
    "Cannot coerce this value at compile time.\nOnly static values can be passed to String().\n\n"
  );
  assert!(uncoercible_value("Number").contains("Number()"));
}

#[test]
fn test_array_length_too_large() {
  assert_eq!(
    array_length_too_large(10_000),
    "Array length is too large to evaluate at compile time.\nAt most 10000 elements are supported.\n\n"
  );
}

#[test]
fn test_expression_too_deep() {
  assert_eq!(
    expression_too_deep(32),
    "Expression is too deeply nested to evaluate at compile time.\nAt most 32 levels of nested evaluation are supported.\n\n"
  );
  // The depth is the caller's, not a constant baked into the message: a
  // configured ceiling has to be the number an author reads.
  assert!(expression_too_deep(320).contains("At most 320 levels"));
}

#[test]
fn test_nesting_too_deep_to_carry() {
  assert_eq!(
    nesting_too_deep_to_carry(8192),
    "Expression is nested too deeply to fold at compile time.\nAt most 8192 levels of nested expressions can be carried.\n\n"
  );

  // Distinct from the depth ceiling's sentence, because the two count different
  // things: an author told to shorten their *evaluation* would be reading about
  // something they had not written.
  assert_ne!(nesting_too_deep_to_carry(8192), expression_too_deep(8192));
}

#[test]
fn test_not_a_function() {
  assert_eq!(
    not_a_function("Math"),
    "Math is not a function.\nOnly its methods can be called.\n\n"
  );
}

#[test]
fn test_static_constants() {
  assert!(!PATH_WITHOUT_NODE.is_empty());
  assert!(!UNEXPECTED_MEMBER_LOOKUP.is_empty());
  assert!(!IMPORT_PATH_RESOLUTION_ERROR.is_empty());
  assert!(!NON_CONSTANT.is_empty());
  assert!(!UNDEFINED_CONST.is_empty());
  assert!(!OBJECT_METHOD.is_empty());
  assert!(!BLOCKED_PROPERTY_ACCESS.is_empty());
}

/// The sentence a read onto the prototype chain hands an author, asserted word
/// for word because it is a fixed one.
#[test]
fn test_blocked_property_access() {
  assert_eq!(
    BLOCKED_PROPERTY_ACCESS,
    concat!(
      "Access to this property is not allowed during compilation.\n",
      "Accessing prototype-chain properties such as 'constructor', '__proto__', or 'prototype'\n",
      "is blocked to prevent arbitrary code execution.\n"
    )
  );
}

// The refusals a fold hands an author when it declines a call. Each names the
// method or the limit it refused on, so each is asserted with that half in
// place rather than as a fixed sentence — the name is the part that says what
// to write instead.

#[test]
fn test_locale_sensitive_method() {
  assert_eq!(
    locale_sensitive_method("toLocaleUpperCase"),
    "Cannot fold 'toLocaleUpperCase' at compile time.\nIts answer depends on locale data the compiler does not carry.\n\n"
  );
  assert!(locale_sensitive_method("localeCompare").contains("'localeCompare'"));
}

#[test]
fn test_numeric_literal_receiver() {
  assert_eq!(
    numeric_literal_receiver("toFixed"),
    "Cannot call 'toFixed' on a number literal.\nOnly a number a fold produced can be a method receiver.\n\n"
  );
}

#[test]
fn test_unbounded_amplified_length() {
  assert_eq!(
    unbounded_amplified_length("repeat", 1_000_000),
    "Cannot bound the string 'repeat' would build.\nIts length must resolve to a number of at most 1000000, on a receiver whose own length can be read.\n\n"
  );
  // The limit is the caller's, like the depth ceiling above: a bound an author
  // can raise has to be the number they read.
  assert!(unbounded_amplified_length("padStart", 32).contains("at most 32"));
}

// The same first line on the other unit, for a length that reached the
// conversion behind the fold rather than the guard. It names no limit, because
// the refusal is that nothing measured the length -- not that a number was too
// large -- so what it offers is the two ways to make the length readable.
#[test]
fn test_unbounded_declared_length() {
  assert_eq!(
    unbounded_declared_length("Array"),
    "Cannot bound the array 'Array' would build.\nWrite the elements out, or keep the rest of the expression foldable so its length can be checked.\n\n"
  );
}

// The other half of the same question, and the one an author reaches by asking
// for a length that *was* read: it names what was asked for beside the limit,
// because the two together say whether this is a typo or a project that has
// outgrown the ceiling.
#[test]
fn test_amplified_length_too_large() {
  // A call that pads builds what it was asked for, so there is one number and
  // it is named once.
  assert_eq!(
    amplified_length_too_large("padStart", 1_200_000, 1_200_000, 1, 1_000_000),
    "Cannot bound the string 'padStart' would build.\nIt asks for 1200000 characters, and at most 1000000 are supported.\n\n"
  );
  // A call that repeats builds the count times its receiver, and the count is
  // the only one of the two an author can find in what they wrote -- so both
  // are named, rather than a total that appears nowhere in the source.
  assert_eq!(
    amplified_length_too_large("repeat", 600_000, 1_200_000, 1, 1_000_000),
    "Cannot bound the string 'repeat' would build.\nIt asks for 600000 copies of the value it is called on, which is 1200000 characters, and at most 1000000 are supported.\n\n"
  );
  // Inside a callback the call is evaluated once per element of the receiver, so
  // the number the ceiling was compared against is a product -- and the sentence
  // has to say so, because a count of two against a limit of ten reads as being
  // well inside it.
  assert_eq!(
    amplified_length_too_large("repeat", 3, 3, 4, 10),
    "Cannot bound the string 'repeat' would build.\nIt asks for 3 characters once per element of the receiver it is written inside, which is 4 evaluations and 12 characters in all, and at most 10 are supported.\n\n"
  );
  // One evaluation is the common case and says nothing about repeats, so the
  // clause appears only where there is more than one.
  assert!(!amplified_length_too_large("repeat", 3, 3, 1, 2).contains("once per element"));
  // Both refusals of an amplifying call open with the same line, so an author
  // learns to recognise one class of failure rather than two.
  assert!(
    amplified_length_too_large("padEnd", 9, 9, 1, 8)
      .starts_with("Cannot bound the string 'padEnd' would build.")
  );
}

// The sibling of the two above in the other unit: a length a call declares
// rather than a count of copies, so there is one number to name and not two.
#[test]
fn test_amplified_entries_too_large() {
  assert_eq!(
    amplified_entries_too_large("Array", 20_000, 1, 10_000),
    "Cannot bound the array 'Array' would build.\nIt declares a length of 20000 elements, and at most 10000 are supported.\n\n"
  );
  // Inside a callback the declaration is made once per element, so the number
  // the ceiling was compared against is the product and the sentence says so.
  assert_eq!(
    amplified_entries_too_large("Array.from", 9_999, 3, 10_000),
    "Cannot bound the array 'Array.from' would build.\nIt declares a length of 9999 elements once per element of the receiver it is written inside, which is 3 evaluations and 29997 elements in all, and at most 10000 are supported.\n\n"
  );
  // Both spellings that declare a length read the same way, and one evaluation
  // says nothing about repeats.
  assert!(!amplified_entries_too_large("Array", 3, 1, 2).contains("once per element"));
}

#[test]
fn test_object_size_too_large() {
  assert_eq!(
    object_size_too_large(10_000),
    "Object is too large to evaluate at compile time.\nAt most 10000 properties are supported.\n\n"
  );
}

#[test]
fn test_unfoldable_fold_result() {
  assert_eq!(
    unfoldable_fold_result("function"),
    "Cannot carry a folded function back from the engine.\nOnly strings, numbers, booleans, null, undefined, arrays and plain objects can be folded.\n\n"
  );
  // The kind is passed through without an article in front of it, which is what
  // lets the caller hand over the language's own `typeof` unchanged.
  assert!(unfoldable_fold_result("symbol").starts_with("Cannot carry a folded symbol back"));
  assert!(unfoldable_fold_result("bigint").starts_with("Cannot carry a folded bigint back"));
}

#[test]
fn test_engine_threw() {
  assert_eq!(
    engine_threw(
      "reduce",
      "TypeError: Reduce of empty array with no initial value"
    ),
    "Cannot fold 'reduce' at compile time.\nTypeError: Reduce of empty array with no initial value\n\n"
  );
  // The method is named by this compiler because the engine's own sentence does
  // not always name it: a call to a method that does not exist reads
  // `undefined` and calls it, so the language says only this much.
  assert!(
    engine_threw("unsupported", "TypeError: not a callable function").contains("'unsupported'")
  );
}

#[test]
fn test_escaping_property() {
  assert_eq!(
    escaping_property("constructor"),
    "Cannot fold a read of 'constructor' at compile time.\nIt leads off the value that was written and onto the language's own function graph.\n\n"
  );
  // The three names that turn an unapplied function back into a call read the
  // same way, because the property is the whole of the reason.
  assert!(escaping_property("bind").contains("'bind'"));
}

#[test]
fn test_amplification_inside_a_callback() {
  assert_eq!(
    amplification_inside_a_callback("string", "repeat"),
    "Cannot bound the string 'repeat' would build inside a callback.\nThe callback's receiver holds an element count that cannot be read here, so a length written into the source bounds one evaluation rather than the call. Write the receiver's elements out, or write the value out instead.\n\n"
  );
  assert!(amplification_inside_a_callback("string", "padStart").contains("'padStart'"));
  // The same sentence in the other unit, which is the whole of what the noun is
  // for: one message rather than two that could come to disagree.
  assert!(
    amplification_inside_a_callback("array", "Array").contains("Cannot bound the array 'Array'")
  );
}

#[test]
fn test_folded_string_too_large() {
  assert_eq!(
    folded_string_too_large(1_000_000),
    "Folded string is too large to evaluate at compile time.\nAt most 1000000 characters are supported.\n\n"
  );
  // The number an author reads is the limit they can raise, so it is the one
  // the caller passed rather than a constant spelled again here.
  assert!(folded_string_too_large(32).contains("At most 32"));
}

#[test]
fn test_grown_string_too_large() {
  assert_eq!(
    grown_string_too_large(CONCATENATION, 1_000_000),
    "This concatenation builds a string too large to evaluate at compile time.\nAt most 1000000 characters are supported.\n\n"
  );
  assert_eq!(
    grown_string_too_large(TEMPLATE_LITERAL, 8),
    "This template literal builds a string too large to evaluate at compile time.\nAt most 8 characters are supported.\n\n"
  );
  // The two kinds are what tells an author which expression on the line grew the
  // string, so neither may read as the other.
  assert_ne!(
    grown_string_too_large(CONCATENATION, 8),
    grown_string_too_large(TEMPLATE_LITERAL, 8)
  );
  // Bounded by the same number as a folded string and stated the same way, so an
  // author who raises the ceiling for one raises it for both.
  assert!(
    grown_string_too_large(CONCATENATION, 32).contains(
      &folded_string_too_large(32)
        .lines()
        .nth(1)
        .unwrap_or_default()
        .to_string()
    )
  );
}

#[test]
fn test_bound_value_too_large() {
  assert_eq!(
    bound_value_too_large("big", 1_000_000),
    "Cannot carry the value of 'big' into a fold.\nAt most 1000000 characters are supported.\n\n"
  );
  // The binding is named rather than the method, because the size belongs to
  // what the name holds and the same call on a shorter value folds.
  assert!(bound_value_too_large("fonts", 32).contains("'fonts'"));
  assert!(bound_value_too_large("fonts", 32).contains("At most 32"));
}

#[test]
fn test_uncallable_printed_fold() {
  assert_eq!(
    uncallable_printed_fold("trim"),
    "Cannot fold 'trim' at compile time.\nThe printed expression did not compile to a function.\n\n"
  );
  assert!(uncallable_printed_fold("join").contains("'join'"));
}

#[test]
fn test_unfoldable_function() {
  assert_eq!(
    unfoldable_function("upper"),
    "Cannot carry the function 'upper' into a fold.\n\
     Its declaration is not one the compiler can evaluate.\n\n"
  );
  // Names the binding rather than the call it was an argument to, which is the
  // whole reason this sentence exists beside `unfoldable_call`.
  assert!(unfoldable_function("byLength").contains("'byLength'"));
  assert!(!unfoldable_function("byLength").contains("Cannot fold"));
}

#[test]
fn test_global_as_a_value() {
  assert_eq!(
    global_as_a_value("Boolean"),
    "Cannot carry the global 'Boolean' into a fold.\n\
     The compiler folds what a global answers, not the global itself.\n\n"
  );
  // Every global the fold names reads the same way, and none of them borrows
  // the sentence about a name nothing declared.
  for name in VALID_CALLEES.iter().chain(VALUE_ONLY_GLOBALS.iter()) {
    assert!(global_as_a_value(name).contains(&format!("'{}'", name)));
    assert!(!global_as_a_value(name).contains("constant"));
  }
}

#[test]
fn test_unfoldable_call() {
  assert_eq!(
    unfoldable_call("startsWith"),
    "Cannot fold 'startsWith' at compile time.\nIts receiver or one of its arguments is not in a form the compiler can evaluate.\n\n"
  );
  // Neither of the two reasons the caller cannot tell apart is named, so the
  // sentence stays true whichever one applied.
  assert!(!unfoldable_call("trim").contains("argument must"));
  assert!(unfoldable_call("trim").contains("'trim'"));
}

#[test]
fn test_unfoldable_static() {
  assert_eq!(
    unfoldable_static("Math", "random"),
    "Cannot fold 'Math.random' at compile time.\nA fold has to answer from the source alone, and this call does not.\n\n"
  );
  // The receiver is the half that disambiguates on a static: two methods this
  // set refuses read alike without it, and the same bare method name on a value
  // is a different refusal entirely.
  assert!(unfoldable_static("Object", "freeze").contains("'Object.freeze'"));
  assert_ne!(
    unfoldable_static("Object", "assign"),
    unfoldable_static("Math", "assign")
  );
  // The sentence names the receiver with the method whatever the method is.
  // A literal list rather than a set: the statics this sentence answers for are
  // every name the allowlists do not hold, which is the whole language and no
  // set to iterate. These five are the ones a module reaches.
  for method in [
    "assign",
    "defineProperty",
    "freeze",
    "seal",
    "preventExtensions",
  ] {
    assert!(unfoldable_static("Object", method).contains(&format!("'Object.{}'", method)));
  }
  // Opens with the same line as every other refusal a fold hands back, so one
  // class of failure reads as one class.
  assert!(unfoldable_static("Math", "random").starts_with("Cannot fold '"));
}

#[test]
fn test_engine_did_not_start() {
  assert_eq!(
    engine_did_not_start("out of memory"),
    "The compile-time JavaScript engine could not start.\nout of memory\n\n"
  );
  // The engine's own sentence is carried through untouched, however it is
  // punctuated, because nothing here is in a position to improve on it.
  assert!(engine_did_not_start("Realm::create failed").contains("Realm::create failed"));
  // An engine that fails without a word to say still produces a sentence that
  // says which stage failed.
  assert_eq!(
    engine_did_not_start(""),
    "The compile-time JavaScript engine could not start.\n\n\n"
  );
}

#[test]
fn test_unfoldable_statement() {
  assert_eq!(
    unfoldable_statement("for loop"),
    "Cannot fold a callback whose body uses a for loop.\n\
     Only a declaration, a branch, a block and a return are read inside a callback body.\n\n"
  );
  // Names the statement kind the author can look for in the body they wrote, so
  // two kinds may not read alike.
  assert_ne!(
    unfoldable_statement("while loop"),
    unfoldable_statement("for loop")
  );
  // Says what *is* read as well as what is not, which is what makes the next
  // step a rewrite rather than a guess.
  assert!(unfoldable_statement("throw").contains("Only a declaration, a branch"));
}

#[test]
fn test_bound_value_has_too_many_entries() {
  assert_eq!(
    bound_value_has_too_many_entries("palette", 10_000),
    "Cannot carry the value of 'palette' into a fold.\nAt most 10000 elements and properties are supported.\n\n"
  );
  // Opens on the same line as the sibling that bounds the *text* a binding
  // holds: an author reads one refusal about carrying a binding in, and the
  // second line says which of the two costs was too high.
  assert_eq!(
    bound_value_has_too_many_entries("palette", 8)
      .lines()
      .next(),
    bound_value_too_large("palette", 8).lines().next()
  );
  assert_ne!(
    bound_value_has_too_many_entries("palette", 8),
    bound_value_too_large("palette", 8)
  );
  // The limit is the caller's, so a raised ceiling is the number an author reads.
  assert!(bound_value_has_too_many_entries("sizes", 32).contains("At most 32"));
}
