//! Typed AST equality ignoring locations, hygiene, literal spellings and parentheses.
//!
//! Callers provide the stack and check directives and provenance separately.

use std::borrow::Cow;
use swc_core::common::{util::take::Take, EqIgnoreSpan, SyntaxContext};
use swc_core::ecma::ast::{Expr, Ident, Module, Number, SimpleAssignTarget};
use swc_core::ecma::visit::{Visit, VisitMut, VisitMutWith, VisitWith};

#[cfg(test)]
pub(crate) fn modules_equivalent(actual: &Module, expected: &Module) -> bool {
    module_equivalent(Cow::Borrowed(actual), expected)
}

/// Postflight owns the module it re-parsed, so that side is normalized in place
/// rather than copied first.
pub(crate) fn reparsed_module_equivalent(actual: Module, expected: &Module) -> bool {
    module_equivalent(Cow::Owned(actual), expected)
}

fn module_equivalent(actual: Cow<'_, Module>, expected: &Module) -> bool {
    let borrowed = actual.as_ref();
    if borrowed == expected {
        // Strict equality includes `Ident.optional`.
        return borrowed.eq_ignore_span(expected) && !contains_nan(borrowed);
    }
    if borrowed.eq_ignore_span(expected) {
        return !contains_nan(borrowed) && optional_identifiers_equivalent(borrowed, expected);
    }
    // Normalize items independently, so one edit does not clone the module.
    if borrowed.shebang != expected.shebang || borrowed.body.len() != expected.body.len() {
        return false;
    }
    match actual {
        Cow::Borrowed(actual) => actual
            .body
            .iter()
            .zip(&expected.body)
            .all(|(actual, expected)| equivalent(Cow::Borrowed(actual), expected)),
        Cow::Owned(actual) => actual
            .body
            .into_iter()
            .zip(&expected.body)
            .all(|(actual, expected)| equivalent(Cow::Owned(actual), expected)),
    }
}

#[cfg(test)]
pub(crate) fn expressions_equivalent(actual: &Expr, expected: &Expr) -> bool {
    equivalent(Cow::Borrowed(actual), expected)
}

#[cfg(test)]
pub(crate) fn items_equivalent(
    actual: &swc_core::ecma::ast::ModuleItem,
    expected: &swc_core::ecma::ast::ModuleItem,
) -> bool {
    equivalent(Cow::Borrowed(actual), expected)
}

