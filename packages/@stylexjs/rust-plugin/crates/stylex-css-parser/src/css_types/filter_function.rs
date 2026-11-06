/*!
CSS filter function parser.
*/

use stylex_utils::number::to_js_string;

use crate::{
  CssParseError,
  css_types::{Angle, Length, common_types::number_or_percentage_parser},
  token_parser::TokenParser,
  token_types::SimpleToken,
};
use std::fmt::{self, Display};

#[derive(Debug, Clone, PartialEq)]
pub enum FilterFunction {
  Blur(BlurFilterFunction),
  Brightness(BrightnessFilterFunction),
  Contrast(ContrastFilterFunction),
  Grayscale(GrayscaleFilterFunction),
  HueRotate(HueRotateFilterFunction),
  Invert(InvertFilterFunction),
  Opacity(OpacityFilterFunction),
  Saturate(SaturateFilterFunction),
  Sepia(SepiaFilterFunction),
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlurFilterFunction {
  pub radius: Length,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BrightnessFilterFunction {
  pub percentage: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContrastFilterFunction {
  pub amount: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GrayscaleFilterFunction {
  pub amount: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HueRotateFilterFunction {
  pub angle: Angle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InvertFilterFunction {
  pub amount: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OpacityFilterFunction {
  pub amount: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SaturateFilterFunction {
  pub amount: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SepiaFilterFunction {
  pub amount: f64,
}

impl FilterFunction {
  pub fn parser() -> TokenParser<FilterFunction> {
    TokenParser::one_of(vec![
      BlurFilterFunction::parse().map(FilterFunction::Blur, Some("blur")),
      BrightnessFilterFunction::parse().map(FilterFunction::Brightness, Some("brightness")),
      ContrastFilterFunction::parse().map(FilterFunction::Contrast, Some("contrast")),
      GrayscaleFilterFunction::parse().map(FilterFunction::Grayscale, Some("grayscale")),
      HueRotateFilterFunction::parse().map(FilterFunction::HueRotate, Some("hue_rotate")),
      InvertFilterFunction::parse().map(FilterFunction::Invert, Some("invert")),
      OpacityFilterFunction::parse().map(FilterFunction::Opacity, Some("opacity")),
      SaturateFilterFunction::parse().map(FilterFunction::Saturate, Some("saturate")),
      SepiaFilterFunction::parse().map(FilterFunction::Sepia, Some("sepia")),
    ])
  }
}

impl BlurFilterFunction {
  pub fn new(radius: Length) -> Self {
    Self { radius }
  }

  pub fn parse() -> TokenParser<BlurFilterFunction> {
    TokenParser::new(
      |tokens| {
        // Parse 'blur(' function start
        match tokens.consume_next_token_infallible() {
          Some(SimpleToken::Function(fn_name)) if fn_name == "blur" => {},
          Some(token) => {
            return Err(CssParseError::ParseError {
              message: format!("Expected blur() function, got {:?}", token),
            });
          },
          None => {
            return Err(CssParseError::ParseError {
              message: "Expected blur() function but reached end of input".to_string(),
            });
          },
        }

        // Skip optional whitespace
        while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
          let _ = tokens.consume_next_token();
        }

        // Parse radius (Length)
        let radius = (Length::parser().run)(tokens)?;

        // Skip optional whitespace
        while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
          let _ = tokens.consume_next_token();
        }

        // Parse closing paren
        match tokens.consume_next_token_infallible() {
          Some(SimpleToken::RightParen) => {},
          Some(token) => {
            return Err(CssParseError::ParseError {
              message: format!("Expected closing paren, got {:?}", token),
            });
          },
          None => {
            return Err(CssParseError::ParseError {
              message: "Expected closing paren but reached end of input".to_string(),
            });
          },
        }

        Ok(BlurFilterFunction::new(radius))
      },
      "blur_parser",
    )
  }
}

impl BrightnessFilterFunction {
  pub fn new(percentage: f64) -> Self {
    Self { percentage }
  }

  pub fn parse() -> TokenParser<BrightnessFilterFunction> {
    TokenParser::new(
      |tokens| {
        // Parse 'brightness(' function start
        match tokens.consume_next_token_infallible() {
          Some(SimpleToken::Function(fn_name)) if fn_name == "brightness" => {},
          Some(token) => {
            return Err(CssParseError::ParseError {
              message: format!("Expected brightness() function, got {:?}", token),
            });
          },
          None => {
            return Err(CssParseError::ParseError {
              message: "Expected brightness() function but reached end of input".to_string(),
            });
          },
        }

        // Skip optional whitespace
        while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
          let _ = tokens.consume_next_token();
        }

        // Parse number or percentage and convert to f64
        let number_or_percentage = (number_or_percentage_parser().run)(tokens)?;
        let value = number_or_percentage.as_fraction();

        if value < 0.0 {
          return Err(CssParseError::ParseError {
            message: "brightness() amount must be non-negative".to_string(),
          });
        }

        // Skip optional whitespace
        while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
          let _ = tokens.consume_next_token();
        }

        // Parse closing paren
        match tokens.consume_next_token_infallible() {
          Some(SimpleToken::RightParen) => {},
          Some(token) => {
            return Err(CssParseError::ParseError {
              message: format!("Expected closing paren, got {:?}", token),
            });
          },
          None => {
            return Err(CssParseError::ParseError {
              message: "Expected closing paren but reached end of input".to_string(),
            });
          },
        }

        Ok(BrightnessFilterFunction::new(value))
      },
      "brightness_parser",
    )
  }
}

impl ContrastFilterFunction {
  pub fn new(amount: f64) -> Self {
    Self { amount }
  }

  pub fn parse() -> TokenParser<ContrastFilterFunction> {
    TokenParser::new(
      |tokens| {
        // Parse 'contrast(' function start
        match tokens.consume_next_token_infallible() {
          Some(SimpleToken::Function(fn_name)) if fn_name == "contrast" => {},
          Some(token) => {
            return Err(CssParseError::ParseError {
              message: format!("Expected contrast() function, got {:?}", token),
            });
          },
          None => {
            return Err(CssParseError::ParseError {
              message: "Expected contrast() function but reached end of input".to_string(),
            });
          },
        }

        // Skip optional whitespace
        while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
          let _ = tokens.consume_next_token();
        }

        // Parse number or percentage and convert to f64
        let number_or_percentage = (number_or_percentage_parser().run)(tokens)?;
        let value = number_or_percentage.as_fraction();

        // Skip optional whitespace
        while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
          let _ = tokens.consume_next_token();
        }

        // Parse closing paren
        match tokens.consume_next_token_infallible() {
          Some(SimpleToken::RightParen) => {},
          Some(token) => {
            return Err(CssParseError::ParseError {
              message: format!("Expected closing paren, got {:?}", token),
            });
          },
          None => {
            return Err(CssParseError::ParseError {
              message: "Expected closing paren but reached end of input".to_string(),
            });
          },
        }

        Ok(ContrastFilterFunction::new(value))
      },
      "contrast_parser",
    )
  }
}

impl GrayscaleFilterFunction {
  pub fn new(amount: f64) -> Self {
    Self { amount }
  }

  pub fn parse() -> TokenParser<GrayscaleFilterFunction> {
    TokenParser::new(
      |tokens| {
        // Parse 'grayscale(' function start
        match tokens.consume_next_token_infallible() {
          Some(SimpleToken::Function(fn_name)) if fn_name == "grayscale" => {},
          Some(token) => {
            return Err(CssParseError::ParseError {
              message: format!("Expected grayscale() function, got {:?}", token),
            });
          },
          None => {
            return Err(CssParseError::ParseError {
              message: "Expected grayscale() function but reached end of input".to_string(),
            });
          },
        }

        // Skip optional whitespace
        while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
          let _ = tokens.consume_next_token();
        }

        // Parse number or percentage and convert to f64
        let number_or_percentage = (number_or_percentage_parser().run)(tokens)?;
        let value = number_or_percentage.as_fraction();

        // Skip optional whitespace
        while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
          let _ = tokens.consume_next_token();
        }

        // Parse closing paren
        match tokens.consume_next_token_infallible() {
          Some(SimpleToken::RightParen) => {},
          Some(token) => {
            return Err(CssParseError::ParseError {
              message: format!("Expected closing paren, got {:?}", token),
            });
          },
          None => {
            return Err(CssParseError::ParseError {
              message: "Expected closing paren but reached end of input".to_string(),
            });
          },
        }

        Ok(GrayscaleFilterFunction::new(value))
      },
      "grayscale_parser",
    )
  }
}

impl HueRotateFilterFunction {
  pub fn new(angle: Angle) -> Self {
    Self { angle }
  }

