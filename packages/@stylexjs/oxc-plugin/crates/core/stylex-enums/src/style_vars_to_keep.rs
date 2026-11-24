/// Generic over the front end's atom type (was SWC `Atom`).
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum NonNullProp<A = String> {
  Atom(A),
  True,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum NonNullProps<A = String> {
  Vec(Vec<A>),
  True,
}
