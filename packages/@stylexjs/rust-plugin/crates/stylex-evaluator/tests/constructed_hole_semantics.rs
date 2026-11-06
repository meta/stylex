use stylex_evaluator::{evaluate, evaluate_result::EvaluateResult};
use stylex_state::{functions::FunctionMap, state_manager::StateManager};
use stylex_structures::stylex_options::StyleXOptions;
use swc_core::{
  common::{FileName, GLOBALS, Globals, SourceMap},
  ecma::{
    ast::{Expr, Lit},
    parser::{EsSyntax, Parser, StringInput, Syntax},
  },
};
fn fold(source: &str) -> Box<EvaluateResult> {
  GLOBALS.set(&Globals::new(), || {
    let map = SourceMap::default();
    let file = map.new_source_file(
      FileName::Custom("holes.js".into()).into(),
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
fn constructed_holes_join_without_becoming_textual_undefined() {
  // Executed Node oracle: Array(3).join(',') and toString are ',,';
  // Get is undefined, but a missing slot is empty in the array join algorithm.
  for (source, expected) in [
    ("Array(3).join(',')", ",,"),
    ("Array(3).toString()", ",,"),
    ("String(Array(3))", ",,"),
    ("Array(3).map(()=>missing).join('-')", "--"),
    ("[...Array(3)].map(()=>1).join(',')", "1,1,1"),
    ("Array(3).concat([1]).join(',')", ",,,1"),
  ] {
    let result = fold(source);
    assert!(result.confident, "{source}: {:?}", result.reason);
    match result.value.as_ref().and_then(|value| value.as_expr()) {
      Some(Expr::Lit(Lit::Str(value))) => {
        assert_eq!(value.value.as_str(), Some(expected), "{source}")
      },
      value => panic!("{source}: {value:?}"),
    }
  }
}
#[test]
fn copying_and_iterating_choose_absence_or_get_exactly() {
  // Node: map/slice/concat preserve absent slots; spread and change-by-copy
  // methods perform Get and make own undefined entries. flatMap skips holes
  // in the callback's returned array, and neither map nor forEach invokes a
  // callback for an absent input slot.
  for (source, expected) in [
    ("Array(3).length", 3.0),
    ("0 in Array(3)?1:0", 0.0),
    ("Array(3).at(0)===undefined?1:0", 1.0),
    ("Object.keys(Array(3)).length", 0.0),
    ("Object.keys(Array(3).map(()=>missing)).length", 0.0),
    ("Object.keys(Array(3).slice()).length", 0.0),
    ("Object.keys(Array(3).concat([1])).length", 1.0),
    ("Object.keys([...Array(3)]).length", 3.0),
    ("Object.keys(Array(3).toReversed()).length", 3.0),
    ("Object.keys(Array(3).toSpliced()).length", 3.0),
    ("Object.keys(Array(3).with(0,1)).length", 3.0),
    ("Object.keys(Array(3).toSorted(()=>missing)).length", 3.0),
    ("[1].flatMap(()=>Array(3)).length", 0.0),
    ("Array(3).flatMap(()=>missing).length", 0.0),
    ("Array(3).flat().length", 0.0),
    ("Array(3).includes(undefined)?1:0", 1.0),
    ("Array(3).indexOf(undefined)", -1.0),
    (
      "typeof Array(3).forEach(()=>missing)==='undefined'?1:0",
      1.0,
    ),
  ] {
    let result = fold(source);
    assert!(result.confident, "{source}: {:?}", result.reason);
    match result.value.as_ref().and_then(|value| value.as_expr()) {
      Some(Expr::Lit(Lit::Num(value))) => assert_eq!(value.value, expected, "{source}"),
      value => panic!("{source}: {value:?}"),
    }
  }
  let result = fold("Array(3).reduce((a,b)=>b)");
  assert!(
    !result.confident,
    "empty reduce without initial value must refuse rather than invent a value"
  );
}

#[test]
fn materialization_keeps_absent_slots_and_effectful_literal_lengths_refuse() {
  let result = fold("Array(3)");
  let value = result
    .value
    .unwrap_or_else(|| panic!("{:?}", result.reason));
  let state = StateManager::new(StyleXOptions::default());
  let expression = match evaluate::materialize_result(&value, &state) {
    Ok(Some(Expr::Array(array))) => array,
    value => panic!("{value:?}"),
  };
  assert_eq!(expression.elems.len(), 3);
  assert!(expression.elems.iter().all(Option::is_none));
  // A count shortcut must not skip a reference/call just because another slot
  // is absent. A source with a hole+spread refuses with the hole's own reason.
  for source in [
    "[missing,,].length",
    "[...['x'],,].length",
    "[, ...['x']].length",
  ] {
    let result = fold(source);
    assert!(!result.confident, "{source}: {:?}", result.value);
  }
}
