//! Nesting limits for SWC's parser and for the passes that later walk its tree.
//!
//! Parsing runs on a large stack and stops cleanly if that stack runs low. The
//! parsed tree is then checked against three nesting budgets before any other
//! recursive pass walks it.

use swc_core::common::{comments::Comments, BytePos, SourceFile, Span, Spanned};
use swc_core::ecma::ast::{
    AwaitExpr, BlockStmt, Class, EsVersion, Expr, FunctionBody, JSXElement, JSXFragment,
    JSXMemberExpr, Module, ModuleItem, Pat, Program, Script, Stmt, TsModuleBlock, TsNamespaceDecl,
    TsQualifiedName, TsType, UnaryExpr, UpdateExpr,
};
use swc_core::ecma::visit::{Visit, VisitWith};
use swc_ecma_parser::{
    error::{Error, SyntaxError},
    input::Tokens,
    unstable::{Token, TokenAndSpan, TokenFlags, TokenValue},
    Context, FlowSyntax, Lexer, PResult, Parser, StringInput, Syntax, SyntaxFlags, TsSyntax,
};

pub(crate) fn syntax_for_filename(filename: &str) -> Syntax {
    if [".ts", ".tsx", ".mts", ".cts"]
        .iter()
        .any(|extension| filename.ends_with(extension))
    {
        Syntax::Typescript(TsSyntax {
            tsx: filename.ends_with(".tsx"),
            decorators: true,
            ..Default::default()
        })
    } else {
        Syntax::Flow(FlowSyntax {
            jsx: true,
            all: true,
            decorators: true,
            enums: true,
            components: true,
            ..Default::default()
        })
    }
}

/// Expressions of any kind on one path, including chains (`a.b().c`, `a + b + c`, casts).
pub(crate) const MAX_SYNTAX_COMPLEXITY: usize = 2048;
/// Delimited and prefix expressions, arrows, types and patterns on one path.
pub(crate) const MAX_RECURSIVE_DEPTH: usize = 1024;
/// Blocks and control statements on one path.
pub(crate) const MAX_STATEMENT_DEPTH: usize = 8192;
// Stack the parser needs before entering recursive descent.
pub(crate) const PARSER_STACK_RED_ZONE: usize = PARSER_STACK_SIZE;
pub(crate) const PARSER_STACK_SIZE: usize = 256 * 1024 * 1024;
/// Parser stack one file may use: about twice what the deepest file within the budgets needs,
/// so an absurdly deep file stops early instead of filling the whole stack.
const PARSER_STACK_BUDGET: usize = if cfg!(debug_assertions) {
    224 * 1024 * 1024
} else {
    48 * 1024 * 1024
};
/// Parsing also stops when less than this much stack is left.
const PARSER_STACK_RESERVE: usize = 16 * 1024 * 1024;

fn nesting_error(span: Span, budget: &str, limit: &str) -> Error {
    Error::new(
        span,
        SyntaxError::Unexpected {
            got: format!("compiler {budget} exceeds {limit}"),
            expected: "bounded syntax (split deeply nested statements or chained expressions)",
        },
    )
}

/// A lexer that stops the parse with an error once the parser stack runs low.
#[derive(Clone)]
pub(crate) struct BoundedLexer<'a> {
    lexer: Lexer<'a>,
    stack_start: usize,
    exhausted: Option<Error>,
    reported: bool,
}

impl<'a> BoundedLexer<'a> {
    fn new(lexer: Lexer<'a>) -> Self {
        Self {
            lexer,
            stack_start: stacker::remaining_stack().unwrap_or(0),
            exhausted: None,
            reported: false,
        }
    }

    fn guard(&mut self, mut token: TokenAndSpan, jsx_scan: bool) -> TokenAndSpan {
        if self.exhausted.is_none()
            && stacker::remaining_stack().is_some_and(|left| {
                left < PARSER_STACK_RESERVE
                    || self.stack_start.saturating_sub(left) > PARSER_STACK_BUDGET
            })
        {
            self.exhausted = Some(nesting_error(
                token.span,
                "syntax nesting/complexity",
                "the parser stack",
            ));
        }
        // Every later token fails too, so the parser unwinds rather than backtracking deeper.
        if let Some(error) = &self.exhausted {
            if jsx_scan {
                if token.token != Token::JSXName {
                    token.token = Token::Eof;
                }
            } else {
                self.lexer
                    .set_token_value(Some(TokenValue::Error(error.clone())));
                token.token = Token::Error;
            }
        }
        token
    }
}

