//! What a name resolves to, against the declarations the state recorded.
//!
//! The state records *that* a file declares something. This module answers the
//! next question: given an identifier or an expression, which declaration binds
//! it, and what does that declaration spell? Both the visitor and the evaluator
//! above ask it.
//!
//! [`lookup`] matches a name against a declaration. [`convertors`] reads the
//! declaration back as a string or an expression. Nothing here folds an
//! expression: a conversion that would have to evaluate one belongs above this
//! crate, and that split is what keeps the state out of the evaluation cycle.

pub mod convertors;
pub mod lookup;
