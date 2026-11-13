/// The kind of declaration a name is bound to, for kinds with no initializer to fold.
#[derive(Clone, Copy)]
pub enum DeclarationType {
  Class,
  Function,
}
