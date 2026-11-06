//! Babel's text contract applies only to explicitly generated quoted JSX strings.
//! JSX parsers retain backslash sequences instead of unescaping them as JS. Check
//! exact generated raw spelling first, then compare the rest of the AST normally.
use super::*;
use swc_core::common::EqIgnoreSpan;
use swc_core::ecma::ast::{JSXAttr, JSXAttrName, JSXAttrValue, Module, Str};
use swc_core::ecma::visit::{Visit, VisitMut, VisitMutWith, VisitWith};

struct GeneratedAttribute {
    index: usize,
    name: JSXAttrName,
    value: Str,
    literal: String,
}
#[derive(Default)]
struct GeneratedAttributes {
    index: usize,
    generated: Vec<GeneratedAttribute>,
    error: Option<String>,
}
impl Visit for GeneratedAttributes {
    fn visit_jsx_attr(&mut self, attribute: &JSXAttr) {
        let index = self.index;
        self.index += 1;
        if attribute.span.is_dummy() {
            if let Some(JSXAttrValue::Str(value)) = &attribute.value {
                if value.span.is_dummy() {
                    let Some(text) = value.value.as_str() else {
                        self.error = Some(
                            "generated JSX raw literal has an unrepresentable isolated surrogate"
                                .into(),
                        );
                        return;
                    };
                    // Quotes require native JSX quote selection, not JS escapes.
                    if text.contains('"') {
                        if value.raw.is_some() {
                            self.error = Some("generated JSX containing a double quote must use native quote selection".into());
                            return;
                        }
                    } else {
                        let literal = js_literal::string_literal(text);
                        if value
                            .raw
                            .as_ref()
                            .is_some_and(|raw| raw.as_ref() != literal)
                        {
                            self.error = Some("generated JSX attribute raw literal is not canonical Babel spelling".into());
                            return;
                        }
                        // The public core produces DUMMY/raw=None literals;
                        // codegen fills their spelling on a clone. Authorize
                        // only this exact intended value and canonical spelling.
                        self.generated.push(GeneratedAttribute {
                            index,
                            name: attribute.name.clone(),
                            value: value.clone(),
                            literal,
                        });
                    }
                }
            }
        }
        attribute.visit_children_with(self);
    }
}

struct ValidateGenerated<'a> {
    index: usize,
    next: usize,
    generated: &'a [GeneratedAttribute],
    error: Option<String>,
}
impl VisitMut for ValidateGenerated<'_> {
    fn visit_mut_jsx_attr(&mut self, attribute: &mut JSXAttr) {
        let index = self.index;
        self.index += 1;
        if let Some(expected) = self
            .generated
            .get(self.next)
            .filter(|expected| expected.index == index)
        {
            self.next += 1;
            match &mut attribute.value {
                Some(JSXAttrValue::Str(actual))
                    if attribute.name.eq_ignore_span(&expected.name)
                        && actual
                            .raw
                            .as_ref()
                            .is_some_and(|raw| raw.as_ref() == expected.literal) =>
                {
                    // This is not a general JSX unescape: the exact text just
                    // matched an explicitly authored generated literal. Keep
                    // actual raw/span and every other node unchanged; only its
                    // intended generated value participates in typed postflight.
                    actual.value = expected.value.value.clone();
                }
                _ => self.error = Some(
                    "emitted generated JSX attribute literal does not match canonical Babel text"
                        .into(),
                ),
            }
        }
        attribute.visit_mut_children_with(self);
    }
}

