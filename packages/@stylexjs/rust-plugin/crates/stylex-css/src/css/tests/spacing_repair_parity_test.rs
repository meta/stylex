//! Spacing coverage asserted at the public normalization entry point.
//!
//! The whitespace repair pass and the helpers around it are where years of
//! individually reported defects accumulated: a function result immediately
//! followed by a unit, `url()` bodies containing characters that look like CSS
//! syntax, comments inside values, adjacent quoted strings, non-ASCII content,
//! a percentage followed by a number, and the leading zero on a negative
//! decimal. Each assertion below exists because somebody hit the bug. They were
//! written against the repair pass itself, so they were re-expressed one level
//! up — at the entry point that survived the pipeline replacement, and that the
//! pass no longer sits behind.
//!
//! **What the cases say, read as a whole.** The repair pass existed to undo
//! spacing damage done by a minifying serializer. Nothing damages the spacing
//! any more: a value is parsed losslessly, a fixed list of normalizers rewrites
//! the tokens they name, and everything else is spelled back out as the author
//! wrote it. So most of what this module now asserts is that a space is
//! *preserved* where the repair pass used to be reinserting it after the fact,
//! and the class name that follows from it is the one the reference compiler
//! produces.
//!
//! Every expectation is a spelling the parity harness in
//! `crates/stylex-rs-compiler/parity` confirms the reference compiler produces
//! — never judgement. Regenerate with:
//!
//! ```sh
//! pnpm run --filter=@stylexswc/rs-compiler build
//! pnpm run --filter=@stylexswc/rs-compiler parity -- --json parity/results/default.json
//! ```
//!
//! Read `entries[].babel.declarations` for the reference spelling and
//! `entries[].rust.declarations` for this compiler's. The `Case` machinery and
//! the shared `check` runner live in `support`. Most cases here use
//! [`unchanged`](super::support::unchanged), which is the case whose
//! expectation is its own input — the majority shape, since most of what this
//! module asserts is that a value is not rewritten.
//!
//! Rejections are asserted outside the case table, because a `Case` compares a
//! spelling and a rejection has none.

use std::panic::{AssertUnwindSafe, catch_unwind};

use super::support::{check, default_options, panic_message, rejects, same, unchanged};
use crate::css::common::normalize_css_property_value;

/// The diagnostic every rejection below is asserted on.
///
/// Kept out of the calls that use it: the parity harness harvests CSS values
/// out of literal tables in these sources, and a second string literal sitting
/// next to a property name is exactly the shape it reads as one.
const UNCLOSED_FUNCTION: &str = "unclosed function";

// ── A unit after a function result (issue #927) ──────────────────────

/// The oldest defect in this family: a dimension whose number came from a
/// function reads as `)` immediately followed by a unit, and inserting a space
/// there would turn `var(--x)px` into two tokens and lose the dimension.
///
/// Every unit the compiler knows is listed rather than looped, so each one
/// carries its own harness verdict, and all of them agree with the reference
/// compiler. Nothing inserts a space here now — but the unit list is still
/// consulted, by the ported whitespace normalizer, so the table still has
/// something to say.
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
      // The rule is about function results, not about `var()`: any `)` can be
      // followed by a unit.
      unchanged("width", "calc(1+2)em"),
    ],
    &default_options(),
  );
}

/// `%` is not in the unit list — it is its own token — but nothing separates it
/// from the `)` before it either, so a percentage built from a function survives
/// the same way a dimension does.
#[test]
fn keeps_a_percent_glued_to_a_function_result() {
  check(&[unchanged("width", "var(--x)%")], &default_options());
}

/// Unit matching is case-sensitive, because CSS spells two of its units with
/// capitals and the rest without. `PX` and `EM` are not units and get
/// separated like any other word; the capitalised units that *are* — `Q`, `Hz`
/// and `kHz` — are asserted glued by
/// [`keeps_a_unit_glued_to_a_function_result`] above, which is where the whole
/// unit list lives. The two halves together are what pin case-sensitivity:
/// lowercasing before the match would keep `PX` glued and fail here, while
/// rejecting every capital would separate `Q` and fail there.
///
/// All three now come back exactly as written, which is what the reference
/// compiler has always done. The case-sensitivity still has to be right: the
/// unit list is what the ported whitespace normalizer consults, and it consults
/// it case-sensitively.
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

