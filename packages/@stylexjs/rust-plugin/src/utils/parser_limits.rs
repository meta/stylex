//! Nesting limits checked on SWC's token stream before recursive descent.

use swc_core::common::{comments::Comments, BytePos, SourceFile, Span};
use swc_core::ecma::ast::EsVersion;
use swc_ecma_parser::{
    error::{Error, SyntaxError},
    input::Tokens,
    lexer::{Lexer, TokenFlags},
    unstable::{Token, TokenAndSpan, TokenValue},
    Context, FlowSyntax, PResult, Parser, StringInput, Syntax, SyntaxFlags, TsSyntax,
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

pub(crate) const MAX_SYNTAX_COMPLEXITY: usize = 2048;
// Expression delimiters, types, prefixes and arrows retain their own budget.
pub(crate) const MAX_RECURSIVE_DEPTH: usize = 1024;
// Statement controls and braced code blocks must not consume expression depth.
pub(crate) const MAX_STATEMENT_DEPTH: usize = 8192;
// Stack the parser needs before entering recursive descent.
pub(crate) const PARSER_STACK_RED_ZONE: usize = PARSER_STACK_SIZE;
pub(crate) const PARSER_STACK_SIZE: usize = 256 * 1024 * 1024;

#[derive(Clone)]
struct Frame {
    closing: Option<Token>,
    operators: usize,
    statements: Statements,
    statement_header: bool,
    function_parameters: bool,
    finish_statement: bool,
    pending_function_body: bool,
    function_body: bool,
    property_value: bool,
    arrows: usize,
}
#[derive(Clone)]
pub(crate) struct Limits {
    frames: Vec<Frame>,
    prefix: usize,
    jsx_depth: usize,
    last: Option<(Token, Span)>,
    pending_jsx_open: Option<(Span, usize)>,
    jsx: Vec<JsxFrame>,
    can_end: bool,
    function_header: Option<usize>,
    // Contextual Flow `hook`/`component`, then its name, before parameters.
    flow_function_keyword: Option<(usize, bool)>,
    resource_error: Option<Error>,
    resource_reported: bool,
    type_depth: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            frames: vec![Frame {
                closing: None,
                operators: 0,
                statements: Statements::root(),
                statement_header: false,
                function_parameters: false,
                finish_statement: false,
                pending_function_body: false,
                function_body: false,
                property_value: false,
                arrows: 0,
            }],
            prefix: 0,
            jsx_depth: 0,
            last: None,
            pending_jsx_open: None,
            jsx: Vec::new(),
            can_end: false,
            function_header: None,
            flow_function_keyword: None,
            resource_error: None,
            resource_reported: false,
            type_depth: 0,
        }
    }
}

