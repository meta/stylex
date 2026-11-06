use indexmap::IndexMap;
use phf::phf_map;
use std::rc::Rc;
use stylex_enums::{css_syntax::CSSSyntax, value_with_default::ValueWithDefault};
use stylex_macros::stylex_panic;
use stylex_state::functions::{FunctionConfig, FunctionType};
use stylex_structures::base_css_type::BaseCSSType;
use swc_core::ecma::ast::Expr;

pub trait HasBase {
  fn new(value: ValueWithDefault) -> Self
  where
    Self: Sized;
}

struct Angle {
  base: BaseCSSType,
}

impl HasBase for Angle {
  fn new(value: ValueWithDefault) -> Self {
    Angle {
      base: BaseCSSType {
        value,
        syntax: CSSSyntax::Angle,
      },
    }
  }
}

struct Color {
  base: BaseCSSType,
}

impl HasBase for Color {
  fn new(value: ValueWithDefault) -> Self {
    Color {
      base: BaseCSSType {
        value,
        syntax: CSSSyntax::Color,
      },
    }
  }
}

struct Url {
  base: BaseCSSType,
}

impl HasBase for Url {
  fn new(value: ValueWithDefault) -> Self {
    Url {
      base: BaseCSSType {
        value,
        syntax: CSSSyntax::Url,
      },
    }
  }
}

struct Image {
  base: BaseCSSType,
}

impl HasBase for Image {
  fn new(value: ValueWithDefault) -> Self {
    Image {
      base: BaseCSSType {
        value,
        syntax: CSSSyntax::Image,
      },
    }
  }
}

struct Integer {
  base: BaseCSSType,
}

impl HasBase for Integer {
  fn new(value: ValueWithDefault) -> Self {
    Integer {
      base: BaseCSSType {
        value: convert_number_to_bare_string(value),
        syntax: CSSSyntax::Integer,
      },
    }
  }
}

struct LengthPercentage {
  base: BaseCSSType,
}

impl HasBase for LengthPercentage {
  fn new(value: ValueWithDefault) -> Self {
    LengthPercentage {
      base: BaseCSSType {
        value: convert_number_to_percentage(value),
        syntax: CSSSyntax::LengthPercentage,
      },
    }
  }
}

struct Length {
  base: BaseCSSType,
}

impl HasBase for Length {
  fn new(value: ValueWithDefault) -> Self {
    Length {
      base: BaseCSSType {
        value: convert_number_to_length(value),
        syntax: CSSSyntax::Length,
      },
    }
  }
}

pub struct Percentage {
  base: BaseCSSType,
}

impl HasBase for Percentage {
  fn new(value: ValueWithDefault) -> Self {
    Percentage {
      base: BaseCSSType {
        value: convert_number_to_percentage(value),
        syntax: CSSSyntax::Percentage,
      },
    }
  }
}

pub struct Num {
  base: BaseCSSType,
}

impl HasBase for Num {
  fn new(value: ValueWithDefault) -> Self {
    Num {
      base: BaseCSSType {
        value: convert_number_to_bare_string(value),
        syntax: CSSSyntax::Number,
      },
    }
  }
}

pub struct Resolution {
  base: BaseCSSType,
}

impl HasBase for Resolution {
  fn new(value: ValueWithDefault) -> Self {
    Resolution {
      base: BaseCSSType {
        value,
        syntax: CSSSyntax::Resolution,
      },
    }
  }
}

pub struct Time {
  base: BaseCSSType,
}

impl HasBase for Time {
  fn new(value: ValueWithDefault) -> Self {
    Time {
      base: BaseCSSType {
        value,
        syntax: CSSSyntax::Time,
      },
    }
  }
}

pub struct TransformFunction {
  base: BaseCSSType,
}

impl HasBase for TransformFunction {
  fn new(value: ValueWithDefault) -> Self {
    TransformFunction {
      base: BaseCSSType {
        value,
        syntax: CSSSyntax::TransformFunction,
      },
    }
  }
}

pub struct TransformList {
  base: BaseCSSType,
}

impl HasBase for TransformList {
  fn new(value: ValueWithDefault) -> Self {
    TransformList {
      base: BaseCSSType {
        value,
        syntax: CSSSyntax::TransformList,
      },
    }
  }
}
/// Rewrites every number the value holds with `transform_number`, and walks a
/// map to reach the numbers below it.
///
/// A string is parsed first, because an author can write a number as a string
/// and the reference implementation converts it the same way. A string that
/// does not read as a number is kept as it was written.
fn convert_number_to_string_using(
  transform_number: fn(f64) -> ValueWithDefault,
  value: ValueWithDefault,
) -> ValueWithDefault {
  match value {
    ValueWithDefault::Number(number) => transform_number(number),
    ValueWithDefault::String(string) => string
      .parse()
      .map_or(ValueWithDefault::String(string), transform_number),
    ValueWithDefault::Map(map) => ValueWithDefault::Map(
      map
        .into_iter()
        .map(|(key, value)| (key, convert_number_to_string_using(transform_number, value)))
        .collect::<IndexMap<_, _>>(),
    ),
  }
}

fn convert_number_to_bare_string(value: ValueWithDefault) -> ValueWithDefault {
  convert_number_to_string_using(|value| ValueWithDefault::String(value.to_string()), value)
}

