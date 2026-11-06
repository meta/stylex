//! Value-normalization coverage at the public entry point.
//!
//! Expectations are `@stylexjs/babel-plugin` spellings from the parity harness in
//! `crates/stylex-rs-compiler/parity`. The second run below covers the
//! [`rem_enabled_options`](super::support::rem_enabled_options) cases:
//!
//! ```sh
//! pnpm run --filter=@stylexswc/rs-compiler build
//! pnpm run --filter=@stylexswc/rs-compiler parity -- --json parity/results/default.json
//! pnpm run --filter=@stylexswc/rs-compiler parity -- --font-size-px-to-rem \
//!   --json parity/results/font-size-px-to-rem.json
//! ```

use std::panic::{AssertUnwindSafe, catch_unwind};

use stylex_constants::constants::messages::{
  LINT_RULE_BREAKING_TOKEN, LINT_UNCLOSED_COMMENT, LINT_UNCLOSED_FUNCTION, LINT_UNCLOSED_STRING,
};

use super::support::{check, default_options, panic_message, rem_enabled_options, same, unchanged};
use crate::css::common::{MAX_VALUE_NESTING_DEPTH, normalize_css_property_value};

// ── Timings ──────────────────────────────────────────────────────────

/// Except below the threshold, where seconds would not be shorter.
#[test]
fn converts_milliseconds_to_seconds() {
  check(
    &[
      same("transitionDuration", "500ms", ".5s"),
      same("transitionDuration", "1000ms", "1s"),
      same("transitionDuration", "0ms", "0s"),
      unchanged("transitionDuration", "2s"),
    ],
    &default_options(),
  );
}

#[test]
fn keeps_milliseconds_below_the_conversion_threshold() {
  check(
    &[unchanged("transitionDuration", "5ms")],
    &default_options(),
  );
}

// ── Zero dimensions ──────────────────────────────────────────────────

/// A zero length loses its unit; a zero fraction, percentage or angle keeps it.
#[test]
fn normalizes_zero_dimensions_by_unit_kind() {
  check(
    &[
      same("margin", "0px", "0"),
      unchanged("width", "0%"),
      unchanged("gridTemplateColumns", "0fr"),
      unchanged("gridTemplateColumns", "0fr 1fr"),
      same("image-resolution", "0dpi", "0"),
      same("color", "0Hz", "0"),
      same("color", "0unknown", "0"),
      same("image-orientation", "0rad", "0deg"),
    ],
    &default_options(),
  );
}

#[test]
fn leaves_non_zero_dimensions_alone() {
  check(
    &[
      unchanged("margin", "10px"),
      unchanged("width", "16px"),
      unchanged("image-orientation", "90deg"),
    ],
    &default_options(),
  );
}

#[test]
fn leaves_zeros_inside_functions_alone() {
  check(
    &[
      unchanged("color", "calc(0 - var(--someVar))"),
      unchanged("color", "calc(0px - var(--someVar) + 10px)"),
      unchanged("width", "calc(0px + 10px)"),
    ],
    &default_options(),
  );
}

/// Follows the reference compiler's function-argument window, quirks included.
#[test]
fn rewrites_a_zero_angle_inside_a_transform_function() {
  check(
    &[same("transform", "rotate(0rad)", "rotate(0deg)")],
    &default_options(),
  );
}

// ── Custom properties ────────────────────────────────────────────────

#[test]
fn exempts_custom_properties_from_value_normalization() {
  check(
    &[
      unchanged("--myVar", "0px"),
      unchanged("--myVar", "backgroundColor"),
      unchanged("--my-var", "foo"),
    ],
    &default_options(),
  );
}

// ── Leading zero ─────────────────────────────────────────────────────

#[test]
fn strips_the_leading_zero_from_a_decimal() {
  check(
    &[
      same("opacity", "0.5", ".5"),
      unchanged("grid-column-start", "-1"),
    ],
    &default_options(),
  );
}

