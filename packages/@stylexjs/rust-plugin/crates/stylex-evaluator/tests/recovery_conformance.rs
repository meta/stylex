use std::rc::Rc;
use stylex_evaluator::{evaluate, evaluate_result::EvaluateResult};
use stylex_state::{functions::FunctionMap, state_manager::StateManager};
use stylex_structures::stylex_options::StyleXOptions;
use swc_core::{
  common::{FileName, GLOBALS, Globals, SourceMap},
  ecma::ast::{Expr, Lit},
};
use swc_ecma_parser::{EsSyntax, Parser, StringInput, Syntax};
fn fold(source: &str) -> Box<EvaluateResult> {
  GLOBALS.set(&Globals::new(), || {
    let map = SourceMap::default();
    let file = map.new_source_file(
      FileName::Custom("native.js".into()).into(),
      source.to_owned(),
    );
    let mut parser = Parser::new(
      Syntax::Es(EsSyntax::default()),
      StringInput::from(&*file),
      None,
    );
    let expr = match parser.parse_expr() {
      Ok(expr) => expr,
      Err(error) => panic!("invalid test source: {error:?}"),
    };
    evaluate::evaluate(
      &expr,
      &mut StateManager::new(StyleXOptions::default()),
      &FunctionMap::default(),
    )
  })
}
fn number(source: &str) -> f64 {
  let result = fold(source);
  assert!(result.confident, "{source}: {:?}", result.reason);
  match result.value.and_then(|value| value.as_expr().cloned()) {
    Some(Expr::Lit(Lit::Num(n))) => n.value,
    value => panic!("{source}: {value:?}"),
  }
}
#[test]
fn native_utf16_and_expanded_array_lengths() {
  for (source, expected) in [
    ("'😀'[0].charCodeAt(0)", 55357.0),
    ("({... '😀'})[1].charCodeAt(0)", 56832.0),
    ("[1,...[2,3],4].length", 4.0),
    ("[...[]].length", 0.0),
    ("[...[1,2],...[3]].length", 3.0),
  ] {
    assert_eq!(number(source), expected, "{source}");
  }
}
#[test]
fn callback_arrays_keep_all_observable_arguments() {
  assert_eq!(number("[1,2,3].map((v,i,a)=>a.length+v+i)[2]"), 8.0);
  assert_eq!(
    number("[1,2].map(v=>()=>v)[0]() + [1,2].map(v=>()=>v)[1]()"),
    3.0
  );
  assert_eq!(number("((a)=>((b)=>a+b))(2)(3)"), 5.0);
}
#[test]
fn function_map_identity_keeps_the_actual_instance() {
  use stylex_state::functions::{FunctionConfig, FunctionConfigType, FunctionType};
  use swc_core::{
    common::DUMMY_SP,
    ecma::ast::{Ident, Number},
  };
  GLOBALS.set(&Globals::new(), || {
    let expr = Expr::Ident(Ident::new("value".into(), DUMMY_SP, Default::default()));
    let mut state = StateManager::new(StyleXOptions::default());
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
      let result = evaluate::evaluate(&expr, &mut state, &functions);
      match result.value.and_then(|v| v.as_expr().cloned()) {
        Some(Expr::Lit(Lit::Num(n))) => assert_eq!(n.value, expected),
        value => panic!("{value:?}"),
      }
    }
  });
}
#[test]
fn captured_ledger_survives_scope_return_and_isolation_unwind() {
  use stylex_js::native::*;
  let lease = with_evaluation(|| {
    assert!(charge_work(MAX_FUEL - 1).is_some());
    capture_budget_lease().unwrap_or_else(|| panic!("active scope"))
  });
  let snapshot = lease
    .upgrade()
    .unwrap_or_else(|| panic!("ledger remains owned"));
  with_captured_evaluation(&snapshot, || {
    assert!(charge_work(1).is_some());
    assert!(charge_work(1).is_none());
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
      with_isolated_budget(|| {
        with_evaluation(|| {
          assert!(charge_work(1).is_some());
          panic!("intentional unwind")
        })
      })
    }));
    assert!(
      charge_work(1).is_none(),
      "outer budget must not be refunded"
    );
  });
  with_isolated_budget(|| with_evaluation(|| assert!(charge_work(1).is_some())));
}

