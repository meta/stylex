use swc_core::ecma::ast::{BindingIdent, Expr, Pat, VarDeclarator};

use crate::resolution::lookup::get_var_decl_by_ident;
use crate::state_writers::fill_state_declarations;
use crate::tests::prelude::make_var_declarator;
use crate::{functions::FunctionMap, state_manager::StateManager};
use stylex_ast::ast::convertors::create_number_expr;
use stylex_ast::ast::factories::create_ident;

// ──────────────────────────────────────────────
// get_var_decl_by_ident - Increase and None actions
// ──────────────────────────────────────────────

mod get_var_decl_by_ident_tests {
  use super::*;

  #[test]
  fn returns_var_decl_for_known_ident() {
    let mut state = StateManager::default();
    let fns = FunctionMap::default();
    let decl = make_var_declarator("x", create_number_expr(10.0));
    fill_state_declarations(&mut state, &decl);
    let ident = create_ident("x");
    let result = get_var_decl_by_ident(&ident, &mut state, &fns);
    assert!(result.is_some());
  }

  #[test]
  fn returns_none_for_unknown_ident() {
    let mut state = StateManager::default();
    let fns = FunctionMap::default();
    let ident = create_ident("nonexistent");
    let result = get_var_decl_by_ident(&ident, &mut state, &fns);
    assert!(result.is_none());
  }
}

// ──────────────────────────────────────────────
// get_var_decl_by_ident - FunctionMap branches
// ──────────────────────────────────────────────

mod get_var_decl_by_ident_function_map_tests {
  use super::*;
  use crate::functions::{FunctionConfig, FunctionConfigType, FunctionMap, FunctionType};
  use std::rc::Rc;

  #[test]
  fn returns_var_decl_from_mapper_function() {
    let mut state = StateManager::default();
    let mut fns = FunctionMap::default();
    let mapper: Rc<dyn Fn() -> Expr + 'static> = Rc::new(|| create_number_expr(99.0));
    fns.identifiers.insert(
      "myMapper".into(),
      Box::new(FunctionConfigType::Regular(FunctionConfig {
        fn_ptr: FunctionType::Mapper(mapper),
        takes_path: false,
      })),
    );
    let ident = create_ident("myMapper");
    let result = get_var_decl_by_ident(&ident, &mut state, &fns);
    assert!(result.is_some());
  }

  #[test]
  fn returns_none_for_env_object() {
    use indexmap::IndexMap;
    let mut state = StateManager::default();
    let mut fns = FunctionMap::default();
    fns.identifiers.insert(
      "envObj".into(),
      Box::new(FunctionConfigType::EnvObject(IndexMap::new().into())),
    );
    let ident = create_ident("envObj");
    let result = get_var_decl_by_ident(&ident, &mut state, &fns);
    assert!(result.is_none());
  }
}

// ──────────────────────────────────────────────
// get_var_decl_by_ident - FunctionMap panic branches
// ──────────────────────────────────────────────

mod get_var_decl_by_ident_fn_map_panic_tests {
  use super::*;
  use crate::functions::{FunctionConfig, FunctionConfigType, FunctionMap, FunctionType};
  use std::rc::Rc;

  /// The three arms below refuse for three different reasons, so each asserts
  /// the message it is refused with: a bare `#[should_panic]` passes on any
  /// panic and cannot tell one arm from another.
  #[test]
  #[should_panic(expected = "Function type not supported:")]
  fn panics_for_non_mapper_regular_function() {
    let mut state = StateManager::default();
    let mut fns = FunctionMap::default();
    fn dummy_fn(
      _args: Vec<Expr>,
      _state: &mut StateManager,
      _fns: &crate::functions::FunctionMap,
    ) -> Expr {
      create_number_expr(0.0)
    }
    fns.identifiers.insert(
      "arrFn".into(),
      Box::new(FunctionConfigType::Regular(FunctionConfig {
        fn_ptr: FunctionType::ArrayArgs(dummy_fn),
        takes_path: false,
      })),
    );
    let ident = create_ident("arrFn");
    get_var_decl_by_ident(&ident, &mut state, &fns);
  }

  #[test]
  #[should_panic(expected = "Map values are not supported in this context.")]
  fn panics_for_map_function_config() {
    let mut state = StateManager::default();
    let mut fns = FunctionMap::default();
    fns.identifiers.insert(
      "mapFn".into(),
      Box::new(FunctionConfigType::Map(Default::default())),
    );
    let ident = create_ident("mapFn");
    get_var_decl_by_ident(&ident, &mut state, &fns);
  }

