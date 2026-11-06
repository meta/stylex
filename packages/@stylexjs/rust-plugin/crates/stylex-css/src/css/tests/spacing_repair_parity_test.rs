//! Spacing coverage at the public normalization entry point.
//!
//! Each case pins a previously reported whitespace defect; most assert that the value is
//! preserved as written. Expectations are reference-compiler spellings from the parity harness in
//! `crates/stylex-rs-compiler/parity`. Regenerate with:
//!
//! ```sh
//! pnpm run --filter=@stylexswc/rs-compiler build
//! pnpm run --filter=@stylexswc/rs-compiler parity -- --json parity/results/default.json
//! ```

use std::panic::{AssertUnwindSafe, catch_unwind};

use super::support::{check, default_options, panic_message, rejects, same, unchanged};
use crate::css::common::normalize_css_property_value;

/// Diagnostic for rejections; a `const` so the parity harness does not harvest it as a value.
const UNCLOSED_FUNCTION: &str = "unclosed function";

/// Issue #927. Units are listed rather than looped so each carries its own parity verdict.
#[test]
fn keeps_a_unit_glued_to_a_function_result() {
  check(
    &[
      // Absolute lengths
      unchanged("width", "var(--x)px"),
      unchanged("width", "var(--x)cm"),
      unchanged("width", "var(--x)mm"),
      unchanged("width", "var(--x)in"),
      unchanged("width", "var(--x)pt"),
      unchanged("width", "var(--x)pc"),
      unchanged("width", "var(--x)Q"),
      // Font-relative lengths
      unchanged("width", "var(--x)em"),
      unchanged("gap", "var(--gap)rem"),
      unchanged("width", "var(--x)ex"),
      unchanged("width", "var(--x)ch"),
      unchanged("width", "var(--x)lh"),
      unchanged("width", "var(--x)rlh"),
      unchanged("width", "var(--x)cap"),
      unchanged("width", "var(--x)ic"),
      // Viewport-relative lengths
      unchanged("width", "var(--x)vw"),
      unchanged("height", "var(--h)vh"),
      unchanged("width", "var(--x)vi"),
      unchanged("width", "var(--x)vb"),
      unchanged("width", "var(--x)vmin"),
      unchanged("width", "var(--x)vmax"),
      unchanged("width", "var(--x)dvw"),
      unchanged("width", "var(--x)dvh"),
      unchanged("width", "var(--x)lvw"),
      unchanged("width", "var(--x)lvh"),
      unchanged("width", "var(--x)svw"),
      unchanged("width", "var(--x)svh"),
      // Container-relative lengths
      unchanged("width", "var(--x)cqw"),
      unchanged("width", "var(--x)cqh"),
      unchanged("width", "var(--x)cqi"),
      unchanged("width", "var(--x)cqb"),
      unchanged("width", "var(--x)cqmin"),
      unchanged("width", "var(--x)cqmax"),
      // Time
      unchanged("transitionDuration", "var(--d)ms"),
      unchanged("transitionDuration", "var(--x)s"),
      // Angles
      unchanged("transform", "rotate(var(--a)deg)"),
      unchanged("transform", "rotate(var(--x)rad)"),
      unchanged("transform", "rotate(var(--x)grad)"),
      unchanged("transform", "rotate(var(--x)turn)"),
      // Resolution, flex, frequency
      unchanged("width", "var(--x)dpi"),
      unchanged("width", "var(--x)dpcm"),
      unchanged("width", "var(--x)dppx"),
      unchanged("gridTemplateColumns", "var(--x)fr"),
      unchanged("width", "var(--x)Hz"),
      unchanged("width", "var(--x)kHz"),
      // Any `)`, not only `var()`, can be followed by a unit.
      unchanged("width", "calc(1+2)em"),
    ],
    &default_options(),
  );
}

#[test]
fn keeps_a_percent_glued_to_a_function_result() {
  check(&[unchanged("width", "var(--x)%")], &default_options());
}

/// `Q`, `Hz` and `kHz` are units (glued above); `PX` and `EM` are not.
#[test]
fn matches_units_case_sensitively() {
  check(
    &[
      unchanged("width", "var(--x)PX"),
      unchanged("width", "var(--x)Px"),
      unchanged("width", "var(--x)EM"),
    ],
    &default_options(),
  );
}

