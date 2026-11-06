/*!
CSS Color type parsing.

Handles all CSS color formats: named colors, hex, rgb, rgba, hsl, hsla, and modern color spaces.
*/

use stylex_macros::stylex_unreachable;
use stylex_utils::number::{to_js_string, write_js_number};

use crate::{
  CssParseError,
  css_types::{
    Angle, Percentage,
    alpha_value::{alpha_as_number, parse_alpha_token},
  },
  token_parser::{TokenParser, tokens},
  token_types::{SimpleToken, TokenList},
};

use std::fmt::{self, Display};

/// The alpha a modern colour space prints, or `None` where it prints no tail.
///
/// The reference implementation spells the tail
/// `${this.alpha ? ` / ${this.alpha}` : ''}` -- a JavaScript *truthiness* test
/// rather than a presence test, so an alpha of zero prints nothing at all and
/// `lch(50 20 30 / 0)` is spelled `lch(50 20 30)`. A `NaN` alpha is falsy there
/// for the same reason and so prints nothing here either.
///
/// The rule lives here rather than at each of the three call sites because
/// getting it wrong is not a cosmetic slip: the printed spelling feeds the
/// class-name hash, so an alpha tail this compiler prints and the reference
/// compiler does not is an observable divergence.
fn printed_alpha(alpha: Option<f64>) -> Option<f64> {
  alpha.filter(|value| *value != 0.0 && !value.is_nan())
}

/// Extracts the numeric value from a Number token. Returns 0.0 for non-Number
/// tokens (defensive fallback — guarded by tokens::number() in callers).
fn extract_number_from_token(token: SimpleToken) -> f64 {
  if let SimpleToken::Number(v) = token {
    v
  } else {
    0.0
  }
}

/// A channel of a legacy `rgb()`/`rgba()` colour, read from the token list.
///
/// One reader for both, because the reference compiler has one: `rgbNumberParser`
/// serves `rgb`, `rgba` and their space-separated forms alike, bounding the
/// value to `0..=255` while keeping it a `number`. The bound is the channel's,
/// not the alpha's -- see [`crate::css_types::alpha_value::parse_alpha_token`],
/// which is deliberately unbounded.
///
/// A fraction inside the bound survives, since the field holds a double and only
/// the range is checked: `rgb(2.5, 0, 0)` is `rgb(2.5,0,0)`.
fn parse_rgb_channel_token(tokens: &mut TokenList) -> Result<f64, CssParseError> {
  let token = tokens
    .consume_next_token_infallible()
    .ok_or(CssParseError::ParseError {
      message: "Expected number token".to_string(),
    })?;

  match token {
    SimpleToken::Number(value) if (0.0..=255.0).contains(&value) => Ok(value),
    SimpleToken::Number(value) => Err(CssParseError::ParseError {
      message: format!("RGB channel must be 0-255, got {}", value),
    }),
    _ => Err(CssParseError::ParseError {
      message: format!("Expected Number token, got {:?}", token),
    }),
  }
}

/// Parses numbers in range 0-255 for RGB color channels
fn rgb_number_parser() -> TokenParser<f64> {
  tokens::number()
    .map(extract_number_from_token, Some("extract_number"))
    .where_fn(|v| *v >= 0.0 && *v <= 255.0, Some("0..255"))
}

/// Reads the `/ <alpha-value>` tail the modern colour spaces share, rewinding
/// to where it started and answering `None` when there is no slash. The rewind
/// puts back the whitespace it skipped looking for the slash, so a caller that
/// asks and gets `None` still has to consume that whitespace itself.
fn parse_optional_slash_alpha(tokens: &mut TokenList) -> Result<Option<f64>, CssParseError> {
  let checkpoint = tokens.current_index;

  if let Some(SimpleToken::Whitespace) = tokens.peek_infallible() {
    tokens.consume_next_token_infallible();
  }

  match tokens.peek_infallible() {
    Some(SimpleToken::Delim('/')) => {
      tokens.consume_next_token_infallible();

      if let Some(SimpleToken::Whitespace) = tokens.peek_infallible() {
        tokens.consume_next_token_infallible();
      }

      // A slash was written, so a malformed alpha after it is an error rather
      // than an absent alpha: no rewind on this arm.
      alpha_as_number().run.as_ref()(tokens).map(Some)
    },
    _ => {
      tokens.set_current_index(checkpoint);
      Ok(None)
    },
  }
}

/// Returns true when `token` is a Function token with the given name. Returns
/// false for all other token kinds (defensive fallback — guarded by
/// tokens::function() in callers).
fn check_function_name(token: &SimpleToken, name: &str) -> bool {
  if let SimpleToken::Function(fn_name) = token {
    fn_name == name
  } else {
    false
  }
}

/// Function name parser helper
fn function_parser(name: &'static str) -> TokenParser<()> {
  tokens::function()
    .where_fn(
      move |token| check_function_name(token, name),
      Some(&format!("{}_function", name)),
    )
    .map(|_| (), Some(&format!("{}_fn", name)))
}

pub struct AdvancedColorParsers;

impl AdvancedColorParsers {
  /// Implements: TokenParser.sequence(fn, r, comma, g, comma, b,
  /// closeParen).map(([_fn, r, _c, g, _c2, b, _cp]) => ...)
  pub fn rgb_comma_full() -> TokenParser<(f64, f64, f64)> {
    function_parser("rgb")
      .flat_map(|_| rgb_number_parser(), Some("r"))
      .flat_map(
        |r| {
          tokens::comma().flat_map(
            move |_| rgb_number_parser().map(move |g| (r, g), Some("rg")),
            Some("comma1"),
          )
        },
        Some("rg_step"),
      )
      .flat_map(
        |(r, g)| {
          tokens::comma().flat_map(
            move |_| rgb_number_parser().map(move |b| (r, g, b), Some("rgb")),
            Some("comma2"),
          )
        },
        Some("rgb_step"),
      )
      .flat_map(
        |(r, g, b)| tokens::close_paren().map(move |_| (r, g, b), Some("final_rgb")),
        Some("close"),
      )
  }

  pub fn rgb_space_full() -> TokenParser<(f64, f64, f64)> {
    function_parser("rgb")
      .flat_map(
        |_| {
          rgb_number_parser()
            .separated_by(tokens::whitespace())
            .one_or_more()
            .where_fn(|vals| vals.len() >= 3, Some("has_3_values"))
        },
        Some("rgb_values"),
      )
      .flat_map(
        |vals| tokens::close_paren().map(move |_| (vals[0], vals[1], vals[2]), Some("final_rgb")),
        Some("close"),
      )
  }

  /// [_fn, r, _comma, g, _comma2, b, _comma3, a, _closeParen] => new Rgba(r, g,
  /// b, a)
  pub fn rgba_comma_full() -> TokenParser<(f64, f64, f64, f64)> {
    function_parser("rgba")
      .flat_map(|_| rgb_number_parser(), Some("r"))
      .flat_map(
        |r| {
          tokens::comma().flat_map(
            move |_| rgb_number_parser().map(move |g| (r, g), Some("rg")),
            Some("comma1"),
          )
        },
        Some("rg_step"),
      )
      .flat_map(
        |(r, g)| {
          tokens::comma().flat_map(
            move |_| rgb_number_parser().map(move |b| (r, g, b), Some("rgb")),
            Some("comma2"),
          )
        },
        Some("rgb_step"),
      )
      .flat_map(
        |(r, g, b)| {
          tokens::comma().flat_map(
            move |_| alpha_as_number().map(move |a| (r, g, b, a), Some("rgba")),
            Some("comma3"),
          )
        },
        Some("rgba_step"),
      )
      .flat_map(
        |(r, g, b, a)| tokens::close_paren().map(move |_| (r, g, b, a), Some("final_rgba")),
        Some("close"),
      )
  }

