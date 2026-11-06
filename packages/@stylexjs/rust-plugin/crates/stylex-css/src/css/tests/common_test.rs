#[cfg(test)]
mod get_number_suffix_tests {
  use crate::css::common::get_number_suffix;

  #[test]
  fn returns_px_for_padding() {
    assert_eq!(get_number_suffix("padding"), "px");
  }

  #[test]
  fn returns_empty_for_opacity() {
    assert_eq!(get_number_suffix("opacity"), "");
  }

  #[test]
  fn returns_ms_for_voice_duration() {
    assert_eq!(get_number_suffix("voiceDuration"), "ms");
  }

  #[test]
  fn returns_empty_for_custom_properties() {
    assert_eq!(get_number_suffix("--my-var"), "");
    assert_eq!(get_number_suffix("--x"), "");
  }

  #[test]
  fn returns_empty_for_unitless_properties() {
    assert_eq!(get_number_suffix("lineHeight"), "");
    assert_eq!(get_number_suffix("zIndex"), "");
    assert_eq!(get_number_suffix("fontWeight"), "");
    assert_eq!(get_number_suffix("flex"), "");
    assert_eq!(get_number_suffix("order"), "");
  }

  #[test]
  fn returns_px_for_standard_properties() {
    assert_eq!(get_number_suffix("margin"), "px");
    assert_eq!(get_number_suffix("width"), "px");
    assert_eq!(get_number_suffix("height"), "px");
    assert_eq!(get_number_suffix("top"), "px");
    assert_eq!(get_number_suffix("borderWidth"), "px");
  }
}

#[cfg(test)]
mod inline_style_to_css_string_tests {
  use std::borrow::Cow;

  use crate::css::common::inline_style_to_css_string;
  use stylex_structures::pair::PairCow;

  /// The declarations a case is given, each half borrowed the way a caller
  /// holding a name and a value hands them over.
  fn declarations<'a>(pairs: &[(&'a str, &'a str)]) -> Vec<PairCow<'a>> {
    pairs
      .iter()
      .map(|(key, value)| PairCow {
        key: Cow::Borrowed(*key),
        value: Cow::Borrowed(*value),
      })
      .collect()
  }

  #[test]
  fn formats_single_pair() {
    assert_eq!(
      inline_style_to_css_string(&declarations(&[("color", "red")])),
      "color:red"
    );
  }

  #[test]
  fn formats_multiple_pairs() {
    assert_eq!(
      inline_style_to_css_string(&declarations(&[("color", "red"), ("marginTop", "10px")])),
      "color:red;margin-top:10px"
    );
  }

  #[test]
  fn handles_empty_pairs() {
    assert_eq!(inline_style_to_css_string(&[]), "");
  }

  #[test]
  fn handles_custom_properties() {
    assert_eq!(
      inline_style_to_css_string(&declarations(&[("--my-var", "blue")])),
      "--my-var:blue"
    );
  }

  /// A custom property takes the same spelling rule as any other name here,
  /// which is where an attribute and a stylesheet part company: a stylesheet
  /// hands `--myColor` back untouched.
  ///
  /// Both answers were measured against @stylexjs/babel-plugin@0.19.0 through
  /// `stylex.attrs({ … })`, which is the one call that reaches this text.
  #[test]
  fn kebab_cases_a_custom_property_written_in_camel_case() {
    assert_eq!(
      inline_style_to_css_string(&declarations(&[("--myColor", "red")])),
      "--my-color:red"
    );

    // Every capital takes its own hyphen, so a run of them is not the single
    // hyphen a stylesheet would write.
    assert_eq!(
      inline_style_to_css_string(&declarations(&[("--ABCDef", "x")])),
      "---a-b-c-def:x"
    );
  }

  /// A value spelled where the call was made is kept as it stands, so a half
  /// the caller had to build reaches the text without being copied again.
  #[test]
  fn formats_an_owned_value() {
    let pairs = vec![PairCow {
      key: Cow::Borrowed("opacity"),
      value: Cow::Owned("0.5".to_owned()),
    }];

    assert_eq!(inline_style_to_css_string(&pairs), "opacity:0.5");
  }
}

#[cfg(test)]
mod build_nested_css_rule_tests {
  use crate::css::common::build_nested_css_rule;

  #[test]
  fn builds_simple_rule() {
    let result = build_nested_css_rule("x1234", "color:red".into(), &mut [], &mut [], &mut []);
    assert_eq!(result, ".x1234{color:red}");
  }

  #[test]
  fn builds_rule_with_pseudo() {
    let result = build_nested_css_rule(
      "x1234",
      "color:red".into(),
      &mut [":hover".to_string()],
      &mut [],
      &mut [],
    );
    assert_eq!(result, ".x1234:hover{color:red}");
  }

  #[test]
  fn builds_rule_with_at_rule() {
    let result = build_nested_css_rule(
      "x1234",
      "color:red".into(),
      &mut [],
      &mut ["@media (max-width: 600px)".to_string()],
      &mut [],
    );
    assert_eq!(result, "@media (max-width: 600px){.x1234.x1234{color:red}}");
  }

  #[test]
  fn builds_rule_with_thumb_pseudo() {
    let result = build_nested_css_rule(
      "x1234",
      "color:red".into(),
      &mut ["::thumb".to_string()],
      &mut [],
      &mut [],
    );
    assert!(result.contains("::-webkit-slider-thumb"));
    assert!(result.contains("::-moz-range-thumb"));
    assert!(result.contains("::-ms-thumb"));
  }

  #[test]
  fn builds_rule_with_where_pseudo() {
    let result = build_nested_css_rule(
      "x1234",
      "color:red".into(),
      &mut [":where(.dark)".to_string()],
      &mut [],
      &mut [],
    );
    // Should have extra class for specificity bump
    assert!(result.contains(".x1234.x1234:where(.dark)"));
  }

