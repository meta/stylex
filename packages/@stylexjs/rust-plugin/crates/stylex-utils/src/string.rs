use std::borrow::Cow;

/// Converts a camelCase or PascalCase string to its hyphenated (kebab-case)
/// equivalent by inserting hyphens before uppercase letters and lowercasing
/// the result.
///
/// This is used to convert JavaScript-style CSS property names (e.g.
/// `marginTop`, `WebkitTransform`) to their CSS equivalents (`margin-top`,
/// `-webkit-transform`).
/// `dashify` lowercases unconditionally, so the borrowed fast path is only
/// sound for input the lowercasing cannot change. ASCII without an uppercase
/// letter qualifies; anything non-ASCII does not, because a scalar can lowercase
/// to something other than itself while satisfying neither `is_uppercase` nor
/// `is_lowercase` — a titlecase scalar such as `ǅ` lowercases to `ǆ`.
pub fn dashify(s: &str) -> Cow<'_, str> {
  if s.is_ascii() && !s.bytes().any(|byte| byte.is_ascii_uppercase()) {
    return Cow::Borrowed(s);
  }

  // The input's length plus room for the hyphens this inserts. A CSS property
  // name spells one or two, and overshooting by a few bytes costs less than the
  // realloc that undershooting buys.
  let mut dashed = String::with_capacity(s.len() + 4);
  let mut previous: Option<char> = None;

  for character in s.chars() {
    // The rule the pattern `(?<=^|[a-z])([A-Z])` spelled: an ASCII uppercase
    // letter takes a hyphen when it opens the string or follows an ASCII
    // lowercase one. Both classes are ASCII in the pattern, so a preceding
    // non-ASCII character is no more a match here than it was there --
    // `Ǆolume` keeps its single leading character either way.
    if character.is_ascii_uppercase()
      && previous.is_none_or(|previous| previous.is_ascii_lowercase())
    {
      dashed.push('-');
    }

    dashed.push(character);
    previous = Some(character);
  }

  // Lowercased in a second pass over the built string rather than per character
  // on the way in, which would save this allocation and be wrong.
  // `str::to_lowercase` is context-sensitive where `char::to_lowercase` is not:
  // a Greek capital sigma lowercases to the final form at the end of a word and
  // the medial form elsewhere, and a single character cannot know which it is.
  // `dashify("aBΣ")` is `a-bς` through the string and `a-bσ` through the chars.
  // The regex this replaced also lowercased the finished string, so this keeps
  // the answer it gave.
  Cow::Owned(dashed.to_lowercase())
}

/// The spelling a `style` attribute gives a property name.
///
/// Every ASCII capital takes a hyphen before it and the whole name is then
/// lowercased, which is the rule the StyleX runtime applies where it writes a
/// `style` attribute.
///
/// It is not [`dashify`], and the difference is deliberate. A stylesheet takes
/// a hyphen only where a capital opens the name or follows a lowercase letter,
/// so `ABCDef` is `-abcdef` there and `-a-b-c-def` here. A stylesheet also
/// hands a custom property back untouched, which is a guard its caller holds
/// rather than a rule inside `dashify`; an attribute has no such guard, so
/// `--myColor` becomes `--my-color`. The two rules are kept apart because the
/// runtime keeps them apart.
pub fn kebab_case(s: &str) -> Cow<'_, str> {
  if s.is_ascii() && !s.bytes().any(|byte| byte.is_ascii_uppercase()) {
    return Cow::Borrowed(s);
  }

  let mut kebab = String::with_capacity(s.len() + 4);

  for character in s.chars() {
    if character.is_ascii_uppercase() {
      kebab.push('-');
    }

    // Lowercased as the name is built, which is one string rather than the two
    // a `to_lowercase` over the finished one costs. Every property name a style
    // object holds is ASCII, and this runs once per declaration.
    kebab.push(character.to_ascii_lowercase());
  }

  match s.is_ascii() {
    true => Cow::Owned(kebab),
    // A name outside ASCII goes through the Unicode rule as well, for the
    // reason `dashify` states above. The pass above left every character it
    // covers untouched, so the two do not disagree.
    //
    // Over the finished name, and not character by character in the loop
    // above. The rule the runtime applies is one `toLowerCase` over the whole
    // name, and a whole name is what a final sigma is judged against: `aΣB`
    // becomes `aς-b` there and `aσ-b` under a per-character lowering. The
    // second pass is what keeps the two agreeing, and it is paid only by a
    // name that is not ASCII -- which no property name a style object holds
    // ever is.
    false => Cow::Owned(kebab.to_lowercase()),
  }
}

