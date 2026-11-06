//! Value-normalization coverage asserted at the public entry point.
//!
//! Every expectation here is a spelling the parity harness in
//! `crates/stylex-rs-compiler/parity` confirms `@stylexjs/babel-plugin`
//! produces — never judgement. Regenerate with the two runs below; the second
//! is what the cases under
//! [`rem_enabled_options`](super::support::rem_enabled_options) come from,
//! since the harness defaults the font-size option off and a default run cannot
//! verdict them at all:
//!
//! ```sh
//! pnpm run --filter=@stylexswc/rs-compiler build
//! pnpm run --filter=@stylexswc/rs-compiler parity -- --json parity/results/default.json
//! pnpm run --filter=@stylexswc/rs-compiler parity -- --font-size-px-to-rem \
//!   --json parity/results/font-size-px-to-rem.json
//! ```
//!
//! The `Case` machinery and the shared `check` runner live in `support`,
//! alongside the same pieces used by `spacing_repair_parity_test`.
//!
//! Read `entries[].babel.declarations` for the reference spelling and
//! `entries[].rust.declarations` for this compiler's. Overlap with the older
//! `normalize_css_property_value_tests` module in `common_test.rs` is
//! deliberate: those assertions predate the harness, so they say what this
//! compiler does without saying whether anything else agrees.

use std::panic::{AssertUnwindSafe, catch_unwind};

use stylex_constants::constants::messages::{
  LINT_RULE_BREAKING_TOKEN, LINT_UNCLOSED_COMMENT, LINT_UNCLOSED_FUNCTION, LINT_UNCLOSED_STRING,
};

use super::support::{check, default_options, panic_message, rem_enabled_options, same, unchanged};
use crate::css::common::{MAX_VALUE_NESTING_DEPTH, normalize_css_property_value};

// ── Timings ──────────────────────────────────────────────────────────

/// Milliseconds shorten to seconds, except below the threshold where the
/// shorter spelling would not actually be shorter.
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

/// Under ten milliseconds the conversion is skipped.
#[test]
fn keeps_milliseconds_below_the_conversion_threshold() {
  check(
    &[unchanged("transitionDuration", "5ms")],
    &default_options(),
  );
}

// ── Zero dimensions ──────────────────────────────────────────────────

/// A zero length loses its unit; a zero fraction, percentage or angle keeps
/// one, because dropping it would change what the value means.
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

/// A zero inside a function argument is left as the author wrote it — the
/// surrounding expression, not the property, decides what the unit means.
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

/// A zero angle inside a function does get its unit rewritten. The window the
/// zero-dimension normalizer opens for function arguments is what decides this,
/// and it is the reference compiler's window, quirks included.
#[test]
fn rewrites_a_zero_angle_inside_a_transform_function() {
  check(
    &[same("transform", "rotate(0rad)", "rotate(0deg)")],
    &default_options(),
  );
}

// ── Custom properties ────────────────────────────────────────────────

/// A custom property's value has no CSS grammar, so none of the value
/// normalizations apply to it.
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

/// The quote character the author chose is the quote character that reaches the
/// hash. No normalizer understands quote characters, so none of them can swap
/// one for the other.
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

/// The same holds where a value is *made* of quoted strings.
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

/// Only the properties whose values name other properties get their values
/// hyphenated.
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

/// References the author wrote flush against each other stay flush. Nothing
/// inserts a separator that the author did not write.
#[test]
fn keeps_flush_references_flush() {
  check(
    &[unchanged("color", "var(--a)var(--b)var(--c)")],
    &default_options(),
  );
}

// ── Colour and math functions ────────────────────────────────────────

/// A colour function takes the same path every other value takes, so its
/// arguments lose their leading zeros like any other number. There is no
/// list of function names anywhere that routes them around anything.
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

/// The same goes for spacing: a space after a comma is removed inside a colour
/// or math function exactly as it is anywhere else.
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

/// A value already spelled the way the normalizers would spell it is left
/// alone, which is what makes a second pass over it a no-op.
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

