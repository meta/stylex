use stylex_evaluator::{evaluate, evaluate_result::EvaluateResult};
use stylex_state::{functions::FunctionMap, state_manager::StateManager};
use stylex_structures::stylex_options::StyleXOptions;
use swc_core::{
  common::{FileName, GLOBALS, Globals, SourceMap},
  ecma::parser::{EsSyntax, Parser, StringInput, Syntax},
};
fn fold(source: &str, depth: usize) -> Box<EvaluateResult> {
  GLOBALS.set(&Globals::new(), || {
    let map = SourceMap::default();
    let file = map.new_source_file(
      FileName::Custom("result-depth.js".into()).into(),
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
    let mut options = StyleXOptions::default();
    options.core.max_evaluation_depth = depth;
    evaluate::evaluate(
      &expression,
      &mut StateManager::new(options),
      &FunctionMap::default(),
    )
  })
}
fn nest(levels: usize) -> String {
  format!("'x'.repeat({levels}).split('').reduce((a,c)=>[a],[])")
}
#[test]
fn generated_result_depth_honours_the_configured_ceiling_and_absolute_cap() {
  // This existing-public-API shape reaches the native value/data renderer,
  // rather than only AST descent: the old renderer capped itself at32.
  let deep_join = format!("{}'x'{}.join('')", "[".repeat(40), "]".repeat(40));
  let joined = fold(&deep_join, 200);
  assert!(
    joined.confident,
    "configured data renderer must fold40 levels: {:?}",
    joined.reason
  );
  assert!(
    matches!(joined.value.as_ref().and_then(|value|value.as_expr()),Some(swc_core::ecma::ast::Expr::Lit(swc_core::ecma::ast::Lit::Str(value))) if value.value.as_str()==Some("x"))
  );
  // Put the raised-ceiling behavior first so the old-base RED reaches the
  // fixed hard-32 regression through existing public evaluator APIs.
  let raised = fold(&nest(40), 200);
  assert!(
    raised.confident,
    "raised data depth must fold: {:?}",
    raised.reason
  );
  // Confidence alone cannot prove nesting survived callback transport.
  let Some(stylex_state::evaluate_result_value::EvaluateResultValue::NativeArray(values)) =
    raised.value.as_ref()
  else {
    panic!("unexpected nested result {:?}", raised.value);
  };
  let mut depth = 1usize;
  let mut current = values.as_ref();
  while !current.is_empty() {
    assert_eq!(current.len(), 1);
    let stylex_js::native::Value::Array(next) = &current[0] else {
      panic!("nested array was flattened")
    };
    depth += 1;
    current = next.as_ref();
  }
  assert_eq!(depth, 41, "forty reduce wraps must retain all nesting");

  let shallow = fold(&nest(40), 32);
  assert!(!shallow.confident);
  assert!(
    shallow
      .reason
      .as_deref()
      .is_some_and(|reason| reason.contains("At most 32 levels")),
    "{:?}",
    shallow.reason
  );
  let too_deep = fold(&nest(400), 200);
  assert!(!too_deep.confident);
  assert!(
    too_deep
      .reason
      .as_deref()
      .is_some_and(|reason| reason.contains("At most 200 levels")),
    "{:?}",
    too_deep.reason
  );
  let capped = fold(&nest(300), 512);
  assert!(!capped.confident);
  assert!(
    capped
      .reason
      .as_deref()
      .is_some_and(|reason| reason.contains("At most 256 levels")),
    "{:?}",
    capped.reason
  );
}
