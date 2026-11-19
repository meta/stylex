//! Shared by the transform's test files: a parse matching the compiler's, and a transform built
//! over it.

use std::rc::Rc;
use std::sync::Arc;

use swc_core::{
  common::{FileName, Mark, SourceMap, comments::SingleThreadedComments, input::StringInput},
  ecma::{
    ast::{Module, Pass, Program},
    parser::{Parser, Syntax, TsSyntax, lexer::Lexer},
    transforms::base::resolver,
  },
};

use crate::{StyleXTransform, StyleXTransformBuilder};

/// No comments, as a module without comments hands the compiler.
pub(crate) type TestComments = Rc<SingleThreadedComments>;

/// Parses `code` as TSX and runs SWC's resolver, as the compiler does before the StyleX pass,
/// so identifiers carry syntax contexts. TSX reads every module a case here writes.
pub(crate) fn resolved_module(code: &str) -> Module {
  resolved_module_in(
    code,
    Syntax::Typescript(TsSyntax {
      tsx: true,
      ..Default::default()
    }),
  )
}

/// The same over TypeScript without JSX, the only syntax that reads `<any>value` casts.
pub(crate) fn resolved_ts_module(code: &str) -> Module {
  resolved_module_in(code, Syntax::Typescript(Default::default()))
}

fn resolved_module_in(code: &str, syntax: Syntax) -> Module {
  let source_map = SourceMap::default();

  // Named by syntax, so a parse failure names the parse that failed.
  let file_name = match syntax {
    Syntax::Typescript(TsSyntax { tsx: true, .. }) => "input.tsx",
    _ => "input.ts",
  };

  let source_file = source_map.new_source_file(
    Arc::new(FileName::Custom(file_name.to_string())),
    code.to_string(),
  );

  let lexer = Lexer::new(
    syntax,
    Default::default(),
    StringInput::from(&*source_file),
    None,
  );

  let module = match Parser::new_from(lexer).parse_module() {
    Ok(module) => module,
    Err(error) => panic!("failed to parse fixture: {:?}", error),
  };

  let mut program = Program::Module(module);
  resolver(Mark::new(), Mark::new(), true).process(&mut program);

  match program {
    Program::Module(module) => module,
    Program::Script(_) => unreachable!("a parsed module never becomes a script"),
  }
}

/// A transform built for a test, with `customize` applied to its builder.
pub(crate) fn test_transform(
  customize: impl FnOnce(StyleXTransformBuilder<TestComments>) -> StyleXTransformBuilder<TestComments>,
) -> StyleXTransform<TestComments> {
  customize(StyleXTransform::test(comments())).build()
}

pub(crate) fn comments() -> TestComments {
  Rc::new(SingleThreadedComments::default())
}
