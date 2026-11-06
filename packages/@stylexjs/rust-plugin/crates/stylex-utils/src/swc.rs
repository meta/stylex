use std::path::Path;

use swc_core::ecma::ast::{BinaryOp, Decl, Expr, Lit, OptChainBase, Stmt};
use swc_ecma_parser::{Syntax, TsSyntax};

/// Returns the ESTree node kind of an expression for diagnostics, e.g. `"CallExpression"`.
/// Logical operators, optional chains and `super.x` follow ESTree rather than SWC's node shapes.
pub fn get_expr_node_kind(expr: &Expr) -> &'static str {
  match expr {
    Expr::This(_) => "ThisExpression",
    Expr::Array(_) => "ArrayExpression",
    Expr::Object(_) => "ObjectExpression",
    Expr::Fn(_) => "FunctionExpression",
    Expr::Unary(_) => "UnaryExpression",
    Expr::Update(_) => "UpdateExpression",
    Expr::Bin(bin) => match bin.op {
      BinaryOp::LogicalOr | BinaryOp::LogicalAnd | BinaryOp::NullishCoalescing => {
        "LogicalExpression"
      },
      _ => "BinaryExpression",
    },
    Expr::Assign(_) => "AssignmentExpression",
    Expr::Member(_) | Expr::SuperProp(_) => "MemberExpression",
    Expr::Cond(_) => "ConditionalExpression",
    Expr::Call(_) => "CallExpression",
    Expr::New(_) => "NewExpression",
    Expr::Seq(_) => "SequenceExpression",
    Expr::Ident(_) => "Identifier",
    Expr::Lit(lit) => match lit {
      Lit::Str(_) => "StringLiteral",
      Lit::Bool(_) => "BooleanLiteral",
      Lit::Null(_) => "NullLiteral",
      Lit::Num(_) => "NumericLiteral",
      Lit::BigInt(_) => "BigIntLiteral",
      Lit::Regex(_) => "RegExpLiteral",
      Lit::JSXText(_) => "JSXText",
    },
    Expr::Tpl(_) => "TemplateLiteral",
    Expr::TaggedTpl(_) => "TaggedTemplateExpression",
    Expr::Arrow(_) => "ArrowFunctionExpression",
    Expr::Class(_) => "ClassExpression",
    Expr::Yield(_) => "YieldExpression",
    Expr::MetaProp(_) => "MetaProperty",
    Expr::Await(_) => "AwaitExpression",
    Expr::Paren(_) => "ParenthesizedExpression",
    Expr::JSXMember(_) => "JSXMemberExpression",
    Expr::JSXNamespacedName(_) => "JSXNamespacedName",
    Expr::JSXEmpty(_) => "JSXEmptyExpression",
    Expr::JSXElement(_) => "JSXElement",
    Expr::JSXFragment(_) => "JSXFragment",
    Expr::TsTypeAssertion(_) => "TSTypeAssertion",
    Expr::TsConstAssertion(_) => "TSAsExpression",
    Expr::TsNonNull(_) => "TSNonNullExpression",
    Expr::TsAs(_) => "TSAsExpression",
    Expr::TsInstantiation(_) => "TSInstantiationExpression",
    Expr::TsSatisfies(_) => "TSSatisfiesExpression",
    Expr::PrivateName(_) => "PrivateName",
    Expr::OptChain(opt_chain) => match opt_chain.base.as_ref() {
      OptChainBase::Member(_) => "OptionalMemberExpression",
      OptChainBase::Call(_) => "OptionalCallExpression",
    },
    Expr::Invalid(_) => "Invalid",
  }
}

/// Returns the ESTree node kind of a statement for diagnostics, e.g. `"ForStatement"`.
pub fn get_stmt_node_kind(stmt: &Stmt) -> &'static str {
  match stmt {
    Stmt::Block(_) => "BlockStatement",
    Stmt::Empty(_) => "EmptyStatement",
    Stmt::Debugger(_) => "DebuggerStatement",
    Stmt::With(_) => "WithStatement",
    Stmt::Return(_) => "ReturnStatement",
    Stmt::Labeled(_) => "LabeledStatement",
    Stmt::Break(_) => "BreakStatement",
    Stmt::Continue(_) => "ContinueStatement",
    Stmt::If(_) => "IfStatement",
    Stmt::Switch(_) => "SwitchStatement",
    Stmt::Throw(_) => "ThrowStatement",
    Stmt::Try(_) => "TryStatement",
    Stmt::While(_) => "WhileStatement",
    Stmt::DoWhile(_) => "DoWhileStatement",
    Stmt::For(_) => "ForStatement",
    Stmt::ForIn(_) => "ForInStatement",
    Stmt::ForOf(_) => "ForOfStatement",
    Stmt::Expr(_) => "ExpressionStatement",
    Stmt::Decl(decl) => match decl {
      Decl::Class(_) => "ClassDeclaration",
      Decl::Fn(_) => "FunctionDeclaration",
      Decl::Var(_) => "VariableDeclaration",
      Decl::Using(_) => "VariableDeclaration",
      Decl::TsInterface(_) => "TSInterfaceDeclaration",
      Decl::TsTypeAlias(_) => "TSTypeAliasDeclaration",
      Decl::TsEnum(_) => "TSEnumDeclaration",
      Decl::TsModule(_) => "TSModuleDeclaration",
    },
  }
}

/// Extensions where TypeScript disallows JSX, so `<T>` is a type parameter or assertion.
const TYPESCRIPT_WITHOUT_JSX_EXTENSIONS: [&str; 3] = ["ts", "mts", "cts"];

/// Returns the TypeScript syntax for `path`, enabling JSX unless the extension is TS-only.
/// Matches `tsc`, Babel and Next.js; extensions are matched case-insensitively.
pub fn syntax_for_file(path: &Path) -> Syntax {
  Syntax::Typescript(TsSyntax {
    tsx: admits_jsx(path),
    ..Default::default()
  })
}

/// Whether an angle bracket in a module at `path` may open a JSX element.
fn admits_jsx(path: &Path) -> bool {
  !path
    .extension()
    .and_then(|extension| extension.to_str())
    .is_some_and(|extension| {
      TYPESCRIPT_WITHOUT_JSX_EXTENSIONS
        .iter()
        .any(|typescript| typescript.eq_ignore_ascii_case(extension))
    })
}

#[cfg(test)]
#[path = "tests/swc_test.rs"]
mod tests;
