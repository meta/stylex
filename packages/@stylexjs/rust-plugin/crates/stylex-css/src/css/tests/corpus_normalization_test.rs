//! The corpus values the normalization pass rewrites and nothing asserted.
//!
//! The corpus in `crates/stylex-rs-compiler/parity/corpus` holds 193 values
//! that this pass rewrites. Most already have a case in a sibling module here.
//! The 68 that did not were reaching the pass through a whole-transform case
//! alone -- a class name and a rule, from which the text the pass produced can
//! only be inferred -- and this module is those, grouped by what the rewrite
//! does. It is the residue of the other modules, not a restatement of them, so
//! a value asserted next door is deliberately absent.
//!
//! **Every expectation is a spelling the parity harness measured**, read from
//! `entries[].babel.declarations` of a report the harness wrote, never from
//! judgement. Regenerate with:
//!
//! ```sh
//! pnpm run --filter=@stylexswc/rs-compiler build
//! pnpm run --filter=@stylexswc/rs-compiler parity -- --json parity/results/default.json
//! ```
//!
//! Three groups of corpus values in the same gap are absent, and each is
//! asserted where the rewrite actually happens:
//!
//! - Content quoting -- `content: x` becomes `content: "x"` -- belongs to
//!   `transform_value`, and is asserted in that crate's own suite.
//! - The logical-value polyfill -- `float: start` becomes `float: left` --
//!   belongs to `generate_ltr`, and is asserted beside it.
//! - Five corpus values that are not declarations at all: an assertion
//!   message, a `join` separator and a JavaScript object key that the
//!   harvester read out of a Rust test as though it were CSS. A case over one
//!   of those would state nothing about CSS. Issue 35 covers the harvester.
//!
//! One further value is absent: a ten-layer `boxShadow` whose only subject is
//! that a long comma-separated list is normalized entry by entry, which
//! `normalizes_every_entry_of_a_long_list` already states over 500 layers.

use super::support::{check, default_options, same};

