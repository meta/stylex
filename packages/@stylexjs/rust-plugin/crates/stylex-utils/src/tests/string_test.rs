/// Expectations match the StyleX runtime's `style` attribute spelling (babel-plugin 0.19.0).
#[cfg(test)]
mod kebab_case_tests {
  use std::borrow::Cow;

  use crate::string::{dashify, kebab_case};

  #[test]
  fn dashes_every_capital_and_lowercases_the_name() {
    assert_eq!(kebab_case("marginTop"), "margin-top");
    assert_eq!(kebab_case("ABC"), "-a-b-c");
    assert_eq!(kebab_case("A"), "-a");
  }

  #[test]
  fn borrows_a_name_that_needs_no_conversion() {
    assert!(matches!(kebab_case("margin"), Cow::Borrowed("margin")));
    assert!(matches!(
      kebab_case("already-kebab"),
      Cow::Borrowed("already-kebab")
    ));
    assert!(matches!(kebab_case(""), Cow::Borrowed("")));
  }

  /// A stylesheet leaves custom properties alone, but a `style` attribute converts them.
  #[test]
  fn converts_a_custom_property() {
    assert_eq!(kebab_case("--myColor"), "--my-color");
    assert!(matches!(kebab_case("--x"), Cow::Borrowed("--x")));
  }

  /// A stylesheet hyphenates `ABCDef` as `-abcdef`, but a `style` attribute dashes each capital.
  #[test]
  fn dashes_a_run_of_capitals_one_by_one() {
    assert_eq!(kebab_case("ABCDef"), "-a-b-c-def");
    assert_eq!(dashify("ABCDef"), "-abcdef");
  }

  /// A final sigma lowercases to `ς`, which only lowercasing the whole name gets right.
  #[test]
  fn lowercases_a_name_that_is_not_ascii() {
    assert_eq!(kebab_case("Ünicode"), "ünicode");
    assert_eq!(kebab_case("aΣB"), "aς-b");
    assert_eq!(kebab_case("ΑΣ"), "ας");
    assert_eq!(kebab_case("aΣ"), "aς");
    assert_eq!(kebab_case("ΣΑ"), "σα");
  }
}

#[cfg(test)]
mod dashify_tests {
  use crate::string::dashify;

  #[test]
  fn converts_simple_camel_case() {
    assert_eq!(dashify("marginTop"), "margin-top");
  }

  #[test]
  fn converts_multiple_uppercase_letters() {
    assert_eq!(
      dashify("borderBottomLeftRadius"),
      "border-bottom-left-radius"
    );
  }

  #[test]
  fn handles_already_lowercase() {
    assert_eq!(dashify("margin"), "margin");
  }

  #[test]
  fn handles_single_char() {
    assert_eq!(dashify("a"), "a");
  }

  #[test]
  fn lowercases_a_trailing_sigma_to_its_final_form() {
    assert_eq!(dashify("aBΣ"), "a-bς");
  }

  #[test]
  fn handles_empty_string() {
    assert_eq!(dashify(""), "");
  }

  #[test]
  fn handles_vendor_prefix_webkit() {
    assert_eq!(
      dashify("WebkitTapHighlightColor"),
      "-webkit-tap-highlight-color"
    );
  }

  #[test]
  fn handles_vendor_prefix_ms() {
    assert_eq!(dashify("MsTransition"), "-ms-transition");
  }

  #[test]
  fn handles_vendor_prefix_moz() {
    assert_eq!(dashify("MozTransition"), "-moz-transition");
  }

  #[test]
  fn handles_consecutive_uppercase() {
    assert_eq!(dashify("fontSize"), "font-size");
  }

  #[test]
  fn preserves_all_lowercase() {
    assert_eq!(dashify("opacity"), "opacity");
  }