#[test]
fn object_deopts_keep_the_reason_and_original_fault_span() {
  let result = fold("({color:missing})");
  assert!(!result.confident);
  let reason = result.reason.unwrap_or_else(|| panic!("deopt reason"));
  assert!(
    reason.starts_with("Referenced constant is not defined."),
    "{reason}"
  );
  assert!(!reason.contains("color >"), "{reason}");
  let fault = result.deopt.unwrap_or_else(|| panic!("deopt expression"));
  assert!(matches!(fault, Expr::Ident(_)), "{fault:?}");
}

#[test]
fn bounded_constructed_arrays_keep_holes_distinct_from_undefined() {
  assert_eq!(number("Array(3).length"), 3.0);
  assert_eq!(number("Object.keys(Array(3)).length"), 0.0);
  assert_eq!(number("Object.keys(Array(3).map(()=>1)).length"), 0.0);
  assert_eq!(number("Array.from(Array(3)).length"), 3.0);
  assert_eq!(number("Array(3).includes(undefined)?1:0"), 1.0);
  assert_eq!(number("Array(3).indexOf(undefined)"), -1.0);
  assert_eq!(number("[undefined].indexOf(undefined)"), 0.0);
}

#[test]
fn scoped_proxy_reads_record_only_executed_keys_even_after_warming() {
  use std::cell::RefCell;
  use stylex_evaluator::evaluate::evaluate_function_in_scope_observed;
  use stylex_state::{evaluate_result_value::EvaluateResultValue, theme_ref::ThemeRef};
  use swc_core::{common::DUMMY_SP, ecma::ast::Ident};
  GLOBALS.set(&Globals::new(), || {
    let source = "()=>false ? tokens.unused : tokens.used";
    let map = SourceMap::default();
    let file = map.new_source_file(
      FileName::Custom("reads.js".into()).into(),
      source.to_owned(),
    );
    let mut parser = Parser::new(
      Syntax::Es(EsSyntax::default()),
      StringInput::from(&*file),
      None,
    );
    let expr = match parser.parse_expr() {
      Ok(expr) => expr,
      Err(error) => panic!("{error:?}"),
    };
    let mut state = StateManager::new(StyleXOptions::default());
    let result = evaluate::evaluate(&expr, &mut state, &FunctionMap::default());
    let function = match result.value {
      Some(EvaluateResultValue::Callback(function)) => function,
      value => panic!("{value:?}"),
    };
    let mut overlays = rustc_hash::FxHashMap::default();
    overlays.insert(
      Ident::new("tokens".into(), DUMMY_SP, Default::default()).to_id(),
      EvaluateResultValue::ThemeRef(ThemeRef::new("/x.stylex.js", "tokens", "x")),
    );
    let reads = Rc::new(RefCell::new(Vec::new()));
    for _ in 0..2 {
      let output = reads.clone();
      let observed = Rc::new(move |receiver: &EvaluateResultValue, key: &str| {
        assert!(matches!(receiver, EvaluateResultValue::ThemeRef(_)));
        output.borrow_mut().push(key.to_owned());
      });
      let result = evaluate_function_in_scope_observed(
        &function,
        vec![],
        &mut state,
        swc_core::common::Span::new(file.end_pos, file.end_pos),
        &overlays,
        Some(observed),
      );
      assert!(result.is_ok(), "{result:?}");
    }
    assert_eq!(&*reads.borrow(), &["used", "used"]);
  });
}