/// A word written flush against a function result stays flush, whether or not
/// it is a unit. The repair pass used to have to guess which it was, because
/// the space had already been lost by the time it looked; no space is lost now,
/// so there is nothing to guess.
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
      // Two function names, which look most like units of anything here and
      // are the two the unit list was originally pinned against.
      unchanged("width", "var(--x)var"),
      unchanged("width", "var(--x)rotate"),
      // A single letter is a word too, and neither `p` nor `a` is a unit.
      unchanged("width", "var(--x)p"),
      unchanged("width", "var(--x)a"),
    ],
    &default_options(),
  );
}

/// A non-ASCII word cannot be a unit, so the unit list is never consulted for
/// it — which is what used to keep the lookahead from slicing a multi-byte
/// character in half. It stays flush like every other word.
#[test]
fn keeps_a_non_ascii_word_flush_against_a_function_result() {
  check(&[unchanged("width", "var(--x)日本語")], &default_options());
}

// ── Adjacent function calls ──────────────────────────────────────────

/// Two function calls written flush against each other stay flush. The minifier
/// used to drop a space here that was never there, and the repair pass put one
/// back; neither happens now, which is what the reference compiler has always
/// done.
#[test]
fn keeps_adjacent_function_calls_flush() {
  check(
    &[
      unchanged("transform", "rotate(10deg)translate3d(0,0,0)"),
      unchanged("width", "calc(1px)calc(2px)calc(3px)"),
      unchanged("color", "rgb(0,0,0)rgba(255,255,255,.5)"),
      // Flush `var()` references, the shape the unit rule above has to be
      // careful not to break. Also covered in the sibling module, from the
      // other direction — there as one of the reported bugs, here as the
      // spacing rule that used to produce it.
      unchanged("color", "var(--a)var(--b)var(--c)"),
    ],
    &default_options(),
  );
}

// ── What follows a closing paren ─────────────────────────────────────

/// Everything a `)` can be followed by, one character class at a time. These
/// inputs are degenerate on purpose — each rule of the old repair pass was
/// pinned against a bare `)`, and this is that table, re-asked of the pipeline
/// that replaced it.
///
/// The `/` pair is the only one that still moves, because the ported whitespace
/// normalizer spaces that operator and so does the reference compiler.
/// Everything else comes back as written.
#[test]
fn keeps_what_follows_a_closing_paren_flush() {
  check(
    &[
      // digit
      unchanged("width", ")3"),
      unchanged("width", "calc(a)42"),
      // hex colour
      unchanged("color", ")#fff"),
      // multiplication operator
      unchanged("width", ")*3"),
      // an uppercase word that is not a unit
      unchanged("color", ")A"),
      // division operator: both compilers space it
      same("width", ")/7", ") / 7"),
      // an uppercase word that is a unit
      unchanged("width", ")Q"),
      // A `-` is a sign or a subtraction operator, never a separator, so this
      // is the one pair that is deliberately left alone.
      unchanged("width", ")-1"),
      // Nothing to separate it from.
      unchanged("width", ")"),
    ],
    &default_options(),
  );
}

/// A `)` immediately followed by `(` never gets as far as any spacing rule:
/// `)(` opens a function that is never closed, and that is what the value is
/// rejected for.
///
/// The harness verdict is `both reject`: the reference compiler refuses these
/// too, so the rejection is parity even though the messages are not compared.
/// Asserted on the diagnostic rather than through the case table, because a
/// rejection has no spelling to compare and a bare "it panicked" would pass on
/// any panic at all.
#[test]
fn rejects_an_unclosed_group_after_a_closing_paren() {
  rejects(
    "color",
    &[")(", ")a )Z )0 )# )("],
    UNCLOSED_FUNCTION,
    &default_options(),
  );
}

/// Balanced degenerate parens are normalized like anything else. A space the
/// author did not write before a `)` does not appear, and one they did write
/// does not vanish — the old pass could only ever *insert*, so a space the
/// minifier had already swallowed stayed swallowed. Nothing swallows one now.
/// The only space that moves below is the one the `/` operator gets.
#[test]
fn does_not_restore_a_space_swallowed_before_a_closing_paren() {
  check(
    &[same("width", ")/1 )*.5", ") / 1 )*.5")],
    &default_options(),
  );
}

// ── A hex colour after a value token ─────────────────────────────────

/// A `#` written against the token before it stays against it. Every token
/// class that can precede one is covered: a dimension, a keyword, a percentage,
/// a bare digit, a capital, and a non-ASCII word.
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
      // A `#` with nothing before it has nothing to be separated from.
      unchanged("color", "#"),
    ],
    &default_options(),
  );
}