  /// Expectations from JS `str.replace(/(^|[a-z])([A-Z])/g, '$1-$2').toLowerCase()`.
  #[test]
  fn lowercases_non_ascii_without_hyphenating() {
    // Titlecase `ǅ` is neither uppercase nor lowercase but lowercases to `ǆ`.
    assert_eq!(dashify("ǅ"), "ǆ");
    assert_eq!(dashify("ǅBar"), "ǆbar");
    assert_eq!(dashify("Ä"), "ä");
    assert_eq!(dashify("é"), "é");
    assert_eq!(dashify("naïveColor"), "naïve-color");
  }

  #[test]
  fn handles_single_uppercase() {
    assert_eq!(dashify("A"), "-a");
  }

  /// Expectations from JS `str.replace(/(^|[a-z])([A-Z])/g, '$1-$2').toLowerCase()`.
  #[test]
  fn only_the_first_of_a_run_of_uppercase_takes_a_hyphen() {
    assert_eq!(dashify("aBC"), "a-bc");
    assert_eq!(dashify("msTransformXY"), "ms-transform-xy");
    assert_eq!(dashify("ABC"), "-abc");
  }

  #[test]
  fn a_digit_before_an_uppercase_letter_takes_no_hyphen() {
    assert_eq!(dashify("grid2Column"), "grid2column");
  }
}

#[cfg(test)]
mod utf16_length_tests {
  use crate::string::utf16_length;

  #[test]
  fn counts_ascii_characters() {
    assert_eq!(utf16_length("abc"), 3);
    assert_eq!(utf16_length(""), 0);
  }

  #[test]
  fn counts_an_astral_scalar_as_two_code_units() {
    assert_eq!(utf16_length("\u{1F600}a"), 3);
    assert_eq!(utf16_length("\u{1F389}"), 2);
  }

  #[test]
  fn counts_code_units_rather_than_bytes_or_scalars() {
    assert_eq!(utf16_length("é"), 1);
    assert_eq!(utf16_length("日本語"), 3);
  }

  /// JS does not normalize before counting, so `"e\u{301}"` has length `2`.
  #[test]
  fn does_not_normalize_a_combining_sequence() {
    assert_eq!(utf16_length("e\u{301}"), 2);
  }

  #[test]
  fn counts_control_characters() {
    assert_eq!(utf16_length("a\u{0}b"), 3);
    assert_eq!(utf16_length("\u{1}\u{2}"), 2);
  }

  #[test]
  fn the_ascii_shortcut_answers_what_the_encoder_answers() {
    for source in [
      "",
      "a",
      "abc",
      "a b\tc\n",
      "\u{7f}",
      "\u{80}",
      "é",
      "aé",
      "éa",
      "日本語",
      "\u{1F600}",
      "a\u{1F600}",
      "\u{1F600}a",
      "a\u{0}b",
    ] {
      assert_eq!(
        utf16_length(source),
        source.encode_utf16().count(),
        "the two readings of {:?} disagree",
        source
      );
    }
  }

  #[test]
  fn counts_one_past_the_last_readable_index() {
    for source in ["abc", "", "\u{1F600}a", "é", "e\u{301}", "a\u{0}b"] {
      let length = utf16_length(source);
      let unit = |index: usize| source.encode_utf16().nth(index);

      assert_eq!(unit(length), None, "past the end of {:?}", source);

      if length > 0 {
        assert!(
          unit(length - 1).is_some(),
          "at the last index of {:?}",
          source
        );
      }
    }
  }
}

#[cfg(test)]
mod is_blank_css_text_tests {
  use crate::string::is_blank_css_text;

  #[test]
  fn recognises_text_that_spells_nothing() {
    assert!(is_blank_css_text(""));
    assert!(is_blank_css_text(" "));
    assert!(is_blank_css_text("  \t\n "));
  }

  #[test]
  fn keeps_text_that_spells_a_value() {
    assert!(!is_blank_css_text("0"));
    assert!(!is_blank_css_text("red"));
    assert!(!is_blank_css_text(" red "));
    // Empty quotes are CSS text.
    assert!(!is_blank_css_text("\"\""));
  }