#[test]
fn trusted_self_proxy_marker_is_boolean_and_not_a_dependency() {
  use std::cell::RefCell;
  use stylex_evaluator::evaluate::evaluate_function_in_scope_observed;
  use stylex_state::{evaluate_result_value::EvaluateResultValue, theme_ref::ThemeRef};
  use swc_core::{common::DUMMY_SP, ecma::ast::Ident};
  GLOBALS.set(&Globals::new(), || {
    let map = SourceMap::default();
    let file = map.new_source_file(
      FileName::Custom("marker.js".into()).into(),
      "()=>tokens.__IS_PROXY ? 1 : 2".to_owned(),
    );
    let mut parser = Parser::new(
      Syntax::Es(EsSyntax::default()),
      StringInput::from(&*file),
      None,
    );
    let expression = match parser.parse_expr() {
      Ok(expression) => expression,
      Err(error) => panic!("{error:?}"),
    };
    let mut state = StateManager::new(StyleXOptions::default());
    let result = evaluate::evaluate(&expression, &mut state, &FunctionMap::default());
    let function = match result.value {
      Some(EvaluateResultValue::Callback(function)) => function,
      value => panic!("{value:?}"),
    };
    let mut overlay = rustc_hash::FxHashMap::default();
    overlay.insert(
      Ident::new("tokens".into(), DUMMY_SP, Default::default()).to_id(),
      EvaluateResultValue::ThemeRef(ThemeRef::new("/x.stylex.js", "tokens", "x")),
    );
    let reads = Rc::new(RefCell::new(Vec::new()));
    let observed = reads.clone();
    let observer =
      Rc::new(move |_: &EvaluateResultValue, key: &str| observed.borrow_mut().push(key.to_owned()));
    let result = evaluate_function_in_scope_observed(
      &function,
      vec![],
      &mut state,
      swc_core::common::Span::new(file.end_pos, file.end_pos),
      &overlay,
      Some(observer),
    );
    match result {
      Ok(EvaluateResultValue::Expr(Expr::Lit(Lit::Num(value)))) => assert_eq!(value.value, 1.0),
      value => panic!("{value:?}"),
    };
    assert!(
      reads.borrow().is_empty(),
      "intrinsic marker must not report a variable dependency"
    );
  });
}

#[test]
fn object_coercion_executes_stored_methods_in_javascript_order() {
  for (source, expected) in [
    ("({valueOf:()=>2})+1", 3.0),
    ("({valueOf:()=>2})*3", 6.0),
    ("+({valueOf:()=>2})", 2.0),
    ("Number({valueOf:()=>2})", 2.0),
    ("Math.max({valueOf:()=>2},1)", 2.0),
    ("({valueOf:()=>({}),toString:()=> '2'})+1", 21.0),
  ] {
    let input = if source.contains("toString") {
      format!("Number({source})")
    } else {
      source.to_owned()
    };
    assert_eq!(number(&input), expected, "{source}");
  }
  let result = fold("({valueOf:()=>({}),toString:()=>({})})+1");
  assert!(
    !result.confident,
    "must refuse an object that supplies no primitive"
  );
  let result = fold("(async()=>1)()");
  assert!(!result.confident, "async arrow must not become a scalar");
}
#[test]
fn folded_result_options_bound_characters_and_entries_separately() {
  GLOBALS.set(&Globals::new(), || {
    for (source, characters, entries, allowed, expected_reason) in [
      ("'x'.repeat(4)", 4, 10, true, None),
      (
        "'x'.repeat(5)",
        4,
        10,
        false,
        Some(
          stylex_constants::constants::evaluation_errors::amplified_length_too_large(
            "repeat", 5, 5, 1, 4,
          ),
        ),
      ),
      ("Array.of(1,2,3)", 100, 3, true, None),
      (
        "Array.of(1,2,3,4)",
        100,
        3,
        false,
        Some(stylex_constants::constants::evaluation_errors::array_length_too_large(3)),
      ),
      ("'abcdef'.length", 1, 1, true, None),
    ] {
      let map = SourceMap::default();
      let file = map.new_source_file(
        FileName::Custom("limits.js".into()).into(),
        source.to_owned(),
      );
      let mut parser = Parser::new(
        Syntax::Es(EsSyntax::default()),
        StringInput::from(&*file),
        None,
      );
      let expression = match parser.parse_expr() {
        Ok(expression) => expression,
        Err(error) => panic!("{error:?}"),
      };
      let mut options = StyleXOptions::default();
      options.core.max_folded_characters = characters;
      options.core.max_folded_entries = entries;
      let mut state = StateManager::new(options);
      let result = evaluate::evaluate(&expression, &mut state, &FunctionMap::default());
      assert_eq!(result.confident, allowed, "{source}: {:?}", result.reason);
      if !allowed {
        assert_eq!(result.reason, expected_reason, "{source}");
      }
    }
  });
}
#[test]
fn intermediate_template_surrogates_remain_lossless_for_numeric_consumers() {
  assert_eq!(number("`\\uD800`.charCodeAt(0)"), 55296.0);
  assert_eq!(number("`${'😀'[0]}`.charCodeAt(0)"), 55357.0);
}

