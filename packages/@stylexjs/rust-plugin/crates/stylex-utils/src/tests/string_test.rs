/// The spelling a `style` attribute gives a name, against the rule the StyleX
/// runtime applies. Each case below was put through `pnpm run parity:probe`
/// from `crates/stylex-rs-compiler` against `@stylexjs/babel-plugin` 0.19.0,
/// and the reference spells every one of them the same way.
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

  /// A name with nothing to convert is handed back as it stands.
  #[test]
  fn borrows_a_name_that_needs_no_conversion() {
    assert!(matches!(kebab_case("margin"), Cow::Borrowed("margin")));
    assert!(matches!(
      kebab_case("already-kebab"),
      Cow::Borrowed("already-kebab")
    ));
    assert!(matches!(kebab_case(""), Cow::Borrowed("")));
  }

  /// A custom property is converted here. A stylesheet hands one back
  /// untouched instead, and it does that with a guard of its own around
  /// `dashify` rather than with a rule inside it, so the contrast cannot be
  /// drawn against `dashify` here.
  #[test]
  fn converts_a_custom_property() {
    assert_eq!(kebab_case("--myColor"), "--my-color");
    assert!(matches!(kebab_case("--x"), Cow::Borrowed("--x")));
  }

  /// Two capitals in a row take a hyphen each here and one between them in a
  /// stylesheet, which is the other place the two rules differ.
  #[test]
  fn dashes_a_run_of_capitals_one_by_one() {
    assert_eq!(kebab_case("ABCDef"), "-a-b-c-def");
    assert_eq!(dashify("ABCDef"), "-abcdef");
  }

  /// A name holding no ASCII capital is still lowercased where it holds
  /// anything else that has a lowercase form.
  ///
  /// The sigma cases are the anchor for lowercasing the whole name at once.
  /// A sigma that ends a word lowercases to `ς` and one that does not to `σ`,
  /// which only a reader of the whole name can tell apart -- so a lowering
  /// done character by character in the loop answers `σ` for both and stops
  /// agreeing with the runtime.
  #[test]
  fn lowercases_a_name_that_is_not_ascii() {
    assert_eq!(kebab_case("Ünicode"), "ünicode");
    assert_eq!(kebab_case("aΣB"), "aς-b");
    assert_eq!(kebab_case("ΑΣ"), "ας");
    assert_eq!(kebab_case("aΣ"), "aς");
    // The same letter inside a word, where the rule answers the other way.
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

  /// The lowercasing has to see the finished string. `char::to_lowercase` is
  /// context-free, so fusing it into the hyphen scan would spell a trailing
  /// Greek sigma in its medial form.
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
    // CSS-like property with consecutive uppercase e.g. "fontSize" not "fontSIZE"
    assert_eq!(dashify("fontSize"), "font-size");
  }

  #[test]
  fn preserves_all_lowercase() {
    assert_eq!(dashify("opacity"), "opacity");
  }

  /// The lowercasing is unconditional, so it must reach scalars that the
  /// `[A-Z]` hyphenation pass does not match. Expectations produced by running
  /// `str.replace(/(^|[a-z])([A-Z])/g, '$1-$2').toLowerCase()`.
  #[test]
  fn lowercases_non_ascii_without_hyphenating() {
    // Titlecase: neither uppercase nor lowercase, but lowercases to `ǆ`.
    assert_eq!(dashify("ǅ"), "ǆ");
    assert_eq!(dashify("ǅBar"), "ǆbar");
    // Uppercase outside ASCII lowercases without gaining a hyphen, because
    // `[A-Z]` does not match it.
    assert_eq!(dashify("Ä"), "ä");
    // Already-lowercase non-ASCII is unchanged.
    assert_eq!(dashify("é"), "é");
    assert_eq!(dashify("naïveColor"), "naïve-color");
  }

  #[test]
  fn handles_single_uppercase() {
    assert_eq!(dashify("A"), "-a");
  }

  /// The hyphen tracks a preceding *lowercase* letter, not merely a
  /// non-uppercase one, so only the first of a run takes one -- which is what
  /// `(?<=^|[a-z])` spelled, and what the hand-rolled scan that replaced it has
  /// to reproduce. Expectations produced by running
  /// `str.replace(/(^|[a-z])([A-Z])/g, '$1-$2').toLowerCase()`.
  #[test]
  fn only_the_first_of_a_run_of_uppercase_takes_a_hyphen() {
    assert_eq!(dashify("aBC"), "a-bc");
    assert_eq!(dashify("msTransformXY"), "ms-transform-xy");
    assert_eq!(dashify("ABC"), "-abc");
  }

  /// A digit is not a lowercase letter, so an uppercase letter after one takes
  /// no hyphen -- the other half of the class the lookbehind named.
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

  /// A scalar outside the basic plane is two code units, so a byte count or a
  /// scalar count would both answer something JavaScript does not.
  #[test]
  fn counts_an_astral_scalar_as_two_code_units() {
    assert_eq!(utf16_length("\u{1F600}a"), 3);
    assert_eq!(utf16_length("\u{1F389}"), 2);
  }

  /// Neither a byte count nor a scalar count agrees with the language here:
  /// `"é"` is two bytes and one code unit, and `"日本語"` is nine bytes and
  /// three.
  #[test]
  fn counts_code_units_rather_than_bytes_or_scalars() {
    assert_eq!(utf16_length("é"), 1);
    assert_eq!(utf16_length("日本語"), 3);
  }

  /// A combining sequence is as many code units as it has scalars — the
  /// language does not normalize before counting, so `"e\u{301}"` is `2` even
  /// though it renders as one character.
  #[test]
  fn does_not_normalize_a_combining_sequence() {
    assert_eq!(utf16_length("e\u{301}"), 2);
  }

  /// A NUL and the other C0 controls are ordinary characters to `length`.
  #[test]
  fn counts_control_characters() {
    assert_eq!(utf16_length("a\u{0}b"), 3);
    assert_eq!(utf16_length("\u{1}\u{2}"), 2);
  }

  /// ASCII is answered from the byte length rather than from the encoder, so the
  /// two readings are compared directly -- across the boundary, since a string
  /// with one non-ASCII scalar in it must leave the fast path whichever end that
  /// scalar sits at.
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

  /// The property that makes this the language's view of a length: the last
  /// index that reads a code unit is one below it, and the length itself reads
  /// nothing. Read off the code units directly, so the claim does not rest on a
  /// second helper agreeing with this one.
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
    // Surrounding whitespace does not make a value blank.
    assert!(!is_blank_css_text(" red "));
    // Empty *quotes* are CSS text, which is what a blank `content` becomes.
    assert!(!is_blank_css_text("\"\""));
  }

  #[test]
  fn unicode_whitespace_is_a_value() {
    // The value scanner reads whitespace as "char code <= 32" and nothing
    // else, so each of these is a *word token* to it, not a gap. The
    // reference compiler emits them: `transformValue("color", "\u{3000}")`
    // produces the declaration rather than dropping the property.
    assert!(!is_blank_css_text("\u{3000}")); // ideographic space
    assert!(!is_blank_css_text("\u{00a0}")); // no-break space
    assert!(!is_blank_css_text("\u{2028}")); // line separator
    // Mixed with scanner whitespace, the word token still carries the value.
    assert!(!is_blank_css_text(" \u{00a0} "));
  }

  /// The other side of the same divergence from `str::trim`, and the side that
  /// changed an answer rather than fixing one.
  ///
  /// A C0 control is not Unicode whitespace, so `trim` used to leave it in
  /// place and the value was rejected downstream by `trim_edges`. Reading the
  /// scanner's rule instead makes it blank, so the property is dropped the way
  /// it already was for `""` and `" "`. Pinned so that undoing it has to be a
  /// decision.
  #[test]
  fn c0_controls_that_are_not_unicode_whitespace_are_blank() {
    assert!(is_blank_css_text("\u{0}")); // NUL
    assert!(is_blank_css_text("\u{1}")); // start of heading
    assert!(is_blank_css_text("\u{1f}")); // unit separator
    // And still true of the controls `trim` already agreed were whitespace.
    assert!(is_blank_css_text("\u{b}")); // vertical tab
    assert!(is_blank_css_text("\u{c}")); // form feed
    // Code 32 is the boundary; 33 is the first character that spells a value.
    assert!(is_blank_css_text("\u{20}"));
    assert!(!is_blank_css_text("\u{21}"));
  }
}