fn equivalent<T>(mut actual: Cow<'_, T>, expected: &T) -> bool
where
    T: Clone
        + PartialEq
        + EqIgnoreSpan
        + VisitWith<NormalizationNeeded>
        + VisitWith<OptionalIdentifiers>
        + VisitWith<NumericValidity>
        + VisitMutWith<Normalize>,
{
    let borrowed = actual.as_ref();
    if borrowed == expected {
        return borrowed.eq_ignore_span(expected) && !contains_nan(borrowed);
    }
    if borrowed.eq_ignore_span(expected) {
        return !contains_nan(borrowed) && optional_identifiers_equivalent(borrowed, expected);
    }
    let normalize_actual = normalization_needed(borrowed);
    let normalize_expected = normalization_needed(expected);
    if !normalize_actual.any() && !normalize_expected.any() {
        return false;
    }
    // The comparison below ignores hygiene contexts, which is the same as
    // clearing them on both sides, so only parentheses need rewriting: in place
    // on an owned side, on a copy of a borrowed one.
    if normalize_actual.parens {
        actual.to_mut().visit_mut_with(&mut Normalize);
    }
    let mut expected = Cow::Borrowed(expected);
    if normalize_expected.parens {
        expected.to_mut().visit_mut_with(&mut Normalize);
    }
    SyntaxContext::within_ignored_ctxt(|| actual.as_ref().eq_ignore_span(expected.as_ref()))
        && !contains_nan(actual.as_ref())
        && optional_identifiers_equivalent(actual.as_ref(), expected.as_ref())
}

// SWC treats NaN literals as equal, but source cannot contain one.
#[derive(Default)]
struct NumericValidity(bool);
impl Visit for NumericValidity {
    fn visit_number(&mut self, number: &Number) {
        self.0 |= number.value.is_nan();
    }
    fn visit_expr(&mut self, expression: &Expr) {
        if !self.0 {
            expression.visit_children_with(self);
        }
    }
}
fn contains_nan<T: VisitWith<NumericValidity>>(value: &T) -> bool {
    let mut visitor = NumericValidity::default();
    value.visit_with(&mut visitor);
    visitor.0
}

#[derive(Default)]
struct NormalizationNeeded {
    parens: bool,
    contexts: bool,
}

impl NormalizationNeeded {
    fn any(&self) -> bool {
        self.parens || self.contexts
    }
}

impl Visit for NormalizationNeeded {
    fn visit_expr(&mut self, expression: &Expr) {
        if !self.parens {
            if matches!(expression, Expr::Paren(_)) {
                self.parens = true;
            } else {
                expression.visit_children_with(self);
            }
        }
    }

    fn visit_simple_assign_target(&mut self, target: &SimpleAssignTarget) {
        if !self.parens {
            if matches!(target, SimpleAssignTarget::Paren(_)) {
                self.parens = true;
            } else {
                target.visit_children_with(self);
            }
        }
    }

    fn visit_syntax_context(&mut self, context: &SyntaxContext) {
        self.contexts |= *context != SyntaxContext::empty();
    }
}

fn normalization_needed<T: VisitWith<NormalizationNeeded>>(value: &T) -> NormalizationNeeded {
    let mut visitor = NormalizationNeeded::default();
    value.visit_with(&mut visitor);
    visitor
}

struct Normalize;

impl VisitMut for Normalize {
    fn visit_mut_syntax_context(&mut self, context: &mut SyntaxContext) {
        *context = SyntaxContext::empty();
    }

    fn visit_mut_expr(&mut self, expression: &mut Expr) {
        while matches!(expression, Expr::Paren(_)) {
            let Expr::Paren(parenthesized) = expression.take() else {
                unreachable!();
            };
            *expression = *parenthesized.expr;
        }
        // Casts and optional chains are not unwrapped.
        expression.visit_mut_children_with(self);
    }

    fn visit_mut_simple_assign_target(&mut self, target: &mut SimpleAssignTarget) {
        target.visit_mut_children_with(self);
        if let SimpleAssignTarget::Paren(parenthesized) = target {
            let span = parenthesized.span;
            let expression = parenthesized.expr.take();
            *target = match SimpleAssignTarget::try_from(expression) {
                Ok(target) => target,
                Err(expression) => SimpleAssignTarget::Paren(swc_core::ecma::ast::ParenExpr {
                    span,
                    expr: expression,
                }),
            };
        }
    }
}

// SWC's `Ident::eq_ignore_span` skips the TypeScript optional flag; compare it too.
#[derive(Default)]
struct OptionalIdentifiers {
    index: usize,
    optional: Vec<usize>,
}

impl Visit for OptionalIdentifiers {
    fn visit_ident(&mut self, identifier: &Ident) {
        if identifier.optional {
            self.optional.push(self.index);
        }
        self.index += 1;
    }
}

fn optional_identifiers_equivalent<T: VisitWith<OptionalIdentifiers>>(
    actual: &T,
    expected: &T,
) -> bool {
    let mut actual_flags = OptionalIdentifiers::default();
    let mut expected_flags = OptionalIdentifiers::default();
    actual.visit_with(&mut actual_flags);
    expected.visit_with(&mut expected_flags);
    actual_flags.index == expected_flags.index && actual_flags.optional == expected_flags.optional
}

#[cfg(test)]
mod tests {
    use super::*;
    use swc_core::common::{FileName, Globals, Mark, SourceMap, DUMMY_SP, GLOBALS};
    use swc_core::ecma::ast::*;
    use swc_core::ecma::parser::{with_file_parser, FlowSyntax, Syntax, TsSyntax};

    fn syntax(filename: &str) -> Syntax {
        if filename.ends_with(".ts") {
            Syntax::Typescript(TsSyntax::default())
        } else {
            Syntax::Flow(FlowSyntax {
                jsx: true,
                all: true,
                ..Default::default()
            })
        }
    }

    fn module(source: &str, filename: &str) -> Module {
        let map = SourceMap::default();
        let file = map.new_source_file(FileName::Custom(filename.into()).into(), source.to_owned());
        let mut errors = Vec::new();
        let result = with_file_parser(
            &file,
            syntax(filename),
            EsVersion::Es2022,
            None,
            &mut errors,
            |parser| parser.parse_module(),
        )
        .unwrap();
        assert!(errors.is_empty(), "{errors:?}");
        result
    }

    fn expression(source: &str, filename: &str) -> Expr {
        let map = SourceMap::default();
        let file = map.new_source_file(FileName::Custom(filename.into()).into(), source.to_owned());
        let mut errors = Vec::new();
        let result = with_file_parser(
            &file,
            syntax(filename),
            EsVersion::Es2022,
            None,
            &mut errors,
            |parser| parser.parse_expr(),
        )
        .unwrap();
        assert!(errors.is_empty(), "{errors:?}");
        *result
    }

    #[test]
    fn locations_and_primitive_raw_are_not_semantics() {
        let actual = module("const a = 'a'; const b = 0x10; const c = 0x10n;", "raw.js");
        let expected = module(
            "\nconst a = '\\u0061';\nconst b = 16;\nconst c = 16n;",
            "raw.js",
        );
        assert_ne!(actual, expected);
        assert!(modules_equivalent(&actual, &expected));
        assert!(items_equivalent(&actual.body[0], &expected.body[0]));
    }

    #[test]
    fn pure_expression_and_assignment_target_parentheses_are_equivalent() {
        for (actual, expected) in [
            ("(((value)))", "value"),
            ("f((a + b))", "f(a + b)"),
            ("((value)) = 1", "value = 1"),
            ("((obj.key)) = (value)", "obj.key = value"),
        ] {
            assert!(
                expressions_equivalent(&expression(actual, "p.js"), &expression(expected, "p.js")),
                "{actual}"
            );
        }
        assert!(modules_equivalent(
            &module("const x = ((f((value))));", "p.js"),
            &module("const x = f(value);", "p.js")
        ));
    }

    #[test]
    fn optional_chain_grouping_and_flags_are_preserved() {
        for (actual, expected) in [
            ("(obj?.method)()", "obj?.method()"),
            ("(obj?.field).key", "obj?.field.key"),
            ("(obj?.field)[key]", "obj?.field[key]"),
            ("obj?.method()", "obj?.method?.()"),
            ("obj?.field", "obj.field"),
        ] {
            assert!(
                !expressions_equivalent(&expression(actual, "o.js"), &expression(expected, "o.js")),
                "{actual} / {expected}"
            );
        }
        assert!(expressions_equivalent(
            &expression("((obj?.field))", "o.js"),
            &expression("obj?.field", "o.js")
        ));
    }

    #[test]
    fn numeric_signed_zero_has_no_partial_eq_shortcut() {
        let number = |value| {
            Expr::Lit(Lit::Num(Number {
                span: DUMMY_SP,
                value,
                raw: None,
            }))
        };
        let positive = number(0.0);
        let negative = number(-0.0);
        assert_ne!(positive, negative, "AST29 PartialEq preserves signed zero");
        assert!(!expressions_equivalent(&positive, &negative));
        let parenthesized = Expr::Paren(ParenExpr {
            span: DUMMY_SP,
            expr: Box::new(negative),
        });
        assert!(!expressions_equivalent(&positive, &parenthesized));
        assert!(expressions_equivalent(&number(-0.0), &number(-0.0)));
        let as_module = |expression| Module {
            span: DUMMY_SP,
            body: vec![ModuleItem::Stmt(Stmt::Expr(ExprStmt {
                span: DUMMY_SP,
                expr: Box::new(expression),
            }))],
            shebang: None,
        };
        let positive_module = as_module(number(0.0));
        let negative_module = as_module(number(-0.0));
        assert_ne!(positive_module, negative_module);
        assert!(!modules_equivalent(&positive_module, &negative_module));
        assert!(!items_equivalent(
            &positive_module.body[0],
            &negative_module.body[0],
        ));
        assert!(!expressions_equivalent(
            &number(f64::NAN),
            &number(f64::NAN)
        ));
        let nan_module = as_module(number(f64::NAN));
        assert!(!modules_equivalent(&nan_module, &nan_module));
        assert!(!items_equivalent(&nan_module.body[0], &nan_module.body[0]));
        assert!(!expressions_equivalent(
            &number(f64::NAN),
            &Expr::Paren(ParenExpr {
                span: DUMMY_SP,
                expr: Box::new(number(f64::NAN)),
            }),
        ));
        assert!(!expressions_equivalent(
            &number(f64::INFINITY),
            &number(f64::NEG_INFINITY)
        ));
        assert!(expressions_equivalent(
            &number(f64::INFINITY),
            &number(f64::INFINITY)
        ));
    }

    #[test]
    fn typed_wtf8_strings_do_not_collapse_surrogates() {
        let lone = expression("'\\uD800'", "s.js");
        assert!(expressions_equivalent(
            &lone,
            &expression("\"\\uD800\"", "s.js")
        ));
        for other in ["'\\uD801'", "'�'", "'\\\\uD800'"] {
            assert!(!expressions_equivalent(&lone, &expression(other, "s.js")));
        }
    }

    #[test]
    fn tagged_template_raw_cooked_and_tail_are_retained() {
        let plain = expression("tag`a`", "t.js");
        let escaped = expression("tag`\\u0061`", "t.js");
        assert!(!expressions_equivalent(&plain, &escaped));
        assert!(expressions_equivalent(
            &plain,
            &expression("(tag)`a`", "t.js")
        ));
        let mut changed = plain.clone();
        if let Expr::TaggedTpl(tagged) = &mut changed {
            tagged.tpl.quasis[0].cooked = None;
        } else {
            panic!()
        }
        assert!(!expressions_equivalent(&plain, &changed));
        if let Expr::TaggedTpl(tagged) = &mut changed {
            tagged.tpl.quasis[0].cooked = Some("a".into());
            tagged.tpl.quasis[0].tail = false;
        } else {
            panic!()
        }
        assert!(!expressions_equivalent(&plain, &changed));
    }

    #[test]
    fn types_and_casts_are_never_erased() {
        assert!(!modules_equivalent(
            &module("const x: number = 1;", "a.ts"),
            &module("const x: string = 1;", "a.ts")
        ));
        assert!(!modules_equivalent(
            &module("function f(x?: number) {}", "a.ts"),
            &module("function f(x: number) {}", "a.ts")
        ));
        assert!(!modules_equivalent(
            &module("type T = number;", "a.ts"),
            &module("type T = (number);", "a.ts")
        ));
        for (actual, expected, filename) in [
            ("value as number", "value", "a.ts"),
            ("value as number", "value as string", "a.ts"),
            ("value satisfies number", "value as number", "a.ts"),
            ("(value: ?number)", "value", "a.js"),
            ("(value: ?number)", "(value: ?string)", "a.js"),
        ] {
            assert!(!expressions_equivalent(
                &expression(actual, filename),
                &expression(expected, filename)
            ));
        }
        assert!(expressions_equivalent(
            &expression("((value as number))", "a.ts"),
            &expression("value as number", "a.ts")
        ));
        assert!(expressions_equivalent(
            &expression("((value: ?number))", "a.js"),
            &expression("(value: ?number)", "a.js")
        ));
        assert!(!expressions_equivalent(
            &expression("((value as number)) = 1", "a.ts"),
            &expression("value = 1", "a.ts")
        ));
    }

    #[test]
    fn optional_identifier_flags_survive_both_equality_paths() {
        let actual = expression("value", "i.js");
        let mut expected = actual.clone();
        let Expr::Ident(identifier) = &mut expected else {
            panic!()
        };
        identifier.optional = true;
        assert!(
            actual.eq_ignore_span(&expected),
            "exercise SWC Ident's optional omission"
        );
        assert!(!expressions_equivalent(&actual, &expected));
        let expected = Expr::Paren(ParenExpr {
            span: DUMMY_SP,
            expr: Box::new(expected),
        });
        assert!(!expressions_equivalent(&actual, &expected));
        let actual = module("const a = x; const b = y;", "i.js");
        let mut expected = actual.clone();
        struct MakeOptional;
        impl VisitMut for MakeOptional {
            fn visit_mut_ident(&mut self, id: &mut Ident) {
                if id.sym == "y" {
                    id.optional = true;
                }
            }
        }
        expected.visit_mut_with(&mut MakeOptional);
        assert!(!modules_equivalent(&actual, &expected));
    }

    #[test]
    fn hygiene_contexts_are_normalized_without_mutating_inputs() {
        GLOBALS.set(&Globals::new(), || {
            let context = SyntaxContext::empty().apply_mark(Mark::new());
            struct Contexts(SyntaxContext);
            impl VisitMut for Contexts {
                fn visit_mut_syntax_context(&mut self, ctxt: &mut SyntaxContext) {
                    *ctxt = self.0;
                }
            }
            let actual = module("function f(x: number) { return x + g(x); }", "c.ts");
            let mut expected = actual.clone();
            expected.visit_mut_with(&mut Contexts(context));
            let snapshot = expected.clone();
            assert!(modules_equivalent(&actual, &expected));
            assert!(items_equivalent(&actual.body[0], &expected.body[0]));
            assert_eq!(expected, snapshot);
            let actual = expression("(f)(x)", "c.js");
            let mut expected = expression("f(x)", "c.js");
            expected.visit_mut_with(&mut Contexts(context));
            assert!(expressions_equivalent(&actual, &expected));
        });
    }

    #[test]
    fn a_reparsed_module_is_normalized_in_place_and_still_detects_differences() {
        GLOBALS.set(&Globals::new(), || {
            let context = SyntaxContext::empty().apply_mark(Mark::new());
            struct Contexts(SyntaxContext);
            impl VisitMut for Contexts {
                fn visit_mut_syntax_context(&mut self, ctxt: &mut SyntaxContext) {
                    *ctxt = self.0;
                }
            }
            let compiled = |source: &str| {
                let mut module = module(source, "r.js");
                module.visit_mut_with(&mut Contexts(context));
                module
            };
            // Parentheses only in the re-parsed output and hygiene contexts only
            // on the compiled module: the pair postflight compares without
            // copying either side.
            let output = "export const a = [({ b: (c + 1) })];";
            let expected = compiled("export const a = [{ b: c + 1 }];");
            let snapshot = expected.clone();
            assert!(reparsed_module_equivalent(
                module(output, "r.js"),
                &expected
            ));
            assert_eq!(expected, snapshot);
            for different in [
                "export const a = [{ b: c + 2 }];",
                "export const a = [{ b: d + 1 }];",
                "export const a = [{ e: c + 1 }];",
                "export const a = [{ b: c - 1 }];",
                "export const a = [{ b: c + 1 }, 0];",
            ] {
                assert!(
                    !reparsed_module_equivalent(module(output, "r.js"), &compiled(different)),
                    "{different}"
                );
            }
        });
    }

    #[test]
    fn directive_escapes_and_grouping_require_the_separate_lexical_guard() {
        let actual = module("'use\\x20strict'; f();", "d.js");
        let expected = module("'use strict'; f();", "d.js");
        assert!(modules_equivalent(&actual, &expected));
        let ModuleItem::Stmt(Stmt::Expr(actual)) = &actual.body[0] else {
            panic!()
        };
        let ModuleItem::Stmt(Stmt::Expr(expected)) = &expected.body[0] else {
            panic!()
        };
        let Expr::Lit(Lit::Str(actual)) = &*actual.expr else {
            panic!()
        };
        let Expr::Lit(Lit::Str(expected)) = &*expected.expr else {
            panic!()
        };
        assert_ne!(actual.raw, expected.raw);
        assert!(modules_equivalent(
            &module("('use strict'); f();", "d.js"),
            &module("'use strict'; f();", "d.js")
        ));
    }

    #[test]
    fn empty_statements_order_shebangs_and_other_flags_are_retained() {
        assert!(!modules_equivalent(
            &module("f();;", "m.js"),
            &module("f();", "m.js")
        ));
        assert!(!modules_equivalent(
            &module("f(); g();", "m.js"),
            &module("g(); f();", "m.js")
        ));
        assert!(!modules_equivalent(
            &module("#!/a\nf();", "m.js"),
            &module("#!/b\nf();", "m.js")
        ));
        assert!(!expressions_equivalent(
            &expression("async x => x", "m.js"),
            &expression("x => x", "m.js")
        ));
        assert!(!modules_equivalent(
            &module("import type {T} from 'm';", "m.ts"),
            &module("import {T} from 'm';", "m.ts")
        ));
    }
}