/// Whether a value spells no CSS text at all — empty, or nothing but
/// characters the value scanner reads as whitespace.
///
/// A declaration built from one of these is `color:`, which no browser accepts,
/// so the property is left undeclared instead. Asked of both an authored value
/// and the value it transforms to, which is why it lives here rather than on
/// either type.
///
/// Deliberately narrower than `str::trim`. The scanner calls a character
/// whitespace when its code is at most 32 and nothing else, so U+00A0 or
/// U+3000 is a *word token* to it, not a gap — and the reference compiler
/// emits such a value verbatim rather than dropping the property. Testing
/// bytes is exact here: every byte of a multi-byte character is at least 0x80.
///
/// Wider than `str::trim` in the other direction, which is the half that
/// changes an answer rather than fixing one. A C0 control is not Unicode
/// whitespace, so `trim` left `"\u{1}"` in place and the value reached
/// `normalize_whitespace`, whose `trim_edges` emptied the token list and
/// rejected with `LINT_VALUE_HAS_NO_TOKENS` — which happens to match the
/// `TypeError` the reference throws at the same point. Reading it as blank
/// drops the property silently instead.
///
/// That is the answer this seam already gave for `""` and `" "`, which the
/// reference also throws on and which the parity corpus already carries as
/// acceptance divergences. Chosen for that consistency: the alternative is a
/// rule under which `"\t"` is dropped and `"\u{1}"` is a hard error, which no
/// author could predict from the value they wrote. Pinned by
/// `c0_controls_that_are_not_unicode_whitespace_are_blank`.
pub fn is_blank_css_text(s: &str) -> bool {
  s.bytes().all(|byte| byte <= 32)
}

/// The length of a string as JavaScript reports it: its count of UTF-16 code
/// units.
///
/// Three conventions disagree about what a string's length is, and only this
/// one is the language's. `str::len` counts bytes, so `"é".length` would read
/// as `2`; `chars().count()` counts Unicode scalars, so an astral character
/// would read as `1` where JavaScript says `2`. `String.prototype.length`
/// counts code units — an astral scalar occupies two of them — so
/// `"\u{1F600}a".length` is `3` and index `2` is where the `a` lives.
///
/// ASCII is answered from the byte length, which is the same number: one byte
/// per code unit, and `is_ascii` is a vectorised scan where the encoder is a
/// scalar-at-a-time walk. Every CSS value a build measures is ASCII, and this
/// runs on every `+` and every template interpolation in every file.
pub fn utf16_length(s: &str) -> usize {
  if s.is_ascii() {
    return s.len();
  }

  s.encode_utf16().count()
}

/// A string rendered as `JSON.stringify` renders it: double-quoted, with the
/// escapes JSON demands and nothing more.
///
/// Exists so that a diagnostic quoting an authored value reads the same here as
/// it does upstream, where these messages are built by interpolating
/// `JSON.stringify(rawValue)`. Formatting the value with `{:?}` would be close
/// but not equal — Rust spells a C0 control `\u{1}` and escapes `'`, where JSON
/// spells `\u0001` and leaves the apostrophe alone — and the difference lands in
/// text a test compares.
///
/// `serde_json` is the renderer rather than a hand-rolled escape table, and the
/// escape sets do agree exactly: `"` and `\`, the five single-letter shortcuts
/// `\b \f \n \r \t`, every remaining code point below U+0020 as a lowercase
/// four-digit `\uXXXX`, and everything from U+0020 up written through unchanged
/// — U+007F and U+2028 included, which `JSON.stringify` also leaves raw even
/// though a JS *source* literal could not carry them. The `json_stringify_tests`
/// module holds that agreement, so a change in either renderer surfaces as a
/// test failure rather than as silent drift in an author-facing message.
///
/// Rendered through `Value`, not `to_string`, so there is no serializer error to
/// answer for: quoting a string cannot fail, and `Display` says so in its
/// signature where `to_string` would hand back a `Result` whose `Err` arm no
/// input reaches.
///
/// One rule of well-formed `JSON.stringify` is unreachable rather than omitted:
/// a lone surrogate escapes to `\ud83c` in JS, and a Rust `str` cannot hold one.
pub fn json_stringify(s: &str) -> String {
  serde_json::Value::String(s.to_owned()).to_string()
}

#[cfg(test)]
#[path = "tests/string_test.rs"]
mod tests;
