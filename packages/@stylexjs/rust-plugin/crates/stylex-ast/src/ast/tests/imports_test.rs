//! Tests for reading the local binding an import specifier introduces.

use swc_core::{
  common::DUMMY_SP,
  ecma::ast::{
    Ident, ImportDefaultSpecifier, ImportNamedSpecifier, ImportSpecifier, ImportStarAsSpecifier,
    ModuleExportName, Str,
  },
};

use crate::ast::imports::local_binding_of;

fn ident(name: &str) -> Ident {
  Ident::new_no_ctxt(name.into(), DUMMY_SP)
}

/// The local name a specifier binds, as text.
fn binding_of(specifier: &ImportSpecifier) -> &str {
  local_binding_of(specifier).sym.as_str()
}

fn named(local: &str, imported: Option<ModuleExportName>) -> ImportSpecifier {
  ImportSpecifier::Named(ImportNamedSpecifier {
    span: DUMMY_SP,
    local: ident(local),
    imported,
    is_type_only: false,
  })
}

// ──────────────────────────────────────────────
// The three import forms
// ──────────────────────────────────────────────

/// The plain form, `import { create } from '...'`. The local name and the
/// imported name are the same.
#[test]
fn reads_a_plain_named_import() {
  assert_eq!(binding_of(&named("create", None)), "create");
}

/// A renamed import, `import { create as make } from '...'`. The binding is
/// the alias, never the name the module exported.
#[test]
fn reads_the_alias_of_a_renamed_named_import() {
  let specifier = named("make", Some(ModuleExportName::Ident(ident("create"))));
  assert_eq!(binding_of(&specifier), "make");
}

/// A string export name, `import { 'create-style' as make } from '...'`. Such
/// a name cannot be an identifier, so the alias is the only usable binding.
#[test]
fn reads_the_alias_when_the_export_name_is_a_string() {
  let specifier = named(
    "make",
    Some(ModuleExportName::Str(Str {
      span: DUMMY_SP,
      value: "create-style".into(),
      raw: None,
    })),
  );
  assert_eq!(binding_of(&specifier), "make");
}

/// A default import, `import stylex from '...'`.
#[test]
fn reads_a_default_import() {
  let specifier = ImportSpecifier::Default(ImportDefaultSpecifier {
    span: DUMMY_SP,
    local: ident("stylex"),
  });
  assert_eq!(binding_of(&specifier), "stylex");
}

/// A namespace import, `import * as stylex from '...'`.
#[test]
fn reads_a_namespace_import() {
  let specifier = ImportSpecifier::Namespace(ImportStarAsSpecifier {
    span: DUMMY_SP,
    local: ident("stylex"),
  });
  assert_eq!(binding_of(&specifier), "stylex");
}

/// A type-only specifier binds its name in the same way. It is not a special
/// case.
#[test]
fn reads_a_type_only_named_import() {
  let specifier = ImportSpecifier::Named(ImportNamedSpecifier {
    span: DUMMY_SP,
    local: ident("StyleXStyles"),
    imported: None,
    is_type_only: true,
  });
  assert_eq!(binding_of(&specifier), "StyleXStyles");
}

// ──────────────────────────────────────────────
// Names at the edge of what an identifier may be
// ──────────────────────────────────────────────

/// JavaScript identifiers are not limited to ASCII. Nothing here parses the
/// name, so it must come back exactly as the parser made it.
#[test]
fn reads_names_that_are_not_plain_ascii() {
  for name in [
    "_",
    "$",
    "$$typeof",
    "ñ",
    "日本語",
    "\u{200c}zwnj",
    "_\u{200d}",
    "Ⅻ",
  ] {
    assert_eq!(binding_of(&named(name, None)), name);
  }
}

/// A minified bundle can carry a name much longer than a hand-written one.
/// The reader must give it back complete.
#[test]
fn reads_an_extremely_long_name() {
  let name = "a".repeat(100_000);
  assert_eq!(binding_of(&named(&name, None)), name);
}

/// An empty name is not valid JavaScript, but the AST can hold one. The
/// function gives back what is there and does not check it.
#[test]
fn reads_an_empty_name_without_failing() {
  assert_eq!(binding_of(&named("", None)), "");
}

// ──────────────────────────────────────────────
// What the caller gets back
// ──────────────────────────────────────────────

/// The result borrows from the specifier and does not copy it. This is what
/// lets a caller compare bindings without an allocation.
#[test]
fn borrows_from_the_specifier_it_was_given() {
  let specifier = named("create", None);
  let binding = local_binding_of(&specifier);

  let expected: *const Ident = match &specifier {
    ImportSpecifier::Named(inner) => &inner.local,
    _ => unreachable!("the specifier was built as named"),
  };
  assert!(std::ptr::eq(binding, expected));
}

/// All specifiers of one statement are read in the same way, so a whole import
/// list gives its bindings in order.
#[test]
fn reads_every_specifier_of_one_import_statement() {
  let specifiers = [
    ImportSpecifier::Default(ImportDefaultSpecifier {
      span: DUMMY_SP,
      local: ident("stylex"),
    }),
    named("create", None),
    named("make", Some(ModuleExportName::Ident(ident("createTheme")))),
    ImportSpecifier::Namespace(ImportStarAsSpecifier {
      span: DUMMY_SP,
      local: ident("all"),
    }),
  ];

  let bindings: Vec<&str> = specifiers.iter().map(binding_of).collect();

  assert_eq!(bindings, ["stylex", "create", "make", "all"]);
}
