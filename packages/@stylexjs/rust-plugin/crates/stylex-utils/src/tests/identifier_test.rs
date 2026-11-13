use crate::identifier::gen_file_based_identifier;

mod gen_file_based_identifier_tests {
  use super::*;

  #[test]
  fn generates_identifier_with_key() {
    let result = gen_file_based_identifier("file.js", "styles", Some("color"));
    assert_eq!(result, "file.js//styles.color");
  }
}
