//! Where each call in a module is written.

use rustc_hash::FxHashSet;
use swc_core::common::Span;

/// The positions one module's calls stand in.
#[derive(Debug, Default, Clone)]
pub struct CallPositions {
  /// The calls a statement of the module itself holds, with no function and no second statement around them.
  program_level: FxHashSet<Span>,
  /// The calls that are a whole expression statement, so nothing reads what they answer.
  bare_statements: FxHashSet<Span>,
  /// The calls a type assertion wraps -- `create({…}) as Styles`.
  type_asserted: FxHashSet<Span>,
}

impl CallPositions {
  /// Records `span` under each position it stands in.
  pub fn record(&mut self, span: Span, position: Position) {
    match position {
      Position::ProgramLevel => self.program_level.insert(span),
      Position::BareStatement => self.bare_statements.insert(span),
      Position::TypeAsserted => self.type_asserted.insert(span),
    };
  }

  /// Whether the call written at `span` stands in `position`.
  pub fn holds(&self, span: Span, position: Position) -> bool {
    if span.is_dummy() {
      return false;
    }

    match position {
      Position::ProgramLevel => self.program_level.contains(&span),
      Position::BareStatement => self.bare_statements.contains(&span),
      Position::TypeAsserted => self.type_asserted.contains(&span),
    }
  }
}

/// One of the positions a call is read for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
  /// A statement of the module itself holds the call, with no function and no second statement between.
  ProgramLevel,
  /// The call is a whole expression statement, so nothing reads what it answers.
  BareStatement,
  /// A type assertion wraps the call.
  TypeAsserted,
}
