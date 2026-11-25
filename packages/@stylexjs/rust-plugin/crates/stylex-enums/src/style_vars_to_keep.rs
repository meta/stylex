use swc_core::atoms::Atom;

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum NonNullProp {
  Atom(Atom),
  True,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum NonNullProps {
  Vec(Vec<Atom>),
  True,
}