  /// Uses rgb() function with space-separated values and slash for alpha: rgb(r
  /// g b / a) [_fn, _preSpace, r, _space, g, _space2, b, _slash, a,
  /// _postSpace, _closeParen] => new Rgba(r, g, b, a)
  pub fn rgba_space_slash_full() -> TokenParser<(f64, f64, f64, f64)> {
    function_parser("rgb") // Note: rgb function, not rgba!
      .flat_map(
        |_| {
          tokens::whitespace()
            .optional()
            .flat_map(move |_| rgb_number_parser(), Some("r"))
        },
        Some("optional_space_r"),
      )
      .flat_map(
        |r| {
          tokens::whitespace().flat_map(
            move |_| rgb_number_parser().map(move |g| (r, g), Some("rg")),
            Some("space1"),
          )
        },
        Some("rg_step"),
      )
      .flat_map(
        |(r, g)| {
          tokens::whitespace().flat_map(
            move |_| rgb_number_parser().map(move |b| (r, g, b), Some("rgb")),
            Some("space2"),
          )
        },
        Some("rgb_step"),
      )
      .flat_map(
        |(r, g, b)| {
          // Parse slash surrounded by optional whitespace
          tokens::whitespace().optional().flat_map(
            move |_| {
              tokens::delim('/').flat_map(
                move |_| {
                  tokens::whitespace().optional().flat_map(
                    move |_| alpha_as_number().map(move |a| (r, g, b, a), Some("rgba")),
                    Some("alpha"),
                  )
                },
                Some("slash"),
              )
            },
            Some("slash_alpha"),
          )
        },
        Some("rgba_step"),
      )
      .flat_map(
        |(r, g, b, a)| {
          tokens::whitespace().optional().flat_map(
            move |_| tokens::close_paren().map(move |_| (r, g, b, a), Some("final_rgba")),
            Some("close"),
          )
        },
        Some("final_close"),
      )
  }

  pub fn hsl_comma_full() -> TokenParser<(Angle, Percentage, Percentage)> {
    function_parser("hsl")
      .flat_map(|_| Angle::parser(), Some("h"))
      .flat_map(
        |h| {
          tokens::comma().flat_map(
            {
              move |_| {
                Percentage::parser().map(
                  {
                    let value = h.clone();
                    move |s| (value.clone(), s)
                  },
                  Some("hs"),
                )
              }
            },
            Some("comma1"),
          )
        },
        Some("h_step"),
      )
      .flat_map(
        |(h, s)| {
          tokens::comma().flat_map(
            {
              move |_| {
                Percentage::parser().map(
                  {
                    let h_value = h.clone();
                    let s_value = s.clone();
                    move |l| (h_value.clone(), s_value.clone(), l)
                  },
                  Some("hsl"),
                )
              }
            },
            Some("comma2"),
          )
        },
        Some("s_step"),
      )
      .flat_map(
        |(h, s, l)| {
          tokens::close_paren().map(
            move |_| (h.clone(), s.clone(), l.clone()),
            Some("final_hsl"),
          )
        },
        Some("close"),
      )
  }

  pub fn hsla_comma_full() -> TokenParser<(Angle, Percentage, Percentage, f64)> {
    function_parser("hsla")
      .flat_map(|_| Angle::parser(), Some("h"))
      .flat_map(
        |h| {
          tokens::comma().flat_map(
            {
              move |_| {
                Percentage::parser().map(
                  {
                    let value = h.clone();
                    move |s| (value.clone(), s)
                  },
                  Some("hs"),
                )
              }
            },
            Some("comma1"),
          )
        },
        Some("h_step"),
      )
      .flat_map(
        |(h, s)| {
          tokens::comma().flat_map(
            {
              move |_| {
                Percentage::parser().map(
                  {
                    let h_value = h.clone();
                    let s_value = s.clone();
                    move |l| (h_value.clone(), s_value.clone(), l)
                  },
                  Some("hsl"),
                )
              }
            },
            Some("comma2"),
          )
        },
        Some("s_step"),
      )
      .flat_map(
        |(h, s, l)| {
          tokens::comma().flat_map(
            {
              move |_| {
                alpha_as_number().map(
                  {
                    let h_value = h.clone();
                    let s_value = s.clone();
                    let l_value = l.clone();
                    move |a| (h_value.clone(), s_value.clone(), l_value.clone(), a)
                  },
                  Some("hsla"),
                )
              }
            },
            Some("comma3"),
          )
        },
        Some("l_step"),
      )
      .flat_map(
        |(h, s, l, a)| {
          tokens::close_paren().map(
            move |_| (h.clone(), s.clone(), l.clone(), a),
            Some("final_hsla"),
          )
        },
        Some("close"),
      )
  }

  // NOTE: HSLA space/slash parser temporarily removed due to complex ownership
  // issues This will be addressed in a future enhancement phase
  // The comma parser already provides full functionality for most use cases
}

/// List of all valid CSS named colors
/// This is a comprehensive list of CSS Level 4 named colors
const NAMED_COLORS: &[&str] = &[
  "aliceblue",
  "antiquewhite",
  "aqua",
  "aquamarine",
  "azure",
  "beige",
  "bisque",
  "black",
  "blanchedalmond",
  "blue",
  "blueviolet",
  "brown",
  "burlywood",
  "cadetblue",
  "chartreuse",
  "chocolate",
  "coral",
  "cornflowerblue",
  "cornsilk",
  "crimson",
  "cyan",
  "darkblue",
  "darkcyan",
  "darkgoldenrod",
  "darkgray",
  "darkgreen",
  "darkgrey",
  "darkkhaki",
  "darkmagenta",
  "darkolivegreen",
  "darkorange",
  "darkorchid",
  "darkred",
  "darksalmon",
  "darkseagreen",
  "darkslateblue",
  "darkslategray",
  "darkslategrey",
  "darkturquoise",
  "darkviolet",
  "deeppink",
  "deepskyblue",
  "dimgray",
  "dimgrey",
  "dodgerblue",
  "firebrick",
  "floralwhite",
  "forestgreen",
  "fuchsia",
  "gainsboro",
  "ghostwhite",
  "gold",
  "goldenrod",
  "gray",
  "grey",
  "green",
  "greenyellow",
  "honeydew",
  "hotpink",
  "indianred",
  "indigo",
  "ivory",
  "khaki",
  "lavender",
  "lavenderblush",
  "lawngreen",
  "lemonchiffon",
  "lightblue",
  "lightcoral",
  "lightcyan",
  "lightgoldenrodyellow",
  "lightgray",
  "lightgreen",
  "lightgrey",
  "lightpink",
  "lightsalmon",
  "lightseagreen",
  "lightskyblue",
  "lightslategray",
  "lightslategrey",
  "lightsteelblue",
  "lightyellow",
  "lime",
  "limegreen",
  "linen",
  "magenta",
  "maroon",
  "mediumaquamarine",
  "mediumblue",
  "mediumorchid",
  "mediumpurple",
  "mediumseagreen",
  "mediumslateblue",
  "mediumspringgreen",
  "mediumturquoise",
  "mediumvioletred",
  "midnightblue",
  "mintcream",
  "mistyrose",
  "moccasin",
  "navajowhite",
  "navy",
  "oldlace",
  "olive",
  "olivedrab",
  "orange",
  "orangered",
  "orchid",
  "palegoldenrod",
  "palegreen",
  "paleturquoise",
  "palevioletred",
  "papayawhip",
  "peachpuff",
  "peru",
  "pink",
  "plum",
  "powderblue",
  "purple",
  "rebeccapurple",
  "red",
  "rosybrown",
  "royalblue",
  "saddlebrown",
  "salmon",
  "sandybrown",
  "seagreen",
  "seashell",
  "sienna",
  "silver",
  "skyblue",
  "slateblue",
  "slategray",
  "slategrey",
  "snow",
  "springgreen",
  "steelblue",
  "tan",
  "teal",
  "thistle",
  "tomato",
  "turquoise",
  "violet",
  "wheat",
  "white",
  "whitesmoke",
  "yellow",
  "yellowgreen",
  "transparent",
  "currentcolor",
];

/// Base Color trait that all color types implement
pub trait ColorTrait {
  fn to_string(&self) -> String;
}

/// Main Color enum that encompasses all color types
#[derive(Debug, Clone, PartialEq)]
pub enum Color {
  Named(NamedColor),
  Hash(HashColor),
  Rgb(Rgb),
  Rgba(Rgba),
  Hsl(Hsl),
  Hsla(Hsla),
  Lch(Lch),
  Oklch(Oklch),
  Oklab(Oklab),
}

