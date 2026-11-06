#[cfg(test)]
mod transform_value_content_property_tests {
  use crate::shared::utils::css::common::transform_value;
  use stylex_state::state_manager::StateManager;
  use stylex_structures::raw_value::TRawValue;
  use stylex_structures::stylex_options::StyleXOptions;

  #[test]
  fn preserves_css_functions_without_quotes() {
    let functions = vec![
      "counters(div, \".\")",
      "counter(chapter)",
      "counter(chapter, upper-roman)",
      "attr(href)",
      "url(image.jpg)",
      "linear-gradient(#e66465, #9198e5)",
      "image-set(\"image1x.png\" 1x, \"image2x.png\" 2x)",
      "\"prefix\"attr(href)",
      "url(foo.jpg)attr(alt)",
      "var(--test)",
      "var(--test, \"default\")",
    ];

    let state_manager = StateManager::new(StyleXOptions::default());

    for input in functions {
      let output = transform_value("content", &TRawValue::from(input), &state_manager);
      assert_eq!(output, input);
    }
  }

  #[test]
  fn preserves_css_keywords_without_quotes() {
    let keywords = vec![
      "normal",
      "none",
      "open-quote",
      "close-quote",
      "no-open-quote",
      "no-close-quote",
      "inherit",
      "initial",
      "revert",
      "revert-layer",
      "unset",
    ];

    let state_manager = StateManager::new(StyleXOptions::default());

    for keyword in keywords {
      let output = transform_value("content", &TRawValue::from(keyword), &state_manager);
      assert_eq!(output, keyword);
    }
  }

  #[test]
  fn handles_mixed_content_values() {
    let mixed_values = vec![
      "open-quote counter(chapter)",
      "\"prefix\"url(image.jpg)",
      "url(\"test.png\")/\"Alt text\"",
      "open-quotecounter(chapter)close-quote",
      "attr(href)normal",
      "\"text\"attr(href)\"more text\"",
      "counter(x)\"text\"counter(y)",
    ];

    let state_manager = StateManager::new(StyleXOptions::default());

    for input in mixed_values {
      let output = transform_value("content", &TRawValue::from(input), &state_manager);
      assert_eq!(output, input);
    }
  }

  #[test]
  fn adds_quotes_to_plain_strings() {
    let strings = vec![
      ("Hello world", "\"Hello world\""),
      ("Simple text", "\"Simple text\""),
      ("123", "\"123\""),
    ];

    let state_manager = StateManager::new(StyleXOptions::default());

    for (input, expected) in strings {
      let output = transform_value("content", &TRawValue::from(input), &state_manager);
      assert_eq!(output, expected);
    }
  }

  #[test]
  fn adds_quotes_to_plain_strings_containing_quote_characters() {
    let strings = vec![
      ("Bob's and Jim's", "\"Bob's and Jim's\""),
      ("It's a test, isn't it", "\"It's a test, isn't it\""),
      ("He said \"hello\"", "\"He said \\\"hello\\\"\""),
      ("say \"hi\" now", "\"say \\\"hi\\\" now\""),
      (
        "\"hello\" is what he said",
        "\"\\\"hello\\\" is what he said\"",
      ),
    ];

    let state_manager = StateManager::new(StyleXOptions::default());

    for (input, expected) in strings {
      let output = transform_value("content", &TRawValue::from(input), &state_manager);
      assert_eq!(output, expected);
    }
  }

  #[test]
  fn preserves_css_escape_sequences_when_adding_quotes() {
    let strings = vec![
      // Inside a CSS string a backslash starts an escape sequence. `\2014` is
      // the escape for an em dash and `\201C` for a left double quotation
      // mark, so escaping the backslash would print the digits instead.
      ("\\2014", "\"\\2014\""),
      ("\\201C hello \\201D", "\"\\201C hello \\201D\""),
      ("back\\slash", "\"back\\slash\""),
      // `\\` is the escape for a literal backslash and stays one escape.
      ("C:\\\\Users", "\"C:\\\\Users\""),
      // A double quote the author already escaped is escaped once, not twice.
      ("He said \\\"hello\\\"", "\"He said \\\"hello\\\"\""),
      // A trailing backslash would escape the closing quote, so it is doubled
      // into the escape for a literal backslash.
      ("50% off \\", "\"50% off \\\\\""),
      // A CSS string cannot hold a line break, so it is written as `\A`.
      ("line one\nline two", "\"line one\\A line two\""),
    ];

    let state_manager = StateManager::new(StyleXOptions::default());

    for (input, expected) in strings {
      let output = transform_value("content", &TRawValue::from(input), &state_manager);
      assert_eq!(output, expected, "quoting `content: {}`", input);
    }
  }

