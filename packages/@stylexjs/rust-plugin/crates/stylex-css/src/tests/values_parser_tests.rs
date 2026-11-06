//! Where a shorthand value is cut, and what each part is spelled with.
//!
//! Source: crates/stylex-css/src/values/parser.rs
//!
//! Every expectation in this file was read off `@stylexjs/babel-plugin@0.19.0`
//! rather than reasoned about. The two compilers hash a class name from the
//! declaration text a part becomes, so a part spelled differently here is a
//! different class name there — which makes a guessed expectation worse than no
//! expectation at all. The fuzz that produced the list is
//! `crates/stylex-rs-compiler/parity/fuzz-shorthand-split.ts`.

use super::split_value_parts;
use crate::css::common::MAX_VALUE_NESTING_DEPTH;

fn parts(value: &str) -> Vec<String> {
  split_value_parts(value)
}

// ── Arity: how many parts a value has ───────────────────────────────

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
  // The run's length and its characters are structure, not content: a part
  // never carries the whitespace that ended it, so all four of these are the
  // same two parts.
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

// ── Separators end a part, and contribute nothing to it ─────────────

#[test]
fn a_top_level_separator_ends_a_part_and_is_not_one() {
  // The defect this closes: each of these used to emit the separator itself as
  // a part, so a shorthand expanded into a declaration whose value was `/` or
  // `:` surrounded by spaces. A browser drops such a declaration, which lost
  // the shorthand half its sides.
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
  // `;` is an ordinary word character to the scanner, so it stays attached to
  // whatever it touches. A value carrying one is refused later, by the guard in
  // front of normalization that stops a declaration terminating its own rule --
  // this function has no opinion about it.
  assert_eq!(parts("1px;2px"), ["1px;2px"]);
  assert_eq!(
    parts("color: red; margin: 0"),
    ["color", "red;", "margin", "0"]
  );
}

#[test]
fn an_operator_does_not_end_a_part() {
  // These read as arithmetic but the scanner does not evaluate anything: the
  // whole run is one word, and cutting it would tear a value upstream keeps
  // whole.
  assert_eq!(parts("1px*2"), ["1px*2"]);
  assert_eq!(parts("1px+2"), ["1px+2"]);
  assert_eq!(parts("1px-2"), ["1px-2"]);
  assert_eq!(parts("1px--2"), ["1px--2"]);
}

// ── Inside a function, a separator is a character ───────────────────

#[test]
fn a_slash_inside_a_function_is_not_a_separator() {
  // The reason the cut is made on node kinds rather than on characters: the
  // same `/` ends a part at the top level and is arithmetic here, and no
  // character test can tell the two apart.
  assert_eq!(parts("calc(100% / 3)"), ["calc(100% / 3)"]);
  assert_eq!(parts("calc(100%/3)"), ["calc(100%/3)"]);
}

#[test]
fn authored_spacing_inside_a_function_is_echoed() {
  // Each of these is one part whose text is the author's own bytes. Re-spacing
  // by rule diverged in both directions at once -- it inserted spaces around an
  // unspaced operator and removed them from around a spaced slash -- and each
  // was a different class name.
  assert_eq!(parts("calc(1.50px*2)"), ["calc(1.50px*2)"]);
  assert_eq!(parts("calc(1px + 2px)"), ["calc(1px + 2px)"]);
  assert_eq!(parts("calc(1px  +  2px)"), ["calc(1px  +  2px)"]);
  assert_eq!(parts("calc(2px-1px)"), ["calc(2px-1px)"]);
  assert_eq!(parts("calc(100% - 20px)"), ["calc(100% - 20px)"]);
}

#[test]
fn a_comma_inside_a_function_loses_the_space_around_it() {
  // The one place spacing is *not* echoed, and it is not an exception invented
  // here: a comma inside a function is a separator node, which carries the
  // whitespace on either side of it rather than letting that whitespace stand
  // as its own node, and printing the node prints only the comma. Upstream
  // reaches the same text the same way.
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
  // A function node carries the whitespace at each end of its argument list on
  // itself, and printing the node does not print it back.
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
  // It used to abort the compiler: the token walk this replaced panicked on an
  // unquoted url rather than emitting it. Upstream compiles it.
  assert_eq!(parts("url(a.png)"), ["url(a.png)"]);
  assert_eq!(parts("url('a.png')"), ["url('a.png')"]);
}

// ── The importance annotation ───────────────────────────────────────

#[test]
fn a_trailing_importance_annotation_moves_onto_every_part() {
  // Written once and meant of the whole shorthand. Left as a part of its own it
  // became the value of the next longhand in line, which is how
  // `padding: '1px !important'` came to emit `padding-inline-end: important`.
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
  // Nothing to move it onto. Dropping it instead would discard what the author
  // wrote, and there is no part it could join.
  assert_eq!(parts("!important"), ["!important"]);
}

#[test]
fn an_annotation_touching_its_value_is_already_one_part() {
  assert_eq!(parts("1px!important"), ["1px!important"]);
}

