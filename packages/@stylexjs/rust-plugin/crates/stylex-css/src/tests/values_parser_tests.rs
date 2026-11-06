//! Shorthand value splitting: where a value is cut and how each part is spelled.
//!
//! Expectations are from `@stylexjs/babel-plugin@0.19.0`, via
//! `crates/stylex-rs-compiler/parity/fuzz-shorthand-split.ts`.

use super::split_value_parts;
use crate::css::common::MAX_VALUE_NESTING_DEPTH;

fn parts(value: &str) -> Vec<String> {
  split_value_parts(value)
}

// ── Arity: how many parts a value has ──

#[test]
fn whitespace_separates_parts() {
  assert_eq!(parts("10px"), ["10px"]);
  assert_eq!(parts("10px 20px 30px"), ["10px", "20px", "30px"]);
  assert_eq!(parts("1rem 2em 3vh 4vw"), ["1rem", "2em", "3vh", "4vw"]);
  assert_eq!(
    parts("1px 2px 3px 4px 5px"),
    ["1px", "2px", "3px", "4px", "5px"]
  );
}

#[test]
fn any_run_of_whitespace_separates_one_pair_of_parts() {
  // A part never carries the whitespace that ended it.
  assert_eq!(parts("1px 2px"), ["1px", "2px"]);
  assert_eq!(parts("1px  2px"), ["1px", "2px"]);
  assert_eq!(parts("1px\n2px"), ["1px", "2px"]);
  assert_eq!(parts("1px\t2px"), ["1px", "2px"]);
}

#[test]
fn a_value_with_nothing_in_it_has_no_parts() {
  assert!(parts("").is_empty());
  assert!(parts("   ").is_empty());
}

#[test]
fn adjacent_functions_are_separate_parts_without_a_separator() {
  assert_eq!(parts("var(--x)var(--y)"), ["var(--x)", "var(--y)"]);
  assert_eq!(
    parts("translateX(1.50px) rotate(1E2deg)"),
    ["translateX(1.50px)", "rotate(1E2deg)"]
  );
}

// ── Separators end a part, and contribute nothing to it ──

#[test]
fn a_top_level_separator_ends_a_part_and_is_not_one() {
  // These used to emit the separator itself as a part.
  assert_eq!(parts("10px/1.5"), ["10px", "1.5"]);
  assert_eq!(parts("10px / 1.5"), ["10px", "1.5"]);
  assert_eq!(parts("a:b"), ["a", "b"]);
  assert_eq!(parts("a : b"), ["a", "b"]);
  assert_eq!(parts("a,b"), ["a", "b"]);
  assert_eq!(parts("a , b"), ["a", "b"]);
  assert_eq!(parts("a, b, c"), ["a", "b", "c"]);
}

#[test]
fn a_semicolon_is_not_a_separator() {
  // `;` is a word character to the scanner; the terminator guard refuses it later.
  assert_eq!(parts("1px;2px"), ["1px;2px"]);
  assert_eq!(
    parts("color: red; margin: 0"),
    ["color", "red;", "margin", "0"]
  );
}

#[test]
fn an_operator_does_not_end_a_part() {
  // The scanner does not evaluate arithmetic; the run is one word.
  assert_eq!(parts("1px*2"), ["1px*2"]);
  assert_eq!(parts("1px+2"), ["1px+2"]);
  assert_eq!(parts("1px-2"), ["1px-2"]);
  assert_eq!(parts("1px--2"), ["1px--2"]);
}

// ── Inside a function, a separator is a character ──

#[test]
fn a_slash_inside_a_function_is_not_a_separator() {
  // Cut on node kinds: the same `/` is a separator at top level and arithmetic here.
  assert_eq!(parts("calc(100% / 3)"), ["calc(100% / 3)"]);
  assert_eq!(parts("calc(100%/3)"), ["calc(100%/3)"]);
}

#[test]
fn authored_spacing_inside_a_function_is_echoed() {
  // The author's bytes; re-spacing by rule used to change the class name.
  assert_eq!(parts("calc(1.50px*2)"), ["calc(1.50px*2)"]);
  assert_eq!(parts("calc(1px + 2px)"), ["calc(1px + 2px)"]);
  assert_eq!(parts("calc(1px  +  2px)"), ["calc(1px  +  2px)"]);
  assert_eq!(parts("calc(2px-1px)"), ["calc(2px-1px)"]);
  assert_eq!(parts("calc(100% - 20px)"), ["calc(100% - 20px)"]);
}