/// A comma separates without a space after it, in a function argument list and
/// between the layers of a comma-separated value alike.
#[test]
fn removes_the_space_after_a_comma() {
  check(
    &[
      same(
        "backgroundImage",
        "linear-gradient(#000000, #ffffff)",
        "linear-gradient(#000000,#ffffff)",
      ),
      same(
        "backgroundImage",
        "url(\"asset.png\"), linear-gradient(red, blue)",
        "url(\"asset.png\"),linear-gradient(red,blue)",
      ),
      same(
        "boxShadow",
        "2px 2px 2px 2px red, inset 1px 1px 1px 1px #000",
        "2px 2px 2px 2px red,inset 1px 1px 1px 1px #000",
      ),
      same(
        "color",
        "var(--a, var(--b, var(--c, var(--d, var(--e, var(--f, red))))))",
        "var(--a,var(--b,var(--c,var(--d,var(--e,var(--f,red))))))",
      ),
      same(
        "color",
        "var(--x, var(--y, #ABCDEF))",
        "var(--x,var(--y,#ABCDEF))",
      ),
      same(
        "fontFamily",
        "\"Helvetica Neue\", sans-serif",
        "\"Helvetica Neue\",sans-serif",
      ),
      same(
        "fontFamily",
        "\"Helvetica \\\"Neue\", sans-serif",
        "\"Helvetica \\\"Neue\",sans-serif",
      ),
      same(
        "fontFamily",
        "\"\\1F600\", sans-serif",
        "\"\\1F600\",sans-serif",
      ),
      same(
        "fontFamily",
        "\"\\\\1F600\", sans-serif",
        "\"\\\\1F600\",sans-serif",
      ),
      same(
        "fontFamily",
        "\"\\\\2014 A\", sans-serif",
        "\"\\\\2014 A\",sans-serif",
      ),
      same(
        "fontFamily",
        "'Helvetica Neue', \"Segoe UI\", sans-serif",
        "'Helvetica Neue',\"Segoe UI\",sans-serif",
      ),
      same(
        "fontFamily",
        "-apple-system, BlinkMacSystemFont, \"Segoe UI\", Roboto, sans-serif",
        "-apple-system,BlinkMacSystemFont,\"Segoe UI\",Roboto,sans-serif",
      ),
      same(
        "fontFamily",
        "My\\ Font, sans-serif",
        "My\\ Font,sans-serif",
      ),
      same(
        "fontFamily",
        "My\\\\ Font, sans-serif",
        "My\\\\ Font,sans-serif",
      ),
      same(
        "gridTemplateColumns",
        "auto minmax(0, 1fr)",
        "auto minmax(0,1fr)",
      ),
      same("gridTemplateColumns", "minmax(0, 1fr)", "minmax(0,1fr)"),
      same(
        "gridTemplateRows",
        "minmax(0, 1fr) auto",
        "minmax(0,1fr) auto",
      ),
      same(
        "height",
        "CALC-SIZE(auto, round(up, size, 50px))",
        "CALC-SIZE(auto,round(up,size,50px))",
      ),
      same(
        "height",
        "calc-size(300px + 2rem, size / 2)",
        "calc-size(300px + 2rem,size / 2)",
      ),
      same(
        "height",
        "calc-size(any, 300px * 1.5)",
        "calc-size(any,300px * 1.5)",
      ),
      same(
        "height",
        "calc-size(auto, size * 0)",
        "calc-size(auto,size * 0)",
      ),
      same(
        "height",
        "calc-size(calc-size(max-content, size), size + 2rem)",
        "calc-size(calc-size(max-content,size),size + 2rem)",
      ),
      same(
        "height",
        "calc-size(fit-content, size / 2)",
        "calc-size(fit-content,size / 2)",
      ),
      same(
        "height",
        "calc-size(var(--intrinsic-size), max(100px, size + 20px))",
        "calc-size(var(--intrinsic-size),max(100px,size + 20px))",
      ),
      same(
        "listStyle",
        "inside \"--\" linear-gradient(90deg, white 100%)",
        "inside \"--\" linear-gradient(90deg,white 100%)",
      ),
      same(
        "listStyle",
        "outside \"+\" linear-gradient(90deg, white 100%)",
        "outside \"+\" linear-gradient(90deg,white 100%)",
      ),
      same(
        "listStyle",
        "simp-chinese-informal linear-gradient(90deg, white 100%)",
        "simp-chinese-informal linear-gradient(90deg,white 100%)",
      ),
      same(
        "margin",
        "max(0px, (48px - var(--x16dnrjz)) / 2)",
        "max(0px,(48px - var(--x16dnrjz)) / 2)",
      ),
      same(
        "minHeight",
        "calc(100dvh - var(--header-height, 0px))",
        "calc(100dvh - var(--header-height,0px))",
      ),
      same(
        "transitionProperty",
        "opacity, inset-inline-start",
        "opacity,inset-inline-start",
      ),
      same(
        "willChange",
        "opacity, inset-inline-start",
        "opacity,inset-inline-start",
      ),
    ],
    &default_options(),
  );
}

/// A fraction below one loses its leading zero, wherever in the value it sits.
#[test]
fn drops_the_zero_before_a_decimal_point() {
  check(
    &[
      same(
        "backgroundPosition",
        "top 0.75rem left 0.625rem",
        "top .75rem left .625rem",
      ),
      same(
        "margin",
        "calc((100% - 50px) * 0.5) 20px 0",
        "calc((100% - 50px) * .5) 20px 0",
      ),
      same(
        "marginTop",
        "calc((100% - 50px) * 0.5)",
        "calc((100% - 50px) * .5)",
      ),
      same(
        "outline",
        "transparent dotted 0.125rem",
        "transparent dotted .125rem",
      ),
      same(
        "transform",
        "translateX(100px) translateY(-300px) scale(0.7)",
        "translateX(100px) translateY(-300px) scale(.7)",
      ),
      same("transitionDuration", "0.01s", ".01s"),
      same("width", "0.30000000000000004px", ".30000000000000004px"),
    ],
    &default_options(),
  );
}

/// Any run of whitespace between tokens becomes one space, and a trailing one
/// goes. A tab and a newline count; the two characters that *spell* one in
/// the source do not, which the two `margin` cases put side by side.
#[test]
fn collapses_a_run_of_whitespace_to_one_space() {
  check(
    &[
      same("color", "red   !   important", "red ! important"),
      same("margin", "1px\t2px\n3px  4px", "1px 2px 3px 4px"),
      same("margin", "1px\\t2px\\n3px  4px", "1px\\t2px\\n3px 4px"),
      same("margin", "4px ", "4px"),
      same(
        "width",
        "calc((100% + 3% -   100px) / 7)",
        "calc((100% + 3% - 100px) / 7)",
      ),
    ],
    &default_options(),
  );
}