impl<'a> Tokens for BoundedLexer<'a> {
    type Checkpoint = <Lexer<'a> as Tokens>::Checkpoint;
    fn set_ctx(&mut self, ctx: Context) {
        self.lexer.set_ctx(ctx);
    }
    fn ctx(&self) -> Context {
        self.lexer.ctx()
    }
    fn ctx_mut(&mut self) -> &mut Context {
        self.lexer.ctx_mut()
    }
    fn syntax(&self) -> SyntaxFlags {
        self.lexer
            .syntax()
            .union(SyntaxFlags::EXPLICIT_RESOURCE_MANAGEMENT)
    }
    fn target(&self) -> EsVersion {
        self.lexer.target()
    }
    fn checkpoint_save(&self) -> Self::Checkpoint {
        self.lexer.checkpoint_save()
    }
    fn checkpoint_load(&mut self, checkpoint: Self::Checkpoint) {
        self.lexer.checkpoint_load(checkpoint);
    }
    fn read_string(&self, span: Span) -> &str {
        self.lexer.read_string(span)
    }
    fn start_pos(&self) -> BytePos {
        self.lexer.start_pos()
    }
    fn set_expr_allowed(&mut self, allow: bool) {
        self.lexer.set_expr_allowed(allow);
    }
    fn set_next_regexp(&mut self, start: Option<BytePos>) {
        self.lexer.set_next_regexp(start);
    }
    fn add_error(&mut self, error: Error) {
        self.lexer.add_error(error);
    }
    fn add_module_mode_error(&mut self, error: Error) {
        self.lexer.add_module_mode_error(error);
    }
    fn end_pos(&self) -> BytePos {
        self.lexer.end_pos()
    }
    fn take_errors(&mut self) -> Vec<Error> {
        let mut errors = self.lexer.take_errors();
        if !self.reported {
            if let Some(error) = &self.exhausted {
                errors.push(error.clone());
                self.reported = true;
            }
        }
        errors
    }
    fn has_errors(&self) -> bool {
        self.exhausted.is_some() || self.lexer.has_errors()
    }
    fn diagnostic_checkpoint_save(&self) -> (usize, usize) {
        self.lexer.diagnostic_checkpoint_save()
    }
    fn diagnostic_checkpoint_load(&mut self, checkpoint: (usize, usize)) {
        self.lexer.diagnostic_checkpoint_load(checkpoint);
    }
    fn take_script_module_errors(&mut self) -> Vec<Error> {
        self.lexer.take_script_module_errors()
    }
    fn set_defer_comments(&mut self, defer: bool) {
        self.lexer.set_defer_comments(defer);
    }
    fn finalize_comments(&mut self) {
        self.lexer.finalize_comments();
    }
    fn update_token_flags(&mut self, update: impl FnOnce(&mut TokenFlags)) {
        self.lexer.update_token_flags(update);
    }
    fn token_flags(&self) -> TokenFlags {
        self.lexer.token_flags()
    }
    fn clone_token_value(&self) -> Option<TokenValue> {
        self.lexer.clone_token_value()
    }
    fn take_token_value(&mut self) -> Option<TokenValue> {
        self.lexer.take_token_value()
    }
    fn get_token_value(&self) -> Option<&TokenValue> {
        self.lexer.get_token_value()
    }
    fn set_token_value(&mut self, value: Option<TokenValue>) {
        self.lexer.set_token_value(value);
    }
    fn first_token(&mut self) -> TokenAndSpan {
        let token = self.lexer.first_token();
        self.guard(token, false)
    }
    fn next_token(&mut self) -> TokenAndSpan {
        let token = self.lexer.next_token();
        self.guard(token, false)
    }
    fn scan_jsx_token(&mut self) -> TokenAndSpan {
        let token = self.lexer.scan_jsx_token();
        self.guard(token, true)
    }
    fn scan_jsx_open_el_terminal_token(&mut self) -> TokenAndSpan {
        let token = self.lexer.scan_jsx_open_el_terminal_token();
        self.guard(token, true)
    }
    fn rescan_jsx_open_el_terminal_token(&mut self, reset: BytePos) -> TokenAndSpan {
        let token = self.lexer.rescan_jsx_open_el_terminal_token(reset);
        self.guard(token, true)
    }
    fn rescan_jsx_token(&mut self, reset: BytePos) -> TokenAndSpan {
        let token = self.lexer.rescan_jsx_token(reset);
        self.guard(token, true)
    }
    fn rescan_type_gt(&mut self, span: Span) -> TokenAndSpan {
        let token = self.lexer.rescan_type_gt(span);
        self.guard(token, false)
    }
    fn scan_jsx_identifier(&mut self, start: BytePos) -> TokenAndSpan {
        let token = self.lexer.scan_jsx_identifier(start);
        self.guard(token, true)
    }
    fn scan_jsx_attribute_value(&mut self) -> TokenAndSpan {
        let token = self.lexer.scan_jsx_attribute_value();
        self.guard(token, true)
    }
    fn rescan_template_token(&mut self, start: BytePos, back_tick: bool) -> TokenAndSpan {
        let token = self.lexer.rescan_template_token(start, back_tick);
        self.guard(token, false)
    }
}