#[test]
fn a_comma_inside_a_function_loses_the_space_around_it() {
  // A comma inside a function is a separator node that prints only the comma, as upstream.
  assert_eq!(parts("min(1px , 2px)"), ["min(1px,2px)"]);
  assert_eq!(parts("min(1px,2px)"), ["min(1px,2px)"]);
  assert_eq!(parts("rgb(255, 0, 0)"), ["rgb(255,0,0)"]);
  assert_eq!(parts("var(--x, 1px)"), ["var(--x,1px)"]);
  assert_eq!(
    parts("translate(1.2345678901234567px, 2px)"),
    ["translate(1.2345678901234567px,2px)"]
  );
}

#[test]
fn padding_just_inside_a_function_is_dropped() {
  // A function node carries its argument-list end whitespace and does not print it.
  assert_eq!(parts("calc( 1px + 2px )"), ["calc(1px + 2px)"]);
  assert_eq!(parts("url(  a.png  )"), ["url(a.png)"]);
}

#[test]
fn a_nested_function_is_still_one_part() {
  assert_eq!(parts("calc(var(--x) + 1px)"), ["calc(var(--x) + 1px)"]);
  assert_eq!(parts("foo(())"), ["foo(())"]);
  assert_eq!(parts("foo()"), ["foo()"]);
}

#[test]
fn an_unquoted_url_is_a_part_like_any_other() {
  // The old token walk panicked on an unquoted url; upstream compiles it.
  assert_eq!(parts("url(a.png)"), ["url(a.png)"]);
  assert_eq!(parts("url('a.png')"), ["url('a.png')"]);
}

// ── The importance annotation ──

#[test]
fn a_trailing_importance_annotation_moves_onto_every_part() {
  // Left as its own part it used to become `padding-inline-end: important`.
  assert_eq!(parts("1px !important"), ["1px !important"]);
  assert_eq!(
    parts("1px 2px !important"),
    ["1px !important", "2px !important"]
  );
  assert_eq!(parts("red !important"), ["red !important"]);
}

#[test]
fn the_annotation_is_recognised_whatever_its_case() {
  assert_eq!(
    parts("1px 2px !IMPORTANT"),
    ["1px !important", "2px !important"]
  );
  assert_eq!(
    parts("1px 2px !Important"),
    ["1px !important", "2px !important"]
  );
}

#[test]
fn an_annotation_with_nothing_to_qualify_stays_as_it_is() {
  assert_eq!(parts("!important"), ["!important"]);
}

#[test]
fn an_annotation_touching_its_value_is_already_one_part() {
  assert_eq!(parts("1px!important"), ["1px!important"]);
}

#[test]
fn a_space_after_the_bang_is_not_an_annotation() {
  // Two words; upstream emits the same three parts.
  assert_eq!(parts("1px ! important"), ["1px", "!", "important"]);
  assert_eq!(parts("!"), ["!"]);
}

// ── Echoed characters: escapes, quotes, digits, hex ──