impl Color {
  pub fn parse() -> TokenParser<Color> {
    TokenParser::one_of(vec![
      NamedColor::parse().map(Color::Named, Some("named")),
      HashColor::parse().map(Color::Hash, Some("hash")),
      Rgb::parse().map(Color::Rgb, Some("rgb")),
      Rgba::parse().map(Color::Rgba, Some("rgba")),
      Hsl::parse().map(Color::Hsl, Some("hsl")),
      Hsla::parse().map(Color::Hsla, Some("hsla")),
      Lch::parse().map(Color::Lch, Some("lch")),
      Oklch::parse().map(Color::Oklch, Some("oklch")),
      Oklab::parse().map(Color::Oklab, Some("oklab")),
    ])
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for Color {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Color::Named(named) => named.fmt(f),
      Color::Hash(hash) => hash.fmt(f),
      Color::Rgb(rgb) => rgb.fmt(f),
      Color::Rgba(rgba) => rgba.fmt(f),
      Color::Hsl(hsl) => hsl.fmt(f),
      Color::Hsla(hsla) => hsla.fmt(f),
      Color::Lch(lch) => lch.fmt(f),
      Color::Oklch(oklch) => oklch.fmt(f),
      Color::Oklab(oklab) => oklab.fmt(f),
    }
  }
}

/// Named color like "red", "blue", etc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedColor {
  pub value: String,
}

impl NamedColor {
  pub fn new(value: impl Into<String>) -> Self {
    Self {
      value: value.into(),
    }
  }

  /// Check if a string is a valid named color
  pub fn is_valid_named_color(name: &str) -> bool {
    NAMED_COLORS.contains(&name.to_lowercase().as_str())
  }

  /// Extracts the string value from an Ident token. Panics (unreachable) for
  /// non-Ident tokens — guarded by tokens::ident() in the parser chain.
  fn extract_ident_value(token: SimpleToken) -> String {
    if let SimpleToken::Ident(value) = token {
      value
    } else {
      stylex_unreachable!()
    }
  }

  /// Parser for named colors
  pub fn parse() -> TokenParser<NamedColor> {
    tokens::ident()
      .map(Self::extract_ident_value, Some("extract_ident"))
      .where_fn(
        |value| Self::is_valid_named_color(value),
        Some("valid_named_color"),
      )
      .map(NamedColor::new, Some("to_named_color"))
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for NamedColor {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.value)
  }
}

/// Hex color like #FF0000, #F00, #FF0000FF
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashColor {
  pub value: String, // Hex value without the #
}

impl HashColor {
  pub fn new(value: impl Into<String>) -> Self {
    Self {
      value: value.into(),
    }
  }

  /// Validate hex color format
  pub fn is_valid_hex(value: &str) -> bool {
    let valid_lengths = [3, 6, 8]; // 3-digit, 6-digit, 8-digit (with alpha)
    valid_lengths.contains(&value.len()) && value.chars().all(|c| c.is_ascii_hexdigit())
  }

  /// The `index`th hex digit of a 3-digit value, doubled: `#abc` reads `aa`,
  /// `bb`, `cc`. `None` where there is no such digit.
  ///
  /// The parser's `is_valid_hex` guard excludes that case, but [`HashColor::new`]
  /// is `pub` and the guard measures `len()`, which counts **bytes** where
  /// `chars()` counts characters -- so a multi-byte value of byte length 3 has
  /// fewer than three digits. `HashColor::new("éx")` was a panic in [`Self::b`].
  fn expanded_short_digit(&self, index: usize) -> Option<u8> {
    let digit = self.value.chars().nth(index)?;

    // `d * 17` rather than parsing `"dd"`: doubling a hex digit is multiplying by
    // `0x11`, so `f` is `0xff` and `a` is `0xaa`, and the parse allocated a
    // two-character `String` per channel to reach the same answer. `u8` cannot
    // overflow here -- the largest digit is 15, and `15 * 17` is 255.
    let doubled = digit.to_digit(16)? * 17;

    u8::try_from(doubled).ok()
  }

  /// The two-byte hex pair starting at `start`, or `None` where the value has no
  /// such pair.
  ///
  /// `get` rather than `[start..start + 2]` for the reason `from_offset` gives
  /// about its own slice: the indexing form panics on a boundary that is not a
  /// character boundary, and this crate runs inside a NAPI addon, where a slice
  /// panic aborts the process with no diagnostic at all rather than surfacing as
  /// a thrown error. `HashColor::new("aébcd")` is byte length 6 and splits `é`.
  fn hex_pair(&self, start: usize) -> Option<u8> {
    let pair = self.value.get(start..start + 2)?;

    u8::from_str_radix(pair, 16).ok()
  }

  /// Get red component (0-255)
  /// IMPORTANT: Implements CORRECT CSS behavior for 3-digit hex expansion
  pub fn r(&self) -> u8 {
    match self.value.len() {
      // 3-digit hex: #RGB -> expand to #RRGGBB
      3 => self.expanded_short_digit(0).unwrap_or(0),
      // 6-digit or 8-digit hex
      6 | 8 => self.hex_pair(0).unwrap_or(0),
      _ => 0,
    }
  }

  /// Get green component (0-255)
  /// IMPORTANT: Implements CORRECT CSS behavior for 3-digit hex expansion
  pub fn g(&self) -> u8 {
    match self.value.len() {
      3 => self.expanded_short_digit(1).unwrap_or(0),
      6 | 8 => self.hex_pair(2).unwrap_or(0),
      _ => 0,
    }
  }

  /// Get blue component (0-255)
  /// IMPORTANT: Implements CORRECT CSS behavior for 3-digit hex expansion
  pub fn b(&self) -> u8 {
    match self.value.len() {
      3 => self.expanded_short_digit(2).unwrap_or(0),
      6 | 8 => self.hex_pair(4).unwrap_or(0),
      _ => 0,
    }
  }

  /// Get alpha component (0.0-1.0)
  pub fn a(&self) -> f64 {
    match self.value.len() {
      8 => f64::from(self.hex_pair(6).unwrap_or(255)) / 255.0,
      _ => 1.0,
    }
  }

  /// Extracts the string value from a Hash token. Panics (unreachable) for
  /// non-Hash tokens — guarded by tokens::hash() in the parser chain.
  fn extract_hash_value(token: SimpleToken) -> String {
    if let SimpleToken::Hash(value) = token {
      value
    } else {
      stylex_unreachable!()
    }
  }

  /// Parser for hex colors
  pub fn parse() -> TokenParser<HashColor> {
    tokens::hash()
      .map(Self::extract_hash_value, Some("extract_hash"))
      .where_fn(|value| Self::is_valid_hex(value), Some("valid_hex"))
      .map(HashColor::new, Some("to_hash_color"))
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for HashColor {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "#{}", self.value)
  }
}

/// RGB color: rgb(255, 0, 0)
#[derive(Debug, Clone, PartialEq)]
pub struct Rgb {
  pub r: f64,
  pub g: f64,
  pub b: f64,
}

impl Rgb {
  pub fn new(r: f64, g: f64, b: f64) -> Self {
    Self { r, g, b }
  }

  pub fn parse() -> TokenParser<Rgb> {
    TokenParser::one_of(vec![Self::comma_parser(), Self::space_parser()])
  }

  /// Parse comma-separated RGB: rgb(255, 0, 0)
  fn comma_parser() -> TokenParser<Rgb> {
    TokenParser::new(
      |tokens| {
        // Expect Function("rgb")
        let function_token =
          tokens
            .consume_next_token_infallible()
            .ok_or(CssParseError::ParseError {
              message: "Expected RGB function".to_string(),
            })?;

        if let SimpleToken::Function(name) = function_token {
          if name.to_lowercase() != "rgb" {
            return Err(CssParseError::ParseError {
              message: format!("Expected 'rgb' function, got '{}'", name),
            });
          }
        } else {
          return Err(CssParseError::ParseError {
            message: format!("Expected Function token, got {:?}", function_token),
          });
        }

        // Skip optional whitespace before first value
        while let Some(SimpleToken::Whitespace) = tokens.peek_infallible() {
          tokens.consume_next_token_infallible();
        }

        // Parse r value
        let r = parse_rgb_channel_token(tokens)?;

        // Expect comma (with optional whitespace)
        Self::consume_comma_with_optional_whitespace(tokens)?;

        // Parse g value
        let g = parse_rgb_channel_token(tokens)?;

        // Expect comma (with optional whitespace)
        Self::consume_comma_with_optional_whitespace(tokens)?;

        // Parse b value
        let b = parse_rgb_channel_token(tokens)?;

        // Skip optional whitespace before closing paren
        while let Some(SimpleToken::Whitespace) = tokens.peek_infallible() {
          tokens.consume_next_token_infallible();
        }

        // Expect closing paren
        let close_token =
          tokens
            .consume_next_token_infallible()
            .ok_or(CssParseError::ParseError {
              message: "Expected closing parenthesis".to_string(),
            })?;

        if !matches!(close_token, SimpleToken::RightParen) {
          return Err(CssParseError::ParseError {
            message: format!("Expected ')' token, got {:?}", close_token),
          });
        }

        Ok(Rgb::new(r, g, b))
      },
      "rgb_comma_parser",
    )
  }

