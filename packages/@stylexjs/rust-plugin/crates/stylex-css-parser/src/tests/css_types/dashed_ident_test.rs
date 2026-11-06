/*!
CSS Dashed Identifier Tests

Test CSS Type: <dashed-ident>
Tests valid dashed identifiers and rejection of invalid formats.
*/

#[cfg(test)]
mod test_css_type_dashed_ident {
  use crate::css_types::dashed_ident::DashedIdentifier;

  #[test]
  fn parses_valid_dashed_identifiers() {
    let result = DashedIdentifier::parser()
      .parse_to_end("--custom-property")
      .unwrap();
    assert_eq!(result.value, "--custom-property");

    let result = DashedIdentifier::parser()
      .parse_to_end("--theme-color")
      .unwrap();
    assert_eq!(result.value, "--theme-color");

    let result = DashedIdentifier::parser().parse_to_end("--123").unwrap();
    assert_eq!(result.value, "--123");

    let result = DashedIdentifier::parser()
      .parse_to_end("--_private")
      .unwrap();
    assert_eq!(result.value, "--_private");
  }

  #[test]
  fn rejects_invalid_dashed_identifiers() {
    assert!(
      DashedIdentifier::parser()
        .parse_to_end("custom-property")
        .is_err()
    );
    assert!(
      DashedIdentifier::parser()
        .parse_to_end("-custom-property")
        .is_err()
    );
    assert!(DashedIdentifier::parser().parse_to_end("property").is_err());
    assert!(DashedIdentifier::parser().parse_to_end("--").is_err());
  }
}
