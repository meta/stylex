//! Rewriting a declaration value into the canonical text that gets hashed.
//!
//! The value is scanned into a loose token list, a fixed list of small
//! transformations is folded over it in a fixed order, and the list is spelled
//! back out. Nothing else happens to it — which is the point. A token no pass
//! names survives byte for byte, so the author's hex spelling,
//! letter case, quote character and whitespace positions all reach the hash
//! exactly as written.
//!
//! ## The order is part of the behaviour
//!
//! [`normalize_timings`] runs before [`normalize_leading_zero`] so that a
//! duration converted from milliseconds to seconds is then stripped of its
//! leading zero: `100ms` becomes `0.1s` and then `.1s`. Reordering the two
//! produces `0.1s`, a different class name, and no error anywhere.
//!
//! The three detectors run first so that a value carrying an unfinished
//! construct, or a reference to a property that cannot exist, is rejected
//! before anything rewrites it — and among themselves in that order, so that
//! `var(foo`, which is unfinished *and* unprefixed, is reported as the
//! unfinished function it is. [`convert_font_size_to_rem`] runs last —
//! appended only when its option is on — so that the number it produces keeps
//! its leading zero.
//!
//! The caller's own injection guard runs *between* the first two detectors and
//! the third, which is where the sequence stops being an internal ordering and
//! becomes a compatibility one. An unclosed function and an unclosed string are
//! the two rejections the reference compiler also makes, so a value carrying
//! one of them and a declaration-terminating token is refused by both compilers
//! for the same reason rather than for whichever guard each spelled first. The
//! token guard still outranks the unprefixed custom property, which is this
//! compiler's rejection alone and has no second opinion to agree with.
//!
//! ## What is not here
//!
//! No pass understands hex colours, letter case or quote characters, so none of
//! them can alter those. Read the absence as deliberate: it is what makes two
//! compilers agree on a value neither of them has an opinion about.
//!
//! Exponent notation is not in that list. Every pass that re-spells a number
//! goes through `to_js_string`, which spells it the way JavaScript's
//! `Number::toString` does — so `0.0000001px` comes back as `1e-7px`, which is
//! what the reference compiler produces too.

use postcss_value_parser::{ValueParser, stringify};
use stylex_macros::stylex_panic;
use stylex_structures::stylex_state_options::StyleXStateOptions;

use crate::css::normalizers::{
  convert_camel_case_values::convert_camel_cased_values, detect_unclosed_fns::detect_unclosed_fns,
  detect_unclosed_strings::detect_unclosed_strings, font_size_px_to_rem::convert_font_size_to_rem,
  leading_zero::normalize_leading_zero, quotes::normalize_quotes, timings::normalize_timings,
  unprefixed_custom_properties::detect_unprefixed_custom_properties,
  whitespace::normalize_whitespace, zero_dimensions::normalize_zero_dimensions,
};

/// One pass over the token list, rewriting it in place — or rejecting it — for
/// the property its second argument names.
///
/// Deliberately not `Normalizer`: three of these only reject and never rewrite,
/// and the nine *ported normalizers* each name their own position in a
/// different sequence — the one their module headers count against. Two
/// sequences of nine meaning different things is one too many, so the list
/// below is passes and the ports stay normalizers.
///
/// Four of the ten read the property — three to decide whether they apply at
/// all, one to name the declaration in a rejection. It is passed to all ten
/// anyway, which is what lets the fold below be a list rather than ten call
/// sites.
type Pass = fn(&mut ValueParser, &str);

/// The rejections the reference compiler also makes, in the order they run in.
///
/// Held apart from the rest so the caller's injection guard has a named place
/// to run: after these, before everything below. Both are rejections and
/// neither rewrites a token, so the split costs the sequence nothing but the
/// ability to say where the boundary is.
const SHARED_REJECTIONS: [Pass; 2] = [detect_unclosed_fns, detect_unclosed_strings];

/// The passes that always run once the shared rejections have had their say.
const PASSES: [Pass; 7] = [
  detect_unprefixed_custom_properties,
  normalize_whitespace,
  normalize_timings,
  normalize_zero_dimensions,
  normalize_leading_zero,
  normalize_quotes,
  convert_camel_cased_values,
];

/// Normalizes `value`, declared for property `key`.
///
/// Fails — as a panic the compiler catches and reports — on an unclosed
/// function, an unclosed string, an unprefixed custom-property reference, and
/// on a value that scans to no tokens at all.
///
/// **This is the fold without the structural guards, and it is a seam rather
/// than a leftover.** The compiler itself enters at
/// [`crate::css::common::normalize_css_property_value`], which reads the raw
/// bytes for an unclosed comment, a nesting depth past the budget, and a
/// declaration-terminating token before handing the value here. Those three are
/// local additions the reference compiler does not make, so a caller that wants
/// only the ported pipeline — every pass the reference compiler runs, and
/// nothing this one adds — wants this function.
///
/// `css/tests/normalize_value_test.rs` is that caller, deliberately: it pins
/// what the fold does with `{`, `}` and `;`, which is to normalize them like any
/// other token, so that the division of labour is written down. If a value able
/// to break out of its own rule ever reaches a stylesheet, that suite says the
/// missing guard is the defect rather than the fold. The guards have their own
/// suite in `css/tests/value_normalization_parity_test.rs`, which enters through
/// `normalize_css_property_value`.
///
/// `pub` because `benches/normalize_value_bench.rs` is a separate target and
/// cannot reach a `pub(crate)` item -- a requirement, not an invitation. This
/// function will normalize a `}` straight into a stylesheet, which is the whole
/// reason the guarded entry point exists; production code wants
/// `normalize_css_property_value`.
pub fn normalize_value(value: &str, key: &str, options: &StyleXStateOptions) -> String {
  normalize_value_guarded(value, key, options, None)
}

/// Normalizes `value`, with the caller's injection guard folded into the
/// sequence at the position the module header describes.
///
/// `deferred_refusal` is the complaint a structural guard has already built, or
/// `None` where the value tripped none of them. The whole sentence rather than
/// its parts, because building it is the caller's vocabulary — the guards read
/// the raw bytes, which nothing here has kept — and because a `Some` is then the
/// whole of the condition this function has to test.
///
/// One seam for two guards rather than one each. Both the declaration-terminating
/// token and the unclosed comment belong *after* `SHARED_REJECTIONS` for the same
/// reason, and giving each its own parameter was how the second came to be left
/// in front of them.
pub(crate) fn normalize_value_guarded(
  value: &str,
  key: &str,
  options: &StyleXStateOptions,
  deferred_refusal: Option<&str>,
) -> String {
  let mut ast = ValueParser::new(value);

  for pass in SHARED_REJECTIONS {
    pass(&mut ast, key);
  }

  if let Some(complaint) = deferred_refusal {
    stylex_panic!("{}", complaint);
  }

  for pass in PASSES {
    pass(&mut ast, key);
  }

  if options.enable_font_size_px_to_rem {
    convert_font_size_to_rem(&mut ast, key);
  }

  // `stringify` rather than `to_string`: `Display` builds this very string and
  // `ToString` then copies it into a second buffer.
  stringify(&ast.nodes)
}
