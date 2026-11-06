#[derive(Debug, PartialEq, Eq, Clone, Hash, Copy)]
pub enum TopLevelExpressionKind {
  NamedExport,
  DefaultExport,
  Stmt,
}