// ── A number after a percentage ──────────────────────────────────────

/// `40%.1147` is two components, not a percentage followed by a fraction of
/// one — and it reaches the hash spelled the way it was written, which is what
/// makes the distinction the author's rather than the compiler's.
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

/// The same shape inside a colour function, which is the same answer: a number
/// written flush against a percentage stays flush.
#[test]
fn leaves_a_number_after_a_percentage_alone_inside_a_colour_function() {
  check(
    &[unchanged("color", "oklab(40.101%.1147 .0453)")],
    &default_options(),
  );
}

// ── The `/` and `*` operators ────────────────────────────────────────

/// `/` is both a division operator and the CSS slash separator, and the ported
/// whitespace normalizer puts a space on both sides in either role — as does
/// the reference compiler, which is why these agree.
///
/// They part company at the start of a value: the reference compiler emits a
/// leading space before a `/` that opens one (` / 7`), and this compiler does
/// not. Same rule, different edge.
#[test]
fn spaces_the_slash_operator() {
  check(
    &[
      same("width", "size/2", "size / 2"),
      same("width", "/ 7", " / 7"),
      same("width", "/.5", " / .5"),
      same("width", "/-1", " / -1"),
      same("width", "/-2", " / -2"),
      // A function this compiler has never heard of is normalized like any
      // other: there is no grammar to fall outside of.
      same(
        "width",
        "calc-size(fit-content,size/2)",
        "calc-size(fit-content,size / 2)",
      ),
    ],
    &default_options(),
  );
}

/// `*` is a multiplication operator, so an operand parked against it is
/// separated: a digit, a `.`, a `(`, or a sign. Unlike `/`, the reference
/// compiler leaves it as written.
#[test]
fn spaces_the_multiplication_operator() {
  check(
    &[unchanged("width", "* 3"), unchanged("width", "*(100%)")],
    &default_options(),
  );
}

/// A `*` followed by an unclosed `(` never reaches any spacing rule: the
/// unclosed function is what the value is rejected for. Harness verdict
/// `both reject`.
#[test]
fn rejects_an_unclosed_operand_after_a_multiplication_operator() {
  rejects(
    "width",
    &["*(", "/.5 /-1 *( *3"],
    UNCLOSED_FUNCTION,
    &default_options(),
  );
}

// ── Adjacent quoted strings ──────────────────────────────────────────