// ── Quotes ───────────────────────────────────────────────────────────

#[test]
fn keeps_the_authored_quote_character() {
  check(
    &[
      unchanged("quotes", r#"'""'"#),
      unchanged("quotes", r#"'"123"'"#),
      unchanged("quotes", r#""""#),
      unchanged("quotes", r#""123""#),
    ],
    &default_options(),
  );
}

#[test]
fn keeps_the_authored_quote_character_in_grid_template_areas() {
  check(
    &[
      unchanged("gridTemplateAreas", r#"'"content"'"#),
      unchanged("gridTemplateAreas", r#"'"content" "sidebar"'"#),
      unchanged("gridTemplateAreas", r#""content""#),
      unchanged("gridTemplateAreas", r#""content" "sidebar""#),
    ],
    &default_options(),
  );
}

// ── Camel-case value conversion ──────────────────────────────────────

#[test]
fn hyphenates_camel_case_values_for_the_properties_that_take_property_names() {
  check(
    &[
      same("transitionProperty", "backgroundColor", "background-color"),
      same(
        "transitionProperty",
        "opacity, margin-top",
        "opacity,margin-top",
      ),
      unchanged("transitionProperty", "--myVar"),
      same("willChange", "marginTop", "margin-top"),
      same("willChange", "backgroundColor", "background-color"),
    ],
    &default_options(),
  );
}

#[test]
fn leaves_ordinary_keyword_values_alone() {
  check(&[unchanged("color", "red")], &default_options());
}

// ── Whole-value shapes ───────────────────────────────────────────────

#[test]
fn normalizes_shadow_values() {
  check(
    &[
      same(
        "boxShadow",
        "0px 2px 4px var(--shadow-1)",
        "0 2px 4px var(--shadow-1)",
      ),
      unchanged("boxShadow", "1px 1px #000"),
    ],
    &default_options(),
  );
}

#[test]
fn normalizes_adjacent_custom_property_references() {
  check(
    &[
      unchanged("color", "var(--a) var(--b) var(--c)"),
      unchanged("color", "var(--someVar)"),
    ],
    &default_options(),
  );
}

#[test]
fn keeps_flush_references_flush() {
  check(
    &[unchanged("color", "var(--a)var(--b)var(--c)")],
    &default_options(),
  );
}

// ── Colour and math functions ────────────────────────────────────────

/// Colour functions take the same path as every other value.
#[test]
fn strips_leading_zeros_inside_colour_functions() {
  check(
    &[
      same(
        "color",
        "oklch(from var(--xs74gcj) l c h / 0.5)",
        "oklch(from var(--xs74gcj) l c h / .5)",
      ),
      same(
        "color",
        "oklab(40.101% 0.1147 0.0453)",
        "oklab(40.101% .1147 .0453)",
      ),
      same(
        "color",
        "oklab(from #0000FF calc(l + 0.1) a b / calc(alpha * 0.9))",
        "oklab(from #0000FF calc(l + .1) a b / calc(alpha * .9))",
      ),
      same(
        "color",
        "oklab(from hsl(180 100% 50%) calc(l - 0.1) a b)",
        "oklab(from hsl(180 100% 50%) calc(l - .1) a b)",
      ),
      same(
        "color",
        "oklab(from green l a b / 0.5)",
        "oklab(from green l a b / .5)",
      ),
    ],
    &default_options(),
  );
}

#[test]
fn removes_spacing_after_commas_inside_functions() {
  check(
    &[
      same(
        "color",
        "clamp(200px,  40%,     400px)",
        "clamp(200px,40%,400px)",
      ),
      same(
        "color",
        "clamp(min(10vw,      20rem),     300px,     max(90vw,     55rem))",
        "clamp(min(10vw,20rem),300px,max(90vw,55rem))",
      ),
      same(
        "color",
        "clamp(0, (var(--l-threshold, 0.623)   /  l - 1)   *    infinity,    1)",
        "clamp(0,(var(--l-threshold,.623) / l - 1) * infinity,1)",
      ),
    ],
    &default_options(),
  );
}

#[test]
fn keeps_an_already_tight_function_value() {
  check(
    &[unchanged(
      "color",
      "clamp(min(10vw,20rem),300px,max(90vw,55rem))",
    )],
    &default_options(),
  );
}

// ── Font-size conversion ─────────────────────────────────────────────

#[test]
fn converts_font_size_pixels_to_rem_when_enabled() {
  check(
    &[
      same("fontSize", "16px", "1rem"),
      same("fontSize", "32px", "2rem"),
      same("fontSize", "0px", "0"),
    ],
    &rem_enabled_options(),
  );
}

#[test]
fn converts_nothing_else_when_font_size_conversion_is_enabled() {
  check(
    &[
      unchanged("fontSize", "2em"),
      unchanged("fontSize", "1rem"),
      unchanged("width", "16px"),
    ],
    &rem_enabled_options(),
  );
}

/// The conversion runs after the leading-zero pass, so its output keeps the zero.
#[test]
fn keeps_the_leading_zero_on_a_sub_rem_font_size() {
  check(&[same("fontSize", "8px", "0.5rem")], &rem_enabled_options());
}

#[test]
fn leaves_font_size_alone_when_the_option_is_off() {
  check(&[unchanged("fontSize", "16px")], &default_options());
}

// ── Vendor prefixes ──────────────────────────────────────────────────

/// Including when camel-case conversion produces one.
#[test]
fn carries_vendor_prefixes_through_unchanged() {
  check(
    &[
      unchanged("display", "-webkit-box"),
      same("transitionProperty", "WebkitTransform", "-webkit-transform"),
      unchanged("transitionProperty", "-webkit-transform"),
    ],
    &default_options(),
  );
}

#[test]
fn keeps_a_hex_colour_inside_a_vendor_prefixed_function() {
  check(
    &[same(
      "backgroundImage",
      "-webkit-linear-gradient(top, #FFFFFF, #000000)",
      "-webkit-linear-gradient(top,#FFFFFF,#000000)",
    )],
    &default_options(),
  );
}

// ── Unicode and escapes ──────────────────────────────────────────────

#[test]
fn preserves_an_escape_outside_a_string() {
  check(
    &[same(
      "fontFamily",
      r"My\ Font, sans-serif",
      r"My\ Font,sans-serif",
    )],
    &default_options(),
  );
}

/// It used to be resolved, changing the hashed bytes.
#[test]
fn keeps_an_escape_inside_a_string_unresolved() {
  check(
    &[
      same(
        "fontFamily",
        r#""\2014 A", sans-serif"#,
        r#""\2014 A",sans-serif"#,
      ),
      same(
        "fontFamily",
        r#""\1F600", sans-serif"#,
        r#""\1F600",sans-serif"#,
      ),
    ],
    &default_options(),
  );
}

#[test]
fn preserves_non_ascii_content() {
  check(
    &[
      same(
        "fontFamily",
        "日本語フォント, sans-serif",
        "日本語フォント,sans-serif",
      ),
      same(
        "fontFamily",
        "\"→ Привет 日本語 🙂\", sans-serif",
        "\"→ Привет 日本語 🙂\",sans-serif",
      ),
    ],
    &default_options(),
  );
}

// ── Malformed input ──────────────────────────────────────────────────

/// Unparsable values are emitted verbatim when they cannot break out of the rule.
#[test]
fn passes_inert_unparsable_values_through() {
  check(
    &[
      unchanged("gridTemplateColumns", "[full-start 1fr [content-start"),
      unchanged("width", "10px ++ 20px"),
    ],
    &default_options(),
  );
}

/// Harness verdict `identical`; a trailing `;` cannot splice a second declaration.
#[test]
fn keeps_a_trailing_semicolon() {
  check(
    &[
      unchanged("backgroundColor", "var(--web-wash);"),
      same("color", "red ; ", "red ;"),
      unchanged("color", "red;;"),
    ],
    &default_options(),
  );
}

/// Harness verdict `acceptance divergent`: the reference compiler emits both declarations.
#[test]
fn rejects_a_semicolon_that_starts_a_second_declaration() {
  let options = default_options();

  for value in ["red; margin: 10px", "red;background:blue", "red; /* x */"] {
    let result = catch_unwind(AssertUnwindSafe(|| {
      normalize_css_property_value("color", value, &options)
    }));

    let message = panic_message(result);

    assert!(
      message.contains("outside of a string or comment"),
      "expected `color: {value}` to be rejected, got: {message}"
    );
  }
}

/// Harness verdict `acceptance divergent`: the reference compiler emits `color:red {`.
#[test]
fn rejects_a_value_carrying_an_opening_brace() {
  let options = default_options();

  let result = catch_unwind(AssertUnwindSafe(|| {
    normalize_css_property_value("color", "red {", &options)
  }));

  let message = panic_message(result);

  assert!(
    message.contains("* { color: red { }"),
    "expected the rejection to quote the generated rule, got: {message}"
  );
}

// ── Which of two true complaints an author is handed ─────────────────

/// The message `property: value` is refused with.
fn refusal_of(property: &str, value: &str) -> String {
  let options = default_options();
  let result = catch_unwind(AssertUnwindSafe(|| {
    normalize_css_property_value(property, value, &options)
  }));

  panic_message(result)
}

/// Asserts the refusal contains `expected` and not `instead_of`.
fn refuses_with(property: &str, value: &str, expected: &str, instead_of: &str) {
  let message = refusal_of(property, value);

  assert!(
    message.contains(expected),
    "expected `{property}: {value}` to be refused with `{expected}`, got: {message}"
  );
  assert!(
    !message.contains(instead_of),
    "expected `{property}: {value}` not to be refused with `{instead_of}`, got: {message}"
  );
}

/// Unclosed wins over rule-breaking, matching the reference compiler's message
/// (verdict `both reject`).
#[test]
fn hands_over_the_complaint_the_reference_compiler_also_writes() {
  for value in [
    "red;calc(1px",
    "red;background:blue;calc(",
    "red {calc(1px",
    "red }calc(1px",
    "calc(;a",
  ] {
    refuses_with(
      "color",
      value,
      LINT_UNCLOSED_FUNCTION,
      LINT_RULE_BREAKING_TOKEN,
    );
  }

  for value in [r#"red;"a"#, "red;'a", r#"red {"a"#] {
    refuses_with(
      "color",
      value,
      LINT_UNCLOSED_STRING,
      LINT_RULE_BREAKING_TOKEN,
    );
  }
}

/// Escaped terminators are accepted (`an_escaped_terminator_is_not_a_terminator`); the quoted
/// rule text is asserted too.
#[test]
fn still_refuses_a_value_whose_only_fault_is_the_token() {
  // Not a table: the parity corpus only harvests a property literal next to a value literal.
  let token = LINT_RULE_BREAKING_TOKEN;
  refuses_with("color", "red; margin: 10px", token, LINT_UNCLOSED_FUNCTION);
  refuses_with(
    "color",
    "red;background:blue",
    token,
    LINT_UNCLOSED_FUNCTION,
  );
  refuses_with("color", "red {", token, LINT_UNCLOSED_FUNCTION);
  refuses_with("color", "red }", token, LINT_UNCLOSED_FUNCTION);
  refuses_with(
    "width",
    "calc(1px);height:2px",
    token,
    LINT_UNCLOSED_FUNCTION,
  );
  refuses_with("--x", "red;a", token, LINT_UNCLOSED_FUNCTION);

  assert!(
    refusal_of("width", "calc(1px);height:2px").contains("* { width: calc(1px);height:2px }"),
    "expected the rejection to quote the generated rule unchanged"
  );
}

/// `\;`, `\{` and `\}` are identifier text; the reference compiler emits all four. A bare `;` is
/// still refused (`still_refuses_a_value_whose_only_fault_is_the_token`).
#[test]
fn an_escaped_terminator_is_not_a_terminator() {
  check(
    &[
      unchanged("fontFamily", r"A\;B"),
      unchanged("fontFamily", r"a\{b"),
      unchanged("fontFamily", r"a\}b"),
      unchanged("color", r"red\;blue"),
    ],
    &default_options(),
  );
}

/// The reference compiler accepts `var(x)`, so the token is reported first.
#[test]
fn keeps_the_token_ahead_of_a_rejection_the_reference_compiler_does_not_make() {
  refuses_with("color", "var(x);a", LINT_RULE_BREAKING_TOKEN, "Unprefixed");
}

/// Nesting past the budget would overflow the stack, so that guard must run first; the unclosed
/// comment also still precedes the token.
#[test]
fn keeps_the_guards_that_run_before_parsing_ahead_of_the_token() {
  refuses_with(
    "color",
    "red;/* x",
    LINT_UNCLOSED_COMMENT,
    LINT_UNCLOSED_FUNCTION,
  );

  let mut deep = String::from("1px");
  for _ in 0..=MAX_VALUE_NESTING_DEPTH {
    deep = format!("calc({deep})");
  }
  refuses_with(
    "width",
    &format!("{deep};a"),
    "nested more deeply",
    LINT_RULE_BREAKING_TOKEN,
  );
}

/// `postcss_value_parser` reads an unterminated comment as a node, so the comment check defers;
/// for `calc(1px /*` the reference compiler names the function.
#[test]
fn hands_an_unclosed_comment_to_the_rejections_the_reference_makes_too() {
  refuses_with(
    "width",
    "calc(1px /*",
    LINT_UNCLOSED_FUNCTION,
    LINT_UNCLOSED_COMMENT,
  );

  refuses_with(
    "color",
    "red /* x",
    LINT_UNCLOSED_COMMENT,
    LINT_UNCLOSED_FUNCTION,
  );
}

#[test]
fn still_accepts_a_token_that_cannot_escape_its_declaration() {
  check(
    &[
      unchanged("content", "\";\""),
      unchanged("content", "\"{}\""),
      unchanged("color", "red /* ; */"),
      unchanged("backgroundImage", "url(data:image/svg+xml;base64,AA)"),
      unchanged("color", "red;;"),
    ],
    &default_options(),
  );
}

#[test]
fn orders_the_guards_the_same_way_around_unicode_and_escapes() {
  refuses_with(
    "fontFamily",
    "日本語;calc(1px",
    LINT_UNCLOSED_FUNCTION,
    LINT_RULE_BREAKING_TOKEN,
  );
  refuses_with(
    "fontFamily",
    r"\2014;calc(",
    LINT_UNCLOSED_FUNCTION,
    LINT_RULE_BREAKING_TOKEN,
  );
  refuses_with(
    "fontFamily",
    "🙂;sans-serif",
    LINT_RULE_BREAKING_TOKEN,
    LINT_UNCLOSED_FUNCTION,
  );
}

/// First-pass spellings of the arrangements `totality_test` finds do not settle; harness verdict
/// `identical`.
#[test]
fn spells_the_arrangements_that_do_not_settle_the_way_the_reference_does() {
  check(
    &[
      same("width", "()/", "() / "),
      same("width", "00)", "0)"),
      same("width", "00\\", "0\\"),
      same("width", "00*", "0*"),
    ],
    &default_options(),
  );
}

// ── Numbers at the edges of a double ─────────────────────────────────

/// The float parse and JS-shaped printer: tiny magnitudes and `-0` read back as zero and keep
/// their unit, a trailing `.` loses it, long fractions round to 17 significant digits. The zero
/// rule reads the authored spelling, not the quantity.
#[test]
fn spells_numbers_at_the_edges_of_a_double_the_way_the_reference_does() {
  check(
    &[
      unchanged("width", "1e309px"),
      same("width", "1e-324px", "0px"),
      unchanged("width", "1.7976931348623157e308px"),
      unchanged("width", "5e-324px"),
      unchanged("width", "9007199254740993px"),
      unchanged("width", "123456789012345678901234567890px"),
      unchanged("width", "1epx"),
      unchanged("width", "."),
      same("width", "0.px", "0"),
      same("width", "-0px", "0px"),
      same(
        "opacity",
        "0.12345678901234567890123456789",
        ".12345678901234568",
      ),
    ],
    &default_options(),
  );
}

// ── Multi-byte characters at a slicing boundary ──────────────────────

/// `totality_test` sweeps these positions for crashes; this pins the output.
#[test]
fn spells_multi_byte_characters_at_a_slicing_boundary_the_way_the_reference_does() {
  check(
    &[
      unchanged("fontFamily", "\"e\u{301}\""),
      unchanged("fontFamily", "e\u{301}"),
      unchanged("fontFamily", "\"a\u{200F}b\""),
      unchanged("fontFamily", "\"a\u{FEFF}b\""),
      unchanged("fontFamily", "a\u{00A0}b"),
      unchanged("fontFamily", "\\\u{1F600}"),
      unchanged("width", "1\u{1F600}px"),
    ],
    &default_options(),
  );
}

// ── Robustness ───────────────────────────────────────────────────────

fn nested_calc(depth: usize) -> String {
  let mut value = String::from("1px");
  for _ in 0..depth {
    value = format!("calc({value} + 1px)");
  }
  value
}

/// Generated, so no harness verdict; `edge-deeply-nested-calc` covers human-written depth.
#[test]
fn survives_function_nesting_up_to_the_limit() {
  let depth = MAX_VALUE_NESTING_DEPTH;

  let result = normalize_css_property_value("width", &nested_calc(depth), &default_options());

  assert_eq!(result.matches("calc(").count(), depth);
  assert!(result.starts_with("calc("));
}

/// Without the limit a deep value overflows the stack, which aborts with no diagnostic.
#[test]
fn rejects_function_nesting_past_the_limit() {
  let options = default_options();
  let value = nested_calc(MAX_VALUE_NESTING_DEPTH + 1);

  let result = catch_unwind(AssertUnwindSafe(|| {
    normalize_css_property_value("width", &value, &options)
  }));

  let message = panic_message(result);

  assert!(
    message.contains("nested more deeply") && message.contains("limit 64, found 65"),
    "expected the rejection to state both depths, got: {message}"
  );

  // The guard reads the value without parsing, so it holds at any depth.
  let far_past = nested_calc(5_000);
  let result = catch_unwind(AssertUnwindSafe(|| {
    normalize_css_property_value("width", &far_past, &options)
  }));

  assert!(
    panic_message(result).contains("limit 64, found 5000"),
    "expected the guard to hold without recursing"
  );
}

#[test]
fn rejects_deep_nesting_on_a_colour_function() {
  let options = default_options();
  let mut value = String::from("red");
  for _ in 0..=MAX_VALUE_NESTING_DEPTH {
    value = format!("oklch(from {value} l c h)");
  }

  let result = catch_unwind(AssertUnwindSafe(|| {
    normalize_css_property_value("color", &value, &options)
  }));

  assert!(
    panic_message(result).contains("nested more deeply"),
    "expected the colour-function path to reject deep nesting too"
  );
}

#[test]
fn normalizes_every_entry_of_a_long_list() {
  let entries = 500;
  let value = (1..=entries)
    .map(|n| format!("0px 0px {n}px #000"))
    .collect::<Vec<_>>()
    .join(", ");

  let result = normalize_css_property_value("boxShadow", &value, &default_options());

  assert_eq!(result.matches('#').count(), entries);
  assert!(result.starts_with("0 0 1px #000,"));
}