#[test]
fn an_escape_stays_an_escape() {
  // Upstream keeps the backslash and reads two parts.
  assert_eq!(parts(r#"A\42 C"#), [r#"A\42"#, "C"]);
  assert_eq!(parts(r#"\31 23"#), [r#"\31"#, "23"]);
  assert_eq!(parts(r#"a\ b"#), [r#"a\ b"#]);
}

#[test]
fn an_escaped_unit_stays_escaped() {
  // Ticket 11: the unit used to be printed unescaped (`1px`).
  assert_eq!(parts(r#"1\70x"#), [r#"1\70x"#]);
  assert_eq!(parts(r#"1.50\70x"#), [r#"1.50\70x"#]);
}

#[test]
fn a_trailing_backslash_escapes_nothing_and_is_kept() {
  assert_eq!(parts(r#"\"#), [r#"\"#]);
  assert_eq!(parts(r#"a\"#), [r#"a\"#]);
}

#[test]
fn a_string_keeps_the_quote_character_it_was_written_with() {
  assert_eq!(parts(r#""a""#), [r#""a""#]);
  assert_eq!(parts("'a'"), ["'a'"]);
  assert_eq!(parts(r#""a" 'b'"#), [r#""a""#, "'b'"]);
  assert_eq!(parts(r#""a'b""#), [r#""a'b""#]);
  assert_eq!(parts(r#""""#), [r#""""#]);
  assert_eq!(parts("''"), ["''"]);
}

#[test]
fn a_hex_colour_keeps_its_spelling() {
  // The old token walk emitted `#\30 07bff`.
  assert_eq!(parts("#007bff"), ["#007bff"]);
  assert_eq!(parts("#fff"), ["#fff"]);
  assert_eq!(parts("#007bff #fff"), ["#007bff", "#fff"]);
  assert_eq!(parts("#-invalid"), ["#-invalid"]);
  assert_eq!(parts("#"), ["#"]);
}

#[test]
fn a_number_keeps_the_digits_it_was_typed_with() {
  assert_eq!(parts("1.50px"), ["1.50px"]);
  assert_eq!(parts("1E2px"), ["1E2px"]);
  assert_eq!(parts("1e2px"), ["1e2px"]);
  assert_eq!(parts("1e21px"), ["1e21px"]);
  assert_eq!(parts("1e+21px"), ["1e+21px"]);
  assert_eq!(parts("000.5px"), ["000.5px"]);
  assert_eq!(parts("-0px"), ["-0px"]);
  assert_eq!(parts("-0"), ["-0"]);
  assert_eq!(parts("+1px"), ["+1px"]);
  assert_eq!(parts("+50%"), ["+50%"]);
}

#[test]
fn a_number_at_the_edges_of_double_precision_is_still_only_echoed() {
  // Not read as numbers, so precision never applies.
  assert_eq!(parts("1e400px"), ["1e400px"]);
  assert_eq!(parts("5e-324px"), ["5e-324px"]);
  assert_eq!(
    parts("1.7976931348623157e308px"),
    ["1.7976931348623157e308px"]
  );
  assert_eq!(parts("33.333333333333336%"), ["33.333333333333336%"]);
  assert_eq!(parts("1.2345678901234567px"), ["1.2345678901234567px"]);
}

#[test]
fn a_number_that_is_not_one_is_a_word_like_any_other() {
  assert_eq!(parts("1.px"), ["1.px"]);
  assert_eq!(parts("1e"), ["1e"]);
  assert_eq!(parts("1e+"), ["1e+"]);
  assert_eq!(parts("1epx"), ["1epx"]);
}

// ── Malformed input is scanned, never rejected ──

#[test]
fn an_unclosed_function_is_closed_in_the_output() {
  // The scanner supplies the missing paren; refusal is normalization's decision.
  assert_eq!(parts("calc(1px"), ["calc(1px)"]);
  assert_eq!(parts("min("), ["min()"]);
}

#[test]
fn an_unclosed_string_is_closed_in_the_output() {
  assert_eq!(parts(r#""1.50px"#), [r#""1.50px""#]);
  assert_eq!(parts("'unterminated"), ["'unterminated'"]);
}

#[test]
fn an_unbalanced_bracket_or_paren_is_a_word() {
  // Only `(` opens a function; brackets are ordinary characters.
  assert_eq!(parts("[a]"), ["[a]"]);
  assert_eq!(parts("[header-start]"), ["[header-start]"]);
  assert_eq!(parts("[foo"), ["[foo"]);
  assert_eq!(parts(")"), [")"]);
  assert_eq!(parts("]"), ["]"]);
  assert_eq!(parts("}"), ["}"]);
  assert_eq!(parts("(1 + 2)"), ["(1 + 2)"]);
  assert_eq!(parts("(a)"), ["(a)"]);
  assert_eq!(parts("foo(bar)[baz]{qux}"), ["foo(bar)", "[baz]{qux}"]);
}

#[test]
fn a_curly_block_is_cut_by_the_colon_inside_it() {
  assert_eq!(parts("{color: red}"), ["{color", "red}"]);
}

#[test]
fn a_comment_contributes_the_text_between_its_delimiters() {
  // A comment node prints its inner text, the part upstream hands a longhand.
  assert_eq!(parts("/* hello */"), [" hello "]);
  assert_eq!(parts("/*c*/"), ["c"]);
  assert_eq!(
    parts("color /* comment */ red"),
    ["color", " comment ", "red"]
  );
  assert_eq!(parts("a/*b*/c"), ["a", "b", "c"]);
}

#[test]
fn an_unterminated_comment_contributes_an_empty_part() {
  // `/*/` does not round-trip: the terminator is found inside the opener.
  assert_eq!(parts("/*"), [""]);
  assert_eq!(parts("/*/"), [""]);
}

#[test]
fn an_operator_pair_from_selector_syntax_is_a_word() {
  assert_eq!(parts("~="), ["~="]);
  assert_eq!(parts("|="), ["|="]);
  assert_eq!(parts("^="), ["^="]);
  assert_eq!(parts("$="), ["$="]);
  assert_eq!(parts("*="), ["*="]);
  assert_eq!(parts("<!--"), ["<!--"]);
  assert_eq!(parts("-->"), ["-->"]);
  assert_eq!(parts("*"), ["*"]);
  assert_eq!(parts("~"), ["~"]);
}

#[test]
fn an_at_keyword_is_a_word() {
  assert_eq!(parts("@media"), ["@media"]);
  assert_eq!(parts("@charset"), ["@charset"]);
  assert_eq!(parts("@font-face"), ["@font-face"]);
  assert_eq!(parts("@import"), ["@import"]);
  assert_eq!(parts("@media screen"), ["@media", "screen"]);
}

#[test]
fn a_unicode_range_is_one_part() {
  // A unicode range is its own token, so `+` is not a signed number.
  assert_eq!(parts("U+0-7F"), ["U+0-7F"]);
  assert_eq!(parts("u+26"), ["u+26"]);
}

// ── Non-ASCII input ──

#[test]
fn a_non_ascii_word_survives_byte_for_byte() {
  assert_eq!(parts("wörld"), ["wörld"]);
  assert_eq!(parts("😀px"), ["😀px"]);
  assert_eq!(
    parts(r#""héllo — wörld" 1.5px"#),
    [r#""héllo — wörld""#, "1.5px"]
  );
}

#[test]
fn the_trim_is_javascripts_and_not_rusts() {
  // JS `trim` removes a BOM but not U+0085, the reverse of `str::trim`; whichever survives would
  // end up in a word.
  assert_eq!(parts("\u{feff}1px"), ["1px"]);
  assert_eq!(parts("\u{85}1px"), ["\u{85}1px"]);
  assert_eq!(parts(" 1px"), ["1px"]);
  assert_eq!(parts("\n1px"), ["1px"]);
  assert_eq!(parts("1px\n"), ["1px"]);
}

// ── Depth ──

#[test]
fn a_value_nested_past_the_budget_is_handed_on_whole() {
  // Dropping the deep tree would recurse and abort, so the split is declined; normalization
  // rejects it with the nesting-depth diagnostic.
  let depth = MAX_VALUE_NESTING_DEPTH + 1;
  let nested = format!("{}{}", "calc(".repeat(depth), ")".repeat(depth));

  assert_eq!(parts(&nested), std::slice::from_ref(&nested));
}

#[test]
fn a_value_nested_far_past_the_budget_still_returns_rather_than_aborting() {
  // Regression for a stack-overflow abort, which `#[should_panic]` cannot express.
  let depth = 200_000;
  let nested = "calc(".repeat(depth);

  assert_eq!(parts(&nested), std::slice::from_ref(&nested));
}

#[test]
fn a_value_nested_up_to_the_budget_is_still_split() {
  // The last accepted depth must still be split.
  let nested = format!(
    "{}{}",
    "calc(".repeat(MAX_VALUE_NESTING_DEPTH),
    ")".repeat(MAX_VALUE_NESTING_DEPTH)
  );

  let result = parts(&format!("1px {nested}"));

  assert_eq!(result.len(), 2);
  assert_eq!(result[0], "1px");
  assert_eq!(result[1], nested);
}

// ── Values a longhand keeps whole ──

#[test]
fn a_keyword_is_a_single_part() {
  assert_eq!(parts("red"), ["red"]);
  assert_eq!(parts("auto"), ["auto"]);
  assert_eq!(parts("inherit"), ["inherit"]);
  assert_eq!(parts("42"), ["42"]);
  assert_eq!(parts("0.5"), ["0.5"]);
  assert_eq!(parts("0"), ["0"]);
  assert_eq!(parts("999999"), ["999999"]);
  assert_eq!(parts("-10px"), ["-10px"]);
  assert_eq!(parts("-3em"), ["-3em"]);
  assert_eq!(parts("50%"), ["50%"]);
  assert_eq!(parts("0%"), ["0%"]);
  assert_eq!(parts("1em"), ["1em"]);
  assert_eq!(parts("100vh"), ["100vh"]);
  assert_eq!(parts("2rem"), ["2rem"]);
  assert_eq!(parts("var(--x)"), ["var(--x)"]);
}

// ── An empty part is a part ──
// No reference answer: Babel throws `Cannot read properties of undefined (reading 'type')`
// on these, so this pins our rule from `src/values/parser.rs`.

#[test]
fn a_terminated_comment_with_nothing_in_it_contributes_an_empty_part() {
  // A closed comment: later parts survive and the empty one need not be last.
  assert_eq!(parts("/**/"), [""]);
  assert_eq!(parts("1px /**/"), ["1px", ""]);
  assert_eq!(parts("/**/ 1px"), ["", "1px"]);
  assert_eq!(parts("1px /**/ 2px"), ["1px", "", "2px"]);
}

#[test]
fn an_empty_part_holds_its_position_rather_than_collapsing() {
  // Empty parts keep their indices, so sides do not shift.
  assert_eq!(parts("/**/ 1px /**/ 2px /**/"), ["", "1px", "", "2px", ""]);
  assert_eq!(parts("/**//**/"), ["", ""]);
  assert_eq!(parts("/**/ /**/ /**/").len(), 3);
}

#[test]
fn an_unterminated_comment_swallows_the_rest_and_is_still_one_empty_part() {
  // An unterminated comment can only be the last part.
  assert_eq!(parts("1px /*"), ["1px", ""]);
  assert_eq!(parts("1px /* 2px"), ["1px", " 2px"]);
  assert_eq!(parts("/* 1px 2px 3px"), [" 1px 2px 3px"]);
}

#[test]
fn a_trailing_importance_annotation_lands_on_an_empty_part_too() {
  // Matches the reference: normalization spells it `!important`.
  assert_eq!(
    parts("1px /**/ !important"),
    ["1px !important", " !important"]
  );
  assert_eq!(
    parts("/**/ /**/ !important"),
    [" !important", " !important"]
  );
}

#[test]
fn an_importance_annotation_alone_after_an_empty_part_still_qualifies_it() {
  // The fold checks the part count, not whether the target part has text.
  assert_eq!(parts("/**/ !important"), [" !important"]);
  assert_eq!(parts("!important"), ["!important"]);
}

#[test]
fn an_empty_part_is_not_reached_by_a_quoted_empty_string() {
  // An empty string is two authored characters, not an empty part.
  assert_eq!(parts(r#""""#), [r#""""#]);
  assert_eq!(parts("''"), ["''"]);
  assert_eq!(parts(r#"1px """#), ["1px", r#""""#]);
}

#[test]
fn a_separator_standing_alone_is_a_part_and_not_an_empty_one() {
  assert_eq!(parts("1px ;"), ["1px", ";"]);
  assert_eq!(parts("; 1px"), [";", "1px"]);
  assert_eq!(parts("1px;2px"), ["1px;2px"]);
}

// ── The reference compiler's own cases ──

/// The reference compiler's `splitValue` suite, ported verbatim and in order.
///
/// Source: `shared/utils/__tests__/split-css-value-test.js`.
#[test]
fn the_reference_compilers_own_split_cases() {
  // simple space-separated numbers
  assert_eq!(parts("0 1 2 3"), ["0", "1", "2", "3"]);
  // simple space-separated lengths
  assert_eq!(parts("0px 1rem 2% 3em"), ["0px", "1rem", "2%", "3em"]);
  // simple comma-separated numbers
  assert_eq!(parts("0, 1, 2, 3"), ["0", "1", "2", "3"]);
  // simple comma-separated lengths
  assert_eq!(parts("0px, 1rem, 2%, 3em"), ["0px", "1rem", "2%", "3em"]);

  // "Does not lists within functions"
  assert_eq!(parts("rgb(255 200 0)"), ["rgb(255 200 0)"]);
  assert_eq!(parts("rgb(255 200 / 0.5)"), ["rgb(255 200/0.5)"]);

  // "Does not lists within calc": the second is an anonymous parenthesised group.
  assert_eq!(
    parts("calc((100% - 50px) * 0.5)"),
    ["calc((100% - 50px) * 0.5)"]
  );
  assert_eq!(
    parts("calc((100% - 50px) * 0.5) var(--rightpadding, 20px)"),
    ["calc((100% - 50px) * 0.5)", "var(--rightpadding,20px)"]
  );
}