  #[test]
  fn builds_rule_with_const_rules() {
    let result = build_nested_css_rule(
      "x1234",
      "color:red".into(),
      &mut [],
      &mut [],
      &mut ["--condition".to_string()],
    );
    assert_eq!(result, "--condition{.x1234.x1234{color:red}}");
  }
}

#[cfg(test)]
mod get_priority_tests {
  use crate::css::common::get_priority;
  use stylex_regex::regex::PSEUDO_PART_REGEX;

  #[test]
  fn shorthand_of_shorthands_gets_1000() {
    assert_eq!(get_priority("all"), 1000.0);
  }

  #[test]
  fn longhand_logical_gets_3000() {
    assert_eq!(get_priority("marginStart"), 3000.0);
  }

  #[test]
  fn longhand_physical_gets_4000() {
    assert_eq!(get_priority("margin-top"), 4000.0);
  }

  #[test]
  fn unknown_property_gets_3000() {
    assert_eq!(get_priority("unknownProp"), 3000.0);
  }

  #[test]
  fn at_media_rule() {
    let p = get_priority("@media (max-width: 600px)");
    assert!(p > 0.0);
  }

  #[test]
  fn at_supports_rule() {
    let p = get_priority("@supports (display: grid)");
    assert!(p > 0.0);
  }

  #[test]
  fn custom_property_at_rule() {
    assert_eq!(get_priority("--some-var"), 1.0);
  }

  #[test]
  fn pseudo_element_priority() {
    let p = get_priority("::before");
    assert!(p > 0.0);
  }

  #[test]
  fn pseudo_class_hover() {
    let p = get_priority(":hover");
    assert!(p > 0.0);
  }

  #[test]
  fn pseudo_class_focus() {
    let p = get_priority(":focus");
    assert!(p > 0.0);
  }

  /// The `contains(':')` guard in `get_compound_pseudo_priority` skips the scan
  /// for a key that holds no colon, which is only sound while every alternation
  /// of `PSEUDO_PART_REGEX` opens with one. An alternation added later that does
  /// not -- an attribute selector, say -- would make the guard answer `None` for
  /// keys it matches, and the generated CSS would reorder with nothing else
  /// failing.
  #[test]
  fn every_pseudo_part_the_regex_matches_carries_a_colon() {
    for key in [
      ":hover:active",
      "::before",
      ":not(:hover)",
      "::-webkit-scrollbar",
      ":nth-child(2)::after",
    ] {
      for part in PSEUDO_PART_REGEX.find_iter(key).flatten() {
        assert!(
          part.as_str().starts_with(':'),
          "{part:?} has no colon to guard on"
        );
      }
    }
  }

  /// The other half of the premise: a key the guard refuses holds nothing the
  /// scan would have found.
  #[test]
  fn a_key_without_a_colon_matches_no_pseudo_part() {
    for key in ["color", "paddingBottom", "--custom-prop", "margin-top"] {
      assert_eq!(PSEUDO_PART_REGEX.find_iter(key).flatten().count(), 0);
    }
  }
}

#[cfg(test)]
mod normalize_css_property_value_tests {
  use crate::css::common::normalize_css_property_value;
  use crate::css::tests::support::{default_options, panic_message, rem_enabled_options};
  use std::panic::{AssertUnwindSafe, catch_unwind};

  // --- Simple values ---

  #[test]
  fn normalizes_simple_color_keyword() {
    let opts = default_options();
    let result = normalize_css_property_value("color", "red", &opts);
    assert_eq!(result, "red");
  }

  /// A six-digit hex colour keeps all six digits. No normalizer understands
  /// hex, so the author's spelling reaches the hash as written.
  #[test]
  fn normalizes_hex_color() {
    let opts = default_options();
    let result = normalize_css_property_value("color", "#ff0000", &opts);
    assert_eq!(result, "#ff0000");
  }

  #[test]
  fn normalizes_transparent() {
    let opts = default_options();
    let result = normalize_css_property_value("color", "transparent", &opts);
    assert_eq!(result, "transparent");
  }

  // --- Numeric values ---

  #[test]
  fn normalizes_pixel_value() {
    let opts = default_options();
    let result = normalize_css_property_value("width", "100px", &opts);
    assert_eq!(result, "100px");
  }

  #[test]
  fn normalizes_percentage_value() {
    let opts = default_options();
    let result = normalize_css_property_value("width", "50%", &opts);
    assert_eq!(result, "50%");
  }

  #[test]
  fn normalizes_em_value() {
    let opts = default_options();
    let result = normalize_css_property_value("margin", "2em", &opts);
    assert_eq!(result, "2em");
  }

  #[test]
  fn normalizes_zero_value() {
    let opts = default_options();
    let result = normalize_css_property_value("margin", "0", &opts);
    assert_eq!(result, "0");
  }

  #[test]
  fn keeps_leading_zero_on_negative_decimals() {
    let opts = default_options();

    assert_eq!(
      normalize_css_property_value("letterSpacing", "-0.24px", &opts),
      "-0.24px"
    );

    assert_eq!(
      normalize_css_property_value("marginTop", "-0.5px", &opts),
      "-0.5px"
    );
  }

  // --- Calc expressions ---

  #[test]
  fn normalizes_calc_expression() {
    let opts = default_options();
    let result = normalize_css_property_value("width", "calc(100% - 20px)", &opts);
    assert_eq!(result, "calc(100% - 20px)");
  }

  #[test]
  fn normalizes_nested_calc() {
    let opts = default_options();
    let result = normalize_css_property_value("width", "calc(100% - calc(20px + 10px))", &opts);
    assert_eq!(result, "calc(100% - calc(20px + 10px))");
  }