/// Counts nesting along each path of a parsed tree and stops at the first budget exceeded.
#[derive(Default)]
pub(crate) struct NestingCheck {
    statements: usize,
    expressions: usize,
    complexity: usize,
    exceeded: Option<Error>,
}

impl NestingCheck {
    /// Enters one level; `None` when a budget is now exceeded and the subtree is skipped.
    fn enter(
        &mut self,
        span: Span,
        statements: usize,
        expressions: usize,
        complexity: usize,
    ) -> Option<(usize, usize, usize)> {
        if self.exceeded.is_some() {
            return None;
        }
        let saved = (self.statements, self.expressions, self.complexity);
        self.statements += statements;
        self.expressions += expressions;
        self.complexity += complexity;
        let exceeded = if self.statements > MAX_STATEMENT_DEPTH {
            Some(("statement/block nesting", MAX_STATEMENT_DEPTH))
        } else if self.expressions > MAX_RECURSIVE_DEPTH {
            Some((
                "recursive expression/delimiter nesting",
                MAX_RECURSIVE_DEPTH,
            ))
        } else if self.complexity > MAX_SYNTAX_COMPLEXITY {
            Some(("syntax nesting/complexity", MAX_SYNTAX_COMPLEXITY))
        } else {
            None
        };
        if let Some((budget, limit)) = exceeded {
            self.exceeded = Some(nesting_error(span, budget, &limit.to_string()));
            return None;
        }
        Some(saved)
    }

    fn leave(&mut self, saved: (usize, usize, usize)) {
        (self.statements, self.expressions, self.complexity) = saved;
    }

    fn nest<N: VisitWith<Self> + ?Sized>(
        &mut self,
        node: &N,
        span: Span,
        statements: usize,
        expressions: usize,
        complexity: usize,
    ) {
        if let Some(saved) = self.enter(span, statements, expressions, complexity) {
            node.visit_children_with(self);
            self.leave(saved);
        }
    }
}

