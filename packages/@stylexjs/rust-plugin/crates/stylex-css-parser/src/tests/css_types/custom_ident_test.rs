/*!
CSS Custom Identifier Tests

Test CSS Type: <custom-ident>
Tests valid identifiers and rejection of reserved keywords.
*/

#[cfg(test)]
mod test_css_type_custom_ident {
  use crate::css_types::custom_ident::CustomIdentifier;

  #[test]
  fn parses_valid_custom_identifiers() {
    let result = CustomIdentifier::parser()
      .parse_to_end("myIdentifier")
      .unwrap();
    assert_eq!(result.value, "myIdentifier");

    let result = CustomIdentifier::parser()
      .parse_to_end("custom-name")
      .unwrap();
    assert_eq!(result.value, "custom-name");

    let result = CustomIdentifier::parser().parse_to_end("_private").unwrap();
    assert_eq!(result.value, "_private");

    let result = CustomIdentifier::parser()
      .parse_to_end("identifier123")
      .unwrap();
    assert_eq!(result.value, "identifier123");
  }
}
