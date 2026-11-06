#[cfg(test)]
mod key_span_index {

  use rustc_hash::FxHashSet;
  use swc_core::{
    atoms::Atom,
    common::{FileName, SourceMap, Span, sync::Lrc},
    ecma::{
      ast::{EsVersion, Module},
      parser::{Parser, StringInput, Syntax, TsSyntax, lexer::Lexer},
    },
  };

  use crate::key_span_index::{FileOffset, KeySpanIndex, NamespaceKeyQuery};

  fn parse(source: &str) -> Module {
    parse_into(&Default::default(), FileName::Anon, source)
  }

  /// Parses into the caller's source map, allowing fixtures with nonzero module bases.
  fn parse_into(source_map: &Lrc<SourceMap>, name: FileName, source: &str) -> Module {
    let source_file = source_map.new_source_file(name.into(), source.to_owned());
    let lexer = Lexer::new(
      Syntax::Typescript(TsSyntax {
        tsx: true,
        ..Default::default()
      }),
      EsVersion::EsNext,
      StringInput::from(&*source_file),
      None,
    );

    match Parser::new_from(lexer).parse_module() {
      Ok(module) => module,
      Err(error) => panic!("failed to parse the fixture: {:?}", error),
    }
  }

  fn keys(names: &[&str]) -> FxHashSet<Atom> {
    names.iter().map(|name| Atom::from(*name)).collect()
  }

  /// Builds a query with explicit ranking signals.
  fn query<'a>(
    key: &'a str,
    siblings: &[&str],
    value_keys: &[&str],
    target_offset: Option<u32>,
  ) -> NamespaceKeyQuery<'a> {
    // Fresh fixture maps start at BytePos(1), so target text indices are file offsets.
    NamespaceKeyQuery {
      namespace_key: key,
      sibling_keys: std::rc::Rc::new(keys(siblings)),
      namespace_value_keys: keys(value_keys),
      target_offset: target_offset.map(FileOffset::at),
      callee: None,
    }
  }

  /// Returns the 1-based line containing `needle`.
  fn line_of(source: &str, needle: &str) -> usize {
    let offset = match source.find(needle) {
      Some(offset) => offset,
      None => panic!("the fixture does not contain {needle}"),
    };

    source[..offset].matches('\n').count() + 1
  }

  /// Returns the 1-based span line; fixture maps start at BytePos(1).
  fn line_at(source: &str, span: Span) -> usize {
    assert!(!span.is_dummy(), "the lookup resolved no position");

    let offset = span.lo.0 as usize - 1;

    source[..offset].matches('\n').count() + 1
  }

  fn resolved_line(source: &str, key: &str, siblings: &[&str], value_keys: &[&str]) -> usize {
    let module = parse(source);
    let span = KeySpanIndex::build(&module).resolve(&query(key, siblings, value_keys, None));

    line_at(source, span)
  }

  #[test]
  fn an_object_that_names_nothing_is_not_a_candidate() {
    let source = "\
const spread = stylex.create({
  ...base,
});
const styles = stylex.create({
  root: { color: 'red' },
});
";

    assert_eq!(
      resolved_line(source, "root", &["root"], &["color"]),
      line_of(source, "root: { color: 'red' }")
    );
  }
}