  #[test]
  fn normalizes_calc_size_expressions() {
    let opts = default_options();
    let cases = [
      (
        "calc-size( auto , size   *   0 )",
        "calc-size(auto,size * 0)",
      ),
      (
        "calc-size(fit-content, size / 2)",
        "calc-size(fit-content,size / 2)",
      ),
      ("calc-size(any, 300px * 1.5)", "calc-size(any,300px * 1.5)"),
      (
        "calc-size(300px + 2rem, size / 2)",
        "calc-size(300px + 2rem,size / 2)",
      ),
      (
        "calc-size(calc-size(max-content, size), size + 2rem)",
        "calc-size(calc-size(max-content,size),size + 2rem)",
      ),
      (
        "calc-size(var(--intrinsic-size), max(100px, size + 20px))",
        "calc-size(var(--intrinsic-size),max(100px,size + 20px))",
      ),
      (
        "CALC-SIZE(auto, round(up, size, 50px))",
        "CALC-SIZE(auto,round(up,size,50px))",
      ),
    ];

    for (value, expected) in cases {
      assert_eq!(
        normalize_css_property_value("height", value, &opts),
        expected
      );
    }
  }

  #[test]
  fn normalizes_property_agnostic_values() {
    let opts = default_options();
    let cases = [
      ("future-fn(foo * 2)", "future-fn(foo * 2)"),
      (r#"future-fn("a,   b" * 2)"#, r#"future-fn("a,   b" * 2)"#),
      (
        "future-fn(foo /* a,   b */ * 2)",
        "future-fn(foo /* a,   b */ * 2)",
      ),
      ("future-fn(foo/2 * @)", "future-fn(foo / 2 * @)"),
      // An escaped quote does not end the string, so the `,` and the run of
      // spaces after it stay untouched even though commas are normalized
      // outside of strings.
      (
        r#"future-fn("a\",   b" * 2)"#,
        r#"future-fn("a\",   b" * 2)"#,
      ),
      // A trailing backslash-escaped backslash still closes the string.
      (r#"future-fn("a\\" * 2)"#, r#"future-fn("a\\" * 2)"#),
      ("foo * bar", "foo * bar"),
      ("[foo]", "[foo]"),
      ("@", "@"),
    ];

    for (value, expected) in cases {
      assert_eq!(
        normalize_css_property_value("height", value, &opts),
        expected
      );
    }
  }

  // --- Color functions ---

  /// A colour function takes the one path every value takes. Its arguments are
  /// normalized like any others: the leading zero goes, and the space after a
  /// comma goes with it.
  #[test]
  fn normalizes_a_space_separated_color_function() {
    let opts = default_options();
    let result = normalize_css_property_value("color", "oklch(0.7 0.15 180)", &opts);
    assert_eq!(result, "oklch(.7 .15 180)");
  }

  #[test]
  fn normalizes_a_comma_separated_color_function() {
    let opts = default_options();
    let result = normalize_css_property_value("color", "hsl(120, 100%, 50%)", &opts);
    assert_eq!(result, "hsl(120,100%,50%)");
  }

  #[test]
  fn normalizes_an_alpha_argument() {
    let opts = default_options();
    let result = normalize_css_property_value("color", "hsla(120, 100%, 50%, 0.5)", &opts);
    assert_eq!(result, "hsla(120,100%,50%,.5)");
  }

  #[test]
  fn color_function_collapses_extra_whitespace() {
    let opts = default_options();
    let result = normalize_css_property_value("color", "oklch(0.7   0.15   180)", &opts);
    assert_eq!(result, "oklch(.7 .15 180)");
  }

  // --- CSS variables ---

  #[test]
  fn normalizes_css_variable_value() {
    let opts = default_options();
    let result = normalize_css_property_value("--myVar", "blue", &opts);
    assert_eq!(result, "blue");
  }

  #[test]
  fn normalizes_css_variable_hex() {
    let opts = default_options();
    let result = normalize_css_property_value("--customColor", "#abcdef", &opts);
    assert_eq!(result, "#abcdef");
  }

  // --- Multiple values ---

  #[test]
  fn normalizes_margin_four_values() {
    let opts = default_options();
    let result = normalize_css_property_value("margin", "10px 20px 30px 40px", &opts);
    assert_eq!(result, "10px 20px 30px 40px");
  }

  #[test]
  fn normalizes_padding_two_values() {
    let opts = default_options();
    let result = normalize_css_property_value("padding", "5px 10px", &opts);
    assert_eq!(result, "5px 10px");
  }

  // --- Shorthand properties ---

  #[test]
  fn normalizes_border_shorthand() {
    let opts = default_options();
    let result = normalize_css_property_value("border", "1px solid red", &opts);
    assert_eq!(result, "1px solid red");
  }

  // --- Font-size px to rem conversion ---

  #[test]
  fn font_size_px_to_rem_when_enabled() {
    let opts = rem_enabled_options();
    let result = normalize_css_property_value("fontSize", "16px", &opts);
    assert_eq!(result, "1rem");
  }

  #[test]
  fn font_size_px_to_rem_32px() {
    let opts = rem_enabled_options();
    let result = normalize_css_property_value("fontSize", "32px", &opts);
    assert_eq!(result, "2rem");
  }

  #[test]
  fn font_size_px_no_conversion_when_disabled() {
    let opts = default_options();
    let result = normalize_css_property_value("fontSize", "16px", &opts);
    assert_eq!(result, "16px");
  }

  // --- Keywords and special values ---

  #[test]
  fn normalizes_inherit_keyword() {
    let opts = default_options();
    let result = normalize_css_property_value("color", "inherit", &opts);
    assert_eq!(result, "inherit");
  }

  #[test]
  fn normalizes_initial_keyword() {
    let opts = default_options();
    let result = normalize_css_property_value("display", "initial", &opts);
    assert_eq!(result, "initial");
  }

  #[test]
  fn normalizes_none_keyword() {
    let opts = default_options();
    let result = normalize_css_property_value("display", "none", &opts);
    assert_eq!(result, "none");
  }

  #[test]
  fn normalizes_auto_value() {
    let opts = default_options();
    let result = normalize_css_property_value("margin", "auto", &opts);
    assert_eq!(result, "auto");
  }

  // --- Var() function ---

  #[test]
  fn normalizes_var_function() {
    let opts = default_options();
    let result = normalize_css_property_value("color", "var(--xColor)", &opts);
    assert_eq!(result, "var(--xColor)");
  }

  #[test]
  fn normalizes_var_with_fallback() {
    let opts = default_options();
    let result = normalize_css_property_value("color", "var(--xColor, red)", &opts);
    assert_eq!(result, "var(--xColor,red)");
  }

  // --- Transform functions (camelCase conversion) ---

  #[test]
  fn normalizes_translatex_to_camel_case() {
    let opts = default_options();
    let result = normalize_css_property_value("transform", "translateX(10px)", &opts);
    assert_eq!(result, "translateX(10px)");
  }

  #[test]
  fn normalizes_rgb_color_value() {
    let opts = default_options();
    let result = normalize_css_property_value("color", "rgb(255, 0, 0)", &opts);
    // SWC preserves rgb() function form
    assert_eq!(result, "rgb(255,0,0)");
  }

  #[test]
  fn normalizes_rgba_color_value() {
    let opts = default_options();
    let result = normalize_css_property_value("color", "rgba(0, 0, 0, 0.5)", &opts);
    assert!(result.contains("0") || result.contains("rgba"));
  }

  // --- Relative color syntax ---
  // Relative color syntax used to need a route of its own, because SWC's CSS
  // parser cannot parse it. Nothing parses CSS to normalize a value now, so
  // `from` is an ordinary word. See issue #1041.

  #[test]
  fn normalizes_a_relative_rgb_color() {
    let opts = default_options();
    let result = normalize_css_property_value("backgroundColor", "rgb(from red r g b)", &opts);
    assert_eq!(result, "rgb(from red r g b)");
  }

  /// Relative colour syntax needs no route of its own: the value parser has no
  /// opinion about which function names exist, so `from` is an ordinary word
  /// and the alpha argument loses its leading zero like any other number.
  #[test]
  fn normalizes_a_relative_rgba_color() {
    let opts = default_options();
    let result =
      normalize_css_property_value("backgroundColor", "rgba(from red r g b / 0.5)", &opts);
    assert_eq!(result, "rgba(from red r g b / .5)");
  }

  #[test]
  fn relative_rgb_color_collapses_extra_whitespace() {
    let opts = default_options();
    let result = normalize_css_property_value("color", "rgb(from   red   r g b)", &opts);
    assert_eq!(result, "rgb(from red r g b)");
  }

  #[test]
  fn relative_rgb_color_collapses_whitespace_after_open_paren() {
    let opts = default_options();
    let result = normalize_css_property_value("color", "rgb(   from red r g b   )", &opts);
    assert_eq!(result, "rgb(from red r g b)");
  }

  #[test]
  fn normalizes_a_relative_color_function() {
    let opts = default_options();
    let result = normalize_css_property_value("color", "color(from green srgb r g b)", &opts);
    assert_eq!(result, "color(from green srgb r g b)");
  }

  #[test]
  fn normalizes_a_relative_rgb_inside_a_gradient() {
    let opts = default_options();
    let result = normalize_css_property_value(
      "backgroundImage",
      "linear-gradient(to right, rgb(from red r g b), blue)",
      &opts,
    );
    assert_eq!(result, "linear-gradient(to right,rgb(from red r g b),blue)");
  }

  #[test]
  fn keeps_a_relative_rgb_inside_a_string_as_string_content() {
    let opts = default_options();
    let result = normalize_css_property_value("content", r#""rgb(from   red   r g b)""#, &opts);
    assert_eq!(result, r#""rgb(from   red   r g b)""#);
  }

  #[test]
  fn non_relative_rgb_still_parsed_by_swc() {
    let opts = default_options();
    // Without the `from` keyword, regular rgb() keeps going through SWC.
    let result = normalize_css_property_value("color", "rgb(255, 0, 0)", &opts);
    assert_eq!(result, "rgb(255,0,0)");
  }

  // --- Whitespace handling ---

  #[test]
  fn normalizes_extra_whitespace_in_value() {
    let opts = default_options();
    let result = normalize_css_property_value("margin", "10px   20px   30px", &opts);
    assert_eq!(result, "10px 20px 30px");
  }

  // --- Display values ---

  #[test]
  fn normalizes_flex_display() {
    let opts = default_options();
    let result = normalize_css_property_value("display", "flex", &opts);
    assert_eq!(result, "flex");
  }

  #[test]
  fn normalizes_grid_display() {
    let opts = default_options();
    let result = normalize_css_property_value("display", "grid", &opts);
    assert_eq!(result, "grid");
  }

  // --- Gradient ---

  #[test]
  fn normalizes_radial_gradient() {
    let opts = default_options();
    let result =
      normalize_css_property_value("background", "radial-gradient(circle, red, blue)", &opts);
    assert_eq!(result, "radial-gradient(circle,red,blue)");
  }

  // --- Lab/LCH functions ---

  #[test]
  fn normalizes_lab_color() {
    let opts = default_options();
    let result = normalize_css_property_value("color", "lab(50% 40 59.5)", &opts);
    assert_eq!(result, "lab(50% 40 59.5)");
  }

  #[test]
  fn normalizes_lch_color() {
    let opts = default_options();
    let result = normalize_css_property_value("color", "lch(52.2% 72.2 50)", &opts);
    assert_eq!(result, "lch(52.2% 72.2 50)");
  }

  // --- HWB color ---

  #[test]
  fn normalizes_hwb_color() {
    let opts = default_options();
    let result = normalize_css_property_value("color", "hwb(194 0% 0%)", &opts);
    assert_eq!(result, "hwb(194 0% 0%)");
  }

  // --- Clamp function ---

  #[test]
  fn normalizes_clamp_function() {
    let opts = default_options();
    let result = normalize_css_property_value("fontSize", "clamp(1rem, 2vw, 3rem)", &opts);
    assert_eq!(result, "clamp(1rem,2vw,3rem)");
  }

  #[test]
  fn malformed_unclosed_function_panics() {
    let opts = default_options();
    let result = catch_unwind(AssertUnwindSafe(|| {
      normalize_css_property_value("color", "var(--token", &opts)
    }));

    assert!(result.is_err());
  }

  /// A custom property has no grammar of its own, so the unclosed-function
  /// error is reported against `color` in its place. The rule embedded in the
  /// message must name `color`, not `--my-var`.
  #[test]
  fn unclosed_function_in_custom_property_reports_against_color() {
    let opts = default_options();
    let result = catch_unwind(AssertUnwindSafe(|| {
      normalize_css_property_value("--my-var", "var(--token", &opts)
    }));

    let message = panic_message(result);

    assert!(
      message.contains("* { color: var(--token }"),
      "expected the error to name `color`, got: {message}"
    );
  }

  #[test]
  fn generic_css_value_is_preserved() {
    let opts = default_options();
    let result = normalize_css_property_value("color", "@", &opts);
    assert_eq!(result, "@");
  }

  /// A value SWC cannot parse is preserved only when it is structurally inert.
  /// `}` / `;` / `{` would close the rule the compiler is generating, so such a
  /// value must still be rejected instead of being spliced into the stylesheet.
  #[test]
  fn rule_breaking_unparsable_values_are_rejected() {
    let opts = default_options();

    for value in [
      "}",
      "1px solid } color: red",
      "@ } .evil{color:red",
      "@ { color: red",
    ] {
      let result = catch_unwind(AssertUnwindSafe(|| {
        normalize_css_property_value("height", value, &opts)
      }));

      assert!(result.is_err(), "expected `{value}` to be rejected");
    }
  }

  /// An unclosed `/*` comments out every rule emitted after this declaration,
  /// so it is as rule-breaking as a stray `}`. The "preserve unknown syntax"
  /// fallback emits the value verbatim, which made this reachable.
  #[test]
  fn unclosed_comment_is_rejected() {
    let opts = default_options();

    for value in ["@ /* unclosed", "1px /* unclosed", "/* unclosed"] {
      let result = catch_unwind(AssertUnwindSafe(|| {
        normalize_css_property_value("height", value, &opts)
      }));

      assert!(result.is_err(), "expected `{value}` to be rejected");
    }
  }

  /// A closed comment is inert and must still be accepted.
  #[test]
  fn closed_comment_is_accepted() {
    let opts = default_options();

    assert_eq!(
      normalize_css_property_value("height", "calc-size(auto, size * 0) /* c */", &opts),
      "calc-size(auto,size * 0) /* c */"
    );
    assert_eq!(
      normalize_css_property_value("height", "/* c */ calc-size(auto, size * 0)", &opts),
      "/* c */ calc-size(auto,size * 0)"
    );
  }

  /// Relative colour syntax is guarded like everything else. Every accepted
  /// value now reaches the stylesheet as the author's own bytes, so a stray `}`
  /// in one would splice an arbitrary rule into the sheet whatever the value
  /// happens to be made of.
  #[test]
  fn rule_breaking_relative_color_values_are_rejected() {
    let opts = default_options();

    for value in [
      "rgb(from red r g b) } .evil{color:blue",
      "rgb(from red r g b) /* unclosed",
    ] {
      let result = catch_unwind(AssertUnwindSafe(|| {
        normalize_css_property_value("color", value, &opts)
      }));

      assert!(result.is_err(), "expected `{value}` to be rejected");
    }

    // The well-formed relative-colour value is still passed through.
    assert_eq!(
      normalize_css_property_value("color", "rgb(from red r g b)", &opts),
      "rgb(from red r g b)"
    );
  }

  /// A property name no CSS identifier grammar accepts does not change what
  /// happens to its value: nothing escapes the name, so the value is spelled
  /// back out as written.
  #[test]
  fn numeric_custom_property_name_leaves_the_value_alone() {
    let opts = default_options();

    assert_eq!(normalize_css_property_value("--3abc", "red", &opts), "red");
  }
}

// ── generate_css_rule tests ──────────────────────────────────────────

#[cfg(test)]
mod generate_css_rule_tests {
  use crate::css::common::generate_css_rule;
  use crate::css::tests::support::default_options;
  use stylex_structures::stylex_state_options::StyleXStateOptions;

  #[test]
  fn generates_simple_ltr_rule() {
    let result = generate_css_rule(
      "x123",
      "color",
      &["red".into()],
      &mut [],
      &mut [],
      &mut [],
      &default_options(),
    );
    assert!(result.ltr.contains(".x123"));
    assert!(result.ltr.contains("color:red"));
    assert!(result.rtl.is_none());
    assert!(result.priority.is_some());
  }

  #[test]
  fn generates_rule_with_pseudo() {
    let result = generate_css_rule(
      "x456",
      "color",
      &["blue".into()],
      &mut [":hover".into()],
      &mut [],
      &mut [],
      &default_options(),
    );
    assert!(result.ltr.contains(":hover"));
    assert!(result.ltr.contains("color:blue"));
  }

  #[test]
  fn generates_rule_with_at_rule() {
    let result = generate_css_rule(
      "xmq",
      "color",
      &["green".into()],
      &mut [],
      &mut ["@media (max-width: 600px)".into()],
      &mut [],
      &default_options(),
    );
    assert!(result.ltr.contains("@media"));
    assert!(result.ltr.contains("color:green"));
  }

  #[test]
  fn generates_rule_with_const_rules() {
    let result = generate_css_rule(
      "xcr",
      "color",
      &["red".into()],
      &mut [],
      &mut [],
      &mut ["--myConst".into()],
      &default_options(),
    );
    assert!(result.ltr.contains("--myConst"));
  }

  #[test]
  fn generates_rule_with_pseudo_and_at_rule() {
    let result = generate_css_rule(
      "xpa",
      "color",
      &["red".into()],
      &mut [":hover".into()],
      &mut ["@media (max-width: 600px)".into()],
      &mut [],
      &default_options(),
    );
    assert!(result.ltr.contains(":hover"));
    assert!(result.ltr.contains("@media"));
  }

  #[test]
  fn generates_rule_with_multiple_values() {
    let result = generate_css_rule(
      "xmv",
      "color",
      &["red".into(), "blue".into()],
      &mut [],
      &mut [],
      &mut [],
      &default_options(),
    );
    assert!(result.ltr.contains("color:red"));
    assert!(result.ltr.contains("color:blue"));
  }

  #[test]
  fn priority_increases_with_pseudos() {
    let base = generate_css_rule(
      "xa",
      "color",
      &["red".into()],
      &mut [],
      &mut [],
      &mut [],
      &default_options(),
    );
    let with_pseudo = generate_css_rule(
      "xb",
      "color",
      &["red".into()],
      &mut [":hover".into()],
      &mut [],
      &mut [],
      &default_options(),
    );
    assert!(with_pseudo.priority.unwrap() > base.priority.unwrap());
  }

  #[test]
  fn priority_increases_with_at_rules() {
    let base = generate_css_rule(
      "xa",
      "color",
      &["red".into()],
      &mut [],
      &mut [],
      &mut [],
      &default_options(),
    );
    let with_at = generate_css_rule(
      "xb",
      "color",
      &["red".into()],
      &mut [],
      &mut ["@media (min-width: 800px)".into()],
      &mut [],
      &default_options(),
    );
    assert!(with_at.priority.unwrap() > base.priority.unwrap());
  }

  #[test]
  fn generates_rtl_for_logical_property() {
    use stylex_enums::style_resolution::StyleResolution;

    let opts =
      StyleXStateOptions::default().with_style_resolution(StyleResolution::PropertySpecificity);
    let result = generate_css_rule(
      "xrtl",
      "margin-start",
      &["10px".into()],
      &mut [],
      &mut [],
      &mut [],
      &opts,
    );
    // margin-start should generate LTR → margin-left, RTL → margin-right
    assert!(result.ltr.contains("margin-left"));
    assert!(result.rtl.is_some());
    let rtl = result.rtl.unwrap();
    assert!(rtl.contains("margin-right"));
  }

  #[test]
  fn generates_rule_with_thumb_pseudo() {
    let result = generate_css_rule(
      "xth",
      "color",
      &["red".into()],
      &mut ["::thumb".into()],
      &mut [],
      &mut [],
      &default_options(),
    );
    assert!(result.ltr.contains("::-webkit-slider-thumb"));
    assert!(result.ltr.contains("::-moz-range-thumb"));
    assert!(result.ltr.contains("::-ms-thumb"));
  }

  #[test]
  fn generates_rule_with_where_pseudo() {
    let result = generate_css_rule(
      "xwh",
      "color",
      &["red".into()],
      &mut [":where(.dark)".into()],
      &mut [],
      &mut [],
      &default_options(),
    );
    // Should contain doubled class for specificity
    assert!(result.ltr.contains(".xwh.xwh"));
  }
}

// ── get_priority additional tests ────────────────────────────────────

#[cfg(test)]
mod get_priority_extended_tests {
  use crate::css::common::get_priority;

  #[test]
  fn compound_pseudo_hover_after() {
    // :hover::after is a compound pseudo that should be handled
    let p = get_priority(":hover::after");
    // Should be sum of :hover (130.0) + ::after (5000.0)
    assert!(p > 5000.0);
  }

  #[test]
  fn compound_pseudo_focus_before() {
    let p = get_priority(":focus::before");
    assert!(p > 5000.0);
  }

  #[test]
  fn compound_pseudo_active_placeholder() {
    let p = get_priority(":active::placeholder");
    assert!(p > 5000.0);
  }

  #[test]
  fn at_container_priority() {
    let p = get_priority("@container (min-width: 300px)");
    assert_eq!(p, 300.0);
  }

  #[test]
  fn at_supports_priority() {
    let p = get_priority("@supports (display: grid)");
    assert_eq!(p, 30.0);
  }

  #[test]
  fn at_media_priority() {
    let p = get_priority("@media (hover: hover)");
    assert_eq!(p, 200.0);
  }

  #[test]
  fn pseudo_element_after() {
    let p = get_priority("::after");
    assert_eq!(p, 5000.0);
  }

  #[test]
  fn pseudo_element_before() {
    let p = get_priority("::before");
    assert_eq!(p, 5000.0);
  }

  #[test]
  fn pseudo_element_placeholder() {
    let p = get_priority("::placeholder");
    assert_eq!(p, 5000.0);
  }

  #[test]
  fn pseudo_class_with_parens_no_compound() {
    // :nth-child(2) has parens, so get_compound_pseudo_priority returns None
    let p = get_priority(":nth-child(2)");
    assert_eq!(p, 60.0);
  }

  #[test]
  fn pseudo_class_first_child() {
    assert_eq!(get_priority(":first-child"), 52.0);
  }

  #[test]
  fn pseudo_class_last_child() {
    assert_eq!(get_priority(":last-child"), 54.0);
  }

  #[test]
  fn pseudo_class_active() {
    assert_eq!(get_priority(":active"), 170.0);
  }

  #[test]
  fn pseudo_class_visited() {
    assert_eq!(get_priority(":visited"), 85.0);
  }

  #[test]
  fn pseudo_class_disabled() {
    assert_eq!(get_priority(":disabled"), 92.0);
  }

  #[test]
  fn shorthand_of_shorthands_margin_gets_1000() {
    assert_eq!(get_priority("margin"), 1000.0);
  }

  #[test]
  fn shorthand_of_longhands_gets_2000() {
    assert_eq!(get_priority("border-color"), 2000.0);
  }

  #[test]
  fn unknown_pseudo_class_gets_default_40() {
    assert_eq!(get_priority(":unknown-pseudo"), 40.0);
  }

  #[test]
  fn custom_property_priority() {
    assert_eq!(get_priority("--myVar"), 1.0);
  }

  #[test]
  fn ancestor_selector_priority() {
    // :where(.cls123:hover *)
    let p = get_priority(":where(.cls123:hover *)");
    // Should be 10.0 + (:hover priority / 100.0)
    assert!(p > 10.0 && p < 15.0);
  }

  #[test]
  fn descendant_selector_priority() {
    // :where(:has(.cls123:focus))
    let p = get_priority(":where(:has(.cls123:focus))");
    assert!(p > 15.0 && p < 20.0);
  }

  #[test]
  fn sibling_before_selector_priority() {
    let p = get_priority(":where(.cls123:hover ~ *)");
    assert!(p > 30.0 && p < 35.0);
  }

  #[test]
  fn sibling_after_selector_priority() {
    let p = get_priority(":where(:has(~ .cls123:hover))");
    assert!(p > 40.0 && p < 45.0);
  }

  #[test]
  fn any_sibling_selector_priority() {
    let p = get_priority(":where(.cls123:hover ~ *, :has(~ .cls123:focus))");
    assert!(p > 20.0 && p < 25.0);
  }
}

// ── convert_css_function_to_camel_case coverage ──────────────────────

#[cfg(test)]
mod convert_css_function_camel_case_tests {
  use crate::css::common::normalize_css_property_value;
  use crate::css::tests::support::default_options;

  #[test]
  fn translatey_becomes_camel_case() {
    let r = normalize_css_property_value("transform", "translateY(20px)", &default_options());
    assert_eq!(r, "translateY(20px)");
  }

  #[test]
  fn scalex_becomes_camel_case() {
    let r = normalize_css_property_value("transform", "scaleX(2)", &default_options());
    assert_eq!(r, "scaleX(2)");
  }

  #[test]
  fn scaley_becomes_camel_case() {
    let r = normalize_css_property_value("transform", "scaleY(0.5)", &default_options());
    assert_eq!(r, "scaleY(.5)");
  }

  #[test]
  fn rotatex_becomes_camel_case() {
    let r = normalize_css_property_value("transform", "rotateX(45deg)", &default_options());
    assert_eq!(r, "rotateX(45deg)");
  }

  #[test]
  fn skewx_becomes_camel_case() {
    let r = normalize_css_property_value("transform", "skewX(10deg)", &default_options());
    assert_eq!(r, "skewX(10deg)");
  }

  #[test]
  fn skewy_becomes_camel_case() {
    let r = normalize_css_property_value("transform", "skewY(5deg)", &default_options());
    assert_eq!(r, "skewY(5deg)");
  }

  #[test]
  fn no_function_returns_as_is() {
    // No parentheses → restore_function_names returns as-is
    let r = normalize_css_property_value("color", "red", &default_options());
    assert_eq!(r, "red");
  }

  #[test]
  fn already_lowercase_function_is_unchanged() {
    let r = normalize_css_property_value("transform", "rotate(45deg)", &default_options());
    assert_eq!(r, "rotate(45deg)");
  }

  /// A dashed function name is a custom function, which SWC neither lowercases
  /// nor reports as a plain identifier — so it carries its own case through.
  #[test]
  fn dashed_function_name_keeps_its_case() {
    let r = normalize_css_property_value("color", "--Foo(1px)", &default_options());
    assert_eq!(r, "--Foo(1px)");
  }
}

// ── normalize_css_property_value: CSS variable property path ──────────

#[cfg(test)]
mod normalize_css_variable_property_tests {
  use crate::css::common::normalize_css_property_value;
  use crate::css::tests::support::default_options;

  #[test]
  fn css_variable_uses_color_for_parsing() {
    // When property starts with "--", parsing uses "color" as the property
    let r = normalize_css_property_value("--xCustom", "10px", &default_options());
    assert_eq!(r, "10px");
  }

  #[test]
  fn css_variable_with_hex() {
    let r = normalize_css_property_value("--xBg", "#ff0000", &default_options());
    assert_eq!(r, "#ff0000");
  }

  #[test]
  fn css_variable_with_keyword() {
    let r = normalize_css_property_value("--xBorder", "solid", &default_options());
    assert_eq!(r, "solid");
  }
}

// ── normalize_css_property_value error paths ─────────────────────────

#[cfg(test)]
mod normalize_css_property_value_error_tests {
  use crate::css::common::normalize_css_property_value;
  use crate::css::tests::support::default_options;

  #[test]
  #[should_panic(expected = "Rule contains an unclosed function")]
  fn panics_on_unclosed_function_paren() {
    normalize_css_property_value("color", "rgb(255, 0, 0", &default_options());
  }

  #[test]
  #[should_panic(expected = "Rule contains an unclosed string")]
  fn panics_on_unclosed_double_quoted_string() {
    normalize_css_property_value("fontFamily", r#""Helvetica Neue"#, &default_options());
  }

  #[test]
  #[should_panic(expected = "Rule contains an unclosed string")]
  fn panics_on_unclosed_single_quoted_string() {
    normalize_css_property_value("fontFamily", "'Helvetica Neue", &default_options());
  }

  #[test]
  #[should_panic(expected = "Rule contains an unclosed string")]
  fn panics_on_unclosed_string_in_multi_value_property() {
    normalize_css_property_value(
      "fontFamily",
      r#""Helvetica Neue, sans-serif"#,
      &default_options(),
    );
  }

  #[test]
  #[should_panic(expected = "Rule contains an unclosed function")]
  fn panics_on_unclosed_function_before_unclosed_string() {
    normalize_css_property_value("backgroundImage", r#"url("foo)"#, &default_options());
  }

  #[test]
  #[should_panic(expected = "Rule contains an unclosed function")]
  fn panics_on_unclosed_colour_function() {
    normalize_css_property_value("color", "hsl(0 0% 0%", &default_options());
  }

  #[test]
  #[should_panic(expected = "Rule contains an unclosed string")]
  fn panics_on_stray_apostrophe_in_font_family_list() {
    normalize_css_property_value(
      "fontFamily",
      "'SF Pro Text', 'SF Pro Icons', Helvetica Neue', 'Helvetica', sans-serif",
      &default_options(),
    );
  }

  #[test]
  fn normalizes_closed_double_quoted_string() {
    let r = normalize_css_property_value(
      "fontFamily",
      r#""Helvetica Neue", sans-serif"#,
      &default_options(),
    );
    assert_eq!(r, r#""Helvetica Neue",sans-serif"#);
  }

  /// The quote character the author chose is the quote character that reaches
  /// the hash. Nothing rewrites a single quote into a double one.
  #[test]
  fn normalizes_closed_single_quoted_string() {
    let r = normalize_css_property_value(
      "fontFamily",
      "'Helvetica Neue', sans-serif",
      &default_options(),
    );
    assert_eq!(r, "'Helvetica Neue',sans-serif");
  }

  #[test]
  fn normalizes_system_font_family_list() {
    let r = normalize_css_property_value(
      "fontFamily",
      r#"-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif"#,
      &default_options(),
    );
    assert_eq!(
      r,
      r#"-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,sans-serif"#
    );
  }

  #[test]
  fn normalizes_escaped_quote_in_string() {
    let r = normalize_css_property_value(
      "fontFamily",
      r#""Helvetica \"Neue", sans-serif"#,
      &default_options(),
    );
    assert!(r.contains("Helvetica"));
  }

  #[test]
  fn normalizes_quote_inside_comment() {
    let r = normalize_css_property_value("color", "red /* \" */", &default_options());
    assert!(r.contains("red"));
  }

  #[test]
  fn css_variable_property_uses_color_for_parsing() {
    // --foo is a CSS variable, so it uses "color" as the parsing property
    let r = normalize_css_property_value("--xSomething", "10px", &default_options());
    assert_eq!(r, "10px");
  }

  #[test]
  fn css_variable_with_complex_value() {
    let r = normalize_css_property_value("--xVar", "1px solid #000", &default_options());
    assert_eq!(r, "1px solid #000");
  }
}

// ── build_nested_css_rule additional tests ────────────────────────────

#[cfg(test)]
mod build_nested_css_rule_extended_tests {
  use crate::css::common::build_nested_css_rule;

  #[test]
  fn builds_rule_with_multiple_at_rules() {
    let result = build_nested_css_rule(
      "xm",
      "color:red".into(),
      &mut [],
      &mut [
        "@media (max-width: 600px)".into(),
        "@supports (display: grid)".into(),
      ],
      &mut [],
    );
    assert!(result.contains("@media"));
    assert!(result.contains("@supports"));
    // Should be nested
    assert!(result.contains(".xm.xm.xm"));
  }

  #[test]
  fn builds_rule_with_pseudo_and_at_rule_combined() {
    let result = build_nested_css_rule(
      "xc",
      "color:red".into(),
      &mut [":hover".into()],
      &mut ["@media (min-width: 800px)".into()],
      &mut [],
    );
    assert!(result.contains(":hover"));
    assert!(result.contains("@media"));
  }

  #[test]
  fn builds_rule_with_at_rules_and_const_rules() {
    let result = build_nested_css_rule(
      "xac",
      "color:red".into(),
      &mut [],
      &mut ["@media (min-width: 800px)".into()],
      &mut ["--condition".into()],
    );
    assert!(result.contains("@media"));
    assert!(result.contains("--condition"));
  }

  #[test]
  fn builds_rule_with_thumb_and_pseudo() {
    let result = build_nested_css_rule(
      "xtp",
      "color:red".into(),
      &mut ["::thumb".into(), ":hover".into()],
      &mut [],
      &mut [],
    );
    assert_eq!(
      result,
      ".xtp:hover::-webkit-slider-thumb, .xtp:hover::-moz-range-thumb, \
       .xtp:hover::-ms-thumb{color:red}"
    );
  }

  #[test]
  fn builds_rule_with_pseudo_classes_before_pseudo_element() {
    // Pseudo-elements (::before, ::after, etc.) must come after pseudo-classes
    // in the selector. e.g. `.class:hover::after` not `.class::after:hover`.
    let result = build_nested_css_rule(
      "xpe",
      "color:red".into(),
      &mut ["::after".into(), ":hover".into(), ":active".into()],
      &mut [],
      &mut [],
    );
    assert_eq!(result, ".xpe:hover:active::after{color:red}");
  }

  #[test]
  fn builds_rule_with_where_and_at_rule() {
    let result = build_nested_css_rule(
      "xwa",
      "color:red".into(),
      &mut [":where(.theme)".into()],
      &mut ["@media (min-width: 800px)".into()],
      &mut [],
    );
    assert!(result.contains(":where(.theme)"));
    assert!(result.contains("@media"));
    // where should trigger extra specificity class
    assert!(result.contains(".xwa.xwa"));
  }
}

// ── Coverage: colon-prefixed property ───────────────────────────────

#[cfg(test)]
mod colon_prefixed_property_tests {
  use crate::css::common::normalize_css_property_value;
  use crate::css::tests::support::{default_options, panic_message};
  use std::panic::{AssertUnwindSafe, catch_unwind};

  /// A pseudo-selector key spells its rejection as a selector wrapping the
  /// value rather than as a declaration inside `* { ... }`, which is the only
  /// thing the key's shape still decides.
  ///
  /// The value carries braces, so it is rejected whatever the key: nothing
  /// unwraps a nested rule here, and a value reaching the stylesheet with a
  /// brace in it would close the rule the compiler is generating.
  #[test]
  fn rejects_a_braced_value_under_a_pseudo_key() {
    let result = catch_unwind(AssertUnwindSafe(|| {
      normalize_css_property_value(":hover", "{color:red}", &default_options())
    }));

    let message = panic_message(result);

    assert!(
      message.contains(":hover {color:red}"),
      "expected the rejection to quote the generated rule, got: {message}"
    );
  }
}