impl Visit for NestingCheck {
    fn visit_stmt(&mut self, node: &Stmt) {
        // Try blocks count only as blocks; a switch also has its body.
        let statements = match node {
            Stmt::If(_)
            | Stmt::While(_)
            | Stmt::DoWhile(_)
            | Stmt::For(_)
            | Stmt::ForIn(_)
            | Stmt::ForOf(_)
            | Stmt::Labeled(_)
            | Stmt::With(_) => 1,
            Stmt::Switch(_) => 2,
            _ => 0,
        };
        self.nest(node, node.span(), statements, 0, 0);
    }
    fn visit_block_stmt(&mut self, node: &BlockStmt) {
        self.nest(node, node.span, 1, 0, 0);
    }
    fn visit_function_body(&mut self, node: &FunctionBody) {
        self.nest(node, node.span, 1, 0, 0);
    }
    fn visit_ts_module_block(&mut self, node: &TsModuleBlock) {
        self.nest(node, node.span, 1, 0, 0);
    }
    fn visit_ts_namespace_decl(&mut self, node: &TsNamespaceDecl) {
        // Each dot of `namespace a.b.c`.
        self.nest(node, node.span, 0, 0, 1);
    }
    fn visit_class(&mut self, node: &Class) {
        self.nest(node, node.span, 0, 1, 1);
    }
    fn visit_expr(&mut self, node: &Expr) {
        let (expressions, complexity) = match node {
            // Leaves, and functions and classes, which count through their bodies.
            Expr::Ident(_)
            | Expr::Lit(_)
            | Expr::This(_)
            | Expr::PrivateName(_)
            | Expr::MetaProp(_)
            | Expr::Invalid(_)
            | Expr::Fn(_)
            | Expr::Class(_) => (0, 0),
            Expr::Tpl(tpl) if tpl.exprs.is_empty() => (0, 0),
            // A prefix operator on a parenthesized operand counts through the parentheses.
            Expr::Unary(UnaryExpr { arg, .. })
            | Expr::Update(UpdateExpr { arg, .. })
            | Expr::Await(AwaitExpr { arg, .. })
                if matches!(**arg, Expr::Paren(_)) =>
            {
                (0, 0)
            }
            // Operator chains count only toward complexity.
            Expr::Member(_)
            | Expr::SuperProp(_)
            | Expr::Call(_)
            | Expr::New(_)
            | Expr::Bin(_)
            | Expr::Assign(_)
            | Expr::Cond(_)
            | Expr::TaggedTpl(_)
            | Expr::OptChain(_)
            | Expr::TsAs(_)
            | Expr::TsSatisfies(_)
            | Expr::TsNonNull(_)
            | Expr::TsTypeAssertion(_)
            | Expr::TsConstAssertion(_)
            | Expr::TsInstantiation(_) => (0, 1),
            _ => (1, 1),
        };
        self.nest(node, node.span(), 0, expressions, complexity);
    }
    fn visit_ts_type(&mut self, node: &TsType) {
        let leaf = match node {
            TsType::TsKeywordType(_) | TsType::TsThisType(_) | TsType::TsLitType(_) => true,
            TsType::TsTypeRef(reference) => reference.type_params.is_none(),
            _ => false,
        };
        self.nest(node, node.span(), 0, usize::from(!leaf), 0);
    }
    fn visit_pat(&mut self, node: &Pat) {
        self.nest(
            node,
            node.span(),
            0,
            usize::from(!matches!(node, Pat::Ident(_))),
            0,
        );
    }
    fn visit_jsx_element(&mut self, node: &JSXElement) {
        self.nest(node, node.span, 0, 0, 1);
    }
    fn visit_jsx_fragment(&mut self, node: &JSXFragment) {
        self.nest(node, node.span, 0, 0, 1);
    }
    fn visit_jsx_member_expr(&mut self, node: &JSXMemberExpr) {
        self.nest(node, node.span(), 0, 0, 1);
    }
    fn visit_ts_qualified_name(&mut self, node: &TsQualifiedName) {
        self.nest(node, node.span(), 0, 0, 1);
    }
}

/// A parse result whose tree is checked against the nesting budgets.
pub(crate) trait Parsed {
    fn check_nesting(&self, check: &mut NestingCheck);
}
macro_rules! parsed {
    ($($ty:ty),*) => {$(
        impl Parsed for $ty {
            fn check_nesting(&self, check: &mut NestingCheck) {
                self.visit_with(check);
            }
        }
    )*};
}
parsed!(Module, Script, Program, Box<Expr>, Vec<ModuleItem>);
impl<T: Parsed, U> Parsed for (T, U) {
    fn check_nesting(&self, check: &mut NestingCheck) {
        self.0.check_nesting(check);
    }
}

pub(crate) fn with_bounded_file_parser<T: Parsed>(
    file: &SourceFile,
    syntax: Syntax,
    target: EsVersion,
    comments: Option<&dyn Comments>,
    errors: &mut Vec<Error>,
    op: impl for<'a> FnOnce(&mut Parser<BoundedLexer<'a>>) -> PResult<T>,
) -> PResult<T> {
    // Never call JavaScript inside this stack switch: V8 has stack affinity.
    stacker::maybe_grow(PARSER_STACK_RED_ZONE, PARSER_STACK_SIZE, || {
        let lexer = Lexer::new(syntax, target, StringInput::from(file), comments);
        let mut parser = Parser::new_from(BoundedLexer::new(lexer));
        let mut result = op(&mut parser);
        parser.input_mut().iter_mut().finalize_comments();
        let recovered = parser.take_errors();
        errors.extend(recovered.into_iter().filter(|e| !babel_accepts(e)));
        // A tree over budget is dropped here, on the parser's stack.
        if let Ok(value) = &result {
            let mut check = NestingCheck::default();
            value.check_nesting(&mut check);
            if let Some(error) = check.exceeded {
                result = Err(error);
            }
        }
        result
    })
}

/// Babel 7 accepts a raw `>` or `}` in JSX text, which SWC reports as a recovered error.
fn babel_accepts(error: &Error) -> bool {
    matches!(
        error.kind(),
        SyntaxError::UnexpectedTokenWithSuggestions { candidate_list }
            if matches!(
                candidate_list.as_slice(),
                ["`{'>'}`", "`&gt;`"] | ["`{'}'}`", "`&rbrace;`"]
            )
    )
}