pub(super) fn validate_generated_attribute_text(
    actual: &mut Module,
    expected: &Module,
) -> Result<()> {
    let mut authored = GeneratedAttributes::default();
    expected.visit_with(&mut authored);
    if let Some(error) = authored.error {
        bail!("{error}");
    }
    if authored.generated.is_empty() {
        return Ok(());
    }
    let mut validator = ValidateGenerated {
        index: 0,
        next: 0,
        generated: &authored.generated,
        error: None,
    };
    actual.visit_mut_with(&mut validator);
    if let Some(error) = validator.error {
        bail!("{error}");
    }
    if validator.next != authored.generated.len() {
        bail!("an explicitly generated JSX attribute is missing from emitted output");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::parser::parse_module;
    use swc_core::common::DUMMY_SP;

    fn expected() -> Module {
        let mut module = parse_module(
            r#"const view=<Box original='a\b' generated="a\b"/>;"#,
            "jsx.jsx",
        )
        .unwrap()
        .module;
        struct Mark;
        impl VisitMut for Mark {
            fn visit_mut_jsx_attr(&mut self, attribute: &mut JSXAttr) {
                if matches!(&attribute.name, JSXAttrName::Ident(name) if name.sym == "generated") {
                    attribute.span = DUMMY_SP;
                    let Some(JSXAttrValue::Str(value)) = &mut attribute.value else {
                        panic!("quoted attribute");
                    };
                    value.span = DUMMY_SP;
                    value.raw =
                        Some(js_literal::string_literal(value.value.as_str().unwrap()).into());
                }
            }
        }
        module.visit_mut_with(&mut Mark);
        module
    }

    #[test]
    fn canonical_generated_raw_text_is_checked_before_comparing_cooked_value() {
        let expected = expected();
        let mut actual = parse_module(
            r#"const view=<Box original='a\b' generated="a\\b"/>;"#,
            "jsx.jsx",
        )
        .unwrap()
        .module;
        assert!(!source_compare::modules_equivalent(&actual, &expected));
        validate_generated_attribute_text(&mut actual, &expected).unwrap();
        assert!(source_compare::modules_equivalent(&actual, &expected));
        // The unescaped spelling has the intended cooked value but is NOT the
        // required generated text; it must not pass the narrow check.
        let mut wrong = parse_module(
            r#"const view=<Box original='a\b' generated="a\b"/>;"#,
            "jsx.jsx",
        )
        .unwrap()
        .module;
        assert!(source_compare::modules_equivalent(&wrong, &expected));
        assert!(validate_generated_attribute_text(&mut wrong, &expected).is_err());
    }

    #[test]
    fn public_core_dummy_raw_none_uses_exact_generated_spelling() {
        let mut expected = expected();
        struct PublicMetadata;
        impl VisitMut for PublicMetadata {
            fn visit_mut_jsx_attr(&mut self, attribute: &mut JSXAttr) {
                if attribute.span.is_dummy() {
                    let Some(JSXAttrValue::Str(value)) = &mut attribute.value else {
                        panic!()
                    };
                    value.raw = None;
                }
            }
        }
        expected.visit_mut_with(&mut PublicMetadata);
        let mut actual = parse_module(
            r#"const view=<Box original='a\b' generated="a\\b"/>;"#,
            "jsx.jsx",
        )
        .unwrap()
        .module;
        validate_generated_attribute_text(&mut actual, &expected).unwrap();
        assert!(source_compare::modules_equivalent(&actual, &expected));
        let mut incorrect = parse_module(
            r#"const view=<Box original='a\b' generated="a\b"/>;"#,
            "jsx.jsx",
        )
        .unwrap()
        .module;
        assert!(validate_generated_attribute_text(&mut incorrect, &expected).is_err());
    }

    #[test]
    fn real_span_raw_none_and_noncanonical_raw_some_are_not_authority() {
        let mut expected = parse_module(
            r#"const view=<Box original='a\b' generated="a\b"/>;"#,
            "jsx.jsx",
        )
        .unwrap()
        .module;
        struct ClearRaw;
        impl VisitMut for ClearRaw {
            fn visit_mut_str(&mut self, value: &mut Str) {
                value.raw = None;
            }
        }
        expected.visit_mut_with(&mut ClearRaw);
        let mut changed = parse_module(
            r#"const view=<Box original='a\b' generated="a\\b"/>;"#,
            "jsx.jsx",
        )
        .unwrap()
        .module;
        validate_generated_attribute_text(&mut changed, &expected).unwrap();
        assert!(!source_compare::modules_equivalent(&changed, &expected));
        let mut expected = self::expected();
        struct WrongRaw;
        impl VisitMut for WrongRaw {
            fn visit_mut_jsx_attr(&mut self, attribute: &mut JSXAttr) {
                if attribute.span.is_dummy() {
                    let Some(JSXAttrValue::Str(value)) = &mut attribute.value else {
                        panic!()
                    };
                    value.raw = Some("\"not canonical\"".into());
                }
            }
        }
        expected.visit_mut_with(&mut WrongRaw);
        assert!(validate_generated_attribute_text(&mut changed, &expected).is_err());
    }

    #[test]
    fn original_attributes_names_order_and_other_ast_fields_remain_strict() {
        let expected = expected();
        let mut wrong_original = parse_module(
            r#"const view=<Box original='a\\b' generated="a\\b"/>;"#,
            "jsx.jsx",
        )
        .unwrap()
        .module;
        validate_generated_attribute_text(&mut wrong_original, &expected).unwrap();
        assert!(!source_compare::modules_equivalent(
            &wrong_original,
            &expected
        ));
        let mut wrong_name = parse_module(
            r#"const view=<Box original='a\b' wrong="a\\b"/>;"#,
            "jsx.jsx",
        )
        .unwrap()
        .module;
        assert!(validate_generated_attribute_text(&mut wrong_name, &expected).is_err());
        let mut reordered = parse_module(
            r#"const view=<Box generated="a\\b" original='a\b'/>;"#,
            "jsx.jsx",
        )
        .unwrap()
        .module;
        assert!(validate_generated_attribute_text(&mut reordered, &expected).is_err());
    }
}
