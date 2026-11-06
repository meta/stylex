#[cfg(test)]
mod convert_style_to_class_name {
  use crate::shared::utils::core::convert_style_to_class_name::convert_style_to_class_name;
  use stylex_enums::style_resolution::StyleResolution;
  use stylex_state::state_manager::StateManager;
  use stylex_structures::pre_rule_value::PreRuleValue;
  use stylex_structures::raw_value::TRawValue;
  use stylex_structures::stylex_state_options::StyleXStateOptions;
  use stylex_types::structures::style_key::ClassName;
  /// The declaration text, for a pair that compiles to one.
  fn convert(styles: (&str, &PreRuleValue)) -> String {
    match try_convert(styles) {
      Some(declaration) => declaration,
      None => panic!("expected `{}` to compile to a declaration", styles.0),
    }
  }

  /// The declaration text, or `None` when the pair carries no CSS text and is
  /// left undeclared.
  fn try_convert(styles: (&str, &PreRuleValue)) -> Option<String> {
    convert_style_to_class_name(
      styles,
      &mut [],
      &mut [],
      &mut [],
      &mut StateManager::default(),
    )
    .map(|(_, _, rule)| extract_body(rule.ltr))
  }

  /// The class name a pair compiles to, under the given options.
  fn class_name_of(styles: (&str, &PreRuleValue), state: &mut StateManager) -> ClassName {
    match convert_style_to_class_name(styles, &mut [], &mut [], &mut [], state) {
      Some((_, class_name, _)) => class_name,
      None => panic!("expected `{}` to compile to a declaration", styles.0),
    }
  }

  fn extract_body(s: String) -> String {
    let start = s.find('{').unwrap_or(0) + 1;
    let end = s.len() - 1;
    s[start..end].to_string()
  }

  #[test]
  fn converts_style_to_class_name() {
    let result = convert(("margin", &PreRuleValue::number(10.0)));

    assert_eq!(result, "margin:10px")
  }

  #[test]
  fn generates_the_same_classname_in_debug_and_production_modes() {
    let base_options = || {
      StyleXStateOptions::default()
        .with_class_name_prefix("x")
        .with_style_resolution(StyleResolution::PropertySpecificity)
        .with_dev(false)
        .with_test(false)
    };

    let debug_class_name = class_name_of(
      ("margin", &PreRuleValue::number(10.0)),
      &mut StateManager::for_test(None, base_options().with_debug(true)),
    );
    let production_class_name = class_name_of(
      ("margin", &PreRuleValue::number(10.0)),
      &mut StateManager::for_test(None, base_options().with_debug(false)),
    );

    assert_eq!(debug_class_name, production_class_name);
    assert!(debug_class_name.as_str().starts_with("x"));
    assert!(!debug_class_name.as_str().starts_with("margin-x"));
  }

  #[test]
  fn converts_margin_number_to_px() {
    let result = convert(("margin", &PreRuleValue::number(10.0)));

    assert_eq!(result, "margin:10px")
  }

  #[test]
  fn keeps_number_for_z_index() {
    let result = convert(("zIndex", &PreRuleValue::number(10.0)));

    assert_eq!(result, "z-index:10")
  }

  #[test]
  fn keeps_fr_for_zero_fraction_values() {
    let result = convert(("gridTemplateRows", &PreRuleValue::string("0fr")));

    assert_eq!(result, "grid-template-rows:0fr")
  }

  #[test]
  fn keeps_percent_for_zero_percentage_values() {
    let result = convert(("flexBasis", &PreRuleValue::string("0%")));

    assert_eq!(result, "flex-basis:0%")
  }

  #[test]
  fn keeps_number_for_opacity() {
    let result = convert(("opacity", &PreRuleValue::string("0.25")));

    assert_eq!(result, "opacity:.25")
  }

  /// `color:` is not a declaration a browser accepts, so a value that carries
  /// no CSS text leaves the property undeclared instead.
  #[test]
  fn declares_nothing_for_a_value_with_no_css_text() {
    assert_eq!(try_convert(("color", &PreRuleValue::string(""))), None);
    assert_eq!(try_convert(("color", &PreRuleValue::string(" "))), None);
    assert_eq!(try_convert(("color", &PreRuleValue::string("  \t "))), None);
  }

