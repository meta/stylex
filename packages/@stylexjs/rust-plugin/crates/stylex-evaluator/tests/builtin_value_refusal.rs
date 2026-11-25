use stylex_evaluator::evaluate;
use stylex_state::{functions::FunctionMap, state_manager::StateManager};
use stylex_structures::stylex_options::StyleXOptions;
use swc_core::{
  common::{FileName, GLOBALS, Globals, SourceMap},
  ecma::{
    ast::{Expr, Lit},
    parser::{EsSyntax, Parser, StringInput, Syntax},
  },
};
fn fold(source: &str) -> Box<stylex_evaluator::evaluate_result::EvaluateResult> {
  GLOBALS.set(&Globals::new(), || {
    let map = SourceMap::default();
    let file = map.new_source_file(
      FileName::Custom("builtin-values.js".into()).into(),
      source.to_owned(),
    );
    let mut parser = Parser::new(
      Syntax::Es(EsSyntax::default()),
      StringInput::from(&*file),
      None,
    );
    let expression = parser
      .parse_expr()
      .unwrap_or_else(|error| panic!("{error:?}"));
    evaluate::evaluate(
      &expression,
      &mut StateManager::new(StyleXOptions::default()),
      &FunctionMap::default(),
    )
  })
}
#[test]
fn unmodelled_builtin_function_values_refuse_instead_of_becoming_undefined() {
  // Node/Babel yield functions here; function values are not emittable style data.
  for source in [
    "'a'.trim",
    "'a'.normalize",
    "[1].map",
    "[1].join",
    "(1).toFixed",
    "({}).hasOwnProperty",
    "({}).toString",
    "Math.sin",
    "Math.random",
    "Object.keys",
    "String.fromCharCode",
    "Array.from",
    "Number.isInteger",
    "['a'].reduce((a,b)=>b.trim,0)",
    "[1].map(x=>[x].map)",
    "[1].map(x=>Object.keys)",
  ] {
    let result = fold(source);
    assert!(
      !result.confident,
      "{source} must refuse a function-value surface; got {:?}",
      result.value
    );
  }
}
#[test]
fn known_data_and_allowlisted_calls_still_fold() {
  for (source, expected) in [
    ("[1,2].length", 2.0),
    ("({toString:7}).toString", 7.0),
    ("Math.abs(-2)", 2.0),
  ] {
    let result = fold(source);
    assert!(result.confident, "{source}: {:?}", result.reason);
    match result.value.and_then(|value| value.as_expr().cloned()) {
      Some(Expr::Lit(Lit::Num(value))) => assert_eq!(value.value, expected),
      value => panic!("{source}: {value:?}"),
    }
  }
  let result = fold("' x '.trim()");
  assert!(result.confident, "{:?}", result.reason);
  assert!(
    matches!(result.value.as_ref().and_then(|value|value.as_expr()),Some(Expr::Lit(Lit::Str(value))) if value.value.as_str()==Some("x"))
  );
  let result = fold("'x'.unknown");
  assert!(
    result.confident,
    "a missing data property must remain ordinary undefined"
  );
}
