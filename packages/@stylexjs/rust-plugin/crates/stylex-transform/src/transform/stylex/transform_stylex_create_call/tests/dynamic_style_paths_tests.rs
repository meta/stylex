//! A dynamic style's key is a prefix of its path, which the legacy shorthand expansion relies
//! on to rewrite a path's head (stated on `dynamic_styles_of_namespace`).

use indexmap::IndexMap;
use stylex_ast::ast::convertors::create_string_expr;
use stylex_enums::style_resolution::StyleResolution;
use stylex_state::types::TInlineStyles;
use stylex_structures::inline_style::InlineStyle;

use super::super::dynamic_style_functions::dynamic_styles_of_namespace;

fn inline_styles_of(paths: &[&[&str]]) -> TInlineStyles {
  let mut inline_styles: TInlineStyles = IndexMap::new();

  for (index, path) in paths.iter().enumerate() {
    inline_styles.insert(
      format!("--x-{index}"),
      Box::new(InlineStyle {
        path: path.iter().map(|step| (*step).to_string()).collect(),
        original_expression: create_string_expr("red"),
        expression: create_string_expr("red"),
      }),
    );
  }

  inline_styles
}

#[test]
fn a_key_is_a_prefix_of_its_path_that_ends_at_a_property() {
  let paths: &[&[&str]] = &[
    &["color"],
    &[":hover", "color"],
    &["@media (min-width: 1px)", ":hover", "color"],
    &["color", "default"],
    &["marginInline", "default"],
    &["@media (min-width: 1px)", "color", ":hover"],
  ];

  let dynamic_styles =
    dynamic_styles_of_namespace(&inline_styles_of(paths), &StyleResolution::ApplicationOrder);

  assert_eq!(dynamic_styles.len(), paths.len());

  for dynamic_style in &dynamic_styles {
    assert!(
      !dynamic_style.key.is_empty(),
      "a key that names no property: {dynamic_style:?}"
    );

    assert!(
      dynamic_style.path == dynamic_style.key
        || dynamic_style
          .path
          .starts_with(&(dynamic_style.key.clone() + "_")),
      "a key that is no prefix of its path: {dynamic_style:?}"
    );
  }
}

/// Hand-built, since `dynamic_styles_of_namespace` never writes one; an empty key is the one
/// answer the shorthand expansion cannot rewrite with.
#[test]
fn a_path_with_no_property_cuts_to_no_key() {
  let dynamic_styles = dynamic_styles_of_namespace(
    &inline_styles_of(&[&[":hover", "@media (min-width: 1px)"]]),
    &StyleResolution::ApplicationOrder,
  );

  assert_eq!(dynamic_styles[0].key, "");
  assert_eq!(dynamic_styles[0].path, ":hover_@media (min-width: 1px)");
}