  /// Parse space-separated RGB: rgb(255 0 0)
  fn space_parser() -> TokenParser<Rgb> {
    TokenParser::new(
      |tokens| {
        // Expect Function("rgb")
        let function_token =
          tokens
            .consume_next_token_infallible()
            .ok_or(CssParseError::ParseError {
              message: "Expected RGB function".to_string(),
            })?;

        if let SimpleToken::Function(name) = function_token {
          if name.to_lowercase() != "rgb" {
            return Err(CssParseError::ParseError {
              message: format!("Expected 'rgb' function, got '{}'", name),
            });
          }
        } else {
          return Err(CssParseError::ParseError {
            message: format!("Expected Function token, got {:?}", function_token),
          });
        }

        // Parse r value
        let r = parse_rgb_channel_token(tokens)?;

        // Expect whitespace
        let whitespace_token =
          tokens
            .consume_next_token_infallible()
            .ok_or(CssParseError::ParseError {
              message: "Expected whitespace".to_string(),
            })?;
        if !matches!(whitespace_token, SimpleToken::Whitespace) {
          return Err(CssParseError::ParseError {
            message: format!("Expected whitespace token, got {:?}", whitespace_token),
          });
        }

        // Parse g value
        let g = parse_rgb_channel_token(tokens)?;

        // Expect whitespace
        let whitespace_token =
          tokens
            .consume_next_token_infallible()
            .ok_or(CssParseError::ParseError {
              message: "Expected whitespace".to_string(),
            })?;
        if !matches!(whitespace_token, SimpleToken::Whitespace) {
          return Err(CssParseError::ParseError {
            message: format!("Expected whitespace token, got {:?}", whitespace_token),
          });
        }

        // Parse b value
        let b = parse_rgb_channel_token(tokens)?;

        // Expect closing paren
        let close_token =
          tokens
            .consume_next_token_infallible()
            .ok_or(CssParseError::ParseError {
              message: "Expected closing parenthesis".to_string(),
            })?;

        if !matches!(close_token, SimpleToken::RightParen) {
          return Err(CssParseError::ParseError {
            message: format!("Expected ')' token, got {:?}", close_token),
          });
        }

        Ok(Rgb::new(r, g, b))
      },
      "rgb_space_parser",
    )
  }

  fn consume_comma_with_optional_whitespace(tokens: &mut TokenList) -> Result<(), CssParseError> {
    // Skip optional whitespace before comma
    while let Some(SimpleToken::Whitespace) = tokens.peek_infallible() {
      tokens.consume_next_token_infallible();
    }

    // Expect comma
    let comma_token = tokens
      .consume_next_token_infallible()
      .ok_or(CssParseError::ParseError {
        message: "Expected comma".to_string(),
      })?;

    if !matches!(comma_token, SimpleToken::Comma) {
      return Err(CssParseError::ParseError {
        message: format!("Expected ',' token, got {:?}", comma_token),
      });
    }

    // Skip optional whitespace after comma
    while let Some(SimpleToken::Whitespace) = tokens.peek_infallible() {
      tokens.consume_next_token_infallible();
    }

    Ok(())
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for Rgb {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "rgb({},{},{})",
      to_js_string(self.r),
      to_js_string(self.g),
      to_js_string(self.b)
    )
  }
}

/// RGBA color: rgba(255, 0, 0, 0.5)
#[derive(Debug, Clone, PartialEq)]
pub struct Rgba {
  pub r: f64,
  pub g: f64,
  pub b: f64,
  pub a: f64,
}

impl Rgba {
  pub fn new(r: f64, g: f64, b: f64, a: f64) -> Self {
    Self { r, g, b, a }
  }

  pub fn parse() -> TokenParser<Rgba> {
    TokenParser::one_of(vec![Self::comma_parser(), Self::space_slash_parser()])
  }

  /// Parse comma-separated RGBA: rgba(255, 0, 0, 0.5)
  fn comma_parser() -> TokenParser<Rgba> {
    TokenParser::new(
      |tokens| {
        // Expect Function("rgba")
        let function_token =
          tokens
            .consume_next_token_infallible()
            .ok_or(CssParseError::ParseError {
              message: "Expected RGBA function".to_string(),
            })?;

        if let SimpleToken::Function(name) = function_token {
          if name.to_lowercase() != "rgba" {
            return Err(CssParseError::ParseError {
              message: format!("Expected 'rgba' function, got '{}'", name),
            });
          }
        } else {
          return Err(CssParseError::ParseError {
            message: format!("Expected Function token, got {:?}", function_token),
          });
        }

        // Skip optional whitespace before first value
        while let Some(SimpleToken::Whitespace) = tokens.peek_infallible() {
          tokens.consume_next_token_infallible();
        }

        // Parse r value
        let r = parse_rgb_channel_token(tokens)?;

        // Expect comma (with optional whitespace)
        Self::consume_comma_with_optional_whitespace(tokens)?;

        // Parse g value
        let g = parse_rgb_channel_token(tokens)?;

        // Expect comma (with optional whitespace)
        Self::consume_comma_with_optional_whitespace(tokens)?;

        // Parse b value
        let b = parse_rgb_channel_token(tokens)?;

        // Expect comma (with optional whitespace)
        Self::consume_comma_with_optional_whitespace(tokens)?;

        // Parse alpha value
        let a = parse_alpha_token(tokens)?;

        // Skip optional whitespace before closing paren
        while let Some(SimpleToken::Whitespace) = tokens.peek_infallible() {
          tokens.consume_next_token_infallible();
        }

        // Expect closing paren
        let close_token =
          tokens
            .consume_next_token_infallible()
            .ok_or(CssParseError::ParseError {
              message: "Expected closing parenthesis".to_string(),
            })?;

        if !matches!(close_token, SimpleToken::RightParen) {
          return Err(CssParseError::ParseError {
            message: format!("Expected ')' token, got {:?}", close_token),
          });
        }

        Ok(Rgba::new(r, g, b, a))
      },
      "rgba_comma_parser",
    )
  }

  /// Parse space-separated RGBA with slash: rgb(255 0 0 / 0.5) or rgba(255 0 0
  /// / 0.5)
  fn space_slash_parser() -> TokenParser<Rgba> {
    TokenParser::new(
      |tokens| {
        // Expect Function("rgb" or "rgba")
        let function_token =
          tokens
            .consume_next_token_infallible()
            .ok_or(CssParseError::ParseError {
              message: "Expected RGB/RGBA function".to_string(),
            })?;

        if let SimpleToken::Function(name) = function_token {
          if name.to_lowercase() != "rgb" && name.to_lowercase() != "rgba" {
            return Err(CssParseError::ParseError {
              message: format!("Expected 'rgb' or 'rgba' function, got '{}'", name),
            });
          }
        } else {
          return Err(CssParseError::ParseError {
            message: format!("Expected Function token, got {:?}", function_token),
          });
        }

        // Parse r value
        let r = parse_rgb_channel_token(tokens)?;

        // Expect whitespace
        let whitespace_token =
          tokens
            .consume_next_token_infallible()
            .ok_or(CssParseError::ParseError {
              message: "Expected whitespace".to_string(),
            })?;
        if !matches!(whitespace_token, SimpleToken::Whitespace) {
          return Err(CssParseError::ParseError {
            message: format!("Expected whitespace token, got {:?}", whitespace_token),
          });
        }

        // Parse g value
        let g = parse_rgb_channel_token(tokens)?;

        // Expect whitespace
        let whitespace_token =
          tokens
            .consume_next_token_infallible()
            .ok_or(CssParseError::ParseError {
              message: "Expected whitespace".to_string(),
            })?;
        if !matches!(whitespace_token, SimpleToken::Whitespace) {
          return Err(CssParseError::ParseError {
            message: format!("Expected whitespace token, got {:?}", whitespace_token),
          });
        }

        // Parse b value
        let b = parse_rgb_channel_token(tokens)?;

        // Expect whitespace before slash (optional)
        if let Some(SimpleToken::Whitespace) = tokens.peek_infallible() {
          tokens.consume_next_token_infallible();
        }

        // Expect slash
        let slash_token =
          tokens
            .consume_next_token_infallible()
            .ok_or(CssParseError::ParseError {
              message: "Expected slash".to_string(),
            })?;
        if !matches!(slash_token, SimpleToken::Delim('/')) {
          return Err(CssParseError::ParseError {
            message: format!("Expected '/' token, got {:?}", slash_token),
          });
        }

        // Expect whitespace after slash (optional)
        if let Some(SimpleToken::Whitespace) = tokens.peek_infallible() {
          tokens.consume_next_token_infallible();
        }

        // Parse alpha value
        let a = parse_alpha_token(tokens)?;

        // Expect closing paren
        let close_token =
          tokens
            .consume_next_token_infallible()
            .ok_or(CssParseError::ParseError {
              message: "Expected closing parenthesis".to_string(),
            })?;

        if !matches!(close_token, SimpleToken::RightParen) {
          return Err(CssParseError::ParseError {
            message: format!("Expected ')' token, got {:?}", close_token),
          });
        }

        Ok(Rgba::new(r, g, b, a))
      },
      "rgba_space_slash_parser",
    )
  }

