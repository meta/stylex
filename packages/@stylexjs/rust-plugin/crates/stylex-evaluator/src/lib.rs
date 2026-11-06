//! The JavaScript evaluator: what an authored expression folds to, or why it
//! cannot.
//!
//! Every `stylex.*` call reaches the stylesheet through a fold of the
//! expressions the author wrote, and a fold either answers with a value or
//! refuses. Refusing is a normal answer here rather than a failure -- a value
//! that cannot be known at compile time becomes an inline style instead -- so
//! nothing on this path may abort the process, and that is what shapes the crate
//! from the bottom up.
//!
//! `growable_stack` is the bottom: every descent a fold runs is given room of
//! its own, because a descent that overflows the thread it started on aborts
//! from inside an evaluation whose whole contract is that it may fail.
//!
//! [`evaluate`] is the way in. It dispatches on the shape of the expression, and
//! the fold behind it hands a self-contained call to a JavaScript engine rather
//! than to a table of method names. [`state`] carries what one evaluation knows
//! -- whether it is still confident, and where it stopped being so --
//! and [`evaluate_result`] is what it answers with.
//!
//! Three smaller modules sit beside them. [`convertors`] reads an expression
//! back as a number or a string, which it can only do by evaluating it, so it
//! belongs above the literal convertors in the declarations crate rather than
//! with them. `check_declaration` names why a resolved binding held nothing to
//! fold. [`stylex_first_that_works`] is the one StyleX function the engine
//! itself calls, so its fallback ordering lives here rather than with the
//! transformers the visitor drives.

pub(crate) mod check_declaration;
pub mod convertors;
pub mod evaluate;
pub mod evaluate_result;
pub(crate) mod growable_stack;
pub mod state;
pub mod stylex_first_that_works;

#[cfg(test)]
mod tests;
