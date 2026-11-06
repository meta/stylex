//! Naming one export of one file.

/// The identifier that names `export_name` of `file_name`, with `key` naming
/// one member inside that export where there is one.
///
/// Two files that export the same name never collide, because the file path is
/// part of the name. The un-hashed sibling of [`crate::hash::create_key_hash`]:
/// this one stays readable, and is hashed later where a short name is needed.
///
/// Built in one exactly-sized allocation. Every part is a `&str` of known
/// length, so the name they form has a known length too -- where formatting the
/// key into a `String` of its own first paid for a second allocation the caller
/// never sees.
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
