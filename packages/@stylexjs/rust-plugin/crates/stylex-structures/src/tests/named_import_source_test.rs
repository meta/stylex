//! Tests for ImportSources and RuntimeInjection enum accessor methods.

use crate::named_import_source::*;

#[test]
fn import_sources_regular_is_not_named() {
  let src = ImportSources::Regular("stylex".to_string());
  assert!(!src.is_named_export());
}

#[test]
fn import_sources_named_is_named() {
  let src = ImportSources::Named(NamedImportSource {
    r#as: "css".to_string(),
    from: "@stylexjs/stylex".to_string(),
  });
  assert!(src.is_named_export());
}

#[test]
fn import_sources_get_import_str_regular() {
  let src = ImportSources::Regular("stylex".to_string());
  assert_eq!(src.get_import_str(), "stylex");
}

#[test]
fn import_sources_get_import_str_named() {
  let src = ImportSources::Named(NamedImportSource {
    r#as: "css".to_string(),
    from: "@stylexjs/stylex".to_string(),
  });
  assert_eq!(src.get_import_str(), "css");
}

#[test]
fn runtime_injection_is_boolean_export() {
  assert!(RuntimeInjection::Boolean(true).is_boolean_export());
  assert!(!RuntimeInjection::Regular("x".into()).is_boolean_export());
  assert!(
    !RuntimeInjection::Named(NamedImportSource {
      r#as: "a".into(),
      from: "b".into(),
    })
    .is_boolean_export()
  );
}

#[test]
fn runtime_injection_is_regular_export() {
  assert!(!RuntimeInjection::Boolean(true).is_regular_export());
  assert!(RuntimeInjection::Regular("x".into()).is_regular_export());
  assert!(
    !RuntimeInjection::Named(NamedImportSource {
      r#as: "a".into(),
      from: "b".into(),
    })
    .is_regular_export()
  );
}

#[test]
fn runtime_injection_is_named_export() {
  assert!(!RuntimeInjection::Boolean(true).is_named_export());
  assert!(!RuntimeInjection::Regular("x".into()).is_named_export());
  assert!(
    RuntimeInjection::Named(NamedImportSource {
      r#as: "a".into(),
      from: "b".into(),
    })
    .is_named_export()
  );
}

#[test]
fn runtime_injection_as_boolean() {
  assert_eq!(RuntimeInjection::Boolean(true).as_boolean(), Some(&true));
  assert_eq!(RuntimeInjection::Boolean(false).as_boolean(), Some(&false));
  assert!(RuntimeInjection::Regular("x".into()).as_boolean().is_none());
  assert!(
    RuntimeInjection::Named(NamedImportSource {
      r#as: "a".into(),
      from: "b".into(),
    })
    .as_boolean()
    .is_none()
  );
}

#[test]
fn runtime_injection_as_regular() {
  assert!(RuntimeInjection::Boolean(true).as_regular().is_none());
  assert_eq!(
    RuntimeInjection::Regular("path".into()).as_regular(),
    Some(&"path".to_string())
  );
  assert!(
    RuntimeInjection::Named(NamedImportSource {
      r#as: "a".into(),
      from: "b".into(),
    })
    .as_regular()
    .is_none()
  );
}

#[test]
fn runtime_injection_as_named() {
  assert!(RuntimeInjection::Boolean(true).as_named().is_none());
  assert!(RuntimeInjection::Regular("x".into()).as_named().is_none());
  let named = NamedImportSource {
    r#as: "css".into(),
    from: "stylex".into(),
  };
  let ri = RuntimeInjection::Named(named.clone());
  assert_eq!(ri.as_named(), Some(&named));
}