#[test]
fn a_space_after_the_bang_is_not_an_annotation() {
  // `!` and `important` are two words here, so neither the fold nor anything
  // else joins them. Upstream reads it the same way and emits the same three
  // parts -- this is not a value either compiler makes sense of.
  assert_eq!(parts("1px ! important"), ["1px", "!", "important"]);
  assert_eq!(parts("!"), ["!"]);
}

// ── Echoed characters: escapes, quotes, digits, hex ─────────────────

#[test]
fn an_escape_stays_an_escape() {
  // Resolving these was a double divergence: `A\42 C` came back as the single
  // part `ABC` -- the escape resolved *and* the space it consumed swallowed --
  // where upstream reads two parts and keeps the backslash.
  assert_eq!(parts(r#"A\42 C"#), [r#"A\42"#, "C"]);
  assert_eq!(parts(r#"\31 23"#), [r#"\31"#, "23"]);
  assert_eq!(parts(r#"a\ b"#), [r#"a\ b"#]);
}

#[test]
fn an_escaped_unit_stays_escaped() {
  // The remainder ticket 11 left open. The unit used to be read off the token
  // and printed as what it escapes to, so `1\70x` became `1px`.
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
  // The token walk this replaced read `#007bff` as an identifier and serialized
  // the leading digit as an escape, emitting `#\30 07bff` -- not the colour the
  // author wrote, and not a colour at all.
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
  // Nothing here is read as a number, so the questions precision would raise
  // never come up: a value too large for a double, one too small to be
  // distinguished from zero, and seventeen significant digits all come back
  // unchanged.
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

// ── Malformed input is scanned, never rejected ──────────────────────

#[test]
fn an_unclosed_function_is_closed_in_the_output() {
  // The scanner records that the parenthesis was missing and spells one anyway.
  // Whether the value is then refused is normalization's decision, not this
  // function's -- there is no input here that produces no answer.
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
  // Only a parenthesis opens a function. A square or curly bracket is an
  // ordinary character, so a bracketed run is one word and a stray closer is a
  // word of its own.
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
  // Nothing here treats `{ ... }` as a block, so the `:` is a top-level
  // separator and the braces stay attached to the words beside it.
  assert_eq!(parts("{color: red}"), ["{color", "red}"]);
}

#[test]
fn a_comment_contributes_the_text_between_its_delimiters() {
  // Not the comment, and not nothing: a comment node's value is its inner text,
  // and printing it prints that. It is a strange part to hand a longhand, and
  // it is the part upstream hands one.
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
  // `/*/` is the one input the scanner does not round-trip: it looks for the
  // terminator from the opening slash and finds it inside the `/*/` itself.
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
  // Recognised as its own token shape so that the `+` in it is not read as a
  // word followed by a signed number.
  assert_eq!(parts("U+0-7F"), ["U+0-7F"]);
  assert_eq!(parts("u+26"), ["u+26"]);
}

// ── Non-ASCII input ────────────────────────────────────────────────

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
  // Two characters separate the two trims, and both are visible here. A
  // byte-order mark is not Unicode whitespace but JavaScript trims it, so
  // `str::trim` would leave it welded to the first part; `U+0085` is Unicode
  // whitespace and JavaScript does not trim it, so `str::trim` would remove one
  // upstream keeps. Neither is whitespace to the scanner, so whichever survives
  // ends up inside a word -- and inside a class name.
  assert_eq!(parts("\u{feff}1px"), ["1px"]);
  assert_eq!(parts("\u{85}1px"), ["\u{85}1px"]);
  assert_eq!(parts(" 1px"), ["1px"]);
  assert_eq!(parts("\n1px"), ["1px"]);
  assert_eq!(parts("1px\n"), ["1px"]);
}

// ── Depth ──────────────────────────────────────────────────────────

#[test]
fn a_value_nested_past_the_budget_is_handed_on_whole() {
  // Not split, and not refused here. Scanning this builds a tree whose
  // destructor recurses once per level and aborts the process, so the split is
  // declined; normalization then rejects the value with the nesting-depth
  // diagnostic, which is where that message is documented to come from.
  let depth = MAX_VALUE_NESTING_DEPTH + 1;
  let nested = format!("{}{}", "calc(".repeat(depth), ")".repeat(depth));

  assert_eq!(parts(&nested), std::slice::from_ref(&nested));
}

#[test]
fn a_value_nested_far_past_the_budget_still_returns_rather_than_aborting() {
  // The shape that used to kill the process. It is here as a regression test
  // for the abort, which no `#[should_panic]` can express: a stack overflow is
  // not a panic, so a test that reached it would take the whole run down.
  let depth = 200_000;
  let nested = "calc(".repeat(depth);

  assert_eq!(parts(&nested), std::slice::from_ref(&nested));
}

#[test]
fn a_value_nested_up_to_the_budget_is_still_split() {
  // The guard declines at one past the budget, so the last accepted depth has
  // to still be split -- otherwise it would be a refusal dressed as a split.
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

// ── Values a longhand keeps whole ──────────────────────────────────

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

// ── An empty part is a part ─────────────────────────────────────────
//
// The rule is stated in the module documentation of
// `crates/stylex-css/src/values/parser.rs`; these pin it. There is no reference
// answer for any of them: the reference compiler throws
// `Cannot read properties of undefined (reading 'type')` on a value whose parts
// include an empty one, so the shape is this compiler's decision and these are
// assertions about that decision rather than transcriptions of upstream's.

#[test]
fn a_terminated_comment_with_nothing_in_it_contributes_an_empty_part() {
  // The other way to reach an empty part, and a different scan from the
  // unterminated one: the comment closes, so the parts after it survive and the
  // empty one is not necessarily last.
  assert_eq!(parts("/**/"), [""]);
  assert_eq!(parts("1px /**/"), ["1px", ""]);
  assert_eq!(parts("/**/ 1px"), ["", "1px"]);
  assert_eq!(parts("1px /**/ 2px"), ["1px", "", "2px"]);
}

#[test]
fn an_empty_part_holds_its_position_rather_than_collapsing() {
  // The whole of the rule, as arity: five parts stay five, and the two empty
  // ones are at the indices the author put them at. A splitter that dropped
  // them would hand the third side's value to the second side.
  assert_eq!(parts("/**/ 1px /**/ 2px /**/"), ["", "1px", "", "2px", ""]);
  assert_eq!(parts("/**//**/"), ["", ""]);
  assert_eq!(parts("/**/ /**/ /**/").len(), 3);
}

#[test]
fn an_unterminated_comment_swallows_the_rest_and_is_still_one_empty_part() {
  // The asymmetry between the two ways to reach an empty part: this one can
  // only ever be the last, because everything after the opener is inside it.
  assert_eq!(parts("1px /*"), ["1px", ""]);
  assert_eq!(parts("1px /* 2px"), ["1px", " 2px"]);
  assert_eq!(parts("/* 1px 2px 3px"), [" 1px 2px 3px"]);
}

#[test]
fn a_trailing_importance_annotation_lands_on_an_empty_part_too() {
  // An empty part is qualified like any other, which reads oddly and is what
  // the reference compiler does: normalization spells the result `!important`,
  // and the class name hashed from it agrees.
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
  // Two parts, so the fold runs: the guard that leaves a lone annotation alone
  // asks about the part count and not about whether the part it would move onto
  // has any text.
  assert_eq!(parts("/**/ !important"), [" !important"]);
  // One part, so there is nothing to qualify and the annotation stays as it was.
  assert_eq!(parts("!important"), ["!important"]);
}

#[test]
fn an_empty_part_is_not_reached_by_a_quoted_empty_string() {
  // The near miss worth pinning: a value that *looks* empty is not. An empty
  // string is two characters the author wrote and a part carries both.
  assert_eq!(parts(r#""""#), [r#""""#]);
  assert_eq!(parts("''"), ["''"]);
  assert_eq!(parts(r#"1px """#), ["1px", r#""""#]);
}

#[test]
fn a_separator_standing_alone_is_a_part_and_not_an_empty_one() {
  // A `;` between two fragments ends the part before it; a `;` with whitespace
  // on both sides is a part of its own, spelled with the character it was
  // written as.
  assert_eq!(parts("1px ;"), ["1px", ";"]);
  assert_eq!(parts("; 1px"), [";", "1px"]);
  assert_eq!(parts("1px;2px"), ["1px;2px"]);
}

// ── The reference compiler's own cases ──────────────────────────────

/// Every case from the reference compiler's `splitValue` suite, in its order.
///
/// Ported verbatim rather than paraphrased. The cases above were written to
/// exercise this compiler's own decisions, which means they were chosen by
/// someone who already knew where the seams are; these were chosen by whoever
/// wrote the function, and two of them reach shapes nothing above does — a
/// `div` inside a function, and an anonymous parenthesised group. That is
/// exactly the node-kind logic this module cuts on, so they are the cases most
/// worth having.
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

  // "Does not lists within functions" -- a function is one part however many
  // separators sit inside it, and the `/` in the second is the `div` node kind
  // that ends a part at the top level and does not here.
  assert_eq!(parts("rgb(255 200 0)"), ["rgb(255 200 0)"]);
  assert_eq!(parts("rgb(255 200 / 0.5)"), ["rgb(255 200/0.5)"]);

  // "Does not lists within calc" -- the second reaches an anonymous
  // parenthesised group, which is a node kind of its own rather than a function.
  assert_eq!(
    parts("calc((100% - 50px) * 0.5)"),
    ["calc((100% - 50px) * 0.5)"]
  );
  assert_eq!(
    parts("calc((100% - 50px) * 0.5) var(--rightpadding, 20px)"),
    ["calc((100% - 50px) * 0.5)", "var(--rightpadding,20px)"]
  );
}