  #[test]
  fn unicode_whitespace_is_a_value() {
    // The upstream value scanner treats only char codes <= 32 as whitespace.
    assert!(!is_blank_css_text("\u{3000}")); // ideographic space
    assert!(!is_blank_css_text("\u{00a0}")); // no-break space
    assert!(!is_blank_css_text("\u{2028}")); // line separator
    assert!(!is_blank_css_text(" \u{00a0} "));
  }

  #[test]
  fn c0_controls_that_are_not_unicode_whitespace_are_blank() {
    assert!(is_blank_css_text("\u{0}")); // NUL
    assert!(is_blank_css_text("\u{1}")); // start of heading
    assert!(is_blank_css_text("\u{1f}")); // unit separator
    assert!(is_blank_css_text("\u{b}")); // vertical tab
    assert!(is_blank_css_text("\u{c}")); // form feed
    assert!(is_blank_css_text("\u{20}"));
    assert!(!is_blank_css_text("\u{21}"));
  }
}

#[cfg(test)]
mod json_stringify_tests {
  use crate::string::json_stringify;

  #[test]
  fn wraps_a_plain_string_in_double_quotes() {
    assert_eq!(json_stringify("none inherit"), "\"none inherit\"");
  }

  #[test]
  fn quotes_the_empty_string() {
    assert_eq!(json_stringify(""), "\"\"");
  }

  #[test]
  fn escapes_an_embedded_double_quote() {
    assert_eq!(json_stringify("a\"b"), "\"a\\\"b\"");
  }

  #[test]
  fn escapes_a_backslash() {
    assert_eq!(json_stringify("a\\b"), "\"a\\\\b\"");
  }

  #[test]
  fn escapes_a_backslash_followed_by_a_quote_separately() {
    assert_eq!(json_stringify("\\\""), "\"\\\\\\\"\"");
  }

  #[test]
  fn uses_the_single_letter_escapes() {
    assert_eq!(json_stringify("\u{8}"), "\"\\b\"");
    assert_eq!(json_stringify("\u{c}"), "\"\\f\"");
    assert_eq!(json_stringify("\n"), "\"\\n\"");
    assert_eq!(json_stringify("\r"), "\"\\r\"");
    assert_eq!(json_stringify("\t"), "\"\\t\"");
  }

  #[test]
  fn escapes_remaining_c0_controls_as_lowercase_four_digit_hex() {
    assert_eq!(json_stringify("\u{0}"), "\"\\u0000\"");
    assert_eq!(json_stringify("\u{1}"), "\"\\u0001\"");
    assert_eq!(json_stringify("\u{b}"), "\"\\u000b\"");
    assert_eq!(json_stringify("\u{e}"), "\"\\u000e\"");
    assert_eq!(json_stringify("\u{1a}"), "\"\\u001a\"");
    assert_eq!(json_stringify("\u{1f}"), "\"\\u001f\"");
  }

  #[test]
  fn writes_the_space_through_unescaped() {
    assert_eq!(json_stringify(" "), "\" \"");
  }

  /// `JSON.stringify` leaves these raw, even though a JS source literal could not hold them.
  #[test]
  fn writes_del_and_the_line_separators_through_raw() {
    assert_eq!(json_stringify("\u{7f}"), "\"\u{7f}\"");
    assert_eq!(json_stringify("\u{2028}"), "\"\u{2028}\"");
    assert_eq!(json_stringify("\u{2029}"), "\"\u{2029}\"");
  }

  #[test]
  fn writes_non_ascii_through_unescaped() {
    assert_eq!(json_stringify("éé"), "\"éé\"");
    assert_eq!(json_stringify("日本"), "\"日本\"");
    assert_eq!(json_stringify("🎉"), "\"🎉\"");
    assert_eq!(json_stringify("\u{feff}"), "\"\u{feff}\"");
  }

  #[test]
  fn handles_a_long_value_of_only_escapes() {
    let input = "\"".repeat(10_000);
    let expected = format!("\"{}\"", "\\\"".repeat(10_000));
    assert_eq!(json_stringify(&input), expected);
  }
}