/// A slash is separated from what it divides.
#[test]
fn spaces_a_slash_from_the_tokens_around_it() {
  check(
    &[
      same(
        "backgroundImage",
        "url(\"asset.png\") no-repeat center/cover",
        "url(\"asset.png\") no-repeat center / cover",
      ),
      same("font", "16px/16 Arial", "16px / 16 Arial"),
      same("height", "future-fn(foo/2 * @)", "future-fn(foo / 2 * @)"),
      same("width", "a() 1px /2", "a() 1px  / 2"),
      same("width", "calc(1) / 2", "calc(1)  / 2"),
      same("width", "url(x) 1px /2", "url(x) 1px / 2"),
    ],
    &default_options(),
  );
}

/// A property name is a value in its own right for `transition-property` and
/// `will-change`, and it is spelled the way CSS spells it.
#[test]
fn kebab_cases_a_property_name_written_as_a_value() {
  check(
    &[
      same("transitionProperty", "marginTop", "margin-top"),
      same("willChange", "insetInlineStart", "inset-inline-start"),
    ],
    &default_options(),
  );
}

/// An importance annotation loses the space before it. The three cases differ
/// only in the property and in how the author spaced the `!`, and the pass
/// reads neither -- they are here because each is a corpus value in its own
/// right, not because they take different paths.
#[test]
fn tightens_the_space_around_an_importance_annotation() {
  check(
    &[
      same("backgroundColor", "red !important", "red!important"),
      same("display", "block !important", "block!important"),
    ],
    &default_options(),
  );
}

/// What none of the groups above describes on its own.
///
/// Mostly compositions -- a leading zero *and* a comma, whitespace *and* a
/// kebab-cased name -- which is the common case in authored CSS, and the
/// composition is what a stylesheet is hashed from. Five carry a rule the
/// gap reaches exactly once, so a group of its own would say no more than
/// the case does: a colon gains spaces (`height`), whitespace inside a
/// parenthesis goes (`backgroundImage`), a signed zero loses its sign
/// (`padding`), a redundant zero in front of an integer goes (`width`), an
/// empty string takes double quotes (`quotes`), and milliseconds shorten to
/// seconds (`transitionDuration`).
#[test]
fn rewrites_more_than_the_groups_above_describe() {
  check(
    &[
      same(
        "backgroundImage",
        "linear-gradient(to bottom, rgba(0, 0, 0, 0) 0%, rgba(0, 0, 0, 0.6) 100%)",
        "linear-gradient(to bottom,rgba(0,0,0,0) 0%,rgba(0,0,0,.6) 100%)",
      ),
      same("backgroundImage", "url ( a )", "url (a)"),
      same("color", "rgba( 1, 222,  33 , 0.5)", "rgba(1,222,33,.5)"),
      same(
        "height",
        "calc-size( auto , size   *   0 )",
        "calc-size(auto,size * 0)",
      ),
      same("height", "color:red", "color : red"),
      same("padding", "+1px +2% 0.5px 000.5px", "+1px +2% .5px .5px"),
      same("padding", "-0px 1e21px", "0px 1e21px"),
      same(
        "padding",
        "calc((100% - 50px) * 0.5) var(--rightpadding, 20px)",
        "calc((100% - 50px) * .5) var(--rightpadding,20px)",
      ),
      same("quotes", "''", "\"\""),
      same(
        "transform",
        "  rotate(10deg)  translate3d( 0 , 0 , 0 )  ",
        "rotate(10deg) translate3d(0,0,0)",
      ),
      same("transitionDuration", "1234ms", "1.234s"),
      same(
        "transitionProperty",
        "opacity, insetInlineStart",
        "opacity,inset-inline-start",
      ),
      same("width", "00\\\\", "0\\\\"),
      same(
        "width",
        "max(4.8125rem, 100vw * 0.12)",
        "max(4.8125rem,100vw * .12)",
      ),
      same(
        "willChange",
        "opacity, insetInlineStart",
        "opacity,inset-inline-start",
      ),
    ],
    &default_options(),
  );
}