  fn consume_comma_with_optional_whitespace(tokens: &mut TokenList) -> Result<(), CssParseError> {
    // Skip optional whitespace before comma
    while let Some(SimpleToken::Whitespace) = tokens.peek_infallible() {
      tokens.consume_next_token_infallible();
    }

    // Expect comma
    let comma_token = tokens
      .consume_next_token_infallible()
      .ok_or(CssParseError::ParseError {
        message: "Expected comma".to_string(),
      })?;

    if !matches!(comma_token, SimpleToken::Comma) {
      return Err(CssParseError::ParseError {
        message: format!("Expected ',' token, got {:?}", comma_token),
      });
    }

    // Skip optional whitespace after comma
    while let Some(SimpleToken::Whitespace) = tokens.peek_infallible() {
      tokens.consume_next_token_infallible();
    }

    Ok(())
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for Rgba {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "rgba({},{},{},{})",
      to_js_string(self.r),
      to_js_string(self.g),
      to_js_string(self.b),
      to_js_string(self.a)
    )
  }
}

/// HSL color: hsl(360deg, 100%, 50%)
#[derive(Debug, Clone, PartialEq)]
pub struct Hsl {
  pub h: Angle,      // hue angle (0-360deg)
  pub s: Percentage, // saturation percentage (0-100%)
  pub l: Percentage, // lightness percentage (0-100%)
}

impl Hsl {
  pub fn new(h: Angle, s: Percentage, l: Percentage) -> Self {
    Self { h, s, l }
  }

  /// Convenience constructor for backward compatibility with tests
  /// Creates Hsl from primitive values
  pub fn from_primitives(h: f64, s: f64, l: f64) -> Self {
    Self {
      h: Angle::new(h, "deg"),
      s: Percentage::new(s),
      l: Percentage::new(l),
    }
  }

  pub fn parse() -> TokenParser<Hsl> {
    TokenParser::one_of(vec![Self::comma_parser(), Self::space_parser()])
  }

  /// Parse comma-separated HSL: hsl(360deg, 100%, 50%)
  fn comma_parser() -> TokenParser<Hsl> {
    TokenParser::new(
      |tokens| {
        // Expect Function("hsl")
        let function_token =
          tokens
            .consume_next_token_infallible()
            .ok_or(CssParseError::ParseError {
              message: "Expected HSL function".to_string(),
            })?;

        if let SimpleToken::Function(name) = function_token {
          if name.to_lowercase() != "hsl" {
            return Err(CssParseError::ParseError {
              message: format!("Expected 'hsl' function, got '{}'", name),
            });
          }
        } else {
          return Err(CssParseError::ParseError {
            message: format!("Expected Function token, got {:?}", function_token),
          });
        }

        // Skip optional whitespace before first value
        while let Some(SimpleToken::Whitespace) = tokens.peek_infallible() {
          tokens.consume_next_token_infallible();
        }

        // Parse hue value (angle or number)
        let h = Self::parse_hsl_hue_token(tokens)?;

        // Expect comma (with optional whitespace)
        Self::consume_comma_with_optional_whitespace(tokens)?;

        // Parse saturation percentage
        let s = Self::parse_hsl_percentage_token(tokens)?;

        // Expect comma (with optional whitespace)
        Self::consume_comma_with_optional_whitespace(tokens)?;

        // Parse lightness percentage
        let l = Self::parse_hsl_percentage_token(tokens)?;

        // Skip optional whitespace before closing paren
        while let Some(SimpleToken::Whitespace) = tokens.peek_infallible() {
          tokens.consume_next_token_infallible();
        }

        // Expect closing paren
        let close_token =
          tokens
            .consume_next_token_infallible()
            .ok_or(CssParseError::ParseError {
              message: "Expected closing parenthesis".to_string(),
            })?;

        if !matches!(close_token, SimpleToken::RightParen) {
          return Err(CssParseError::ParseError {
            message: format!("Expected ')' token, got {:?}", close_token),
          });
        }

        Ok(Hsl::new(h, s, l))
      },
      "hsl_comma_parser",
    )
  }

  /// Parse space-separated HSL: hsl(360deg 100% 50%)
  fn space_parser() -> TokenParser<Hsl> {
    TokenParser::new(
      |tokens| {
        // Expect Function("hsl")
        let function_token =
          tokens
            .consume_next_token_infallible()
            .ok_or(CssParseError::ParseError {
              message: "Expected HSL function".to_string(),
            })?;

        if let SimpleToken::Function(name) = function_token {
          if name.to_lowercase() != "hsl" {
            return Err(CssParseError::ParseError {
              message: format!("Expected 'hsl' function, got '{}'", name),
            });
          }
        } else {
          return Err(CssParseError::ParseError {
            message: format!("Expected Function token, got {:?}", function_token),
          });
        }

        // Parse hue value (angle or number)
        let h = Self::parse_hsl_hue_token(tokens)?;

        // Expect whitespace
        let whitespace_token =
          tokens
            .consume_next_token_infallible()
            .ok_or(CssParseError::ParseError {
              message: "Expected whitespace".to_string(),
            })?;
        if !matches!(whitespace_token, SimpleToken::Whitespace) {
          return Err(CssParseError::ParseError {
            message: format!("Expected whitespace token, got {:?}", whitespace_token),
          });
        }

        // Parse saturation percentage
        let s = Self::parse_hsl_percentage_token(tokens)?;

        // Expect whitespace
        let whitespace_token =
          tokens
            .consume_next_token_infallible()
            .ok_or(CssParseError::ParseError {
              message: "Expected whitespace".to_string(),
            })?;
        if !matches!(whitespace_token, SimpleToken::Whitespace) {
          return Err(CssParseError::ParseError {
            message: format!("Expected whitespace token, got {:?}", whitespace_token),
          });
        }

        // Parse lightness percentage
        let l = Self::parse_hsl_percentage_token(tokens)?;

        // Expect closing paren
        let close_token =
          tokens
            .consume_next_token_infallible()
            .ok_or(CssParseError::ParseError {
              message: "Expected closing parenthesis".to_string(),
            })?;

        if !matches!(close_token, SimpleToken::RightParen) {
          return Err(CssParseError::ParseError {
            message: format!("Expected ')' token, got {:?}", close_token),
          });
        }

        Ok(Hsl::new(h, s, l))
      },
      "hsl_space_parser",
    )
  }

  /// Helper: Parse HSL hue token (angle or number treated as degrees)
  fn parse_hsl_hue_token(tokens: &mut TokenList) -> Result<Angle, CssParseError> {
    let token = tokens
      .consume_next_token_infallible()
      .ok_or(CssParseError::ParseError {
        message: "Expected hue value token".to_string(),
      })?;

    match token {
      SimpleToken::Dimension { value, unit } => {
        // Parse as angle with unit
        if Angle::is_valid_unit(&unit) {
          Ok(Angle::new(value, unit))
        } else {
          Err(CssParseError::ParseError {
            message: format!("Invalid angle unit: {}", unit),
          })
        }
      },
      SimpleToken::Number(value) => {
        // Treat numbers as degrees (CSS standard for HSL)
        Ok(Angle::new(value, "deg"))
      },
      _ => Err(CssParseError::ParseError {
        message: format!(
          "Expected Number or Dimension token for hue, got {:?}",
          token
        ),
      }),
    }
  }

