//! How a member read off a `defineVars` group is named.
//!
//! Two readers derive a member's name from the same two facts — the group's
//! identity and the key — and they run in different places: the evaluator's own
//! lookup, and the compile-time engine, which holds the identity and not the
//! group. So the derivation is one function and its answers are asserted here
//! rather than through either caller.
//!
//! Every expected value below is what `@stylexjs/babel-plugin` 0.19.0 derives
//! for the same key.

use crate::theme_ref::var_group_member;

const BASE: &str = "vars.stylex.js//vars";
const PREFIX: &str = "x";

/// The ordinary read: a `var()` naming a variable hashed from the group and the
/// key together.
#[test]
fn a_named_member_is_a_variable_hashed_from_the_group_and_the_key() {
  assert_eq!(var_group_member(BASE, PREFIX, "primary"), "var(--x1ineb92)");
}

/// Two keys of one group are two variables, which is the whole of what the
/// derivation is for.
#[test]
fn two_keys_of_one_group_name_two_variables() {
  assert_ne!(
    var_group_member(BASE, PREFIX, "primary"),
    var_group_member(BASE, PREFIX, "secondary")
  );
}

/// The same key under two groups is two variables, so a name is a property of
/// the group as much as of the key.
#[test]
fn one_key_under_two_groups_names_two_variables() {
  assert_ne!(
    var_group_member(BASE, PREFIX, "primary"),
    var_group_member("other.stylex.js//other", PREFIX, "primary")
  );
}

/// A dotted key is one key: a group whose members are groups names one variable
/// per path rather than one per level.
#[test]
fn a_dotted_key_names_one_variable_for_the_whole_path() {
  assert_eq!(
    var_group_member(BASE, PREFIX, "brand.primary"),
    "var(--x1tr9ywo)"
  );
}

/// A key an author spelled as a variable of their own is used exactly as
/// written, hashed or prefixed by nothing.
#[test]
fn a_key_spelled_as_a_variable_is_used_as_written() {
  assert_eq!(var_group_member(BASE, PREFIX, "--custom"), "var(--custom)");
}

/// The group's own key answers a bare name rather than a `var()`, because it is
/// the group's hash and not a variable anything reads.
#[test]
fn the_group_s_own_key_answers_a_bare_hash() {
  assert_eq!(
    var_group_member(BASE, PREFIX, "__varGroupHash__"),
    "xop34xu"
  );
}

/// The prefix is the project's own and prefixes every name the derivation
/// answers.
#[test]
fn the_class_name_prefix_is_carried_into_every_name() {
  assert!(
    var_group_member(BASE, "zz", "primary").starts_with("var(--zz"),
    "expected the project's prefix in front of the hash"
  );
}

/// An empty key is a key: it names the group's own identity without being the
/// group's own hash, which is what says the two branches are separate.
#[test]
fn an_empty_key_is_still_a_key() {
  let named = var_group_member(BASE, PREFIX, "");

  assert!(named.starts_with("var(--x"), "got `{}`", named);
  assert_ne!(named, "var(--xop34xu)");
}

/// A key far longer than anything an author writes still answers one name, and a
/// different one per key: the derivation hashes rather than copies.
#[test]
fn an_enormous_key_still_answers_one_name() {
  let long = "k".repeat(100_000);
  let named = var_group_member(BASE, PREFIX, &long);

  assert!(
    named.len() < 32,
    "expected a hash, got {} bytes",
    named.len()
  );
  assert_ne!(named, var_group_member(BASE, PREFIX, &format!("{}k", long)));
}

/// A key outside ASCII is hashed as written, so it names a variable like any
/// other key.
#[test]
fn a_non_ascii_key_is_hashed_as_written() {
  let named = var_group_member(BASE, PREFIX, "ключ");

  assert!(named.starts_with("var(--x"), "got `{}`", named);
}

/// A reference to a `defineVars` group, as the evaluator holds one.
mod theme_reference {
  use stylex_constants::constants::common::VAR_GROUP_HASH_KEY;
  use stylex_enums::theme_ref::ThemeRefResult;

  use crate::theme_ref::{IS_PROXY_KEY, ThemeRef, var_group_member};

  fn theme_ref() -> ThemeRef {
    ThemeRef::new("vars.stylex.js", "vars", "x")
  }

