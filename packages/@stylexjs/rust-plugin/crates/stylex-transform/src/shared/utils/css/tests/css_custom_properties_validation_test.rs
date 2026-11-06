#[cfg(test)]
mod css_tests {
  use crate::shared::utils::css::common::transform_value_cached;
  use stylex_state::state_manager::StateManager;
  use stylex_structures::raw_value::TRawValue;

  #[test]
  #[should_panic(expected = "Rule contains an unclosed function")]
  fn disallow_unclosed_style_value_functions() {
    assert_eq!(
      transform_value_cached(
        "color",
        &TRawValue::from("var(--foo"),
        &mut StateManager::default()
      ),
      "1px",
    );
  }

  #[test]
  #[should_panic(expected = "Unprefixed custom properties")]
  fn disallow_unprefixed_custom_properties() {
    assert_eq!(
      transform_value_cached(
        "color",
        &TRawValue::from("var(foo)"),
        &mut StateManager::default()
      ),
      "1px",
    );
  }

  #[test]
  fn basic_var_properties() {
    assert_eq!(
      transform_value_cached(
        "color",
        &TRawValue::from("var(--foo)"),
        &mut StateManager::default()
      ),
      "var(--foo)",
    );
    assert_eq!(
      transform_value_cached(
        "backgroundColor",
        &TRawValue::from("var(--bar)"),
        &mut StateManager::default()
      ),
      "var(--bar)"
    );
  }

  #[test]
  fn transition_properties() {
    assert_eq!(
      transform_value_cached(
        "transitionProperty",
        &TRawValue::from("opacity, margin-top"),
        &mut StateManager::default()
      ),
      "opacity,margin-top"
    );
    assert_eq!(
      transform_value_cached(
        "transitionProperty",
        &TRawValue::from("opacity, marginTop"),
        &mut StateManager::default()
      ),
      "opacity,margin-top"
    );
  }

  #[test]
  fn shadow_properties() {
    assert_eq!(
      transform_value_cached(
        "boxShadow",
        &TRawValue::from("0px 2px 4px var(--shadow-1)"),
        &mut StateManager::default()
      ),
      "0 2px 4px var(--shadow-1)"
    );
    assert_eq!(
      transform_value_cached(
        "boxShadow",
        &TRawValue::from("1px 1px #000"),
        &mut StateManager::default()
      ),
      "1px 1px #000",
    );
  }

  #[test]
  fn spacing_and_calculations() {
    assert_eq!(
      transform_value_cached(
        "padding",
        &TRawValue::from("var(--rightpadding, 20px)"),
        &mut StateManager::default()
      ),
      "var(--rightpadding,20px)"
    );
    assert_eq!(
      transform_value_cached(
        "padding",
        &TRawValue::from("calc((100% - 50px) * 0.5) var(--rightpadding, 20px)"),
        &mut StateManager::default()
      ),
      "calc((100% - 50px) * .5) var(--rightpadding,20px)"
    );
    assert_eq!(
      transform_value_cached(
        "margin",
        &TRawValue::from("max(0px, (48px - var(--x16dnrjz)) / 2)"),
        &mut StateManager::default()
      ),
      "max(0px,(48px - var(--x16dnrjz)) / 2)"
    );
  }

  #[test]
  fn hashed_vars() {
    assert_eq!(
      transform_value_cached(
        "backgroundColor",
        &TRawValue::from("var(----__hashed_var__1jqb1tb, revert)"),
        &mut StateManager::default()
      ),
      "var(----__hashed_var__1jqb1tb,revert)"
    );
    assert_eq!(
      transform_value_cached(
        "--__hashed_var__1jqb1tb",
        &TRawValue::from("var(----__hashed_var__1jqb1tb, revert)"),
        &mut StateManager::default()
      ),
      "var(----__hashed_var__1jqb1tb,revert)"
    );
  }