#[cfg(test)]
mod json_stringify_tests {
  use crate::string::json_stringify;

  /// The plain case: quotes added, nothing else touched. Every rejection
  /// message that names a value spends most of its life here.
  #[test]
  fn wraps_a_plain_string_in_double_quotes() {
    assert_eq!(json_stringify("none inherit"), "\"none inherit\"");
  }

  #[test]
  fn quotes_the_empty_string() {
    assert_eq!(json_stringify(""), "\"\"");
  }

  /// A quote inside the value is what would otherwise end the quoted run, so
  /// this is the escape that keeps the message parseable at all.
  #[test]
  fn escapes_an_embedded_double_quote() {
    assert_eq!(json_stringify("a\"b"), "\"a\\\"b\"");
  }

  #[test]
  fn escapes_a_backslash() {
    assert_eq!(json_stringify("a\\b"), "\"a\\\\b\"");
  }

  /// A backslash immediately before a quote must escape as two independent
  /// escapes and not collapse into one — `\\` then `\"`, never `\\"`.
  #[test]
  fn escapes_a_backslash_followed_by_a_quote_separately() {
    assert_eq!(json_stringify("\\\""), "\"\\\\\\\"\"");
  }

  /// The five controls JSON gives a single-letter escape. Anything else below
  /// U+0020 takes the `\uXXXX` form instead, which the next test pins.
  #[test]
  fn uses_the_single_letter_escapes() {
    assert_eq!(json_stringify("\u{8}"), "\"\\b\"");
    assert_eq!(json_stringify("\u{c}"), "\"\\f\"");
    assert_eq!(json_stringify("\n"), "\"\\n\"");
    assert_eq!(json_stringify("\r"), "\"\\r\"");
    assert_eq!(json_stringify("\t"), "\"\\t\"");
  }