  /// A character that *looks* blank but is not one the trim reads is a value
  /// like any other, and it reaches the stylesheet as itself.
  ///
  /// Two of them, and each is a different reason the trim could have been
  /// wrong: the ideographic space and the no-break space are both spaces the
  /// language does not call whitespace. Measured against
  /// `@stylexjs/babel-plugin@0.19.0`, which declares them the same way.
  #[test]
  fn declares_a_value_that_only_looks_blank() {
    for text in ["\u{3000}", "\u{a0}"] {
      assert_eq!(
        try_convert(("color", &PreRuleValue::string(text))),
        Some(format!("color:{text}")),
        "value {text:?}"
      );
    }
  }

  /// A control character carries no CSS text, so it leaves the property
  /// undeclared like any other value that does not.
  ///
  /// The reference implementation neither declares it nor refuses it: it fails
  /// inside its own value parser, reading a property of `undefined`. There is
  /// no answer of its to agree with, and leaving the property out is what this
  /// compiler already does for every value with no text.
  #[test]
  fn declares_nothing_for_a_control_character() {
    assert_eq!(try_convert(("color", &PreRuleValue::string("\u{1}"))), None);
  }

  /// The test is on the transformed value, not the authored one: quoting is what
  /// gives a blank `content` its text.
  ///
  /// That ordering is load-bearing. `content` acquires its quotes before the
  /// blank check that leaves other properties undeclared, so a blank `content`
  /// never reaches it — whatever the blank is made of.
  #[test]
  fn declares_a_blank_content_value_as_empty_quotes() {
    for blank in [" ", "   ", "\t", ""] {
      assert_eq!(
        try_convert(("content", &PreRuleValue::string(blank))),
        Some("content:\"\"".to_string()),
        "blank content value {blank:?}"
      );
    }

    assert_eq!(
      try_convert(("hyphenateCharacter", &PreRuleValue::string("   "))),
      Some("hyphenate-character:\"\"".to_string())
    );
  }

  /// `hyphenateCharacter` takes a *character*, not a property name, so the
  /// value goes through untouched -- the camel case is the author's text rather
  /// than a spelling to hyphenate. Its own key is hyphenated all the same.
  ///
  /// Both spellings of the key, because the property reaches the CSS layer as
  /// either one and the bypass is keyed by the name.
  #[test]
  fn keeps_a_hyphenate_character_value_as_it_was_written() {
    for key in ["hyphenateCharacter", "hyphenate-character"] {
      assert_eq!(
        try_convert((key, &PreRuleValue::string("fooBar"))),
        Some("hyphenate-character:\"fooBar\"".to_string()),
        "key {key:?}"
      );
    }
  }

  /// A blank entry drops out of a fallback array rather than emitting an empty
  /// declaration beside the ones that spell a value.
  #[test]
  fn drops_a_blank_entry_from_a_fallback_array() {
    assert_eq!(
      try_convert((
        "color",
        &PreRuleValue::Vec(vec![" ".into(), "red".into(), "".into()])
      )),
      Some("color:red".to_string())
    );
  }

  /// Blank entries drop before the `var()` chain is composed, so they cannot
  /// break the contiguity the chain requires.
  #[test]
  fn drops_a_blank_entry_between_variable_fallbacks() {
    assert_eq!(
      try_convert((
        "height",
        &PreRuleValue::Vec(vec!["var(--x)".into(), " ".into(), "var(--y)".into()])
      )),
      Some("height:var(--y,var(--x))".to_string())
    );
  }

  /// An array with nothing left to declare answers the same as a lone blank.
  #[test]
  fn declares_nothing_for_a_fallback_array_of_blanks() {
    assert_eq!(
      try_convert(("color", &PreRuleValue::Vec(vec![" ".into(), "".into()]))),
      None
    );
  }

  /// `0` carries text even though JS calls it falsy.
  #[test]
  fn declares_a_zero_value() {
    assert_eq!(
      try_convert(("zIndex", &PreRuleValue::number(0.0))),
      Some("z-index:0".to_string())
    );
  }