  pub fn parse() -> TokenParser<HueRotateFilterFunction> {
    TokenParser::new(
      |tokens| {
        // Parse 'hue-rotate(' function start
        match tokens.consume_next_token_infallible() {
          Some(SimpleToken::Function(fn_name)) if fn_name == "hue-rotate" => {},
          Some(token) => {
            return Err(CssParseError::ParseError {
              message: format!("Expected hue-rotate() function, got {:?}", token),
            });
          },
          None => {
            return Err(CssParseError::ParseError {
              message: "Expected hue-rotate() function but reached end of input".to_string(),
            });
          },
        }

        // Skip optional whitespace
        while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
          let _ = tokens.consume_next_token();
        }

        // Parse angle
        let angle = (Angle::parser().run)(tokens)?;

        // Skip optional whitespace
        while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
          let _ = tokens.consume_next_token();
        }

        // Parse closing paren
        match tokens.consume_next_token_infallible() {
          Some(SimpleToken::RightParen) => {},
          Some(token) => {
            return Err(CssParseError::ParseError {
              message: format!("Expected closing paren, got {:?}", token),
            });
          },
          None => {
            return Err(CssParseError::ParseError {
              message: "Expected closing paren but reached end of input".to_string(),
            });
          },
        }

        Ok(HueRotateFilterFunction::new(angle))
      },
      "hue_rotate_parser",
    )
  }
}

