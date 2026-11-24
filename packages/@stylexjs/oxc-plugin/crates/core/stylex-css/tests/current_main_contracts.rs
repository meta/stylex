//! Current-main Babel contracts (2d42d72), distinct from published 0.19.1.
use postcss_value_parser::{ValueParser, stringify};
use stylex_css::css::{
  common::get_number_suffix, generate_rtl::generate_rtl, normalize_value::normalize_value,
};
use stylex_css::order::constants::legacy_expand_shorthands_order::Shorthands;
use stylex_structures::{
  pair::Pair, raw_value::TRawValue, stylex_state_options::StyleXStateOptions,
};

#[test]
fn current_main_font_size_adjust_is_unitless() {
  assert_eq!(get_number_suffix("fontSizeAdjust"), "");
  assert_eq!(get_number_suffix("fontWeight"), "");
  assert_eq!(get_number_suffix("animationDuration"), "ms");
  // Current Babel positionTry deliberately dashifies before normalization.
  assert_eq!(get_number_suffix("animation-duration"), "px");
}

#[test]
fn current_main_urls_and_later_function_zero_units_are_preserved() {
  let options = StyleXStateOptions::default();
  for (value, expected) in [
    ("url(0.5x.png)", "url(0.5x.png)"),
    ("URL(/img/logo.png)", "URL(/img/logo.png)"),
    ("url(500ms.png)", "url(500ms.png)"),
    ("url(/assets/0.50ms/0px.png)", "url(/assets/0.50ms/0px.png)"),
    (
      "url(  data:image/svg+xml;utf8,<svg width='0px'>500ms</svg>  )",
      "url(data:image/svg+xml;utf8,<svg width='0px'>500ms</svg>)",
    ),
    (
      "translateX(1px) translateY(0px)",
      "translateX(1px) translateY(0px)",
    ),
    (
      "translateX(0px) 0px translateY(0px)",
      "translateX(0px) 0 translateY(0px)",
    ),
    ("min(0px, calc(0px + 1px))", "min(0px,calc(0px + 1px))"),
  ] {
    assert_eq!(
      normalize_value(value, "transform", &options),
      expected,
      "{value}"
    );
  }
}

#[test]
fn decimal_shadow_lengths_and_nested_dividers_match_babel() {
  let mut options = StyleXStateOptions::default();
  options.core.enable_legacy_value_flipping = true;
  for (value, expected) in [
    ("1.50em", "-1.50em"),
    (".5px", "-.5px"),
    ("-0.25", "0.25"),
    ("1e3px", "-1e3px"),
    ("rgb(20 40 60 / .5)", "rgb(20 -40 60 / .5)"),
    ("1px 2px rgb(20,40,60)", "-1px 2px rgb(20,40,60)"),
    ("inset 1px 2px / 3px red", "inset -1px 2px,-3px red"),
  ] {
    let pair = Pair::new("box-shadow", value);
    let actual = generate_rtl(&pair, &options).map(|pair| pair.value.into_owned());
    assert_eq!(actual.as_deref(), Some(expected), "{value}");
  }
}

#[test]
fn numeric_legacy_list_style_preserves_absence_not_css_text() {
  let Some(expand) = Shorthands::get("listStyle") else {
    panic!("missing listStyle")
  };
  for number in [0.0, 10.0, -0.25] {
    let result = match expand(Some(TRawValue::Number(number))) {
      Ok(result) => result,
      Err(error) => panic!("{error}"),
    };
    assert_eq!(result.len(), 3);
    assert!(result.iter().all(|pair| pair.1.is_none()));
  }
}

#[test]
fn postcss_roundtrip_does_not_add_nested_slash_spacing() {
  let input = "rgb(20 40 60 / .5)";
  let parsed = ValueParser::new(input);
  assert_eq!(stringify(&parsed.nodes), input);
}