  #[test]
  fn quotes_handling() {
    assert_eq!(
      transform_value_cached(
        "quotes",
        &TRawValue::from(r#""''""#),
        &mut StateManager::default()
      ),
      r#""''""#
    );
    assert_eq!(
      transform_value_cached(
        "quotes",
        &TRawValue::from(r#""'123'""#),
        &mut StateManager::default()
      ),
      r#""'123'""#
    );
  }

  #[test]
  fn grid_properties() {
    assert_eq!(
      transform_value_cached(
        "gridTemplateAreas",
        &TRawValue::from(r#"'"content"'"#),
        &mut StateManager::default()
      ),
      r#"'"content"'"#
    );
    assert_eq!(
      transform_value_cached(
        "gridTemplateAreas",
        &TRawValue::from(r#"'"content" "sidebar"'"#),
        &mut StateManager::default()
      ),
      r#"'"content" "sidebar"'"#
    );
    assert_eq!(
      transform_value_cached(
        "gridTemplateAreas",
        &TRawValue::from(r#"'"content""sidebar"'"#),
        &mut StateManager::default()
      ),
      r#"'"content""sidebar"'"#
    );
    assert_eq!(
      transform_value_cached(
        "gridTemplateColumns",
        &TRawValue::from(r#"auto 0fr 0fr"#),
        &mut StateManager::default()
      ),
      r#"auto 0fr 0fr"#
    );
  }

  #[test]
  fn transform_properties() {
    assert_eq!(
      transform_value_cached(
        "--span-t",
        &TRawValue::from(r#"translateX(4px)"#),
        &mut StateManager::default()
      ),
      r#"translateX(4px)"#
    );
  }

  #[test]
  fn modern_color_formats() {
    assert_eq!(
      transform_value_cached(
        "color",
        &TRawValue::from(r#"oklch(42.1% 0.192 328.6 / 1)"#),
        &mut StateManager::default()
      ),
      r#"oklch(42.1% .192 328.6 / 1)"#
    );
    assert_eq!(
      transform_value_cached(
        "color",
        &TRawValue::from(r#"oklch(from var(--xs74gcj) l c h / 0.5)"#),
        &mut StateManager::default()
      ),
      r#"oklch(from var(--xs74gcj) l c h / .5)"#
    );
    assert_eq!(
      transform_value_cached(
        "color",
        &TRawValue::from(r#"oklch(59.69% 0.156 49.77  /  .5)"#),
        &mut StateManager::default()
      ),
      r#"oklch(59.69% .156 49.77 / .5)"#
    );
  }

  #[test]
  fn complex_gradients() {
    assert_eq!(
      transform_value_cached(
        "color",
        &TRawValue::from(
          r#"radial-gradient(circle at 0% 0%, oklch(from   var(--colors-tile-background) calc(l +  0.1) calc(c + 0.2) h) 0,  transparent  15%),  radial-gradient(circle at 80% 100%,oklch(from   var(--colors-tile-background)  calc(l - 0.25) calc(c + 0.01) h) 0, transparent 30%), linear-gradient(45deg,var(--colors-tile-background) 0%, oklch(from var(--colors-tile-background) calc(l - 0.1) calc(c + 0.3) h) 100%)"#
        ),
        &mut StateManager::default()
      ),
      r#"radial-gradient(circle at 0% 0%,oklch(from var(--colors-tile-background) calc(l + .1) calc(c + .2) h) 0,transparent 15%),radial-gradient(circle at 80% 100%,oklch(from var(--colors-tile-background) calc(l - .25) calc(c + .01) h) 0,transparent 30%),linear-gradient(45deg,var(--colors-tile-background) 0%,oklch(from var(--colors-tile-background) calc(l - .1) calc(c + .3) h) 100%)"#
    );
    assert_eq!(
      transform_value_cached(
        "color",
        &TRawValue::from(
          r#"linear-gradient(to right, oklch(from #000 calc(l + 0.1)  c  h  /  0.1) 10%, oklch(from #000 calc(l  +  0.2)  c  h)  18%,  oklch(from #000 calc(l  + 0.1)  c h / 0.1) 33%)"#
        ),
        &mut StateManager::default()
      ),
      r#"linear-gradient(to right,oklch(from #000 calc(l + .1) c h / .1) 10%,oklch(from #000 calc(l + .2) c h) 18%,oklch(from #000 calc(l + .1) c h / .1) 33%)"#
    );
  }

  #[test]
  fn relative_rgb_colors() {
    assert_eq!(
      transform_value_cached(
        "backgroundColor",
        &TRawValue::from(r#"rgb(from red r g b)"#),
        &mut StateManager::default()
      ),
      r#"rgb(from red r g b)"#
    );
    assert_eq!(
      transform_value_cached(
        "backgroundColor",
        &TRawValue::from(r#"rgba(from red r g b  /  0.5)"#),
        &mut StateManager::default()
      ),
      r#"rgba(from red r g b / .5)"#
    );
    assert_eq!(
      transform_value_cached(
        "color",
        &TRawValue::from(r#"rgb(from var(--xColor) calc(r * 2) g b)"#),
        &mut StateManager::default()
      ),
      r#"rgb(from var(--xColor) calc(r * 2) g b)"#
    );
  }

  #[test]
  fn oklab_colors() {
    assert_eq!(
      transform_value_cached(
        "color",
        &TRawValue::from(r#"oklab(40.101%   0.1147   0.0453)"#),
        &mut StateManager::default()
      ),
      r#"oklab(40.101% .1147 .0453)"#
    );

    assert_eq!(
      transform_value_cached(
        "color",
        &TRawValue::from(r#"var(--a)   var(--b)      var(--c)"#),
        &mut StateManager::default()
      ),
      r#"var(--a) var(--b) var(--c)"#
    );

    assert_eq!(
      transform_value_cached(
        "color",
        &TRawValue::from(r#"oklab(from #0000FF calc(l  +  0.1)  a  b  /  calc(alpha  *  0.9))"#),
        &mut StateManager::default()
      ),
      r#"oklab(from #0000FF calc(l + .1) a b / calc(alpha * .9))"#
    );
    assert_eq!(
      transform_value_cached(
        "color",
        &TRawValue::from(r#"oklab(from hsl(180 100% 50%) calc(l  -  0.1)  a  b)"#),
        &mut StateManager::default()
      ),
      r#"oklab(from hsl(180 100% 50%) calc(l - .1) a b)"#
    );
    assert_eq!(
      transform_value_cached(
        "color",
        &TRawValue::from(r#"oklab(from green l  a  b  /  0.5)"#),
        &mut StateManager::default()
      ),
      r#"oklab(from green l a b / .5)"#
    );
  }

  #[test]
  fn clamp_colors() {
    assert_eq!(
      transform_value_cached(
        "color",
        &TRawValue::from(r#"clamp(200px,  40%,     400px)"#),
        &mut StateManager::default()
      ),
      r#"clamp(200px,40%,400px)"#
    );

    assert_eq!(
      transform_value_cached(
        "color",
        &TRawValue::from(r#"clamp(min(10vw,      20rem),     300px,     max(90vw,     55rem))"#),
        &mut StateManager::default()
      ),
      r#"clamp(min(10vw,20rem),300px,max(90vw,55rem))"#
    );

    assert_eq!(
      transform_value_cached(
        "color",
        &TRawValue::from(
          r#"clamp(0, (var(--l-threshold, 0.623)   /  l - 1)   *    infinity,    1)"#
        ),
        &mut StateManager::default()
      ),
      r#"clamp(0,(var(--l-threshold,.623) / l - 1) * infinity,1)"#
    );
  }

  #[test]
  fn allow_url_properties() {
    assert_eq!(
      transform_value_cached(
        "backgroundImage",
        &TRawValue::from(r#"url("https://images.unsplash.com/photo-1634170380004-4b3b3b3b3b3b")"#),
        &mut StateManager::default()
      ),
      r#"url("https://images.unsplash.com/photo-1634170380004-4b3b3b3b3b3b")"#,
    );

    assert_eq!(
      transform_value_cached(
        "backgroundImage",
        &TRawValue::from(r#"url("http://images.unsplash.com/photo-1634170380004-4b3b3b3b3b3b")"#),
        &mut StateManager::default()
      ),
      r#"url("http://images.unsplash.com/photo-1634170380004-4b3b3b3b3b3b")"#,
    );

    assert_eq!(
      transform_value_cached(
        "backgroundImage",
        &TRawValue::from(r#"url("https://1.2.3.4/photo-1634170380004-4b3b3b3b3b3b")"#),
        &mut StateManager::default()
      ),
      r#"url("https://1.2.3.4/photo-1634170380004-4b3b3b3b3b3b")"#,
    );

    assert_eq!(
      transform_value_cached(
        "backgroundImage",
        &TRawValue::from(r#"url("http://1.2.3.4/photo-1634170380004-4b3b3b3b3b3b")"#),
        &mut StateManager::default()
      ),
      r#"url("http://1.2.3.4/photo-1634170380004-4b3b3b3b3b3b")"#,
    );

    assert_eq!(
      transform_value_cached(
        "backgroundImage",
        &TRawValue::from(r#"url("/photo-1634170380004-4b3b3b3b3b3b")"#),
        &mut StateManager::default()
      ),
      r#"url("/photo-1634170380004-4b3b3b3b3b3b")"#,
    );

    assert_eq!(
      transform_value_cached(
        "backgroundImage",
        &TRawValue::from(r#"url("./photo-1634170380004-4b3b3b3b3b3b")"#),
        &mut StateManager::default()
      ),
      r#"url("./photo-1634170380004-4b3b3b3b3b3b")"#,
    );

    assert_eq!(
      transform_value_cached(
        "backgroundImage",
        &TRawValue::from(
          r#"url(asset:communityEmpowermentRoles/Communities-Empowerment-Roles-Platform-Spark-New-Convo-QP-WWW_light)"#
        ),
        &mut StateManager::default()
      ),
      r#"url(asset:communityEmpowermentRoles/Communities-Empowerment-Roles-Platform-Spark-New-Convo-QP-WWW_light)"#,
    );

    assert_eq!(
      transform_value_cached(
        "backgroundImage",
        &TRawValue::from(
          r#"url("data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAYAAAAf8/9hAAAAMUlEQVQ4T2NkYGAQYcAP3uCTZhw1gGGYhAGBZIA/nYDCgBDAm9BGDWAAJyRCgLaBCAAgXwixzAS0pgAAAABJRU5ErkJggg==")"#
        ),
        &mut StateManager::default()
      ),
      r#"url("data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAYAAAAf8/9hAAAAMUlEQVQ4T2NkYGAQYcAP3uCTZhw1gGGYhAGBZIA/nYDCgBDAm9BGDWAAJyRCgLaBCAAgXwixzAS0pgAAAABJRU5ErkJggg==")"#,
    );
  }
  #[test]
  fn filter_properties() {
    assert_eq!(
      transform_value_cached(
        "filter",
        &TRawValue::from("drop-shadow(0 2px 10px rgba(0, 0, 0, 0.1))"),
        &mut StateManager::default()
      ),
      "drop-shadow(0 2px 10px rgba(0,0,0,.1))"
    );

    assert_eq!(
      transform_value_cached(
        "filter",
        &TRawValue::from("drop-shadow(0 -2px 10px rgba(0, 0, 0, 0.1))"),
        &mut StateManager::default()
      ),
      "drop-shadow(0 -2px 10px rgba(0,0,0,.1))"
    );
  }

  #[test]
  fn should_normalize_dimensions() {
    assert_eq!(
      transform_value_cached(
        "gridColumnStart",
        &TRawValue::from("1"),
        &mut StateManager::default()
      ),
      "1"
    );

    assert_eq!(
      transform_value_cached(
        "gridColumnStart",
        &TRawValue::from("-1"),
        &mut StateManager::default()
      ),
      "-1"
    );

    assert_eq!(
      transform_value_cached(
        "color",
        &TRawValue::from("calc(0 - var(--someVar))"),
        &mut StateManager::default()
      ),
      "calc(0 - var(--someVar))"
    );

    assert_eq!(
      transform_value_cached(
        "color",
        &TRawValue::from("calc(0px - var(--someVar))"),
        &mut StateManager::default()
      ),
      "calc(0px - var(--someVar))"
    );
  }
}