#[test]
fn keeps_a_non_unit_word_flush_against_a_function_result() {
  check(
    &[
      unchanged("width", "var(--x)auto"),
      unchanged("width", "var(--x)calc"),
      unchanged("width", "var(--x)rgb"),
      unchanged("width", "var(--x)translate3d"),
      unchanged("width", "var(--x)solid"),
      unchanged("width", "var(--x)abc"),
      unchanged("width", "var(--x)hello"),
      unchanged("width", "var(--x)div"),
      unchanged("width", "var(--x)span"),
      unchanged("width", "var(--x)ABC"),
      // Function names, the words that look most like units.
      unchanged("width", "var(--x)var"),
      unchanged("width", "var(--x)rotate"),
      unchanged("width", "var(--x)p"),
      unchanged("width", "var(--x)a"),
    ],
    &default_options(),
  );
}

#[test]
fn keeps_a_non_ascii_word_flush_against_a_function_result() {
  check(&[unchanged("width", "var(--x)日本語")], &default_options());
}

#[test]
fn keeps_adjacent_function_calls_flush() {
  check(
    &[
      unchanged("transform", "rotate(10deg)translate3d(0,0,0)"),
      unchanged("width", "calc(1px)calc(2px)calc(3px)"),
      unchanged("color", "rgb(0,0,0)rgba(255,255,255,.5)"),
      unchanged("color", "var(--a)var(--b)var(--c)"),
    ],
    &default_options(),
  );
}

/// Only the `/` pair moves: both compilers space that operator.
#[test]
fn keeps_what_follows_a_closing_paren_flush() {
  check(
    &[
      unchanged("width", ")3"),
      unchanged("width", "calc(a)42"),
      unchanged("color", ")#fff"),
      unchanged("width", ")*3"),
      unchanged("color", ")A"),
      same("width", ")/7", ") / 7"),
      unchanged("width", ")Q"),
      // `-` is a sign or subtraction, never spaced.
      unchanged("width", ")-1"),
      unchanged("width", ")"),
    ],
    &default_options(),
  );
}

/// `)(` opens a function that is never closed; the reference compiler rejects these too.
#[test]
fn rejects_an_unclosed_group_after_a_closing_paren() {
  rejects(
    "color",
    &[")(", ")a )Z )0 )# )("],
    UNCLOSED_FUNCTION,
    &default_options(),
  );
}

/// The old pass could only insert spaces; nothing removes or restores them now.
#[test]
fn does_not_restore_a_space_swallowed_before_a_closing_paren() {
  check(
    &[same("width", ")/1 )*.5", ") / 1 )*.5")],
    &default_options(),
  );
}

#[test]
fn keeps_a_hex_colour_flush_against_the_token_before_it() {
  check(
    &[
      unchanged("boxShadow", "1px#000"),
      unchanged("color", "solid#abc"),
      unchanged("backgroundPosition", "50%#fff"),
      unchanged("color", "A#fff"),
      unchanged("color", "9#fff"),
      unchanged("color", "%#fff"),
      unchanged("color", "日本語#fff"),
      unchanged("color", "a#fff Z#fff 1#fff %#fff"),
      unchanged("color", "#"),
    ],
    &default_options(),
  );
}

/// `40%.1147` is two components, kept as written.
#[test]
fn keeps_a_number_flush_against_a_percentage() {
  check(
    &[
      unchanged("color", "40%.1147"),
      unchanged("backgroundPosition", "50%10"),
      unchanged("backgroundPosition", "100%.5"),
      unchanged("backgroundPosition", "50%10 40%.5"),
    ],
    &default_options(),
  );
}

#[test]
fn leaves_a_number_after_a_percentage_alone_inside_a_colour_function() {
  check(
    &[unchanged("color", "oklab(40.101%.1147 .0453)")],
    &default_options(),
  );
}

/// Both compilers space `/` on both sides, including at the start of a value (` / 7`).
#[test]
fn spaces_the_slash_operator() {
  check(
    &[
      same("width", "size/2", "size / 2"),
      same("width", "/ 7", " / 7"),
      same("width", "/.5", " / .5"),
      same("width", "/-1", " / -1"),
      same("width", "/-2", " / -2"),
      same(
        "width",
        "calc-size(fit-content,size/2)",
        "calc-size(fit-content,size / 2)",
      ),
    ],
    &default_options(),
  );
}

/// Unlike `/`, the reference compiler leaves `*` as written.
#[test]
fn spaces_the_multiplication_operator() {
  check(
    &[unchanged("width", "* 3"), unchanged("width", "*(100%)")],
    &default_options(),
  );
}