  /// Lowercase hex, four digits, zero-padded. `{:04X}` would read as valid JSON
  /// and still not be the text upstream emits.
  #[test]
  fn escapes_remaining_c0_controls_as_lowercase_four_digit_hex() {
    assert_eq!(json_stringify("\u{0}"), "\"\\u0000\"");
    assert_eq!(json_stringify("\u{1}"), "\"\\u0001\"");
    assert_eq!(json_stringify("\u{b}"), "\"\\u000b\"");
    assert_eq!(json_stringify("\u{e}"), "\"\\u000e\"");
    assert_eq!(json_stringify("\u{1a}"), "\"\\u001a\"");
    assert_eq!(json_stringify("\u{1f}"), "\"\\u001f\"");
  }

  /// U+0020 is the boundary: the space is the first code point written through
  /// rather than escaped.
  #[test]
  fn writes_the_space_through_unescaped() {
    assert_eq!(json_stringify(" "), "\" \"");
  }

  /// DEL and the two line separators are the tempting exceptions. A JS source
  /// literal could not carry them, but `JSON.stringify` is not building source
  /// and leaves all three raw.
  #[test]
  fn writes_del_and_the_line_separators_through_raw() {
    assert_eq!(json_stringify("\u{7f}"), "\"\u{7f}\"");
    assert_eq!(json_stringify("\u{2028}"), "\"\u{2028}\"");
    assert_eq!(json_stringify("\u{2029}"), "\"\u{2029}\"");
  }

  /// Non-ASCII is not escaped, whether it fits one UTF-16 code unit or two.
  /// Escaping it would be valid JSON and the wrong text.
  #[test]
  fn writes_non_ascii_through_unescaped() {
    assert_eq!(json_stringify("éé"), "\"éé\"");
    assert_eq!(json_stringify("日本"), "\"日本\"");
    assert_eq!(json_stringify("🎉"), "\"🎉\"");
    assert_eq!(json_stringify("\u{feff}"), "\"\u{feff}\"");
  }

  /// A value can be long without being special. Nothing here scales with
  /// nesting or with the number of escapes, and this says so.
  #[test]
  fn handles_a_long_value_of_only_escapes() {
    let input = "\"".repeat(10_000);
    let expected = format!("\"{}\"", "\\\"".repeat(10_000));
    assert_eq!(json_stringify(&input), expected);
  }
}