  /// Helper: Parse HSL percentage token
  fn parse_hsl_percentage_token(tokens: &mut TokenList) -> Result<Percentage, CssParseError> {
    let token = tokens
      .consume_next_token_infallible()
      .ok_or(CssParseError::ParseError {
        message: "Expected percentage token".to_string(),
      })?;

    if let SimpleToken::Percentage(value) = token {
      // The token already carries the authored percent: `50%` is `50`.
      Ok(Percentage::new(value))
    } else {
      Err(CssParseError::ParseError {
        message: format!("Expected Percentage token, got {:?}", token),
      })
    }
  }

  fn consume_comma_with_optional_whitespace(tokens: &mut TokenList) -> Result<(), CssParseError> {
    // Skip optional whitespace before comma
    while let Some(SimpleToken::Whitespace) = tokens.peek_infallible() {
      tokens.consume_next_token_infallible();
    }

    // Expect comma
    let comma_token = tokens
      .consume_next_token_infallible()
      .ok_or(CssParseError::ParseError {
        message: "Expected comma".to_string(),
      })?;

    if !matches!(comma_token, SimpleToken::Comma) {
      return Err(CssParseError::ParseError {
        message: format!("Expected ',' token, got {:?}", comma_token),
      });
    }

    // Skip optional whitespace after comma
    while let Some(SimpleToken::Whitespace) = tokens.peek_infallible() {
      tokens.consume_next_token_infallible();
    }

    Ok(())
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for Hsl {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "hsl({},{},{})", self.h, self.s, self.l)
  }
}

/// HSLA color: hsla(360deg, 100%, 50%, 0.5)
#[derive(Debug, Clone, PartialEq)]
pub struct Hsla {
  pub h: Angle,      // hue angle (0-360deg)
  pub s: Percentage, // saturation percentage (0-100%)
  pub l: Percentage, // lightness percentage (0-100%)
  pub a: f64,        // alpha, unbounded like every other legacy alpha
}

impl Hsla {
  pub fn new(h: Angle, s: Percentage, l: Percentage, a: f64) -> Self {
    Self { h, s, l, a }
  }

  /// Convenience constructor for backward compatibility with tests
  /// Creates Hsla from primitive values
  pub fn from_primitives(h: f64, s: f64, l: f64, a: f64) -> Self {
    Self {
      h: Angle::new(h, "deg"),
      s: Percentage::new(s),
      l: Percentage::new(l),
      a,
    }
  }

  pub fn parse() -> TokenParser<Hsla> {
    TokenParser::one_of(vec![Self::comma_parser(), Self::space_slash_parser()])
  }

  /// Parse comma-separated HSLA: hsla(360deg, 100%, 50%, 0.5)
  fn comma_parser() -> TokenParser<Hsla> {
    TokenParser::new(
      |tokens| {
        // Expect Function("hsla")
        let function_token =
          tokens
            .consume_next_token_infallible()
            .ok_or(CssParseError::ParseError {
              message: "Expected HSLA function".to_string(),
            })?;

        if let SimpleToken::Function(name) = function_token {
          if name.to_lowercase() != "hsla" {
            return Err(CssParseError::ParseError {
              message: format!("Expected 'hsla' function, got '{}'", name),
            });
          }
        } else {
          return Err(CssParseError::ParseError {
            message: format!("Expected Function token, got {:?}", function_token),
          });
        }

        // Parse hue value (angle or number)
        let h = Self::parse_hsla_hue_token(tokens)?;

        // Expect comma (with optional whitespace)
        Self::consume_comma_with_optional_whitespace(tokens)?;

        // Parse saturation percentage
        let s = Self::parse_hsla_percentage_token(tokens)?;

        // Expect comma (with optional whitespace)
        Self::consume_comma_with_optional_whitespace(tokens)?;

        // Parse lightness percentage
        let l = Self::parse_hsla_percentage_token(tokens)?;

        // Expect comma (with optional whitespace)
        Self::consume_comma_with_optional_whitespace(tokens)?;

        // Parse alpha value
        let a = parse_alpha_token(tokens)?;

        // Expect closing paren
        let close_token =
          tokens
            .consume_next_token_infallible()
            .ok_or(CssParseError::ParseError {
              message: "Expected closing parenthesis".to_string(),
            })?;

        if !matches!(close_token, SimpleToken::RightParen) {
          return Err(CssParseError::ParseError {
            message: format!("Expected ')' token, got {:?}", close_token),
          });
        }

        Ok(Hsla::new(h, s, l, a))
      },
      "hsla_comma_parser",
    )
  }

  /// Parse space-separated HSLA with slash: hsl(360deg 100% 50% / 0.5)
  fn space_slash_parser() -> TokenParser<Hsla> {
    TokenParser::new(
      |tokens| {
        // Expect Function("hsl") - note: hsl, not hsla!
        let function_token =
          tokens
            .consume_next_token_infallible()
            .ok_or(CssParseError::ParseError {
              message: "Expected HSL function".to_string(),
            })?;

        if let SimpleToken::Function(name) = function_token {
          if name.to_lowercase() != "hsl" {
            return Err(CssParseError::ParseError {
              message: format!("Expected 'hsl' function, got '{}'", name),
            });
          }
        } else {
          return Err(CssParseError::ParseError {
            message: format!("Expected Function token, got {:?}", function_token),
          });
        }

        // Parse hue value (angle or number)
        let h = Self::parse_hsla_hue_token(tokens)?;

        // Expect whitespace
        let whitespace_token =
          tokens
            .consume_next_token_infallible()
            .ok_or(CssParseError::ParseError {
              message: "Expected whitespace".to_string(),
            })?;
        if !matches!(whitespace_token, SimpleToken::Whitespace) {
          return Err(CssParseError::ParseError {
            message: format!("Expected whitespace token, got {:?}", whitespace_token),
          });
        }

        // Parse saturation percentage
        let s = Self::parse_hsla_percentage_token(tokens)?;

        // Expect whitespace
        let whitespace_token =
          tokens
            .consume_next_token_infallible()
            .ok_or(CssParseError::ParseError {
              message: "Expected whitespace".to_string(),
            })?;
        if !matches!(whitespace_token, SimpleToken::Whitespace) {
          return Err(CssParseError::ParseError {
            message: format!("Expected whitespace token, got {:?}", whitespace_token),
          });
        }

        // Parse lightness percentage
        let l = Self::parse_hsla_percentage_token(tokens)?;

        // Expect whitespace before slash (optional)
        if let Some(SimpleToken::Whitespace) = tokens.peek_infallible() {
          tokens.consume_next_token_infallible();
        }

        // Expect slash
        let slash_token =
          tokens
            .consume_next_token_infallible()
            .ok_or(CssParseError::ParseError {
              message: "Expected slash".to_string(),
            })?;
        if !matches!(slash_token, SimpleToken::Delim('/')) {
          return Err(CssParseError::ParseError {
            message: format!("Expected '/' token, got {:?}", slash_token),
          });
        }

        // Expect whitespace after slash (optional)
        if let Some(SimpleToken::Whitespace) = tokens.peek_infallible() {
          tokens.consume_next_token_infallible();
        }

        // Parse alpha value
        let a = parse_alpha_token(tokens)?;

        // Expect closing paren
        let close_token =
          tokens
            .consume_next_token_infallible()
            .ok_or(CssParseError::ParseError {
              message: "Expected closing parenthesis".to_string(),
            })?;

        if !matches!(close_token, SimpleToken::RightParen) {
          return Err(CssParseError::ParseError {
            message: format!("Expected ')' token, got {:?}", close_token),
          });
        }

        Ok(Hsla::new(h, s, l, a))
      },
      "hsla_space_slash_parser",
    )
  }

