use crate::identifier::gen_file_based_identifier;

mod gen_file_based_identifier_tests {
  use super::*;

  #[test]
  fn generates_identifier_without_key() {
    let result = gen_file_based_identifier("file.js", "styles", None);
    assert_eq!(result, "file.js//styles");
  }

  #[test]
  fn generates_identifier_with_key() {
    let result = gen_file_based_identifier("file.js", "styles", Some("color"));
    assert_eq!(result, "file.js//styles.color");
  }

  #[test]
  fn handles_empty_file_name() {
    let result = gen_file_based_identifier("", "export", None);
    assert_eq!(result, "//export");
  }

  #[test]
  fn handles_empty_export_name() {
    let result = gen_file_based_identifier("file.js", "", None);
    assert_eq!(result, "file.js//");
  }

  /// A key that is present but empty still spells the separator, because the
  /// name says "one member of this export" and an empty member name is not the
  /// same thing as no member at all.
  #[test]
  fn keeps_the_separator_for_an_empty_key() {
    let result = gen_file_based_identifier("file.js", "styles", Some(""));
    assert_eq!(result, "file.js//styles.");
  }

  #[test]
  fn keeps_every_part_of_a_key_that_holds_the_separators() {
    let result = gen_file_based_identifier("a//b.js", "styles", Some("a.b//c"));
    assert_eq!(result, "a//b.js//styles.a.b//c");
  }

  #[test]
  fn passes_multi_byte_names_through_unchanged() {
    let result = gen_file_based_identifier("файл.js", "стили", Some("цвет"));
    assert_eq!(result, "файл.js//стили.цвет");
  }

  /// A path far longer than any real one, to show that nothing here has a
  /// length bound of its own.
  #[test]
  fn handles_a_name_far_longer_than_any_real_path() {
    let file_name = "d/".repeat(100_000);
    let result = gen_file_based_identifier(&file_name, "styles", Some("color"));

    assert_eq!(result.len(), file_name.len() + "//styles.color".len());
    assert!(result.starts_with(&file_name));
    assert!(result.ends_with("//styles.color"));
  }
}