impl InvertFilterFunction {
  pub fn new(amount: f64) -> Self {
    Self { amount }
  }

  pub fn parse() -> TokenParser<InvertFilterFunction> {
    TokenParser::new(
      |tokens| {
        // Parse 'invert(' function start
        match tokens.consume_next_token_infallible() {
          Some(SimpleToken::Function(fn_name)) if fn_name == "invert" => {},
          Some(token) => {
            return Err(CssParseError::ParseError {
              message: format!("Expected invert() function, got {:?}", token),
            });
          },
          None => {
            return Err(CssParseError::ParseError {
              message: "Expected invert() function but reached end of input".to_string(),
            });
          },
        }

        // Skip optional whitespace
        while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
          let _ = tokens.consume_next_token();
        }

        // Parse number or percentage and convert to f64
        let number_or_percentage = (number_or_percentage_parser().run)(tokens)?;
        let value = number_or_percentage.as_fraction();

        // Skip optional whitespace
        while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
          let _ = tokens.consume_next_token();
        }

        // Parse closing paren
        match tokens.consume_next_token_infallible() {
          Some(SimpleToken::RightParen) => {},
          Some(token) => {
            return Err(CssParseError::ParseError {
              message: format!("Expected closing paren, got {:?}", token),
            });
          },
          None => {
            return Err(CssParseError::ParseError {
              message: "Expected closing paren but reached end of input".to_string(),
            });
          },
        }

        Ok(InvertFilterFunction::new(value))
      },
      "invert_parser",
    )
  }
}

impl OpacityFilterFunction {
  pub fn new(amount: f64) -> Self {
    Self { amount }
  }

  pub fn parse() -> TokenParser<OpacityFilterFunction> {
    TokenParser::new(
      |tokens| {
        // Parse 'opacity(' function start
        match tokens.consume_next_token_infallible() {
          Some(SimpleToken::Function(fn_name)) if fn_name == "opacity" => {},
          Some(token) => {
            return Err(CssParseError::ParseError {
              message: format!("Expected opacity() function, got {:?}", token),
            });
          },
          None => {
            return Err(CssParseError::ParseError {
              message: "Expected opacity() function but reached end of input".to_string(),
            });
          },
        }

        // Skip optional whitespace
        while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
          let _ = tokens.consume_next_token();
        }

        // Parse number or percentage and convert to f64
        let number_or_percentage = (number_or_percentage_parser().run)(tokens)?;
        let value = number_or_percentage.as_fraction();

        // Skip optional whitespace
        while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
          let _ = tokens.consume_next_token();
        }

        // Parse closing paren
        match tokens.consume_next_token_infallible() {
          Some(SimpleToken::RightParen) => {},
          Some(token) => {
            return Err(CssParseError::ParseError {
              message: format!("Expected closing paren, got {:?}", token),
            });
          },
          None => {
            return Err(CssParseError::ParseError {
              message: "Expected closing paren but reached end of input".to_string(),
            });
          },
        }

        Ok(OpacityFilterFunction::new(value))
      },
      "opacity_parser",
    )
  }
}

impl SaturateFilterFunction {
  pub fn new(amount: f64) -> Self {
    Self { amount }
  }