/// The conversion applies only to `fontSize`, and only to pixel lengths.
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

/// A converted value below one rem keeps its leading zero: the conversion runs
/// last, after the leading-zero normalizer has already had its turn, so the
/// number it produces is never revisited.
#[test]
fn keeps_the_leading_zero_on_a_sub_rem_font_size() {
  check(&[same("fontSize", "8px", "0.5rem")], &rem_enabled_options());
}

#[test]
fn leaves_font_size_alone_when_the_option_is_off() {
  check(&[unchanged("fontSize", "16px")], &default_options());
}

// ── Vendor prefixes ──────────────────────────────────────────────────

/// A vendor prefix is an ordinary part of a keyword or function name and is
/// carried through untouched — including when camel-case conversion has to
/// produce one.
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

/// A hex colour inside a vendor-prefixed function keeps all six digits and its
/// letter case, like any other hex colour.
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

/// An escape outside a string keeps its source spelling.
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

/// An escape inside a string keeps the author's spelling rather than being
/// resolved to the character it names. It used to be resolved, which put
/// different bytes into the hash than the reference compiler used.
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

/// Non-ASCII text that needs no escape survives byte for byte, inside a string
/// and as a bare identifier.
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

/// A value SWC cannot parse is emitted verbatim as long as it cannot break out
/// of the rule being generated — which is why an unclosed bracket and a
/// nonsense operator sequence are both accepted rather than rejected.
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

/// A semicolon at the end of a value opens nothing, so it survives into the
/// declaration exactly as written — however many of them there are, and
/// whatever whitespace trails them. The reference compiler emits the same
/// bytes, so the harness verdict for all three is `identical`.
///
/// Stray trailing semicolons are common in hand-written style objects — the
/// project's own large fixture carries more than twenty — and none of them can
/// splice a second declaration into the stylesheet.
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

/// A semicolon with a declaration behind it is the shape the guard exists for:
/// emitted verbatim, `color: 'red; margin: 10px'` would close its own
/// declaration and add one the author never asked this rule for.
///
/// The harness verdict is `acceptance divergent` — the reference compiler emits
/// both declarations. Before the pipeline swap this compiler emitted only the
/// first and dropped the rest with no diagnostic, which is the behaviour this
/// replaces.
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

/// An opening brace could open a block inside the generated stylesheet, so it
/// is rejected here. The harness verdict is `acceptance divergent`: the
/// reference compiler emits `color:red {`. This is a local guard rather than a
/// spelling difference, so it is asserted on the message rather than through
/// the case table — a bare "it panicked" would pass on any panic at all,
/// including one from a future bug elsewhere in the pipeline.
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

/// The complaint `property: value` is refused with.
///
/// Every case below refuses, and what each one asserts is *which* sentence came
/// back — so a helper that only caught the panic would pass on all of them
/// whatever the order became.
fn refusal_of(property: &str, value: &str) -> String {
  let options = default_options();
  let result = catch_unwind(AssertUnwindSafe(|| {
    normalize_css_property_value(property, value, &options)
  }));

  panic_message(result)
}

/// Asserts that `property: value` is refused with a message containing
/// `expected` and not one containing `instead_of`.
///
/// Both halves matter here: the point of the ordering is that one true
/// complaint is handed over rather than another, and an assertion that only
/// checked the winner would pass on a message carrying both.
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

/// A value that is both rule-breaking and unclosed is refused for being
/// unclosed, because that is the complaint the reference compiler writes.
///
/// The build stops either way and no emitted value depends on which sentence
/// fires, so the whole of what this buys is the sentence an author whose build
/// was refused reads: the same one from both compilers. The harness verdict for
/// all of these moves from `both reject (diverged)` to `both reject`.
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

