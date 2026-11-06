//! Where each call in a module is written.
//!
//! Three questions about one call, all of them about the parent an SWC visitor
//! does not carry, and all of them answered by the span of the call. The walk
//! that fills this is `state_writers::fill_call_positions`.

use rustc_hash::FxHashSet;
use swc_core::common::Span;

/// The positions one module's calls stand in.
///
/// One record rather than three sets on the state manager, because the three
/// are filled by one walk and asked of one call. It is shared behind an `Rc`,
/// for the reason every other index on the state manager is: a dynamic style's
/// callback clones the whole state per invocation.
#[derive(Debug, Default, Clone)]
pub struct CallPositions {
  /// The calls a statement of the module itself holds, with no function and no
  /// second statement around them.
  program_level: FxHashSet<Span>,
  /// The calls that are a whole expression statement, so nothing reads what
  /// they answer.
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
  ///
  /// A span-less call stands nowhere: it is synthesized rather than parsed, so
  /// the walk that read the module cannot have seen it, and a dummy span would
  /// otherwise be read as position zero.
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
  /// A statement of the module itself holds the call, with no function and no
  /// second statement between. This decides whether the compiled styles stay
  /// where the call was written or are hoisted to a declaration above.
  ProgramLevel,
  /// The call is a whole expression statement, so nothing reads what it
  /// answers.
  BareStatement,
  /// A type assertion wraps the call.
  TypeAsserted,
}
