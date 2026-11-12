//! Conservative check for modules the StyleX visitor cannot change; anything
//! it is unsure of goes through the ordinary compiler.
use crate::shared::StyleXTransformOptions;
use swc_core::ecma::{
    ast::{Ident, IdentName, Module, Str, TplElement},
    visit::{Visit, VisitWith},
};

pub(super) fn may_transform(module: &Module, options: &StyleXTransformOptions) -> bool {
    let sx = match options.additional_options.get("sxPropName") {
        Some(serde_json::Value::Bool(false)) => None,
        Some(serde_json::Value::String(name)) => Some(name.as_str()),
        _ => Some("sx"),
    };
    let mut discovery = Discovery {
        options,
        sx,
        relevant: false,
    };
    module.visit_with(&mut discovery);
    discovery.relevant
}

struct Discovery<'a> {
    options: &'a StyleXTransformOptions,
    sx: Option<&'a str>,
    relevant: bool,
}

impl Discovery<'_> {
    fn text(&mut self, text: &str) {
        self.relevant |= self.options.matches_source(text)
            || text == "@stylexjs/atoms"
            || self.sx == Some(text)
            || (self.options.runtime_injection_enabled()
                && text == self.options.runtime_injection_path())
            // `rewriteAliases` rewrites theme imports in every file, as Babel does.
            || (self.options.rewrite_aliases && super::is_theme_import(text));
    }
}

impl Visit for Discovery<'_> {
    fn visit_str(&mut self, value: &Str) {
        // The core may refuse imports with lone surrogates; never convert them lossily.
        match value.value.as_str() {
            Some(text) => self.text(text),
            None => self.relevant = true,
        }
    }

    fn visit_ident(&mut self, value: &Ident) {
        self.relevant |= self.sx == Some(value.sym.as_ref());
    }

    fn visit_ident_name(&mut self, value: &IdentName) {
        // Includes JSX attributes and object keys.
        self.relevant |= self.sx == Some(value.sym.as_ref());
    }

    fn visit_tpl_element(&mut self, value: &TplElement) {
        self.text(value.raw.as_ref());
        if let Some(cooked) = &value.cooked {
            match cooked.as_str() {
                Some(text) => self.text(text),
                None => self.relevant = true,
            }
        }
    }

    fn visit_module_items(&mut self, items: &[swc_core::ecma::ast::ModuleItem]) {
        for item in items {
            if self.relevant {
                break;
            }
            item.visit_with(self);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::parser::{parse_module_with_options, syntax_for_file, SourceType};

    #[test]
    fn a_theme_import_needs_the_compiler_only_under_rewrite_aliases() {
        let source = "import { tokens } from './tokens.stylex.js';export { tokens };";
        let parsed = parse_module_with_options(
            source,
            "proof.js",
            syntax_for_file("proof.js"),
            SourceType::Module,
        )
        .unwrap();
        let mut options = StyleXTransformOptions::default();
        assert!(!may_transform(&parsed.module, &options));
        options.rewrite_aliases = true;
        assert!(may_transform(&parsed.module, &options));
    }

    #[test]
    fn discovery_is_a_conservative_no_change_proof() {
        let options = StyleXTransformOptions::default();
        for (source, relevant) in [
            (
                "import type {P} from './types';export const C=(p:P)=><Widget {...p}/>;",
                false,
            ),
            ("const value=1;export function read(){return value;}", false),
            ("import type {create} from '@stylexjs/stylex';", true),
            ("import 'stylex';", true),
            ("const view=<div sx={external}/>;", true),
            ("const view=_jsx('div',{sx:external});", true),
            ("const view=_jsx('div',{sx});", true),
            ("_$setAttribute(element,'sx',external);", true),
            ("const css=require('@stylexjs/atoms');", true),
        ] {
            let parsed = parse_module_with_options(
                source,
                "proof.tsx",
                syntax_for_file("proof.tsx"),
                SourceType::Module,
            )
            .unwrap();
            assert_eq!(
                may_transform(&parsed.module, &options),
                relevant,
                "{source}"
            );
        }
    }
}