  /// The identity is the file and the export joined the way every name derived
  /// from the group is derived.
  #[test]
  fn the_identity_is_the_file_and_the_export() {
    assert_eq!(theme_ref().base_id(), "vars.stylex.js//vars");
  }

  #[test]
  fn the_prefix_is_kept_as_handed_in() {
    assert_eq!(theme_ref().class_name_prefix(), "x");
    assert_eq!(
      ThemeRef::new("vars.stylex.js", "vars", "").class_name_prefix(),
      ""
    );
  }

  /// `toString` is the one key that answers a bare name rather than a `var()`,
  /// and it is the same name `to_string_value` answers.
  #[test]
  fn the_to_string_key_answers_the_group_s_own_name() {
    let mut reference = theme_ref();

    let ThemeRefResult::ToString(name) = reference.get("toString") else {
      panic!("`toString` did not answer the group's own name");
    };

    // The literal is the group hash of `BASE` under `PREFIX`, pinned at the top
    // of this file against Babel. Reading `to_string_value` for the expectation
    // would let both halves be wrong together.
    assert_eq!(name, "xop34xu");
    assert_eq!(name, reference.to_string_value());
  }

  /// The marker key answers that the value stands in for a group rather than
  /// holding one, which is how every reader tells a reference from an object.
  #[test]
  fn the_proxy_key_answers_that_this_stands_in_for_a_group() {
    let mut reference = theme_ref();

    assert!(matches!(reference.get(IS_PROXY_KEY), ThemeRefResult::Proxy));
  }

  /// Every other key answers the variable the derivation names, so a read
  /// through the reference and a read through the derivation agree.
  #[test]
  fn a_member_read_answers_the_variable_the_derivation_names() {
    let mut reference = theme_ref();

    // The literal is what `var_group_member` is pinned to against Babel at the
    // top of this file. Re-deriving it here would only prove the reference
    // calls the derivation, not that either one is right.
    assert_eq!(
      reference.get("primary").as_css_var(),
      Some("var(--x1ineb92)")
    );
    assert_eq!(
      reference.get("primary").as_css_var(),
      Some(var_group_member(super::BASE, super::PREFIX, "primary").as_str())
    );
  }

  /// The second read of one key answers the first read's own value: the name is
  /// derived once and kept.
  #[test]
  fn a_second_read_of_one_key_answers_the_first_read_s_value() {
    let mut reference = theme_ref();

    let Some(first) = reference.get("primary").as_css_var().map(str::to_string) else {
      panic!("the first read named no variable");
    };
    let Some(second) = reference.get("primary").as_css_var().map(str::to_string) else {
      panic!("the second read named no variable");
    };

    assert_eq!(first, second);
  }

  /// The group's hash key answers a bare name rather than a `var()`, because it
  /// names the group and not a variable of it.
  #[test]
  fn the_group_hash_key_answers_a_bare_name() {
    let mut reference = theme_ref();

    let answer = reference.get(VAR_GROUP_HASH_KEY);
    let Some(name) = answer.as_css_var() else {
      panic!("the group hash key named nothing");
    };

    assert!(!name.starts_with("var("), "got `{}`", name);
    assert_eq!(name, "xop34xu");
    assert_eq!(name, reference.to_string_value());
  }

  /// A variable the author named is used as written, and reading it twice
  /// answers the same text -- it is derived from nothing, so there is nothing to
  /// keep between the reads.
  #[test]
  fn an_author_named_variable_is_used_as_written() {
    let mut reference = theme_ref();

    assert_eq!(
      reference.get("--brand-color").as_css_var(),
      Some("var(--brand-color)")
    );
    assert_eq!(
      reference.get("--brand-color").as_css_var(),
      Some("var(--brand-color)")
    );
  }

  /// A clone shares the derived names with the value it was cloned from, which
  /// is what makes the factory that hands out references cheap.
  #[test]
  fn a_clone_shares_the_names_already_derived() {
    let mut reference = theme_ref();

    let Some(original) = reference.get("primary").as_css_var().map(str::to_string) else {
      panic!("the read named no variable");
    };

    let mut clone = reference.clone();

    assert_eq!(clone.get("primary").as_css_var(), Some(original.as_str()));
  }
}
