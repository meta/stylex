use std::rc::Rc;
use stylex_evaluator::evaluate;
use stylex_state::{functions::FunctionMap, state_manager::StateManager};
use stylex_structures::stylex_options::StyleXOptions;
use swc_core::{
  common::{FileName, GLOBALS, Globals, SourceMap},
  ecma::ast::{Expr, Lit},
};
use swc_ecma_parser::{EsSyntax, Parser, StringInput, Syntax};
fn fold(source: &str) -> Box<stylex_evaluator::evaluate_result::EvaluateResult> {
  GLOBALS.set(&Globals::new(), || {
    let source_map = SourceMap::default();
    let file = source_map.new_source_file(
      FileName::Custom("native.js".into()).into(),
      source.to_owned(),
    );
    let mut parser = Parser::new(
      Syntax::Es(EsSyntax::default()),
      StringInput::from(&*file),
      None,
    );
    let expr = parser.parse_expr().expect("valid source");
    evaluate::evaluate(
      &expr,
      &mut StateManager::new(StyleXOptions::default()),
      &FunctionMap::default(),
    )
  })
}
#[test]
fn modeled_native_calls_and_utf16_work_without_boa() {
  for (source, expected) in [
    ("'😀'[0].charCodeAt(0)", 55357.0),
    ("({... '😀'})[0].charCodeAt(0)", 55357.0),
    ("'😀'.charCodeAt(0)", 55357.0),
    ("'\\uD800'.charCodeAt(0)", 55296.0),
    ("Math.max(1,2,3)", 3.0),
    ("Number(' 0x10 ')", 16.0),
  ] {
    let result = fold(source);
    assert!(result.confident, "{source}: {:?}", result.reason);
    let Some(value) = result.value else {
      panic!("no value");
    };
    let Some(Expr::Lit(Lit::Num(number))) = value.as_expr() else {
      panic!("{value:?}");
    };
    assert_eq!(number.value, expected, "{source}");
  }
}
#[test]
fn memo_keys_include_the_actual_injected_binding_instance() {
  use stylex_state::functions::{FunctionConfig, FunctionConfigType, FunctionType};
  use swc_core::{
    common::DUMMY_SP,
    ecma::ast::{Ident, Number},
  };
  GLOBALS.set(&Globals::new(), || {
    let expr = Expr::Ident(Ident::new("value".into(), DUMMY_SP, Default::default()));
    let mut traversal = StateManager::new(StyleXOptions::default());
    for expected in [1.0, 2.0] {
      let mut functions = FunctionMap::default();
      functions.identifiers.insert(
        "value".into(),
        Box::new(FunctionConfigType::Regular(FunctionConfig {
          takes_path: false,
          fn_ptr: FunctionType::Mapper(Rc::new(move || {
            Expr::Lit(Lit::Num(Number {
              span: DUMMY_SP,
              value: expected,
              raw: None,
            }))
          })),
        })),
      );
      let result = evaluate::evaluate(&expr, &mut traversal, &functions);
      assert!(result.confident);
      let Some(value) = result.value else { panic!() };
      let Some(Expr::Lit(Lit::Num(number))) = value.as_expr() else {
        panic!("{value:?}")
      };
      assert_eq!(
        number.value, expected,
        "an earlier function-map value must not answer this new lexical binding"
      );
    }
  });
}
#[test]
fn callback_parameter_values_do_not_share_stale_results() {
  let result = fold("[1,2,3].map(x=>x*2).join(',')");
  assert!(result.confident, "{:?}", result.reason);
  let Some(value) = result.value else { panic!() };
  let Some(Expr::Lit(Lit::Str(text))) = value.as_expr() else {
    panic!("{value:?}")
  };
  assert_eq!(text.value.as_str(), Some("2,4,6"));
}