fn convert_number_to_length(value: ValueWithDefault) -> ValueWithDefault {
  convert_number_to_string_using(
    |value: f64| {
      if value == 0.0 {
        ValueWithDefault::String("0".to_string())
      } else {
        ValueWithDefault::String(format!("{}px", value))
      }
    },
    value,
  )
}

fn convert_number_to_percentage(value: ValueWithDefault) -> ValueWithDefault {
  convert_number_to_string_using(
    |value| {
      if value == 0.0 {
        ValueWithDefault::String("0".to_string())
      } else {
        ValueWithDefault::String(format!("{}%", value * 100.0))
      }
    },
    value,
  )
}

impl From<Angle> for BaseCSSType {
  fn from(instance: Angle) -> Self {
    instance.base
  }
}

impl From<Color> for BaseCSSType {
  fn from(instance: Color) -> Self {
    instance.base
  }
}

impl From<Image> for BaseCSSType {
  fn from(instance: Image) -> Self {
    instance.base
  }
}

impl From<Integer> for BaseCSSType {
  fn from(instance: Integer) -> Self {
    instance.base
  }
}

impl From<Length> for BaseCSSType {
  fn from(instance: Length) -> Self {
    instance.base
  }
}

impl From<LengthPercentage> for BaseCSSType {
  fn from(instance: LengthPercentage) -> Self {
    instance.base
  }
}

impl From<Num> for BaseCSSType {
  fn from(instance: Num) -> Self {
    instance.base
  }
}

impl From<Percentage> for BaseCSSType {
  fn from(instance: Percentage) -> Self {
    instance.base
  }
}

impl From<Resolution> for BaseCSSType {
  fn from(instance: Resolution) -> Self {
    instance.base
  }
}

impl From<Time> for BaseCSSType {
  fn from(instance: Time) -> Self {
    instance.base
  }
}

impl From<TransformFunction> for BaseCSSType {
  fn from(instance: TransformFunction) -> Self {
    instance.base
  }
}

impl From<TransformList> for BaseCSSType {
  fn from(instance: TransformList) -> Self {
    instance.base
  }
}

impl From<Url> for BaseCSSType {
  fn from(instance: Url) -> Self {
    instance.base
  }
}

fn angle(value: ValueWithDefault) -> Expr {
  let base_css_type: BaseCSSType = Angle::new(value).into();

  base_css_type.into()
}

fn color(value: ValueWithDefault) -> Expr {
  let base_css_type: BaseCSSType = Color::new(value).into();

  base_css_type.into()
}

fn image(value: ValueWithDefault) -> Expr {
  let base_css_type: BaseCSSType = Image::new(value).into();

  base_css_type.into()
}

fn integer(value: ValueWithDefault) -> Expr {
  let base_css_type: BaseCSSType = Integer::new(value).into();

  base_css_type.into()
}

fn length(value: ValueWithDefault) -> Expr {
  let base_css_type: BaseCSSType = Length::new(value).into();

  base_css_type.into()
}

fn length_percentage(value: ValueWithDefault) -> Expr {
  let base_css_type: BaseCSSType = LengthPercentage::new(value).into();

  base_css_type.into()
}

fn num(value: ValueWithDefault) -> Expr {
  let base_css_type: BaseCSSType = Num::new(value).into();

  base_css_type.into()
}

fn resolution(value: ValueWithDefault) -> Expr {
  let base_css_type: BaseCSSType = Resolution::new(value).into();

  base_css_type.into()
}

fn percentage(value: ValueWithDefault) -> Expr {
  let base_css_type: BaseCSSType = Percentage::new(value).into();

  base_css_type.into()
}

fn time(value: ValueWithDefault) -> Expr {
  let base_css_type: BaseCSSType = Time::new(value).into();

  base_css_type.into()
}

fn transform_function(value: ValueWithDefault) -> Expr {
  let base_css_type: BaseCSSType = TransformFunction::new(value).into();

  base_css_type.into()
}

fn transform_list(value: ValueWithDefault) -> Expr {
  let base_css_type: BaseCSSType = TransformList::new(value).into();

  base_css_type.into()
}

fn url(value: ValueWithDefault) -> Expr {
  let base_css_type: BaseCSSType = Url::new(value).into();

  base_css_type.into()
}

pub(crate) static FN_MAP: phf::Map<&'static str, fn(value: ValueWithDefault) -> Expr> = phf_map! {
  "angle" => angle,
  "color" => color,
  "image" => image,
  "integer" => integer,
  "length" => length,
  "lengthPercentage" => length_percentage,
  "number" => num,
  "percentage" => percentage,
  "resolution" => resolution,
  "time" => time,
  "transformFunction" => transform_function,
  "transformList" => transform_list,
  "url" => url,

};

pub(crate) fn get_types_fn() -> FunctionConfig {
  FunctionConfig {
    fn_ptr: FunctionType::StylexFnsFactory(
      |prop_name| -> Rc<dyn Fn(ValueWithDefault) -> Expr + 'static> {
        Rc::new(
          *FN_MAP
            .get(prop_name.as_str())
            .unwrap_or_else(|| stylex_panic!(r#"Function "{}" not found"#, prop_name)),
        )
      },
    ),
    takes_path: false,
  }
}