/// Two strings written flush against each other stay flush, and `""` is not
/// split into `" "` on the way — which is why an empty string, the one input
/// where the closing quote is also the opening one, is here.
#[test]
fn keeps_adjacent_quoted_strings_flush() {
  check(
    &[
      unchanged("gridTemplateAreas", r#""content""sidebar""#),
      unchanged("quotes", r#""a""b""#),
      unchanged("quotes", r#""""""#),
      // Three in a row, so a separator has to be inserted twice — the case
      // that catches a scan which only ever looks one quote back.
      unchanged("quotes", r#""""""""#),
      // A lone empty string keeps its quotes and gains nothing.
      unchanged("quotes", r#""""#),
    ],
    &default_options(),
  );
}

/// The same neighbours in single quotes, or with the two quote characters
/// mixed. Neither the separation nor the quote character moves: the quote the
/// author chose is the quote that reaches the hash.
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

/// A string already separated from its neighbours by a space keeps that space.
///
/// This is the reported whitespace loss in its plainest form: the space used to
/// be dropped outright — not displaced, not collapsed, gone — and the class name
/// changed with it. Nothing removes it now, because nothing rewrites a value
/// except where a normalizer names it.
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

// ── Inside a quoted string ───────────────────────────────────────────

/// String contents are value data, not CSS syntax. Every character that means
/// something outside a string is inert inside one, and both compilers agree
/// byte for byte.
#[test]
fn applies_no_spacing_rule_inside_a_string() {
  check(
    &[
      unchanged("fontFamily", r#""hello)world""#),
      unchanged("fontFamily", r##""a)b#c%d.5/1*2(""##),
      // A doubled backslash is an escaped backslash, not an escaped quote, so
      // the string closes where it looks like it closes.
      unchanged("fontFamily", r#""a\\b""#),
    ],
    &default_options(),
  );
}

/// The same contents in single quotes, which is the same answer: the quote
/// character is content too.
#[test]
fn keeps_the_quote_character_around_inert_contents() {
  check(
    &[unchanged("fontFamily", r##"'a)b#c%d.5/1*2('"##)],
    &default_options(),
  );
}

/// An escaped quote inside a string survives, escape and all.
///
/// This used to be the worst failure in the suite: the round trip through a
/// minifying codegen returned a double-quoted string with its quotes *removed*
/// (`"a\"b#c"` → `a"b#c`) and a single-quoted one with the escaped quote
/// silently dropped. The unquoted result was no longer a string, so the value
/// extraction that followed did not recognise the generated rule's closing
/// brace as a terminator and the `}` landed inside the declaration — escaping
/// the rule the structural guard exists to protect, by a `}` manufactured
/// downstream of the guard.
///
/// Nothing rewrites the string now, so nothing can un-quote it.
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

// ── `url()` bodies ───────────────────────────────────────────────────

/// A `url()` body is not CSS-tokenized. Slashes, colons, semicolons, `#`, `*`
/// and query strings inside one are URL characters, and the body is carried
/// through byte for byte rather than spaced as if it were a value. Every one of
/// these agrees with the reference compiler, which is what "verbatim" has to
/// mean.
///
/// Which bodies get that treatment is pinned separately, by
/// [`steps_over_only_the_bodies_the_parser_takes_whole`].
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
      // A `/*` inside a quoted body is part of the URL, not a comment opener.
      unchanged("backgroundImage", r#"url("a/*b.png")"#),
      // A data URL carries both a `;` and a `/`, either of which would end the
      // value if the body were read as CSS. The payload is broken with Rust
      // line continuations rather than bound to a `const` or joined with
      // `concat!`: the harness harvests string literals, and either of those
      // would take the case out of the corpus and leave it with no verdict.
      // A continuation is still one literal to the scanner, which decodes it
      // the same way rustc does.
      unchanged("backgroundImage", "url(data:image/svg+xml;utf8,<svg/>)"),
      unchanged(
        "backgroundImage",
        "url(\"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAYAAAAf8/9hAAAAMUlEQVQ4T2NkYG\
          AQYcAP3uCTZhw1gGGYhAGBZIA/nYDCgBDAm9BGDWAAJyRCgLaBCAAgXwixzAS0pgAAAABJRU5ErkJggg==\")",
      ),
      // One `url()` among other components, so the body has to be recognised
      // partway through the value rather than at offset zero.
      unchanged(
        "backgroundImage",
        "linear-gradient(red,blue),url(http://example.com/a.png)",
      ),
    ],
    &default_options(),
  );
}

/// An unquoted `(` inside a body nests, so the *matching* `)` ends the URL, not
/// the first one. Without that the body would terminate early and its remainder
/// would be spaced as if it were CSS.
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

/// The body is copied verbatim, and so is the value *after* it. An earlier fast
/// path used to skip the rest of the declaration and leave a later component
/// with the minifier's spacing; there is no minifier and no fast path now, so
/// what follows a `url()` is spelled the way it was written.
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

/// A `)` inside a *quoted* body does not close the function. Reading it as one
/// re-entered the spacing rules mid-URL, which both injected a space into the
/// URL and swallowed the separator after it.
#[test]
fn does_not_close_a_url_on_a_quoted_paren() {
  check(
    &[unchanged("backgroundImage", r#"url("a)b.png") 10px"#)],
    &default_options(),
  );
}

/// The slash separator of a shorthand sits right after a `url()` body, so the
/// two rules meet: the body is copied through and the slash is still spaced.
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

/// The same body in single quotes, which keeps both its quote character and
/// the author's spacing around the call that follows it.
#[test]
fn keeps_the_quote_character_inside_a_url_body() {
  check(
    &[unchanged("backgroundImage", r#"url('a)b.png')calc(1px)"#)],
    &default_options(),
  );
}

/// A quoted `url()` body carrying an escaped quote survives the same way a
/// bare string does. Same finding as
/// [`keeps_an_escaped_quote_inside_a_string`], kept at the URL shape because
/// that is where it was originally pinned.
#[test]
fn keeps_an_escaped_quote_inside_a_url_body() {
  check(
    &[unchanged("backgroundImage", r#"url("a\")b.png") 10px"#)],
    &default_options(),
  );
}

/// An unquoted `'` or `/*` inside a body is an ordinary URL character, and it
/// stays one all the way through: the structural guard steps over a `url()`
/// body whole rather than reading it as CSS, so neither the quote nor the
/// comment opener is taken as the start of a construct that never closes.
///
/// A browser reads the body the same way — an unquoted url token runs to its
/// closing paren — so emitting these verbatim is safe, and the harness verdict
/// for both is `identical`.
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

/// The `url(` lookbehind must not fire for an identifier that merely *ends* in
/// `url`, and it must walk characters rather than bytes: `open_paren - 3` is not
/// necessarily a UTF-8 boundary, and slicing there used to panic.
#[test]
fn does_not_treat_a_url_suffixed_identifier_as_a_url() {
  check(
    &[
      // `éurl(` is not `url(`, so the body is spaced as CSS — and the reference
      // compiler spaces the `/` there too.
      same("width", "éurl(foo/2)", "éurl(foo / 2)"),
      same("width", "日本(1/2)", "日本(1 / 2)"),
      // Fewer than three characters before the `(`, which is where the
      // byte-indexed lookbehind used to go out of bounds.
      unchanged("width", "éab(1)"),
      unchanged("width", "é(1)"),
    ],
    &default_options(),
  );
}

// ── Comments ─────────────────────────────────────────────────────────

/// A comment is carried through untouched — it has to be, since spacing a `/*`
/// would turn it into `/ *` and destroy the declaration.
///
/// This is the case that shows why these were migrated to the public seam. The
/// old repair pass's own contract was "a comment is copied verbatim" while the
/// compiler's was "a comment is dropped", because the codegen in between threw
/// it away; only the second was observable, and only the seam could see it. The
/// comment now survives, which is what the reference compiler has always done —
/// including for `/*/ x */`, which the old path mangled into `/**/ x * /`.
#[test]
fn keeps_comments_in_the_value() {
  check(
    &[
      unchanged("width", "/* a */1px"),
      same("width", "/*/ x */ 1px", "/**/ x * / 1px"),
      same("width", "1px /*/ y */", "1px /**/ y * / "),
      // A `}` inside a comment is comment text: the comment is dropped whole
      // rather than terminating the rule, which is why this is not rejected.
      unchanged("width", "/* a }b */ 1px"),
    ],
    &default_options(),
  );
}

/// A value that is nothing but a comment keeps the comment.
///
/// It used to normalize to nothing at all, which dropped the declaration and
/// left the two compilers emitting a different number of them. A comment is
/// text no normalizer names, so it now survives like any other text.
#[test]
fn keeps_a_value_that_is_only_a_comment() {
  check(
    &[unchanged("width", "/* a */"), unchanged("width", "/**/")],
    &default_options(),
  );
}

// ── Values normalization must leave alone ───────────────────────────

/// A value already spelled the way the compiler would spell it comes back byte
/// for byte, which is what makes running it a second time a no-op — and what
/// makes every case here agree with the reference compiler.
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
      // A single character has no pair to place a space between.
      unchanged("color", "a"),
    ],
    &default_options(),
  );
}

/// An empty value, and one that is nothing but whitespace, are both rejected:
/// there is no token to trim the edges of, and the whitespace normalizer reads
/// the first and last elements of the list without guarding.
///
/// The reference compiler fails on the same values at the same point, so the
/// harness verdict is `both reject`. The message is local rather than an
/// imitation of a foreign runtime error, which is why this is asserted on the
/// diagnostic: a bare "it panicked" would pass on any panic at all.
#[test]
fn rejects_a_value_with_nothing_to_normalize() {
  rejects(
    "color",
    &["", "   "],
    "nothing to normalize",
    &default_options(),
  );
}

// ── Non-ASCII content ────────────────────────────────────────────────

/// Text that needs no escape survives byte for byte, bare and inside a string,
/// across scripts and across UTF-8 lengths up to four bytes. The pass scans
/// characters rather than bytes, and every one of these used to be a way to
/// slice one in half.
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

/// The same content in single quotes, which keeps its quote character too.
#[test]
fn keeps_the_quote_character_around_non_ascii_content() {
  check(
    &[unchanged("fontFamily", r#"'•✓日本語😀'"#)],
    &default_options(),
  );
}

// ── The leading zero on a negative decimal ───────────────────────────

/// The minifier used to strip the `0` from `-0.5px`, leaving something CSS
/// reads as a subtraction rather than a negative number, and a repair pass put
/// it back — but only where it could tell a sign from an operator.
///
/// Nothing strips it now, so nothing has to restore it and nothing has to make
/// that distinction. The author's `-.5px` stays `-.5px`, which is what the
/// reference compiler produces.
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
      // A `-` in sign position that is not followed by `.<digit>` is left
      // alone, and a later one is still restored.
      unchanged("margin", "-5px -.5px"),
    ],
    &default_options(),
  );
}

/// A positive decimal keeps the stripped zero — that spelling is the
/// compiler's, not an accident — and a `-` that follows a complete token is a
/// subtraction operator whatever the token was: a dimension, or a closing
/// paren. Both compilers agree on all three, which is what makes them the
/// boundary of the rule above.
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

/// A percentage followed by `-.5px` keeps the author's spelling.
///
/// It used to come back as `10%-0.5px` — a zero inserted by a restoration pass
/// that ran only on the branch this particular value happened to take, while
/// the other branch left it alone. There is one branch now, and it inserts
/// nothing.
#[test]
fn keeps_a_negative_decimal_after_a_percentage() {
  check(&[unchanged("width", "10%-.5px")], &default_options());
}

/// The restoration never reaches inside a string, so a value that happens to
/// contain the pattern is left as written — including when the string carries
/// multi-byte content, and when a restoration in the surrounding value has to
/// step over it.
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

/// The same string in single quotes, which keeps its quote character too.
#[test]
fn keeps_the_quote_character_around_a_protected_decimal() {
  check(&[unchanged("fontFamily", r#"'-.5'"#)], &default_options());
}

/// The escaped-quote shape again, on a string whose contents would otherwise
/// look like something to rewrite.
#[test]
fn keeps_an_escaped_quote_around_a_protected_decimal() {
  check(
    &[unchanged("fontFamily", r#""a\"-.5""#)],
    &default_options(),
  );
}

/// An unterminated `url()` is reported as an unclosed function, not as a rule
/// terminator.
///
/// The body swallows everything after it — that is what "unterminated" means
/// here — so a `;` or a `{` inside one is url text that never reaches the
/// stylesheet as syntax. Letting the structural scan reject it first would give
/// the same input two diagnostics depending on which check ran first, which is
/// exactly what moving the unclosed checks into the normalizers was meant to
/// stop.
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

/// Nesting inside a `url()` body is body text, not structure. A value that
/// looks deeply nested only because its url body is full of parentheses is
/// neither rejected for depth nor recursed into: the parser reads that body as
/// one word, so the depth this guard counts is the depth the parser will
/// actually reach.
///
/// Generated, so the harness carries no verdict for it. What it is really
/// asserting is that the process survives — the depth guard is the only thing
/// standing between a deep value and a stack overflow, which aborts rather than
/// panicking and so produces no diagnostic at all.
#[test]
fn counts_nesting_inside_a_url_body_as_the_parser_does() {
  let mut body = String::from("a");
  for _ in 0..500 {
    body = format!("({body})");
  }

  let value = format!("url(x{body})");

  // Asserted directly rather than through the case table: the value is built
  // here, and a `Case` holds `&'static str`.
  assert_eq!(
    normalize_css_property_value("backgroundImage", &value, &default_options()),
    value
  );
}

// ── The scan's `url()` rule against the parser's ─────────────────────

/// The structural guard steps over a `url()` body without parsing, so its idea
/// of what a `url()` is has to be the value parser's idea. Where the two
/// disagree, the guard waves through a body the parser will spell straight back
/// out — and a `}` in that body closes the rule the compiler is generating.
///
/// The rule is narrower than CSS: the parser compares the name literally, so
/// `URL(` is an ordinary function to it however case-insensitive CSS itself is.
/// This is the test that says so, and the one that fails if either side is
/// "corrected" on its own.
///
/// Asserted as an invariant over the output rather than as a spelling: whatever
/// the compiler accepts must not carry a rule terminator into the declaration.
#[test]
fn steps_over_only_the_bodies_the_parser_takes_whole() {
  let options = default_options();

  // Each of these puts a rule terminator inside something url-shaped. The ones
  // the parser protects come back verbatim; the ones it does not are rejected.
  let protected = [
    "url(a}b)",
    "url(a{b)",
    "url(a;b)",
    "url(a/*b)",
    // Where the body ends is decided by a parity-of-backslashes rule that both
    // scans carry: the escaped `)` does not close the body, so the `}` after it
    // is still body text.
    r"url(a\)}b)",
  ];
  let unprotected = [
    // Not `url` to the parser, which compares the name literally.
    "URL(a}b)",
    "Url(a}b)",
    // Not `url` to anyone: longer identifiers that merely end in those
    // letters, and one where the name is escaped.
    "blurl(a}b)",
    "noturl(a}b)",
    r"\url(a}b)",
    // The same parity rule read the other way: a doubled backslash is an
    // escaped backslash, so this `)` *does* close the body and the `}` after it
    // is outside it.
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