  pub fn parse() -> TokenParser<SaturateFilterFunction> {
    TokenParser::new(
      |tokens| {
        // Parse 'saturate(' function start
        match tokens.consume_next_token_infallible() {
          Some(SimpleToken::Function(fn_name)) if fn_name == "saturate" => {},
          Some(token) => {
            return Err(CssParseError::ParseError {
              message: format!("Expected saturate() function, got {:?}", token),
            });
          },
          None => {
            return Err(CssParseError::ParseError {
              message: "Expected saturate() function but reached end of input".to_string(),
            });
          },
        }

        // Skip optional whitespace
        while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
          let _ = tokens.consume_next_token();
        }

        // Parse number or percentage and convert to f64
        let number_or_percentage = (number_or_percentage_parser().run)(tokens)?;
        let value = number_or_percentage.as_fraction();

        // Skip optional whitespace
        while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
          let _ = tokens.consume_next_token();
        }

        // Parse closing paren
        match tokens.consume_next_token_infallible() {
          Some(SimpleToken::RightParen) => {},
          Some(token) => {
            return Err(CssParseError::ParseError {
              message: format!("Expected closing paren, got {:?}", token),
            });
          },
          None => {
            return Err(CssParseError::ParseError {
              message: "Expected closing paren but reached end of input".to_string(),
            });
          },
        }

        Ok(SaturateFilterFunction::new(value))
      },
      "saturate_parser",
    )
  }
}

impl SepiaFilterFunction {
  pub fn new(amount: f64) -> Self {
    Self { amount }
  }

  pub fn parse() -> TokenParser<SepiaFilterFunction> {
    TokenParser::new(
      |tokens| {
        // Parse 'sepia(' function start
        match tokens.consume_next_token_infallible() {
          Some(SimpleToken::Function(fn_name)) if fn_name == "sepia" => {},
          Some(token) => {
            return Err(CssParseError::ParseError {
              message: format!("Expected sepia() function, got {:?}", token),
            });
          },
          None => {
            return Err(CssParseError::ParseError {
              message: "Expected sepia() function but reached end of input".to_string(),
            });
          },
        }

        // Skip optional whitespace
        while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
          let _ = tokens.consume_next_token();
        }

        // Parse number or percentage and convert to f64
        let number_or_percentage = (number_or_percentage_parser().run)(tokens)?;
        let value = number_or_percentage.as_fraction();

        // Skip optional whitespace
        while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
          let _ = tokens.consume_next_token();
        }

        // Parse closing paren
        match tokens.consume_next_token_infallible() {
          Some(SimpleToken::RightParen) => {},
          Some(token) => {
            return Err(CssParseError::ParseError {
              message: format!("Expected closing paren, got {:?}", token),
            });
          },
          None => {
            return Err(CssParseError::ParseError {
              message: "Expected closing paren but reached end of input".to_string(),
            });
          },
        }

        Ok(SepiaFilterFunction::new(value))
      },
      "sepia_parser",
    )
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for FilterFunction {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      FilterFunction::Blur(blur) => blur.fmt(f),
      FilterFunction::Brightness(brightness) => brightness.fmt(f),
      FilterFunction::Contrast(contrast) => contrast.fmt(f),
      FilterFunction::Grayscale(grayscale) => grayscale.fmt(f),
      FilterFunction::HueRotate(hue_rotate) => hue_rotate.fmt(f),
      FilterFunction::Invert(invert) => invert.fmt(f),
      FilterFunction::Opacity(opacity) => opacity.fmt(f),
      FilterFunction::Saturate(saturate) => saturate.fmt(f),
      FilterFunction::Sepia(sepia) => sepia.fmt(f),
    }
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for BlurFilterFunction {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "blur({})", self.radius)
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for BrightnessFilterFunction {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "brightness({})", to_js_string(self.percentage))
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for ContrastFilterFunction {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "contrast({})", to_js_string(self.amount))
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for GrayscaleFilterFunction {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "grayscale({})", to_js_string(self.amount))
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for HueRotateFilterFunction {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "hue-rotate({})", self.angle)
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for InvertFilterFunction {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "invert({})", to_js_string(self.amount))
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for OpacityFilterFunction {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "opacity({})", to_js_string(self.amount))
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for SaturateFilterFunction {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "saturate({})", to_js_string(self.amount))
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for SepiaFilterFunction {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "sepia({})", to_js_string(self.amount))
  }
}

#[cfg(test)]
#[path = "../tests/css_types/filter_function_test.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/css_types/filter_function_coverage_test.rs"]
mod filter_function_coverage_test;
