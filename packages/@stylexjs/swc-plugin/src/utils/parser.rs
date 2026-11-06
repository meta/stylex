use anyhow::{anyhow, Result};
use swc_common::{sync::Lrc, FileName, SourceFile, SourceMap};
use swc_ecma_ast::{EsVersion, Expr, Module, ModuleItem};
use swc_ecma_codegen::{text_writer::JsWriter, Config as CodegenConfig, Emitter};
use swc_ecma_parser::{lexer::Lexer, EsSyntax, Parser, StringInput, Syntax, TsSyntax};

pub(crate) fn syntax_for_file(filename: &str) -> Syntax {
    if filename.ends_with(".ts") || filename.ends_with(".tsx") {
        Syntax::Typescript(TsSyntax {
            tsx: filename.ends_with(".tsx"),
            ..Default::default()
        })
    } else {
        Syntax::Es(EsSyntax {
            jsx: true,
            ..Default::default()
        })
    }
}

pub(crate) struct ParsedModule {
    pub source_map: Lrc<SourceMap>,
    pub file: Lrc<SourceFile>,
    pub module: Module,
}

pub(crate) fn parse_module(source: &str, filename: &str) -> Option<ParsedModule> {
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
        .ok()
        .map(|module| ParsedModule {
            source_map,
            file,
            module,
        })
}

pub(crate) fn parse_expr(source: &str, filename: &str) -> Result<Box<Expr>> {
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
        .parse_expr()
        .map_err(|error| anyhow!("failed to parse expression snippet: {:?}", error))
}

pub(crate) fn parse_module_items(source: &str, filename: &str) -> Result<Vec<ModuleItem>> {
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
    let module = parser
        .parse_module()
        .map_err(|error| anyhow!("failed to parse {}: {:?}", filename, error))?;
    Ok(module.body)
}

pub fn normalize_js(code: &str, filename: &str) -> Result<String> {
    let source_map = Lrc::new(SourceMap::default());
    let file = source_map.new_source_file(FileName::Real(filename.into()).into(), code.to_owned());
    let lexer = Lexer::new(
        syntax_for_file(filename),
        EsVersion::Es2022,
        StringInput::from(&*file),
        None,
    );
    let mut parser = Parser::new_from(lexer);
    let module = parser
        .parse_module()
        .map_err(|error| anyhow!("failed to parse {}: {:?}", filename, error))?;

    let mut output = Vec::new();
    {
        let mut emitter = Emitter {
            cfg: CodegenConfig::default(),
            cm: source_map.clone(),
            comments: None,
            wr: JsWriter::new(source_map, "\n", &mut output, None),
        };
        emitter
            .emit_module(&module)
            .map_err(|error| anyhow!("failed to emit {}: {}", filename, error))?;
    }

    Ok(String::from_utf8(output).map_err(|error| anyhow!(error))?)
}