  #[test]
  fn preserves_quote_keywords_combined_with_strings() {
    let values = vec![
      "\"a\" \"b\"",
      "open-quote \"hello\"",
      "\"prefix\" no-close-quote",
      "open-quote \"text\" close-quote",
    ];

    let state_manager = StateManager::new(StyleXOptions::default());

    for input in values {
      let output = transform_value("content", &TRawValue::from(input), &state_manager);
      assert_eq!(output, input);
    }
  }

  /// `hyphenate-character` takes the same component list as `content`, and the
  /// property arrives under both its CSS name and its camel-case name.
  #[test]
  fn quotes_a_hyphenate_character_value_under_either_key() {
    let cases = vec![
      // A plain character is text, so it is quoted.
      ("\u{2010}", "\"\u{2010}\""),
      ("-", "\"-\""),
      // A value that is already a string is left as it stands.
      ("\"-\"", "\"-\""),
      // The two properties share one keyword list, and `auto` is not on it.
      // The initial value of `hyphenate-character` is therefore quoted, which
      // is what the reference compiler does with it too.
      ("auto", "\"auto\""),
      // An apostrophe does not end a double-quoted string, so it is copied as
      // it stands. Only a double quote is escaped.
      ("it's", "\"it's\""),
    ];

    let state_manager = StateManager::new(StyleXOptions::default());

    for key in ["hyphenateCharacter", "hyphenate-character"] {
      for (input, expected) in &cases {
        let output = transform_value(key, &TRawValue::from(*input), &state_manager);

        assert_eq!(output, *expected, "quoting `{}: {}`", key, input);
      }
    }
  }

  #[test]
  fn preserve_units_in_zero_values_css_variables() {
    let variables = vec![
      // CSS variables should preserve units
      ("--test", "0px", "0px"),
      ("--test", "0vdh", "0vdh"),
      // Regular properties still normalize
      ("transform", "0rad", "0deg"),
      ("animation-duration", "0ms", "0s"),
      // grid-template-rows preserves fr units
      ("grid-template-rows", "0fr", "0fr"),
      // Percentages are always preserved
      ("width", "0%", "0%"),
      // Regular properties normalize px to unitless
      ("margin", "0px", "0"),
    ];

    let state_manager = StateManager::new(StyleXOptions::default());

    for (key, value, expected) in variables {
      let output = transform_value(key, &TRawValue::from(value), &state_manager);
      assert_eq!(
        output, expected,
        "Failed for property '{}' with value '{}': expected '{}', got '{}'",
        key, value, expected, output
      );
    }
  }

  /// The corpus values whose only rewrite is this quoting, which reached the
  /// pass through a whole-transform case alone -- a class name and a rule, from
  /// which the quoting can only be inferred.
  ///
  /// Every expectation is what `@stylexjs/babel-plugin` emits for the same
  /// declaration, read from a report the parity harness in
  /// `crates/stylex-rs-compiler/parity` wrote.
  #[test]
  fn quotes_the_corpus_content_values() {
    let cases = vec![
      ("a", "\"a\""),
      ("abc", "\"abc\""),
      ("next", "\"next\""),
      ("x", "\"x\""),
      // Non-ASCII text is quoted as it stands, which is what the class-name
      // hash is then taken over.
      ("\u{2022}", "\"\u{2022}\""),
      ("\u{1F389}", "\"\u{1F389}\""),
      // An unclosed string is not a CSS string, so the value is quoted whole
      // and its lone quote character is escaped.
      ("\"unterminated", "\"\\\"unterminated\""),
    ];

    let state_manager = StateManager::new(StyleXOptions::default());

    for (input, expected) in cases {
      let output = transform_value("content", &TRawValue::from(input), &state_manager);

      assert_eq!(output, expected, "quoting `content: {}`", input);
    }
  }
}
