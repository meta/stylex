//! Naming one export of one file.

/// Builds the readable identifier `file_name//export_name[.key]` for one export of one file.
pub fn gen_file_based_identifier(file_name: &str, export_name: &str, key: Option<&str>) -> String {
  const EXPORT_SEPARATOR: &str = "//";
  const KEY_SEPARATOR: &str = ".";

  let key_length = key.map_or(0, |key| KEY_SEPARATOR.len() + key.len());
  let mut identifier = String::with_capacity(
    file_name.len() + EXPORT_SEPARATOR.len() + export_name.len() + key_length,
  );

  identifier.push_str(file_name);
  identifier.push_str(EXPORT_SEPARATOR);
  identifier.push_str(export_name);

  if let Some(key) = key {
    identifier.push_str(KEY_SEPARATOR);
    identifier.push_str(key);
  }

  identifier
}

#[cfg(test)]
#[path = "tests/identifier_test.rs"]
mod tests;