#[test]
fn rejects_an_unclosed_operand_after_a_multiplication_operator() {
  rejects(
    "width",
    &["*(", "/.5 /-1 *( *3"],
    UNCLOSED_FUNCTION,
    &default_options(),
  );
}

/// `""` must not be split into `" "`.
#[test]
fn keeps_adjacent_quoted_strings_flush() {
  check(
    &[
      unchanged("gridTemplateAreas", r#""content""sidebar""#),
      unchanged("quotes", r#""a""b""#),
      unchanged("quotes", r#""""""#),
      // Three in a row: a scan that only looks one quote back misses the second pair.
      unchanged("quotes", r#""""""""#),
      unchanged("quotes", r#""""#),
    ],
    &default_options(),
  );
}

#[test]
fn keeps_adjacent_single_quoted_strings_as_written() {
  check(
    &[
      unchanged("quotes", r#"'a''b'"#),
      unchanged("quotes", r#""a"'b'"#),
    ],
    &default_options(),
  );
}

/// The space used to be dropped outright, changing the class name.
#[test]
fn keeps_the_space_around_a_separated_string() {
  check(
    &[
      unchanged("quotes", r#""a" x "b""#),
      unchanged("fontFamily", r#""a" 1px"#),
    ],
    &default_options(),
  );
}

#[test]
fn applies_no_spacing_rule_inside_a_string() {
  check(
    &[
      unchanged("fontFamily", r#""hello)world""#),
      unchanged("fontFamily", r##""a)b#c%d.5/1*2(""##),
      // `\\` is an escaped backslash, so the string closes here.
      unchanged("fontFamily", r#""a\\b""#),
    ],
    &default_options(),
  );
}

#[test]
fn keeps_the_quote_character_around_inert_contents() {
  check(
    &[unchanged("fontFamily", r##"'a)b#c%d.5/1*2('"##)],
    &default_options(),
  );
}

/// The old minifying round trip un-quoted these (`"a\"b#c"` → `a"b#c`), letting `}` escape.
#[test]
fn keeps_an_escaped_quote_inside_a_string() {
  check(
    &[
      unchanged("fontFamily", r##""a\"b#c""##),
      unchanged("fontFamily", r##"'a\'b#c'"##),
    ],
    &default_options(),
  );
}

/// Which bodies are taken whole is pinned by [`steps_over_only_the_bodies_the_parser_takes_whole`].
#[test]
fn copies_url_bodies_verbatim() {
  check(
    &[
      unchanged("backgroundImage", "url(image.png)"),
      unchanged("backgroundImage", "url(http://example.com/img.png)"),
      unchanged(
        "backgroundImage",
        "url(https://fonts.googleapis.com/css2?family=Roboto)",
      ),
      unchanged("backgroundImage", r#"url("a/*b.png")"#),
      // `;` and `/` would end the value if read as CSS. Line continuations, not `concat!` or a
      // `const`, keep this one literal for the parity harness.
      unchanged("backgroundImage", "url(data:image/svg+xml;utf8,<svg/>)"),
      unchanged(
        "backgroundImage",
        "url(\"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAYAAAAf8/9hAAAAMUlEQVQ4T2NkYG\
          AQYcAP3uCTZhw1gGGYhAGBZIA/nYDCgBDAm9BGDWAAJyRCgLaBCAAgXwixzAS0pgAAAABJRU5ErkJggg==\")",
      ),
      // A `url()` partway through the value, not at offset zero.
      unchanged(
        "backgroundImage",
        "linear-gradient(red,blue),url(http://example.com/a.png)",
      ),
    ],
    &default_options(),
  );
}

#[test]
fn balances_nested_parens_inside_a_url_body() {
  check(
    &[
      unchanged("backgroundImage", "url(a(b).png)"),
      unchanged("backgroundImage", "url(a(b(c)).png) 10px"),
    ],
    &default_options(),
  );
}

/// An earlier fast path skipped the rest of the declaration after a `url()`.
#[test]
fn keeps_the_value_after_a_url_as_written() {
  check(
    &[
      unchanged("backgroundImage", "url(a(b).png)calc(1px)"),
      unchanged("backgroundImage", "url(a.png)calc(1px)"),
      unchanged(
        "backgroundImage",
        r#"url("/icons/•#hash.svg")calc(1px)rgb(0,0,0)"#,
      ),
    ],
    &default_options(),
  );
}

#[test]
fn does_not_close_a_url_on_a_quoted_paren() {
  check(
    &[unchanged("backgroundImage", r#"url("a)b.png") 10px"#)],
    &default_options(),
  );
}

#[test]
fn spaces_a_slash_separator_after_a_url() {
  check(
    &[
      same(
        "backgroundImage",
        "url(a.png) no-repeat center/cover",
        "url(a.png) no-repeat center / cover",
      ),
      same(
        "backgroundImage",
        r#"url("asset.png") no-repeat center/cover"#,
        r#"url("asset.png") no-repeat center / cover"#,
      ),
    ],
    &default_options(),
  );
}

#[test]
fn keeps_the_quote_character_inside_a_url_body() {
  check(
    &[unchanged("backgroundImage", r#"url('a)b.png')calc(1px)"#)],
    &default_options(),
  );
}

#[test]
fn keeps_an_escaped_quote_inside_a_url_body() {
  check(
    &[unchanged("backgroundImage", r#"url("a\")b.png") 10px"#)],
    &default_options(),
  );
}

/// The structural guard steps over the body whole, as a browser does; harness verdict `identical`.
#[test]
fn keeps_a_url_body_that_reads_as_an_unclosed_construct() {
  check(
    &[
      unchanged("backgroundImage", "url(it's-fine.png)"),
      unchanged("backgroundImage", "url(a/*b.png)"),
    ],
    &default_options(),
  );
}

/// The lookbehind walks characters, not bytes: byte slicing before `(` used to panic.
#[test]
fn does_not_treat_a_url_suffixed_identifier_as_a_url() {
  check(
    &[
      same("width", "éurl(foo/2)", "éurl(foo / 2)"),
      same("width", "日本(1/2)", "日本(1 / 2)"),
      // Fewer than three characters before `(`: the old byte lookbehind went out of bounds.
      unchanged("width", "éab(1)"),
      unchanged("width", "é(1)"),
    ],
    &default_options(),
  );
}

/// Matches the reference compiler, including `/*/ x */` (the old path produced `/**/ x * /`).
#[test]
fn keeps_comments_in_the_value() {
  check(
    &[
      unchanged("width", "/* a */1px"),
      same("width", "/*/ x */ 1px", "/**/ x * / 1px"),
      same("width", "1px /*/ y */", "1px /**/ y * / "),
      // A `}` inside a comment is comment text, so this is not rejected.
      unchanged("width", "/* a }b */ 1px"),
    ],
    &default_options(),
  );
}

/// It used to normalize to nothing, dropping the declaration.
#[test]
fn keeps_a_value_that_is_only_a_comment() {
  check(
    &[unchanged("width", "/* a */"), unchanged("width", "/**/")],
    &default_options(),
  );
}

#[test]
fn leaves_an_already_spaced_value_alone() {
  check(
    &[
      unchanged("boxShadow", "1px solid red"),
      unchanged("boxShadow", "1px solid #000"),
      unchanged("width", "calc(100% - 20px)"),
      unchanged("width", "calc(100% - 2px)"),
      unchanged("color", "var(--color) var(--bg)"),
      unchanged("color", "var(--my-color)"),
      unchanged("margin", "10px 20px"),
      unchanged("width", "10px"),
      unchanged("color", "red"),
      unchanged("color", "a"),
    ],
    &default_options(),
  );
}

/// The reference compiler also rejects these (`both reject`); asserted on the diagnostic.
#[test]
fn rejects_a_value_with_nothing_to_normalize() {
  rejects(
    "color",
    &["", "   "],
    "nothing to normalize",
    &default_options(),
  );
}

/// Bare and quoted, across UTF-8 lengths up to four bytes.
#[test]
fn preserves_non_ascii_content() {
  check(
    &[
      unchanged("color", "•"),
      unchanged("fontFamily", "✓"),
      unchanged("fontFamily", "日本語"),
      unchanged("fontFamily", "привет"),
      unchanged("fontFamily", "שלום"),
      unchanged("fontFamily", "مرحبا"),
      unchanged("fontFamily", "😀"),
      unchanged("fontFamily", "🎉"),
      unchanged("fontFamily", r#""•""#),
      unchanged("fontFamily", r#""•✓日本語😀""#),
    ],
    &default_options(),
  );
}

#[test]
fn keeps_the_quote_character_around_non_ascii_content() {
  check(
    &[unchanged("fontFamily", r#"'•✓日本語😀'"#)],
    &default_options(),
  );
}

/// The author's `-.5px` stays `-.5px`, as in the reference compiler.
#[test]
fn keeps_the_leading_zero_as_the_author_wrote_it() {
  check(
    &[
      unchanged("marginTop", "-.24px"),
      unchanged("transitionDuration", "-.9s"),
      unchanged("opacity", "-.5"),
      unchanged("width", "calc(-.5px + 1px)"),
      // Inside a function argument list, and after a multi-byte character.
      unchanged("transform", "translate(-.5px,-.25px)"),
      unchanged("transform", "translate(🎉,-.5px)"),
      unchanged("margin", "-5px -.5px"),
    ],
    &default_options(),
  );
}

/// A `-` after a complete token (dimension or `)`) is subtraction; both compilers agree.
#[test]
fn leaves_subtraction_operators_alone() {
  check(
    &[
      unchanged("opacity", ".5px"),
      unchanged("width", "calc(1px-.5px)"),
      unchanged("width", "calc(var(--x)-.5px)"),
    ],
    &default_options(),
  );
}

/// It used to come back as `10%-0.5px`.
#[test]
fn keeps_a_negative_decimal_after_a_percentage() {
  check(&[unchanged("width", "10%-.5px")], &default_options());
}

#[test]
fn keeps_the_restoration_out_of_strings() {
  check(
    &[
      unchanged("fontFamily", r#""-.5""#),
      unchanged("fontFamily", r#""🎉 -.5""#),
      unchanged("transform", r#"translate(-.5px,"-.25px")"#),
    ],
    &default_options(),
  );
}

#[test]
fn keeps_the_quote_character_around_a_protected_decimal() {
  check(&[unchanged("fontFamily", r#"'-.5'"#)], &default_options());
}

#[test]
fn keeps_an_escaped_quote_around_a_protected_decimal() {
  check(
    &[unchanged("fontFamily", r#""a\"-.5""#)],
    &default_options(),
  );
}

/// Not a rule-terminator error: a `;` or `{` inside an unterminated body is url text, so each
/// input gets one diagnostic regardless of check order.
#[test]
fn reports_an_unterminated_url_body_as_an_unclosed_function() {
  rejects(
    "backgroundImage",
    &[
      "url(data:image/png;base64,AAA",
      "url(a{b",
      "url(a}b",
      "url(",
    ],
    UNCLOSED_FUNCTION,
    &default_options(),
  );
}

/// The parser reads a url body as one word, so its parens do not count toward the depth guard.
/// Generated, so no harness verdict; mainly asserts no stack overflow.
#[test]
fn counts_nesting_inside_a_url_body_as_the_parser_does() {
  let mut body = String::from("a");
  for _ in 0..500 {
    body = format!("({body})");
  }

  let value = format!("url(x{body})");

  // Not a `Case`: the value is built at runtime and a `Case` holds `&'static str`.
  assert_eq!(
    normalize_css_property_value("backgroundImage", &value, &default_options()),
    value
  );
}

/// The structural guard's notion of `url()` must match the parser's, or a `}` in the body escapes
/// the rule. The parser compares the name literally, so `URL(` is an ordinary function.
#[test]
fn steps_over_only_the_bodies_the_parser_takes_whole() {
  let options = default_options();

  let protected = [
    "url(a}b)",
    "url(a{b)",
    "url(a;b)",
    "url(a/*b)",
    // The escaped `)` does not close the body.
    r"url(a\)}b)",
  ];
  let unprotected = [
    // The parser compares the name literally.
    "URL(a}b)",
    "Url(a}b)",
    "blurl(a}b)",
    "noturl(a}b)",
    r"\url(a}b)",
    // `\\` is an escaped backslash, so this `)` closes the body.
    r"url(a\\)}b)",
  ];

  for value in protected {
    assert_eq!(
      normalize_css_property_value("backgroundImage", value, &options),
      value,
      "expected the parser's own `url()` body to survive whole"
    );
  }

  for value in unprotected {
    let result = catch_unwind(AssertUnwindSafe(|| {
      normalize_css_property_value("backgroundImage", value, &options)
    }));

    let message = panic_message(result);

    assert!(
      message.contains("outside of a string or comment"),
      "expected `{value}` to be rejected — the parser does not take that body \
       whole, so its `}}` would reach the declaration; got: {message}"
    );
  }
}