#[test]
fn array_callback_transport_shares_the_full_third_argument_linearly() {
  use std::cell::{Cell, RefCell};
  use stylex_state::{
    evaluate_result_value::EvaluateResultValue,
    functions::{FunctionConfig, FunctionConfigType, FunctionType},
    types::EvaluatedFunction,
  };
  GLOBALS.set(&Globals::new(), || {
    for size in [250usize, 500, 1000, 2000] {
      let calls = Rc::new(Cell::new(0));
      let array_identity = Rc::new(RefCell::new(None));
      let seen_calls = calls.clone();
      let seen_array = array_identity.clone();
      let invoke: Rc<stylex_state::types::CallbackOperation> = Rc::new(move |args, state| {
        let _ = state;
        seen_calls.set(seen_calls.get() + 1);
        assert_eq!(args.len(), 3, "all callback arguments must be present");
        let EvaluateResultValue::NativeArray(array) = &args[2] else {
          panic!("third argument must stay a shared native allocation")
        };
        assert_eq!(array.len(), size);
        let identity = Rc::as_ptr(array) as usize;
        let mut previous = seen_array.borrow_mut();
        if let Some(previous) = *previous {
          assert_eq!(
            identity, previous,
            "one array allocation must be shared across every callback invocation"
          );
        } else {
          *previous = Some(identity);
        }
        Ok(args[0].clone())
      });
      let delegate = invoke.clone();
      let overlay: Rc<stylex_state::types::OverlayOperation> =
        Rc::new(move |args, state, _| delegate(args, state));
      let original = Rc::new(swc_core::ecma::ast::ArrowExpr {
        span: Default::default(),
        ctxt: Default::default(),
        params: vec![],
        body: Box::new(swc_core::ecma::ast::ArrowFunctionBody::Expr(Box::new(
          stylex_ast::ast::convertors::create_number_expr(0.0),
        ))),
        is_async: false,
        is_generator: false,
        type_params: None,
        return_type: None,
      });
      let callback = EvaluateResultValue::Callback(Rc::new(EvaluatedFunction {
        original,
        invoke,
        invoke_with_overlays: overlay,
        budget: None,
      }));
      let mut functions = FunctionMap::default();
      functions.identifiers.insert(
        "record".into(),
        Box::new(FunctionConfigType::Regular(FunctionConfig {
          takes_path: false,
          fn_ptr: FunctionType::ValueMapper(Rc::new(move || callback.clone())),
        })),
      );
      let source = format!("Array.from({{length:{size}}}).map(record).length");
      let map = SourceMap::default();
      let file = map.new_source_file(FileName::Custom("linear.js".into()).into(), source);
      let mut parser = Parser::new(
        Syntax::Es(EsSyntax::default()),
        StringInput::from(&*file),
        None,
      );
      let expression = match parser.parse_expr() {
        Ok(expression) => expression,
        Err(error) => panic!("{error:?}"),
      };
      let result = evaluate::evaluate(
        &expression,
        &mut StateManager::new(StyleXOptions::default()),
        &functions,
      );
      assert!(result.confident, "{:?}", result.reason);
      match result.value.and_then(|value| value.as_expr().cloned()) {
        Some(Expr::Lit(Lit::Num(number))) => assert_eq!(number.value, size as f64),
        value => panic!("{value:?}"),
      };
      assert_eq!(calls.get(), size);
    }
  });
}
