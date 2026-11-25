#[cfg(test)]
mod stylex_first_that_works {
  use std::{cell::Cell, rc::Rc};

  use stylex_ast::ast::convertors::{create_ident_expr, create_string_expr};

  use crate::stylex_first_that_works::stylex_first_that_works;
  use stylex_ast::ast::factories::create_object_lit;
  use stylex_state::{
    functions::{FunctionConfig, FunctionConfigType, FunctionMap, FunctionType},
    state_manager::StateManager,
  };

  #[test]
  fn an_argument_is_read_to_text_once() {
    let reads = Rc::new(Cell::new(0_usize));
    let counted = Rc::clone(&reads);

    let mut functions = FunctionMap::default();

    functions.identifiers.insert(
      "accent".into(),
      Box::new(FunctionConfigType::Regular(FunctionConfig {
        fn_ptr: FunctionType::Mapper(Rc::new(move || {
          counted.set(counted.get() + 1);
          create_string_expr("var(--accent)")
        })),
        takes_path: false,
      })),
    );

    let result = stylex_first_that_works(
      vec![create_ident_expr("accent"), create_string_expr("blue")],
      &mut StateManager::default(),
      &functions,
    );

    assert_eq!(result, create_string_expr("var(--accent, blue)"));
    assert_eq!(reads.get(), 1);
  }

  #[test]
  #[should_panic(expected = "Expected a string value but received a non-string expression.")]
  fn panics_for_an_argument_with_no_string() {
    stylex_first_that_works(
      vec![create_string_expr("red"), create_object_lit(vec![]).into()],
      &mut StateManager::default(),
      &FunctionMap::default(),
    );
  }
}

#[cfg(test)]
mod fallback_plan {}