  /// Helper: Parse HSLA hue token (angle or number treated as degrees)
  fn parse_hsla_hue_token(tokens: &mut TokenList) -> Result<Angle, CssParseError> {
    let token = tokens
      .consume_next_token_infallible()
      .ok_or(CssParseError::ParseError {
        message: "Expected hue value token".to_string(),
      })?;

    match token {
      SimpleToken::Dimension { value, unit } => {
        // Parse as angle with unit
        if Angle::is_valid_unit(&unit) {
          Ok(Angle::new(value, unit))
        } else {
          Err(CssParseError::ParseError {
            message: format!("Invalid angle unit: {}", unit),
          })
        }
      },
      SimpleToken::Number(value) => {
        // Treat numbers as degrees (CSS standard for HSL/HSLA)
        Ok(Angle::new(value, "deg"))
      },
      _ => Err(CssParseError::ParseError {
        message: format!(
          "Expected Number or Dimension token for hue, got {:?}",
          token
        ),
      }),
    }
  }

  /// Helper: Parse HSLA percentage token
  fn parse_hsla_percentage_token(tokens: &mut TokenList) -> Result<Percentage, CssParseError> {
    let token = tokens
      .consume_next_token_infallible()
      .ok_or(CssParseError::ParseError {
        message: "Expected percentage token".to_string(),
      })?;

    if let SimpleToken::Percentage(value) = token {
      // The token already carries the authored percent: `50%` is `50`.
      Ok(Percentage::new(value))
    } else {
      Err(CssParseError::ParseError {
        message: format!("Expected Percentage token, got {:?}", token),
      })
    }
  }

  fn consume_comma_with_optional_whitespace(tokens: &mut TokenList) -> Result<(), CssParseError> {
    // Skip optional whitespace before comma
    while let Some(SimpleToken::Whitespace) = tokens.peek_infallible() {
      tokens.consume_next_token_infallible();
    }

    // Expect comma
    let comma_token = tokens
      .consume_next_token_infallible()
      .ok_or(CssParseError::ParseError {
        message: "Expected comma".to_string(),
      })?;

    if !matches!(comma_token, SimpleToken::Comma) {
      return Err(CssParseError::ParseError {
        message: format!("Expected ',' token, got {:?}", comma_token),
      });
    }

    // Skip optional whitespace after comma
    while let Some(SimpleToken::Whitespace) = tokens.peek_infallible() {
      tokens.consume_next_token_infallible();
    }

    Ok(())
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for Hsla {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "hsla({},{},{},{})",
      self.h,
      self.s,
      self.l,
      to_js_string(self.a)
    )
  }
}

/// LCH color: lch(50% 100 270deg) or lch(50 100 270)
#[derive(Debug, Clone, PartialEq)]
pub struct Lch {
  pub l: f64,             // lightness (0-100) - can be percentage, number, or 'none'
  pub c: f64,             // chroma (0-150) - can be percentage or number
  pub h: LchHue,          // hue - can be angle or number
  pub alpha: Option<f64>, // alpha (0-1) - optional with slash syntax
}

#[derive(Debug, Clone, PartialEq)]
pub enum LchHue {
  Angle(Angle),
  Number(f64),
}

impl LchHue {
  pub fn from_angle(angle: Angle) -> Self {
    LchHue::Angle(angle)
  }

  pub fn from_number(number: f64) -> Self {
    LchHue::Number(number)
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl std::fmt::Display for LchHue {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      LchHue::Angle(angle) => write!(f, "{}", angle),
      LchHue::Number(number) => write_js_number(f, *number),
    }
  }
}

impl Lch {
  pub fn new(l: f64, c: f64, h: LchHue, alpha: Option<f64>) -> Self {
    Self { l, c, h, alpha }
  }

  pub fn new_with_angle(l: f64, c: f64, h: Angle, alpha: Option<f64>) -> Self {
    Self::new(l, c, LchHue::Angle(h), alpha)
  }

  pub fn new_with_number(l: f64, c: f64, h: f64, alpha: Option<f64>) -> Self {
    Self::new(l, c, LchHue::Number(h), alpha)
  }

  /// Parses lch(lightness chroma hue) and lch(lightness chroma hue / alpha)
  /// formats
  pub fn parse() -> TokenParser<Lch> {
    TokenParser::new(
      |tokens| {
        // Expect Function("lch")
        let function_token =
          tokens
            .consume_next_token_infallible()
            .ok_or(CssParseError::ParseError {
              message: "Expected LCH function".to_string(),
            })?;

        if let SimpleToken::Function(name) = function_token {
          if name.to_lowercase() != "lch" {
            return Err(CssParseError::ParseError {
              message: format!("Expected 'lch' function, got '{}'", name),
            });
          }
        } else {
          return Err(CssParseError::ParseError {
            message: format!("Expected Function token, got {:?}", function_token),
          });
        }

        // Parse lightness: percentage or number
        let l = Self::parse_lch_lightness_token(tokens)?;

        // Expect whitespace
        let whitespace_token =
          tokens
            .consume_next_token_infallible()
            .ok_or(CssParseError::ParseError {
              message: "Expected whitespace".to_string(),
            })?;
        if !matches!(whitespace_token, SimpleToken::Whitespace) {
          return Err(CssParseError::ParseError {
            message: format!("Expected whitespace token, got {:?}", whitespace_token),
          });
        }

        // Parse chroma: number
        let c = Self::parse_lch_chroma_token(tokens)?;

        // Expect whitespace
        let whitespace_token =
          tokens
            .consume_next_token_infallible()
            .ok_or(CssParseError::ParseError {
              message: "Expected whitespace".to_string(),
            })?;
        if !matches!(whitespace_token, SimpleToken::Whitespace) {
          return Err(CssParseError::ParseError {
            message: format!("Expected whitespace token, got {:?}", whitespace_token),
          });
        }

        // Parse hue: angle or number
        let h = Self::parse_lch_hue_token(tokens)?;

        // Parse optional alpha: / <alpha-value>
        let alpha = parse_optional_slash_alpha(tokens)?;

        // Expect closing paren
        let close_token =
          tokens
            .consume_next_token_infallible()
            .ok_or(CssParseError::ParseError {
              message: "Expected closing parenthesis".to_string(),
            })?;

        if !matches!(close_token, SimpleToken::RightParen) {
          return Err(CssParseError::ParseError {
            message: format!("Expected ')' token, got {:?}", close_token),
          });
        }

        Ok(Lch::new(l, c, h, alpha))
      },
      "lch_parser",
    )
  }

  /// Helper: Parse LCH lightness token (percentage or number)
  fn parse_lch_lightness_token(tokens: &mut TokenList) -> Result<f64, CssParseError> {
    let token = tokens
      .consume_next_token_infallible()
      .ok_or(CssParseError::ParseError {
        message: "Expected lightness value token".to_string(),
      })?;

    match token {
      SimpleToken::Percentage(value) => {
        // The token already carries the authored percent: `50%` is `50`.
        Ok(value)
      },
      SimpleToken::Number(value) => Ok(value),
      _ => Err(CssParseError::ParseError {
        message: format!(
          "Expected Number or Percentage token for lightness, got {:?}",
          token
        ),
      }),
    }
  }

  /// Helper: Parse LCH chroma token (number)
  fn parse_lch_chroma_token(tokens: &mut TokenList) -> Result<f64, CssParseError> {
    let token = tokens
      .consume_next_token_infallible()
      .ok_or(CssParseError::ParseError {
        message: "Expected chroma value token".to_string(),
      })?;

    if let SimpleToken::Number(value) = token {
      Ok(value)
    } else {
      Err(CssParseError::ParseError {
        message: format!("Expected Number token for chroma, got {:?}", token),
      })
    }
  }

