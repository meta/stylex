//! What a dynamic style's key and path say about each other.
//!
//! The legacy shorthand expansion rewrites a path by replacing the key at its
//! head, and it can only do that because the key is a prefix of the path. The
//! rule is stated on `dynamic_styles_of_namespace` and measured here.

use indexmap::IndexMap;
use stylex_ast::ast::convertors::create_string_expr;
use stylex_enums::style_resolution::StyleResolution;
use stylex_state::types::TInlineStyles;
use stylex_structures::inline_style::InlineStyle;

use super::super::dynamic_style_functions::dynamic_styles_of_namespace;

/// One inline style, written under `path`.
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
  // A property on its own, a property under one selector and under two, a
  // property with steps below it, and an at-rule above a pseudo selector.
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

/// A path with no property in it cuts to no key.
///
/// `dynamic_styles_of_namespace` does not write such a path -- a declaration is
/// written under a property -- so this one is built by hand. The case records
/// what the cut answers for it, because an empty key is the one answer the
/// shorthand expansion cannot rewrite a path with.
#[test]
fn a_path_with_no_property_cuts_to_no_key() {
  let dynamic_styles = dynamic_styles_of_namespace(
    &inline_styles_of(&[&[":hover", "@media (min-width: 1px)"]]),
    &StyleResolution::ApplicationOrder,
  );

  assert_eq!(dynamic_styles[0].key, "");
  assert_eq!(dynamic_styles[0].path, ":hover_@media (min-width: 1px)");
}
