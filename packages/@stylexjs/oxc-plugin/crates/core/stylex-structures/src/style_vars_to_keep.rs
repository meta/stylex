use stylex_enums::style_vars_to_keep::{NonNullProp, NonNullProps};

/// Generic over the front end's binding id `I` and atom `A` (was SWC `Id` and `Atom`).
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct StyleVarsToKeep<I, A = String>(pub I, pub NonNullProp<A>, pub NonNullProps<A>);