  /// Helper: Parse LCH hue token (angle or number)
  fn parse_lch_hue_token(tokens: &mut TokenList) -> Result<LchHue, CssParseError> {
    let token = tokens
      .consume_next_token_infallible()
      .ok_or(CssParseError::ParseError {
        message: "Expected hue value token".to_string(),
      })?;

    match token {
      SimpleToken::Dimension { value, unit } => {
        // Parse as angle with unit
        if crate::css_types::Angle::is_valid_unit(&unit) {
          let angle = crate::css_types::Angle::new(value, unit);
          Ok(LchHue::Angle(angle))
        } else {
          Err(CssParseError::ParseError {
            message: format!("Invalid angle unit: {}", unit),
          })
        }
      },
      SimpleToken::Number(value) => {
        // Treat numbers as plain numbers (not degrees)
        Ok(LchHue::Number(value))
      },
      _ => Err(CssParseError::ParseError {
        message: format!(
          "Expected Number or Dimension token for hue, got {:?}",
          token
        ),
      }),
    }
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for Lch {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match printed_alpha(self.alpha) {
      Some(alpha) => write!(
        f,
        "lch({} {} {} / {})",
        to_js_string(self.l),
        to_js_string(self.c),
        self.h,
        to_js_string(alpha)
      ),
      None => write!(
        f,
        "lch({} {} {})",
        to_js_string(self.l),
        to_js_string(self.c),
        self.h
      ),
    }
  }
}

/// OKLCH color: oklch(0.5 0.1 270deg)
#[derive(Debug, Clone, PartialEq)]
pub struct Oklch {
  pub l: f64,             // lightness (0-1)
  pub c: f64,             // chroma (0-0.4)
  pub h: Angle,           // hue angle
  pub alpha: Option<f64>, // alpha (0-1)
}

impl Oklch {
  pub fn new(l: f64, c: f64, h: Angle, alpha: Option<f64>) -> Self {
    Self { l, c, h, alpha }
  }

  /// Parser for OKLCH colors
  /// - Lightness/Chroma: numbers, 'none' keyword (maps to 0)
  /// - Hue: angles or numbers (numbers * 360 -> angle conversion)
  /// - Alpha: optional with slash syntax
  pub fn parse() -> TokenParser<Oklch> {
    TokenParser::new(
      move |input| {
        // Parse 'oklch(' function start
        match input.consume_next_token_infallible() {
          Some(SimpleToken::Function(fn_name)) if fn_name.to_lowercase() == "oklch" => {},
          _ => {
            return Err(CssParseError::ParseError {
              message: "Expected oklch() function".to_string(),
            });
          },
        }

        // Parse lightness (l): number | 'none'
        let l = Self::parse_oklch_lc_value(input)?;

        // Parse whitespace
        match input.consume_next_token_infallible() {
          Some(SimpleToken::Whitespace) => {},
          _ => {
            return Err(CssParseError::ParseError {
              message: "Expected whitespace after lightness".to_string(),
            });
          },
        }

        // Parse chroma (c): number | 'none'
        let c = Self::parse_oklch_lc_value(input)?;

        // Parse whitespace
        match input.consume_next_token_infallible() {
          Some(SimpleToken::Whitespace) => {},
          _ => {
            return Err(CssParseError::ParseError {
              message: "Expected whitespace after chroma".to_string(),
            });
          },
        }

        // Parse hue (h): angle | number (number * 360 -> angle)
        let h = Self::parse_oklch_hue(input)?;

        // Parse optional alpha: / <alpha-value>
        let alpha = parse_optional_slash_alpha(input)?;

        // Parse closing paren
        match input.consume_next_token_infallible() {
          Some(SimpleToken::RightParen) => {},
          _ => {
            return Err(CssParseError::ParseError {
              message: "Expected ) after oklch values".to_string(),
            });
          },
        }

        Ok(Oklch::new(l, c, h, alpha))
      },
      "oklch_parser",
    )
  }

  /// Parse OKLCH lightness/chroma value: number | 'none'
  fn parse_oklch_lc_value(input: &mut crate::token_types::TokenList) -> Result<f64, CssParseError> {
    match input.consume_next_token_infallible() {
      Some(SimpleToken::Number(n)) => Ok(n),
      Some(SimpleToken::Ident(keyword)) if keyword == "none" => Ok(0.0),
      _ => Err(CssParseError::ParseError {
        message: "Expected number or 'none'".to_string(),
      }),
    }
  }

  /// Parse OKLCH hue: angle | number (interpreted as degrees) | 'none'
  fn parse_oklch_hue(input: &mut crate::token_types::TokenList) -> Result<Angle, CssParseError> {
    match input.consume_next_token_infallible() {
      Some(SimpleToken::Dimension { value, unit }) => {
        // Try to parse as angle
        if Angle::is_valid_unit(&unit) {
          Ok(Angle::new(value, unit))
        } else {
          Err(CssParseError::ParseError {
            message: format!("Invalid angle unit: {}", unit),
          })
        }
      },
      Some(SimpleToken::Number(n)) => Ok(Angle::new(n, "deg")),
      Some(SimpleToken::Ident(keyword)) if keyword == "none" => Ok(Angle::new(0.0, "deg")),
      _ => Err(CssParseError::ParseError {
        message: "Expected hue: angle, number, or 'none'".to_string(),
      }),
    }
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for Oklch {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match printed_alpha(self.alpha) {
      Some(alpha) => write!(
        f,
        "oklch({} {} {} / {})",
        to_js_string(self.l),
        to_js_string(self.c),
        self.h,
        to_js_string(alpha)
      ),
      None => write!(
        f,
        "oklch({} {} {})",
        to_js_string(self.l),
        to_js_string(self.c),
        self.h
      ),
    }
  }
}

/// OKLAB color: oklab(0.5 0.1 0.1)
#[derive(Debug, Clone, PartialEq)]
pub struct Oklab {
  pub l: f64,             // lightness (0-1)
  pub a: f64,             // green-red (-0.4 to 0.4)
  pub b: f64,             // blue-yellow (-0.4 to 0.4)
  pub alpha: Option<f64>, // alpha (0-1)
}

impl Oklab {
  pub fn new(l: f64, a: f64, b: f64, alpha: Option<f64>) -> Self {
    Self { l, a, b, alpha }
  }

  /// Parser for OKLAB colors
  /// - L/A/B values: numbers, 'none' keyword (maps to 0)
  /// - Alpha: optional with slash syntax
  pub fn parse() -> TokenParser<Oklab> {
    TokenParser::new(
      move |input| {
        // Parse 'oklab(' function start
        match input.consume_next_token_infallible() {
          Some(SimpleToken::Function(fn_name)) if fn_name.to_lowercase() == "oklab" => {},
          _ => {
            return Err(CssParseError::ParseError {
              message: "Expected oklab() function".to_string(),
            });
          },
        }

        // Parse lightness (l): number | 'none'
        let l = Self::parse_oklab_lab_value(input)?;

        // Parse whitespace
        match input.consume_next_token_infallible() {
          Some(SimpleToken::Whitespace) => {},
          _ => {
            return Err(CssParseError::ParseError {
              message: "Expected whitespace after lightness".to_string(),
            });
          },
        }

        // Parse a component (green-red): number | 'none'
        let a = Self::parse_oklab_lab_value(input)?;

        // Parse whitespace
        match input.consume_next_token_infallible() {
          Some(SimpleToken::Whitespace) => {},
          _ => {
            return Err(CssParseError::ParseError {
              message: "Expected whitespace after a component".to_string(),
            });
          },
        }

        // Parse b component (blue-yellow): number | 'none'
        let b = Self::parse_oklab_lab_value(input)?;

        // Parse optional alpha: / <alpha-value>
        let alpha = parse_optional_slash_alpha(input)?;

        // Parse closing paren
        match input.consume_next_token_infallible() {
          Some(SimpleToken::RightParen) => {},
          _ => {
            return Err(CssParseError::ParseError {
              message: "Expected ) after oklab values".to_string(),
            });
          },
        }

        Ok(Oklab::new(l, a, b, alpha))
      },
      "oklab_parser",
    )
  }

  /// Parse OKLAB l/a/b value: number | 'none'
  fn parse_oklab_lab_value(
    input: &mut crate::token_types::TokenList,
  ) -> Result<f64, CssParseError> {
    match input.consume_next_token_infallible() {
      Some(SimpleToken::Number(n)) => Ok(n),
      Some(SimpleToken::Ident(keyword)) if keyword == "none" => Ok(0.0),
      _ => Err(CssParseError::ParseError {
        message: "Expected number or 'none'".to_string(),
      }),
    }
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for Oklab {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match printed_alpha(self.alpha) {
      Some(alpha) => write!(
        f,
        "oklab({} {} {} / {})",
        to_js_string(self.l),
        to_js_string(self.a),
        to_js_string(self.b),
        to_js_string(alpha)
      ),
      None => write!(
        f,
        "oklab({} {} {})",
        to_js_string(self.l),
        to_js_string(self.a),
        to_js_string(self.b)
      ),
    }
  }
}

#[cfg(test)]
#[path = "../tests/css_types/color_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/css_types/color_test.rs"]
mod color_test;

#[cfg(test)]
#[path = "../tests/css_types/color_coverage_test.rs"]
mod color_coverage_test;
