/// The kind of declaration a name is bound to.
///
/// Only the kinds that hold no initializer to read, because those are the ones
/// a caller must report on rather than fold.
#[derive(Clone, Copy)]
pub enum DeclarationType {
  Class,
  Function,
}
