use stylex_evaluator::evaluate;
use stylex_state::{functions::FunctionMap, state_manager::StateManager};
use stylex_structures::stylex_options::StyleXOptions;
use swc_core::{
  common::{FileName, GLOBALS, Globals, SourceMap},
  ecma::ast::Expr,
};
use swc_ecma_parser::{EsSyntax, Parser, StringInput, Syntax};
#[test]
fn object_property_deopt_preserves_bare_reason_and_original_identifier_span() {
  GLOBALS.set(&Globals::new(), || {
    let source = "({color:missing})";
    let map = SourceMap::default();
    let file = map.new_source_file(
      FileName::Custom("deopt.js".into()).into(),
      source.to_owned(),
    );
    let mut parser = Parser::new(
      Syntax::Es(EsSyntax::default()),
      StringInput::from(&*file),
      None,
    );
    let expr = match parser.parse_expr() {
      Ok(expr) => expr,
      Err(error) => panic!("invalid fixture: {error:?}"),
    };
    let result = evaluate::evaluate(
      &expr,
      &mut StateManager::new(StyleXOptions::default()),
      &FunctionMap::default(),
    );
    assert!(!result.confident);
    let reason = match result.reason {
      Some(reason) => reason,
      None => panic!("missing reason"),
    };
    assert!(
      reason.starts_with("Referenced constant is not defined."),
      "{reason}"
    );
    assert!(!reason.contains("color >"), "{reason}");
    let fault = match result.deopt {
      Some(Expr::Ident(id)) => id,
      value => panic!("wrong fault node: {value:?}"),
    };
    assert_eq!(fault.sym.as_ref(), "missing");
    assert_eq!(fault.span.lo.0 - file.start_pos.0, 8);
    assert_eq!(fault.span.hi.0 - file.start_pos.0, 15);
  });
}