  #[test]
  #[should_panic(expected = "IndexMap values are not supported in this context.")]
  fn panics_for_indexmap_function_config() {
    let mut state = StateManager::default();
    let mut fns = FunctionMap::default();
    fns.identifiers.insert(
      "imapFn".into(),
      Box::new(FunctionConfigType::IndexMap(Rc::new(Default::default()))),
    );
    let ident = create_ident("imapFn");
    get_var_decl_by_ident(&ident, &mut state, &fns);
  }
}

// ──────────────────────────────────────────────
// get_var_decl_parts_by_ident
// ──────────────────────────────────────────────

mod get_var_decl_parts_by_ident_tests {
  use super::*;
  use crate::functions::{FunctionConfig, FunctionConfigType, FunctionType};
  use crate::resolution::lookup::get_var_decl_parts_by_ident;
  use std::rc::Rc;
  use swc_core::common::{BytePos, Span};
  use swc_core::ecma::ast::Lit;

  /// A declarator whose span is distinguishable from `DUMMY_SP`, so a test can
  /// tell the span it asked for from the one every synthesized node carries.
  fn make_spanned_declarator(name: &str, init: Expr, lo: u32, hi: u32) -> VarDeclarator {
    VarDeclarator {
      span: Span::new(BytePos(lo), BytePos(hi)),
      name: Pat::Ident(BindingIdent {
        id: create_ident(name),
        type_ann: None,
      }),
      init: Some(Box::new(init)),
      definite: false,
    }
  }

  #[test]
  fn answers_the_span_and_the_initializer_the_state_recorded() {
    let mut state = StateManager::default();
    let fns = FunctionMap::default();
    let decl = make_spanned_declarator("x", create_number_expr(7.0), 10, 20);
    fill_state_declarations(&mut state, &decl);

    let (span, init) = get_var_decl_parts_by_ident(&create_ident("x"), &mut state, &fns)
      .expect("Expected the recorded declaration");

    assert_eq!(span.lo, BytePos(10));
    assert_eq!(span.hi, BytePos(20));
    match init.as_deref() {
      Some(Expr::Lit(Lit::Num(number))) => assert_eq!(number.value, 7.0),
      other => panic!("Expected the recorded initializer, got {:?}", other),
    }
  }

  #[test]
  fn answers_none_where_nothing_binds_the_name() {
    let mut state = StateManager::default();
    let fns = FunctionMap::default();

    assert!(get_var_decl_parts_by_ident(&create_ident("absent"), &mut state, &fns).is_none());
  }

  #[test]
  fn falls_through_to_the_declarator_the_function_map_builds() {
    let mut state = StateManager::default();
    let mut fns = FunctionMap::default();
    let mapper: Rc<dyn Fn() -> Expr + 'static> = Rc::new(|| create_number_expr(99.0));
    fns.identifiers.insert(
      "mapped".into(),
      Box::new(FunctionConfigType::Regular(FunctionConfig {
        fn_ptr: FunctionType::Mapper(mapper),
        takes_path: false,
      })),
    );

    let (_, init) = get_var_decl_parts_by_ident(&create_ident("mapped"), &mut state, &fns)
      .expect("Expected the synthesized declaration");

    match init.as_deref() {
      Some(Expr::Lit(Lit::Num(number))) => assert_eq!(number.value, 99.0),
      other => panic!("Expected the mapped initializer, got {:?}", other),
    }
  }

  #[test]
  fn a_recorded_declaration_wins_over_a_function_of_the_same_name() {
    // The state hit is probed first, and the two answers differ, so which one
    // comes back says which path ran.
    let mut state = StateManager::default();
    let mut fns = FunctionMap::default();
    fill_state_declarations(
      &mut state,
      &make_spanned_declarator("both", create_number_expr(1.0), 30, 40),
    );
    let mapper: Rc<dyn Fn() -> Expr + 'static> = Rc::new(|| create_number_expr(2.0));
    fns.identifiers.insert(
      "both".into(),
      Box::new(FunctionConfigType::Regular(FunctionConfig {
        fn_ptr: FunctionType::Mapper(mapper),
        takes_path: false,
      })),
    );

    let (span, init) = get_var_decl_parts_by_ident(&create_ident("both"), &mut state, &fns)
      .expect("Expected the recorded declaration");

    assert_eq!(span.lo, BytePos(30));
    match init.as_deref() {
      Some(Expr::Lit(Lit::Num(number))) => assert_eq!(number.value, 1.0),
      other => panic!("Expected the recorded initializer, got {:?}", other),
    }
  }
}