#[derive(Clone)]
pub(crate) struct BoundedLexer<'a> {
    lexer: Lexer<'a>,
    limits: Limits,
}
impl<'a> BoundedLexer<'a> {
    fn new(lexer: Lexer<'a>) -> Self {
        Self {
            lexer,
            limits: Limits::default(),
        }
    }
}
#[derive(Clone)]
struct JsxFrame {
    syntax_frame: usize,
    header: bool,
    closing: bool,
    self_closing: bool,
    operators: usize,
    arrows: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum StatementKind {
    If,
    Loop,
    Label,
    Do,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum StatementPhase {
    Header,
    Body,
    InBody,
    AwaitElse,
    ElseBody,
    InElse,
    DoWhile,
    DoHeader,
    DoEnd,
}
#[derive(Clone, Copy, Debug)]
struct StatementControl {
    kind: StatementKind,
    phase: StatementPhase,
}
#[derive(Clone, Copy, Debug)]
struct StatementLeaf {
    first: Token,
    label: bool,
    declaration: bool,
    clause: bool,
    questions: usize,
}
#[derive(Clone, Default)]
struct Statements {
    code: bool,
    undecided: bool,
    controls: Vec<StatementControl>,
    leaf: Option<StatementLeaf>,
}
impl Statements {
    fn root() -> Self {
        Self {
            code: true,
            ..Default::default()
        }
    }
    fn expects_statement(&self) -> bool {
        self.code
            && self.leaf.is_none()
            && self
                .controls
                .last()
                .is_none_or(|c| matches!(c.phase, StatementPhase::Body | StatementPhase::ElseBody))
    }
    fn complete(&mut self) {
        self.leaf = None;
        while let Some(control) = self.controls.last_mut() {
            match (control.kind, control.phase) {
                (StatementKind::If, StatementPhase::InBody) => {
                    control.phase = StatementPhase::AwaitElse;
                    break;
                }
                (StatementKind::Do, StatementPhase::InBody) => {
                    control.phase = StatementPhase::DoWhile;
                    break;
                }
                (_, StatementPhase::InBody | StatementPhase::InElse) => {
                    self.controls.pop();
                }
                _ => break,
            }
        }
    }
    fn resolve_else(&mut self, token: Token) {
        while self
            .controls
            .last()
            .is_some_and(|c| c.phase == StatementPhase::AwaitElse)
        {
            if token == Token::Else {
                self.controls.last_mut().unwrap().phase = StatementPhase::ElseBody;
                return;
            }
            self.controls.pop();
            self.complete();
        }
    }
    fn header_closed(&mut self) {
        if let Some(c) = self.controls.last_mut() {
            c.phase = if c.phase == StatementPhase::DoHeader {
                StatementPhase::DoEnd
            } else {
                StatementPhase::Body
            };
        }
    }
    fn observe(
        &mut self,
        token: TokenAndSpan,
        ctx: Context,
        previous_can_end: bool,
        noncontinuation: bool,
    ) {
        if self.undecided {
            self.code = ctx.contains(Context::AllowUsingDecl);
            self.undecided = false;
        }
        if !self.code || ctx.contains(Context::InType) {
            return;
        }
        let kind = token.token;
        if self
            .controls
            .last()
            .is_some_and(|c| c.phase == StatementPhase::DoEnd)
        {
            self.controls.pop();
            self.complete();
            if kind == Token::Semi {
                return;
            }
        }
        // A reserved statement start cannot continue a completed expression.
        let strong = matches!(
            kind,
            Token::If
                | Token::While
                | Token::For
                | Token::With
                | Token::Do
                | Token::Switch
                | Token::Try
                | Token::Function
                | Token::Class
                | Token::Const
                | Token::Var
                | Token::Return
                | Token::Throw
                | Token::Else
                | Token::Export
                | Token::Import
        );
        if self
            .leaf
            .is_some_and(|leaf| !leaf.declaration && !leaf.clause)
            && previous_can_end
            && ((token.had_line_break && noncontinuation) || strong)
        {
            self.complete();
        }
        self.resolve_else(kind);
        if kind == Token::Else
            && self
                .controls
                .last()
                .is_some_and(|c| c.phase == StatementPhase::ElseBody)
        {
            return;
        }
        if kind == Token::While
            && self
                .controls
                .last()
                .is_some_and(|c| c.phase == StatementPhase::DoWhile)
        {
            self.controls.last_mut().unwrap().phase = StatementPhase::DoHeader;
            return;
        }
        if kind == Token::Semi {
            if self.leaf.is_some() {
                self.complete();
            } else if self.expects_statement() {
                self.start_body();
                self.complete();
            }
            return;
        }
        if let Some(leaf) = self.leaf.as_mut() {
            if leaf.clause {
                if kind == Token::QuestionMark {
                    leaf.questions += 1;
                }
                if kind == Token::Colon {
                    if leaf.questions == 0 {
                        self.leaf = None;
                    } else {
                        leaf.questions -= 1;
                    }
                }
                return;
            }
            if kind == Token::Colon && leaf.label {
                self.leaf = None;
                self.controls.push(StatementControl {
                    kind: StatementKind::Label,
                    phase: StatementPhase::Body,
                });
                return;
            }
            leaf.label = false;
            if matches!(leaf.first, Token::Module | Token::Namespace)
                && matches!(kind, Token::Dot | Token::Eq | Token::LParen)
            {
                leaf.declaration = false;
            }
            if matches!(
                leaf.first,
                Token::Export | Token::Default | Token::Async | Token::Declare | Token::At
            ) && matches!(
                kind,
                Token::Function
                    | Token::Class
                    | Token::Enum
                    | Token::Interface
                    | Token::Namespace
                    | Token::Module
            ) {
                leaf.declaration = true;
                leaf.first = kind;
            }
            return;
        }
        if !self.expects_statement() {
            return;
        }
        self.start_body();
        let control = match kind {
            Token::If => Some((StatementKind::If, StatementPhase::Header)),
            Token::While | Token::For | Token::With | Token::Switch => {
                Some((StatementKind::Loop, StatementPhase::Header))
            }
            Token::Do => Some((StatementKind::Do, StatementPhase::Body)),
            _ => None,
        };
        if let Some((kind, phase)) = control {
            self.controls.push(StatementControl { kind, phase });
        } else if kind != Token::RBrace && kind != Token::Eof {
            self.leaf = Some(StatementLeaf {
                first: kind,
                label: kind.is_ident_ref(ctx),
                declaration: matches!(
                    kind,
                    Token::Function
                        | Token::Class
                        | Token::Enum
                        | Token::Interface
                        | Token::Namespace
                        | Token::Module
                ),
                clause: matches!(kind, Token::Case | Token::Default),
                questions: 0,
            });
        }
    }
    fn start_body(&mut self) {
        if let Some(c) = self.controls.last_mut() {
            match c.phase {
                StatementPhase::Body => c.phase = StatementPhase::InBody,
                StatementPhase::ElseBody => c.phase = StatementPhase::InElse,
                _ => {}
            }
        }
    }
    fn header(&self) -> bool {
        self.controls
            .last()
            .is_some_and(|c| matches!(c.phase, StatementPhase::Header | StatementPhase::DoHeader))
    }
    fn finishes_block(&self) -> bool {
        self.leaf
            .is_some_and(|leaf| leaf.first == Token::LBrace || leaf.declaration)
    }
}

fn can_end_expression(kind: Token, ctx: Context) -> bool {
    kind.is_ident_ref(ctx)
        || matches!(
            kind,
            Token::Num
                | Token::Str
                | Token::BigInt
                | Token::Regex
                | Token::Template
                | Token::NoSubstitutionTemplateLiteral
                | Token::TemplateTail
                | Token::True
                | Token::False
                | Token::Null
                | Token::This
                | Token::Super
                | Token::RParen
                | Token::RBracket
                | Token::RBrace
        )
}

impl BoundedLexer<'_> {
    fn within_jsx_header(&self) -> bool {
        self.limits
            .jsx
            .last()
            .is_some_and(|frame| frame.header && frame.syntax_frame == self.limits.frames.len() - 1)
    }
    fn jsx_header(&self) -> bool {
        !self.lexer.ctx().contains(Context::InType) && self.within_jsx_header()
    }
    fn confirm_jsx_open(&mut self, name_or_end: BytePos) {
        if let Some((span, frame)) = self.limits.pending_jsx_open {
            if span.hi <= name_or_end
                && self
                    .lexer
                    .read_string(Span::new(span.hi, name_or_end))
                    .chars()
                    .all(char::is_whitespace)
            {
                if let Some(frame) = self.limits.frames.get_mut(frame) {
                    frame.operators = frame.operators.saturating_sub(1);
                }
                self.limits.jsx.push(JsxFrame {
                    syntax_frame: frame,
                    header: true,
                    closing: false,
                    self_closing: false,
                    operators: 0,
                    arrows: 0,
                });
                self.limits.jsx_depth = self.limits.jsx.len();
                self.limits.pending_jsx_open = None;
            }
        }
    }
    fn observe(&mut self, mut token: TokenAndSpan, jsx_scan: bool) -> TokenAndSpan {
        if let Some(error) = &self.limits.resource_error {
            if jsx_scan {
                // SWC's JSX child scanner accepts EOF, not Error.
                if token.token != Token::JSXName {
                    token.token = Token::Eof;
                }
            } else if token.token != Token::Eof {
                self.lexer
                    .set_token_value(Some(TokenValue::Error(error.clone())));
                token.token = Token::Error;
            }
            return token;
        }
        if self.limits.last == Some((token.token, token.span)) {
            return token;
        }
        let previous = self.limits.last.map(|(kind, _)| kind);
        let kind = token.token;
        let frame_index = self.limits.frames.len() - 1;
        if !jsx_scan {
            // `hook()`, `component.as` and `component as T` are expressions, not headers.
            if let Some((frame, named)) = self.limits.flow_function_keyword.take() {
                if frame == frame_index {
                    if !named && kind.is_word() {
                        self.limits.flow_function_keyword = Some((frame, true));
                    } else if named && matches!(kind, Token::LParen | Token::Lt) {
                        self.limits.function_header = Some(frame);
                        if let Some(leaf) = self.limits.frames[frame].statements.leaf.as_mut() {
                            leaf.first = Token::Function;
                            leaf.declaration = true;
                        }
                    }
                }
            }
            let statements = &self.limits.frames[frame_index].statements;
            let declaration_start = statements.expects_statement()
                || statements.leaf.is_some_and(|leaf| {
                    matches!(leaf.first, Token::Export | Token::Default | Token::Declare)
                });
            if self.lexer.syntax().flow_components()
                && !self.lexer.ctx().contains(Context::InType)
                && declaration_start
                && kind == Token::Ident
                && matches!(self.lexer.get_token_value(), Some(TokenValue::Word(word)) if word == "hook" || word == "component")
            {
                self.limits.flow_function_keyword = Some((frame_index, false));
            }
        }
        // Fragments have no JSX-name rescan: ordinary `>` must confirm the `<`.
        if kind == Token::Gt
            && previous == Some(Token::Lt)
            && !self.lexer.ctx().contains(Context::InType)
        {
            self.confirm_jsx_open(token.span.lo);
        }
        if kind == Token::LessSlash && jsx_scan {
            if let Some(frame) = self.limits.jsx.last_mut() {
                frame.header = true;
                frame.closing = true;
            }
        }
        let header = self.jsx_header();
        let within_header = self.within_jsx_header();
        // The parser may lex the next `as` before leaving the preceding cast's type context.
        let cast_word = self.lexer.syntax().typescript()
            && !token.had_line_break
            && matches!(kind, Token::As | Token::Satisfies)
            && self.limits.can_end;
        let mut block_statement = self.limits.frames[frame_index]
            .statements
            .expects_statement();
        if !within_header
            && !jsx_scan
            && kind == Token::LBrace
            && token.had_line_break
            && self.limits.can_end
            && !self.lexer.ctx().contains(Context::InType)
            && !self.limits.frames[frame_index].pending_function_body
            && self.limits.frames[frame_index].statements.code
            && self.limits.frames[frame_index]
                .statements
                .leaf
                .is_some_and(|leaf| !leaf.declaration && !leaf.clause)
        {
            // A block after an ASI-ended expression is a statement.
            let frame = &mut self.limits.frames[frame_index];
            frame.statements.complete();
            frame.statements.resolve_else(kind);
            block_statement = frame.statements.expects_statement();
            frame.operators = 0;
            frame.arrows = 0;
        }
        let statement_frame = matches!(
            self.limits.frames.last().unwrap().closing,
            None | Some(Token::RBrace)
        );
        let starts_noncontinuation = !cast_word
            && (kind.is_ident_ref(self.lexer.ctx())
                || matches!(
                    kind,
                    Token::Num
                        | Token::Str
                        | Token::BigInt
                        | Token::True
                        | Token::False
                        | Token::Null
                        | Token::This
                        | Token::Super
                        | Token::New
                        | Token::Bang
                        | Token::Tilde
                        | Token::TypeOf
                        | Token::Void
                        | Token::Delete
                        | Token::Await
                        | Token::Yield
                        | Token::Function
                        | Token::Class
                        | Token::Const
                        | Token::Var
                        | Token::Return
                        | Token::Throw
                        | Token::Break
                        | Token::Continue
                        | Token::If
                        | Token::While
                        | Token::For
                        | Token::With
                        | Token::Do
                        | Token::Switch
                        | Token::Try
                        | Token::Else
                        | Token::Catch
                        | Token::Finally
                        | Token::Export
                        | Token::Import
                ));
        if !within_header && !jsx_scan {
            self.limits.frames[frame_index].statements.observe(
                token,
                self.lexer.ctx(),
                self.limits.can_end,
                starts_noncontinuation,
            );
        }
        // Only statement starts reset, never a continuation like `(`, `[` or `.`.
        if !header
            && !jsx_scan
            && statement_frame
            && !self.lexer.ctx().contains(Context::InType)
            && self.limits.can_end
            && (token.had_line_break && starts_noncontinuation
                || matches!(
                    kind,
                    Token::Function | Token::Class | Token::Const | Token::Var
                ))
        {
            self.limits.frames.last_mut().unwrap().operators = 0;
            self.limits.frames.last_mut().unwrap().arrows = 0;
        }
        let function_parameters = kind == Token::LParen
            && self.limits.function_header == Some(frame_index)
            // Lookahead can lex the parameter opener before leaving the generic's type context.
            && (!self.lexer.ctx().contains(Context::InType)
                || (self.limits.type_depth == 0
                    && matches!(previous, Some(Token::Gt | Token::RShift | Token::ZeroFillRShift))));
        if function_parameters {
            self.limits.function_header = None;
        }
        if kind == Token::Function && previous != Some(Token::Dot) {
            self.limits.function_header = Some(frame_index);
        }
        if matches!(
            kind,
            Token::Bang
                | Token::Tilde
                | Token::Plus
                | Token::Minus
                | Token::Void
                | Token::TypeOf
                | Token::Delete
                | Token::Await
                | Token::Yield
                | Token::New
        ) {
            self.limits.prefix += 1;
        } else {
            self.limits.prefix = 0;
        }
        let suffix_start = self.limits.can_end
            && (matches!(
                kind,
                Token::LParen
                    | Token::LBracket
                    | Token::Template
                    | Token::NoSubstitutionTemplateLiteral
                    | Token::TemplateHead
            ));
        let property = self.limits.frames.last_mut().unwrap();
        if property.closing == Some(Token::RBrace)
            && !property.statements.code
            && !property.pending_function_body
            && !self.lexer.ctx().contains(Context::InType)
            && matches!(kind, Token::Colon | Token::Eq | Token::DotDotDot)
        {
            property.property_value = true;
        }
        // A method's `(` is not a call in a property value (`key: f()`).
        let method_parameters = kind == Token::LParen
            && self.limits.frames.last().unwrap().closing == Some(Token::RBrace)
            && !self.limits.frames.last().unwrap().statements.code
            && !self.limits.frames.last().unwrap().property_value
            && !self.lexer.ctx().contains(Context::InClassField)
            && (!self.lexer.ctx().contains(Context::InType)
                || (self.limits.type_depth == 0
                    && matches!(
                        previous,
                        Some(Token::Gt | Token::RShift | Token::ZeroFillRShift)
                    )));
        if method_parameters && previous == Some(Token::New) {
            // `new` as a method name: undo its tentative constructor cost.
            let frame = self.limits.frames.last_mut().unwrap();
            frame.operators = frame.operators.saturating_sub(1);
        }
        if kind == Token::Arrow {
            if within_header {
                self.limits.jsx.last_mut().unwrap().arrows += 1;
            } else {
                self.limits.frames.last_mut().unwrap().arrows += 1;
            }
        }
        if kind == Token::Lt && self.lexer.ctx().contains(Context::InType) {
            self.limits.type_depth += 1;
        }
        if self.limits.type_depth > 0 {
            let closes = match kind {
                Token::Gt => 1,
                Token::RShift => 2,
                Token::ZeroFillRShift => 3,
                _ => 0,
            };
            self.limits.type_depth = self.limits.type_depth.saturating_sub(closes);
        }
        let adds_operator = kind.is_bin_op()
            || kind.is_assign_op()
            || cast_word
            || matches!(
                kind,
                Token::Arrow | Token::QuestionMark | Token::Dot | Token::OptionalChain
            )
            || (kind == Token::New && !matches!(previous, Some(Token::Dot | Token::OptionalChain)))
            || (suffix_start && !function_parameters && !method_parameters);
        if within_header {
            // JSX name and type subtrees have their own cost.
            if kind == Token::Dot || (self.lexer.ctx().contains(Context::InType) && adds_operator) {
                self.limits.jsx.last_mut().unwrap().operators += 1;
            }
        } else if adds_operator {
            self.limits.frames.last_mut().unwrap().operators += 1;
        }
        if kind == Token::Lt {
            self.limits.pending_jsx_open = Some((token.span, frame_index));
        }
        let closing = match kind {
            Token::LParen => Some(Token::RParen),
            Token::LBracket => Some(Token::RBracket),
            Token::LBrace => Some(Token::RBrace),
            Token::TemplateHead | Token::TemplateMiddle => Some(Token::RBrace),
            _ => None,
        };
        if let Some(closing) = closing {
            let parent = &mut self.limits.frames[frame_index];
            let function_body = kind == Token::LBrace
                && parent.pending_function_body
                && !self.lexer.ctx().contains(Context::InType);
            if function_body {
                parent.pending_function_body = false;
            }
            let declaration = parent.statements.leaf.filter(|leaf| leaf.declaration);
            let namespace = declaration
                .is_some_and(|leaf| matches!(leaf.first, Token::Namespace | Token::Module));
            let try_body = kind == Token::LBrace
                && parent.statements.leaf.is_some_and(|leaf| {
                    matches!(leaf.first, Token::Try | Token::Catch | Token::Finally)
                });
            let code = kind == Token::LBrace
                && !self.lexer.ctx().contains(Context::InType)
                && (block_statement
                    || function_body
                    || previous == Some(Token::Arrow)
                    || previous == Some(Token::Static)
                    || namespace
                    || try_body);
            let finish_statement = kind == Token::LBrace
                && parent.statements.finishes_block()
                && (!self.lexer.ctx().contains(Context::InType)
                    || declaration.is_some_and(|leaf| {
                        matches!(
                            leaf.first,
                            Token::Interface | Token::Enum | Token::Namespace | Token::Module
                        )
                    }));
            let statement_header = kind == Token::LParen && parent.statements.header();
            let function_parameters = kind == Token::LParen
                && (function_parameters
                    || method_parameters
                    || previous == Some(Token::Catch)
                    || (!parent.statements.code
                        && parent.closing == Some(Token::RBrace)
                        && self.lexer.ctx().contains(Context::InClass)
                        && !self.lexer.ctx().contains(Context::InClassField)
                        && !self.lexer.ctx().contains(Context::InType)));
            let undecided = kind == Token::LBrace && frame_index == 0 && previous.is_none();
            self.limits.frames.push(Frame {
                closing: Some(closing),
                operators: 0,
                statements: Statements {
                    code,
                    undecided,
                    ..Default::default()
                },
                statement_header,
                function_parameters,
                finish_statement,
                pending_function_body: false,
                function_body,
                property_value: false,
                arrows: 0,
            });
        } else if self.limits.frames.last().and_then(|frame| frame.closing) == Some(kind) {
            let closed = self.limits.frames.pop().unwrap();
            let parent = self.limits.frames.last_mut().unwrap();
            if closed.statement_header {
                parent.statements.header_closed();
            }
            if closed.function_parameters {
                parent.pending_function_body = true;
            }
            if closed.finish_statement {
                parent.statements.complete();
            }
            if closed.function_body
                && !parent.statements.code
                && !parent.property_value
                && parent.closing == Some(Token::RBrace)
            {
                parent.operators = 0;
                parent.arrows = 0;
            }
        }
        if kind == Token::Semi {
            self.limits.frames.last_mut().unwrap().pending_function_body = false;
        }
        if matches!(kind, Token::Semi | Token::Comma) {
            self.limits.frames.last_mut().unwrap().operators = 0;
            self.limits.frames.last_mut().unwrap().arrows = 0;
            self.limits.frames.last_mut().unwrap().property_value = false;
        }
        let mut completed_jsx = false;
        if header {
            if kind == Token::Slash {
                self.limits.jsx.last_mut().unwrap().self_closing = true;
            }
            if kind == Token::Gt {
                self.limits.pending_jsx_open = None;
                let frame = self.limits.jsx.last_mut().unwrap();
                if frame.closing || frame.self_closing {
                    self.limits.jsx.pop();
                    completed_jsx = true;
                } else {
                    frame.header = false;
                    frame.operators = 0;
                    frame.arrows = 0;
                }
            }
        }
        self.limits.jsx_depth = self.limits.jsx.len();
        let const_cast_end = self.lexer.syntax().typescript()
            && kind == Token::Const
            && previous == Some(Token::As)
            && !token.had_line_break;
        self.limits.can_end = completed_jsx
            || const_cast_end
            || (!header && !cast_word && can_end_expression(kind, self.lexer.ctx()));
        self.limits.last = Some((kind, token.span));
        let mut code_blocks = 0usize;
        let mut frame_operators = 0usize;
        let mut frame_arrows = 0usize;
        let mut controls = 0usize;
        for frame in &self.limits.frames {
            if frame.closing == Some(Token::RBrace) && frame.statements.code {
                code_blocks += 1;
            }
            frame_operators += frame.operators;
            frame_arrows += frame.arrows;
            controls += frame.statements.controls.len();
        }
        let mut jsx_operators = 0usize;
        let mut jsx_arrows = 0usize;
        for frame in &self.limits.jsx {
            jsx_operators += frame.operators;
            jsx_arrows += frame.arrows;
        }
        let expression_delimiters = self.limits.frames.len() - 1 - code_blocks;
        let complexity = expression_delimiters
            + self.limits.jsx_depth
            + self.limits.prefix
            + frame_operators
            + jsx_operators;
        let expression_depth = expression_delimiters
            + self.limits.prefix
            + self.limits.type_depth
            + frame_arrows
            + jsx_arrows;
        let statement_depth = code_blocks + controls;
        if complexity > MAX_SYNTAX_COMPLEXITY
            || expression_depth > MAX_RECURSIVE_DEPTH
            || statement_depth > MAX_STATEMENT_DEPTH
        {
            let error = Error::new(
                token.span,
                SyntaxError::Unexpected {
                    got: format!(
                        "compiler {} exceeds {}",
                        if statement_depth > MAX_STATEMENT_DEPTH {
                            "statement/block nesting"
                        } else if expression_depth > MAX_RECURSIVE_DEPTH {
                            "recursive expression/delimiter nesting"
                        } else {
                            "syntax nesting/complexity"
                        },
                        if statement_depth > MAX_STATEMENT_DEPTH {
                            MAX_STATEMENT_DEPTH
                        } else if expression_depth > MAX_RECURSIVE_DEPTH {
                            MAX_RECURSIVE_DEPTH
                        } else {
                            MAX_SYNTAX_COMPLEXITY
                        }
                    ),
                    expected:
                        "bounded syntax (split deeply nested statements or chained expressions)",
                },
            );
            self.limits.resource_error = Some(error.clone());
            if jsx_scan {
                if token.token != Token::JSXName {
                    token.token = Token::Eof;
                }
            } else {
                self.lexer.set_token_value(Some(TokenValue::Error(error)));
                token.token = Token::Error;
            }
        }
        token
    }
}

impl<'a> Tokens for BoundedLexer<'a> {
    type Checkpoint = (<Lexer<'a> as Tokens>::Checkpoint, Limits);
    fn set_ctx(&mut self, ctx: Context) {
        // Lookahead may lex a `{` in the wrong type context; reclassify those after parameters.
        let len = self.limits.frames.len();
        if len > 1 && self.limits.frames[len - 1].closing == Some(Token::RBrace) {
            let (parents, current) = self.limits.frames.split_at_mut(len - 1);
            let parent = parents.last_mut().unwrap();
            let frame = &mut current[0];
            if ctx.contains(Context::InType) && frame.function_body {
                frame.function_body = false;
                frame.statements.code = false;
                frame.finish_statement = false;
                parent.pending_function_body = true;
            } else if !ctx.contains(Context::InType)
                && parent.pending_function_body
                && !frame.statements.code
            {
                frame.function_body = true;
                frame.statements.code = true;
                frame.finish_statement = parent.statements.finishes_block();
                parent.pending_function_body = false;
            }
        }
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
        (self.lexer.checkpoint_save(), self.limits.clone())
    }
    fn checkpoint_load(&mut self, checkpoint: Self::Checkpoint) {
        self.lexer.checkpoint_load(checkpoint.0);
        self.limits = checkpoint.1;
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
        if !self.limits.resource_reported {
            if let Some(error) = &self.limits.resource_error {
                errors.push(error.clone());
                self.limits.resource_reported = true;
            }
        }
        errors
    }
    fn has_errors(&self) -> bool {
        self.limits.resource_error.is_some() || self.lexer.has_errors()
    }
    fn diagnostic_checkpoint_save(&self) -> (usize, usize) {
        self.lexer.diagnostic_checkpoint_save()
    }
    fn diagnostic_checkpoint_load(&mut self, checkpoint: (usize, usize)) {
        // Restored with the token checkpoint.
        self.lexer.diagnostic_checkpoint_load(checkpoint);
    }
    fn set_defer_comments(&mut self, defer: bool) {
        self.lexer.set_defer_comments(defer);
    }
    fn finalize_comments(&mut self) {
        self.lexer.finalize_comments();
    }
    fn take_script_module_errors(&mut self) -> Vec<Error> {
        self.lexer.take_script_module_errors()
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
        self.observe(token, false)
    }
    fn next_token(&mut self) -> TokenAndSpan {
        let token = self.lexer.next_token();
        self.observe(token, false)
    }
    fn scan_jsx_token(&mut self) -> TokenAndSpan {
        let token = self.lexer.scan_jsx_token();
        self.observe(token, true)
    }
    fn scan_jsx_open_el_terminal_token(&mut self) -> TokenAndSpan {
        let token = self.lexer.scan_jsx_open_el_terminal_token();
        self.observe(token, true)
    }
    fn rescan_jsx_open_el_terminal_token(&mut self, reset: BytePos) -> TokenAndSpan {
        let token = self.lexer.rescan_jsx_open_el_terminal_token(reset);
        self.observe(token, true)
    }
    fn rescan_jsx_token(&mut self, reset: BytePos) -> TokenAndSpan {
        let token = self.lexer.rescan_jsx_token(reset);
        self.observe(token, true)
    }
    fn scan_jsx_identifier(&mut self, start: BytePos) -> TokenAndSpan {
        if self.limits.resource_error.is_none() {
            self.confirm_jsx_open(start);
        }
        let token = self.lexer.scan_jsx_identifier(start);
        self.observe(token, true)
    }
    fn scan_jsx_attribute_value(&mut self) -> TokenAndSpan {
        let token = self.lexer.scan_jsx_attribute_value();
        self.observe(token, true)
    }
    fn rescan_template_token(&mut self, start: BytePos, back_tick: bool) -> TokenAndSpan {
        let token = self.lexer.rescan_template_token(start, back_tick);
        self.observe(token, false)
    }
}

pub(crate) fn with_bounded_file_parser<T>(
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
        let result = op(&mut parser);
        parser.input_mut().iter_mut().finalize_comments();
        let recovered = parser.take_errors();
        errors.extend(recovered.into_iter().filter(|e| !babel_accepts(e)));
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
