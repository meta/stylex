//! Reading what an import specifier brings into scope.

use swc_core::ecma::ast::{Ident, ImportSpecifier};

/// The name a specifier binds locally, whichever import form it takes.
pub fn local_binding_of(specifier: &ImportSpecifier) -> &Ident {
  match specifier {
    ImportSpecifier::Named(named) => &named.local,
    ImportSpecifier::Default(default) => &default.local,
    ImportSpecifier::Namespace(namespace) => &namespace.local,
  }
}