  #[test]
  fn handles_array_of_values() {
    let result = convert((
      "height",
      &PreRuleValue::Vec(vec![
        TRawValue::Number(500.0),
        "100vh".into(),
        "100dvh".into(),
      ]),
    ));

    assert_eq!(result, "height:500px;height:100vh;height:100dvh")
  }

  #[test]
  fn handles_array_of_values_with_var() {
    let result = convert((
      "height",
      &PreRuleValue::Vec(vec![
        TRawValue::Number(500.0),
        "var(--height)".into(),
        "100dvh".into(),
      ]),
    ));

    assert_eq!(result, "height:var(--height,500px);height:100dvh")
  }

  #[test]
  fn handles_array_with_multiple_vars() {
    let result = convert((
      "height",
      &PreRuleValue::Vec(vec![
        TRawValue::Number(500.0),
        "var(--x)".into(),
        "var(--y)".into(),
        "100dvh".into(),
      ]),
    ));

    assert_eq!(result, "height:var(--y,var(--x,500px));height:100dvh")
  }

  #[test]
  fn handles_array_with_multiple_vars_and_multiple_fallbacks() {
    let result = convert((
      "height",
      &PreRuleValue::Vec(vec![
        TRawValue::Number(500.0),
        "100vh".into(),
        "var(--x)".into(),
        "var(--y)".into(),
        "100dvh".into(),
      ]),
    ));

    assert_eq!(
      result,
      "height:var(--y,var(--x,500px));height:var(--y,var(--x,100vh));height:100dvh"
    )
  }

  #[test]
  fn handles_array_with_variable_default_and_multiple_constant_fallbacks() {
    let result = convert((
      "height",
      &PreRuleValue::Vec(vec![
        "var(--x)".into(),
        TRawValue::Number(500.0),
        "100dvh".into(),
      ]),
    ));

    assert_eq!(result, "height:var(--x);height:500px;height:100dvh")
  }

  #[test]
  fn handles_array_with_variable_default_and_multiple_variable_and_constant_fallbacks() {
    let result = convert((
      "height",
      &PreRuleValue::Vec(vec![
        "var(--x)".into(),
        "var(--y)".into(),
        "var(--z)".into(),
        "100dvh".into(),
      ]),
    ));

    assert_eq!(result, "height:var(--z,var(--y,var(--x)));height:100dvh")
  }

  #[test]
  fn handles_array_of_all_variables() {
    let result = convert((
      "height",
      &PreRuleValue::Vec(vec![
        "var(--w)".into(),
        "var(--x)".into(),
        "var(--y)".into(),
        "var(--z)".into(),
      ]),
    ));

    assert_eq!(result, "height:var(--z,var(--y,var(--x,var(--w))))")
  }

  /// A fallback chain composes the `var()` entries into one value, so they have
  /// to stand together. A plain value between two of them cannot be composed
  /// and is refused rather than written into the middle of the chain.
  #[test]
  #[should_panic(expected = "All variables passed to firstThatWorks() must be contiguous.")]
  fn refuses_a_value_between_two_variables() {
    convert((
      "height",
      &PreRuleValue::Vec(vec!["var(--x)".into(), "100px".into(), "var(--y)".into()]),
    ));
  }

  /// A value the compiler could not fold is not a style value, and neither is
  /// an absent one. A class name cannot be hashed from either.
  #[test]
  #[should_panic(expected = "A style value can only contain an array, string or number.")]
  fn refuses_a_value_that_did_not_fold() {
    convert((
      "color",
      &PreRuleValue::Expr(crate::tests::support::expr("a + b")),
    ));
  }

  #[test]
  #[should_panic(expected = "A style value can only contain an array, string or number.")]
  fn refuses_an_absent_value() {
    convert(("color", &PreRuleValue::Null));
  }

  /// A plain value after the last variable is written beside the chain rather
  /// than into it.
  #[test]
  fn keeps_a_plain_value_after_the_last_variable() {
    let result = convert((
      "height",
      &PreRuleValue::Vec(vec!["var(--x)".into(), "100px".into()]),
    ));

    assert_eq!(result, "height:var(--x);height:100px")
  }
}