/// A value whose *only* fault is the token still reads the token complaint,
/// which is the half of the reorder that must not have been traded away.
///
/// Every `;`, `{` and `}` below is bare. An escaped one is not a fault at all
/// and is accepted — `an_escaped_terminator_is_not_a_terminator` holds those.
///
/// The rule text the message quotes is asserted too: it is built from the raw
/// value before anything parses it, and moving where the guard fires must not
/// have moved what it quotes.
#[test]
fn still_refuses_a_value_whose_only_fault_is_the_token() {
  // Spelled out rather than looped over a table of pairs: the parity corpus is
  // harvested from these sources by recognizing a property literal next to a
  // value literal, and a pair inside a table is a shape the scan does not read
  // — so a table here would keep these values out of the corpus that measures
  // them.
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

/// An escaped terminator is not a terminator, and is not refused.
///
/// The guard exists because emitting a value verbatim must not close the rule
/// being generated and splice arbitrary CSS after it. A backslash in front of
/// the character is the one reading of those bytes where that cannot happen:
/// `\;`, `\{` and `\}` are part of the identifier they sit in, so the browser
/// reads them as text and the rule stays open exactly as long as it should.
/// The guard's own sentence says "outside of a string or comment", and an
/// escape is neither — the exclusion was simply missing from the scan, where
/// the quote arms beside it already had it.
///
/// Measured against the reference compiler rather than reasoned about: it emits
/// `font-family:A\;B`, `font-family:a\{b`, `font-family:a\}b` and
/// `color:red\;blue` for these four, each on its own class. Refusing them here
/// failed four programs that compile there.
///
/// This is *not* a claim that the guard is too strict in general. The reference
/// compiler emits a **bare** `;` too — `font-family:A;B` really does come back
/// from it, spliced rule and all — and this compiler still refuses that, which
/// `still_refuses_a_value_whose_only_fault_is_the_token` holds. The divergence
/// closed here is only over the characters that cannot break anything.
#[test]
fn an_escaped_terminator_is_not_a_terminator() {
  // Raw strings: the backslash is the whole of what these cases are about, and
  // a `"A\\;B"` would leave a reader counting them.
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

/// The token still outranks the unprefixed custom property, which is this
/// compiler's own rejection and has no second opinion to agree with.
///
/// The reference compiler accepts `var(x)`, so nothing is gained by letting it
/// speak first and a curated row that reads `declaration-terminating token`
/// would stop reading it.
#[test]
fn keeps_the_token_ahead_of_a_rejection_the_reference_compiler_does_not_make() {
  refuses_with("color", "var(x);a", LINT_RULE_BREAKING_TOKEN, "Unprefixed");
}

/// The two guards that read the raw bytes still speak before the token, and
/// they have to: both exist because the value cannot safely be parsed at all.
///
/// A value nested past the recursion budget takes the process down without a
/// diagnostic rather than panicking, so that one cannot wait for a pass that runs
/// on a parsed value. The unclosed comment *can*, and now does — see
/// `hands_an_unclosed_comment_to_the_rejections_the_reference_makes_too`. It is
/// still ahead of the token, which is what the first case below says.
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

/// An unclosed comment waits for the two rejections the reference compiler also
/// makes, the same way the token does.
///
/// It used to speak first, on the grounds that it "runs before parsing". That is
/// true of the nesting guard, whose alternative is an abort — but not of this
/// one: `postcss_value_parser` reads an unterminated comment as a comment node
/// carrying `unclosed: true`, which is what
/// `an_unterminated_comment_contributes_an_empty_part` relies on. So it had no
/// more claim to preempt them than the token did.
///
/// The cost of speaking first was a value refused for the wrong fault.
/// `calc(1px /*` carries an unclosed function *and* an unclosed comment; the
/// reference compiler names the function, and this compiler named the comment.
/// Same accept-or-refuse decision, different sentence — which is the whole thing
/// the reorder that moved the token was for.
#[test]
fn hands_an_unclosed_comment_to_the_rejections_the_reference_makes_too() {
  refuses_with(
    "width",
    "calc(1px /*",
    LINT_UNCLOSED_FUNCTION,
    LINT_UNCLOSED_COMMENT,
  );

  // A comment with no unclosed function or string beside it still reads the
  // comment: the rejections it now defers to simply have nothing to say.
  refuses_with(
    "color",
    "red /* x",
    LINT_UNCLOSED_COMMENT,
    LINT_UNCLOSED_FUNCTION,
  );
}

/// A token inside a string or a comment was never a fault, and moving the guard
/// did not make it one — the scan that answers the question is the same one.
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

/// Non-ASCII and escaped text around the two faults changes neither which
/// complaint fires nor the bytes it quotes back.
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

/// The four arrangements the totality sweep found that do not settle after a
/// second normalization pass, pinned at the spelling of the *first* pass —
/// which is the only one a declaration ever gets.
///
/// The sweep can say that these move; it cannot say whether where they move
/// from is right. That is this table's job, and the harness answers it:
/// `identical` for all four, so the reference compiler carries the same
/// trailing characters and the same acquired space. Their non-settling is
/// pinned separately in `totality_test`.
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

/// Two passes re-spell a number through a float parse and a JavaScript-shaped
/// printer, so the ends of a double's range are where the two compilers would
/// part company silently — a wrong digit in a class name looks like nothing at
/// all.
///
/// Every spelling below is the harness's, and the interesting ones are the
/// four where the value is *not* returned as written: a magnitude too small
/// for a double reads back as a zero and keeps its unit, a negative zero reads
/// back as a positive one and keeps its unit too, a trailing decimal point
/// reads as a zero and loses it, and a fraction longer than a double's
/// precision comes back rounded to seventeen significant digits. A magnitude
/// too *large* is left alone, because nothing in the passes has an opinion
/// about it.
///
/// Note what separates the two zeros that keep their unit from the one that
/// loses it: the rule reads the authored spelling, not the quantity, so only a
/// value that *reads* as zero has its unit dropped.
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

/// The passes index by byte, so a multi-byte character sitting where one of
/// them cuts is where a boundary panic would live. `totality_test` sweeps those
/// positions for crashes; this pins what comes out of the ones that matter.
///
/// A combining mark is a code point of its own and must not be separated from
/// the character it modifies. A right-to-left mark and a byte-order mark are
/// invisible, so a pass that dropped one would change the hash with nothing to
/// show for it in the rendered CSS. A non-breaking space is a word character
/// here rather than a separator, so it must not collapse the way a space does.
/// An astral-plane code point is four bytes, straight after an escape and
/// between a number and its unit.
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

/// Builds a value nesting `calc()` `depth` levels deep.
fn nested_calc(depth: usize) -> String {
  let mut value = String::from("1px");
  for _ in 0..depth {
    value = format!("calc({value} + 1px)");
  }
  value
}

/// Nesting far past anything a person writes by hand must not lose a level.
/// Generated, so the parity harness carries no verdict for it; the checked-in
/// `edge-deeply-nested-calc` case covers the depth a human would write.
#[test]
fn survives_function_nesting_up_to_the_limit() {
  let depth = MAX_VALUE_NESTING_DEPTH;

  let result = normalize_css_property_value("width", &nested_calc(depth), &default_options());

  assert_eq!(result.matches("calc(").count(), depth);
  assert!(result.starts_with("calc("));
}

/// One level past the limit is rejected with a diagnostic naming both depths.
///
/// The limit exists because every stage of the pipeline recurses once per
/// nesting level: without it a deep enough value exhausts the stack, and a
/// stack overflow aborts the process rather than panicking, so nothing catches
/// it and no diagnostic is produced. Rejecting early is what keeps the failure
/// reportable.
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

  // The guard reads the value rather than parsing it, so it holds at any depth
  // — including ones that used to take the process down with them.
  let far_past = nested_calc(5_000);
  let result = catch_unwind(AssertUnwindSafe(|| {
    normalize_css_property_value("width", &far_past, &options)
  }));

  assert!(
    panic_message(result).contains("limit 64, found 5000"),
    "expected the guard to hold without recursing"
  );
}

/// The limit is a property of the compiler, not of the value. There is one path
/// left for a value to take, so a colour function is guarded exactly as a
/// `calc()` is — which is what this pins now that the branch it was written
/// against is gone.
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

/// A long comma-separated list is normalized entry by entry, with no truncation
/// at any list length.
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
