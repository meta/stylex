#[cfg(target_arch = "wasm32")]
use swc_core::{
    common::{sync::Lrc, FileName, SourceMap},
    ecma::{
        ast::{EsVersion, Module, Program},
        codegen::{text_writer::JsWriter, Config as CodegenConfig, Emitter},
        parser::{lexer::Lexer, EsSyntax, Parser, StringInput, Syntax, TsSyntax},
        visit::{VisitMut, VisitMutWith},
    },
    plugin::{
        metadata::TransformPluginMetadataContextKind, plugin_transform,
        proxies::TransformPluginProgramMetadata,
    },
};

mod core;
mod shared;
mod utils;
mod visitors;

pub use shared::{
    flatten_nested_consts_config, flatten_nested_overrides_config, flatten_nested_string_config,
    flatten_nested_vars_config, unflatten_object, CollectedImports, ImportSource,
    RuntimeInjectionOption, StyleXTransformOptions, TransformOutput,
};
pub use core::{
    compile_create_theme, compile_create_theme_nested, compile_define_consts,
    compile_define_consts_nested, compile_define_vars, compile_define_vars_nested, CreateOptions,
    process_stylex_rules, ProcessStylexRulesConfig, RuleEntry, RuleFields, StyleValue, UseLayers,
    LayersConfig,
};
pub use utils::{normalize_js, process_metadata_to_css, transform_file, transform_source};
pub use visitors::collect_stylex_imports;

#[cfg(target_arch = "wasm32")]
pub struct TransformVisitor {
    filename: String,
    options: StyleXTransformOptions,
}

#[cfg(target_arch = "wasm32")]
impl TransformVisitor {
    fn new(filename: String, options: StyleXTransformOptions) -> Self {
        Self { filename, options }
    }
}

#[cfg(target_arch = "wasm32")]
impl VisitMut for TransformVisitor {
    fn visit_mut_module(&mut self, module: &mut Module) {
        let source = emit_module(module).expect("emit program for stylex plugin");
        let output = transform_source(&source, &self.filename, &self.options)
            .unwrap_or_else(|error| panic!("stylex_swc_plugin: {}", error));
        *module = parse_module(&output.code, &self.filename)
            .expect("parse transformed stylex output");
    }
}

#[cfg(target_arch = "wasm32")]
fn syntax_for_file(filename: &str) -> Syntax {
    if filename.ends_with(".ts") || filename.ends_with(".tsx") {
        Syntax::Typescript(TsSyntax {
            tsx: filename.ends_with(".tsx"),
            ..Default::default()
        })
    } else {
        Syntax::Es(EsSyntax {
            jsx: filename.ends_with(".jsx"),
            ..Default::default()
        })
    }
}

#[cfg(target_arch = "wasm32")]
fn emit_module(module: &Module) -> anyhow::Result<String> {
    let source_map = Lrc::new(SourceMap::default());
    let mut output = Vec::new();
    let mut emitter = Emitter {
        cfg: CodegenConfig::default(),
        cm: source_map.clone(),
        comments: None,
        wr: JsWriter::new(source_map, "\n", &mut output, None),
    };
    emitter.emit_module(module)?;
    String::from_utf8(output).map_err(Into::into)
}

#[cfg(target_arch = "wasm32")]
fn parse_module(source: &str, filename: &str) -> anyhow::Result<Module> {
    let source_map = Lrc::new(SourceMap::default());
    let file =
        source_map.new_source_file(FileName::Real(filename.into()).into(), source.to_owned());
    let lexer = Lexer::new(
        syntax_for_file(filename),
        EsVersion::Es2022,
        StringInput::from(&*file),
        None,
    );
    let mut parser = Parser::new_from(lexer);
    parser
        .parse_module()
        .map_err(|error| anyhow::anyhow!("{:?}", error))
}

#[cfg(target_arch = "wasm32")]
fn parse_options(metadata: &TransformPluginProgramMetadata) -> StyleXTransformOptions {
    metadata
        .get_transform_plugin_config()
        .and_then(|config| serde_json::from_str::<StyleXTransformOptions>(&config).ok())
        .unwrap_or_default()
}

#[cfg(target_arch = "wasm32")]
fn filename_from_metadata(metadata: &TransformPluginProgramMetadata) -> String {
    metadata
        .get_context(&TransformPluginMetadataContextKind::Filename)
        .unwrap_or_else(|| "stylex-plugin.js".to_owned())
}

#[cfg(target_arch = "wasm32")]
#[plugin_transform]
pub fn process_transform(
    mut program: Program,
    metadata: TransformPluginProgramMetadata,
) -> Program {
    let filename = filename_from_metadata(&metadata);
    let options = parse_options(&metadata);
    let mut visitor = TransformVisitor::new(filename, options);
    program.visit_mut_with(&mut visitor);
    program
}
