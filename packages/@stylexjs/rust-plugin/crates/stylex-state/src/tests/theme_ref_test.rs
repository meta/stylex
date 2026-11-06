use crate::theme_ref::var_group_member;

const BASE: &str = "vars.stylex.js//vars";
const PREFIX: &str = "x";

#[test]
fn a_named_member_is_a_variable_hashed_from_the_group_and_the_key() {
  assert_eq!(var_group_member(BASE, PREFIX, "primary"), "var(--x1ineb92)");
}

#[test]
fn two_keys_of_one_group_name_two_variables() {
  assert_ne!(
    var_group_member(BASE, PREFIX, "primary"),
    var_group_member(BASE, PREFIX, "secondary")
  );
}

#[test]
fn one_key_under_two_groups_names_two_variables() {
  assert_ne!(
    var_group_member(BASE, PREFIX, "primary"),
    var_group_member("other.stylex.js//other", PREFIX, "primary")
  );
}

#[test]
fn a_dotted_key_names_one_variable_for_the_whole_path() {
  assert_eq!(
    var_group_member(BASE, PREFIX, "brand.primary"),
    "var(--x1tr9ywo)"
  );
}

#[test]
fn a_key_spelled_as_a_variable_is_used_as_written() {
  assert_eq!(var_group_member(BASE, PREFIX, "--custom"), "var(--custom)");
}

#[test]
fn the_group_s_own_key_answers_a_bare_hash() {
  assert_eq!(
    var_group_member(BASE, PREFIX, "__varGroupHash__"),
    "xop34xu"
  );
}

#[test]
fn the_class_name_prefix_is_carried_into_every_name() {
  assert!(
    var_group_member(BASE, "zz", "primary").starts_with("var(--zz"),
    "expected the project's prefix in front of the hash"
  );
}

#[test]
fn an_empty_key_is_still_a_key() {
  let named = var_group_member(BASE, PREFIX, "");

  assert!(named.starts_with("var(--x"), "got `{}`", named);
  assert_ne!(named, "var(--xop34xu)");
}

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

#[test]
fn a_non_ascii_key_is_hashed_as_written() {
  let named = var_group_member(BASE, PREFIX, "ключ");

  assert!(named.starts_with("var(--x"), "got `{}`", named);
}

mod theme_reference {
  use stylex_constants::constants::common::VAR_GROUP_HASH_KEY;
  use stylex_enums::theme_ref::ThemeRefResult;

  use crate::theme_ref::{IS_PROXY_KEY, ThemeRef, var_group_member};

  fn theme_ref() -> ThemeRef {
    ThemeRef::new("vars.stylex.js", "vars", "x")
  }

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

  #[test]
  fn the_to_string_key_answers_the_group_s_own_name() {
    let mut reference = theme_ref();

    let ThemeRefResult::ToString(name) = reference.get("toString") else {
      panic!("`toString` did not answer the group's own name");
    };

    // The literal is the group hash of `BASE` under `PREFIX`, pinned at the top of this file against Babel.
    assert_eq!(name, "xop34xu");
    assert_eq!(name, reference.to_string_value());
  }

  #[test]
  fn the_proxy_key_answers_that_this_stands_in_for_a_group() {
    let mut reference = theme_ref();

    assert!(matches!(reference.get(IS_PROXY_KEY), ThemeRefResult::Proxy));
  }

  #[test]
  fn a_member_read_answers_the_variable_the_derivation_names() {
    let mut reference = theme_ref();

    // The literal is what `var_group_member` is pinned to against Babel at the top of this file.
    assert_eq!(
      reference.get("primary").as_css_var(),
      Some("var(--x1ineb92)")
    );
    assert_eq!(
      reference.get("primary").as_css_var(),
      Some(var_group_member(super::BASE, super::PREFIX, "primary").as_str())
    );
  }

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
