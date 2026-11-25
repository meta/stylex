use std::mem;

use stylex_macros::stylex_panic;
use swc_core::ecma::ast::Expr;

use stylex_ast::ast::convertors::create_string_expr;
use stylex_ast::ast::factories::{create_array_expression, create_expr_or_spread};
use stylex_constants::constants::messages::EXPRESSION_IS_NOT_A_STRING;
use stylex_regex::regex::IS_CSS_VAR;
use stylex_state::resolution::convertors::convert_expr_to_str;
use stylex_state::{functions::FunctionMap, state_manager::StateManager};

/// The `var(` prefix a CSS variable reference starts with, and whose length is what the name inside it begins after.
const CSS_VAR_PREFIX: &str = "var(";

/// Return the variable referenced by a bare identifier.
pub(crate) fn css_variable_name(text: &str) -> Option<&str> {
  match IS_CSS_VAR.is_match(text).unwrap_or(false) {
    true => text
      .strip_prefix(CSS_VAR_PREFIX)
      .and_then(|inner| inner.strip_suffix(')')),
    false => None,
  }
}

/// Cuts `text` down to the variable name it references, and answers whether it referenced one.
pub(crate) fn cut_to_css_variable_name(text: &mut String) -> bool {
  if css_variable_name(text).is_none() {
    return false;
  }

  text.truncate(text.len() - 1);
  text.drain(..CSS_VAR_PREFIX.len());

  true
}

/// Plan firstThatWorks output from argument positions.
pub(crate) enum Fallbacks {
  /// No argument names a variable, so there is no chain to build and no value to strip a name out of.
  Reversed,
  /// One `var()` chain and nothing behind it, which stays a single value.
  Chain(Vec<usize>),
  /// The chain, followed by the arguments that stay declarations of their own, highest priority last.
  ChainAndRest(Vec<usize>, Vec<usize>),
}

/// How `count` arguments fall back to one another, given which of them name a variable.
pub(crate) fn plan_fallbacks(count: usize, mut is_var: impl FnMut(usize) -> bool) -> Fallbacks {
  let Some(first_var) = (0..count).find(|&index| is_var(index)) else {
    return Fallbacks::Reversed;
  };

  let end = ((first_var + 1)..count)
    .find(|&index| !is_var(index))
    .map_or(count, |first_value| first_value + 1);

  let chain = (first_var..end).rev().collect();
  let rest: Vec<usize> = (0..first_var).rev().collect();

  match rest.is_empty() {
    true => Fallbacks::Chain(chain),
    false => Fallbacks::ChainAndRest(chain, rest),
  }
}

/// The single value a fallback chain collapses to, folded from its parts in the order [`Fallbacks::chain`] lists them.
pub(crate) fn fold_fallback_chain(parts: impl IntoIterator<Item = String>) -> String {
  parts
    .into_iter()
    .fold(String::new(), |so_far, part| match so_far.is_empty() {
      false => {
        let mut next = String::with_capacity(part.len() + so_far.len() + 7);
        next.push_str(CSS_VAR_PREFIX);
        next.push_str(&part);
        next.push_str(", ");
        next.push_str(&so_far);
        next.push(')');
        next
      },
      true if part.starts_with("--") => {
        let mut next = String::with_capacity(part.len() + 5);
        next.push_str(CSS_VAR_PREFIX);
        next.push_str(&part);
        next.push(')');
        next
      },
      true => part,
    })
}

/// Each argument's text, read at most once.
struct ArgTexts<'a> {
  args: &'a [Expr],
  /// One slot per argument, filled by the first read of that argument.
  texts: Vec<Option<String>>,
  state: &'a mut StateManager,
  functions: &'a FunctionMap,
}

impl<'a> ArgTexts<'a> {
  fn new(args: &'a [Expr], state: &'a mut StateManager, functions: &'a FunctionMap) -> Self {
    ArgTexts {
      args,
      texts: vec![None; args.len()],
      state,
      functions,
    }
  }

  /// The filled slot of `args[index]`.
  fn filled(&mut self, index: usize) -> &mut String {
    let (args, state, functions) = (self.args, &mut self.state, self.functions);

    self.texts[index].get_or_insert_with(|| {
      match convert_expr_to_str(&args[index], state, functions) {
        Some(text) => text,
        None => stylex_panic!("{}", EXPRESSION_IS_NOT_A_STRING),
      }
    })
  }

  /// Take one argument's text.
  fn take_text(&mut self, index: usize) -> String {
    let text = mem::take(self.filled(index));

    self.texts[index] = None;

    text
  }
}

pub fn stylex_first_that_works(
  args: Vec<Expr>,
  state: &mut StateManager,
  functions: &FunctionMap,
) -> Expr {
  // Babel's fallback helper discards nullish entries before deciding whether to construct a CSS variable chain.
  let args = args
    .into_iter()
    .filter(|arg| {
      let arg = stylex_ast::ast::convertors::normalize_expr(arg);
      !matches!(arg, Expr::Lit(swc_core::ecma::ast::Lit::Null(_)))
        && !matches!(arg,Expr::Ident(id) if stylex_ast::ast::convertors::is_js_undefined(id))
    })
    .collect::<Vec<_>>();

  let (plan, chain_text) = {
    let mut texts = ArgTexts::new(&args, state, functions);

    let plan = plan_fallbacks(args.len(), |index| {
      css_variable_name(texts.filled(index)).is_some()
    });

    // A shape with no chain folds nothing, and the empty text it stands for is never read.
    let chain_text = match &plan {
      Fallbacks::Reversed => String::new(),
      Fallbacks::Chain(chain) | Fallbacks::ChainAndRest(chain, _) => {
        fold_fallback_chain(chain.iter().map(|&index| {
          let mut text = texts.take_text(index);

          cut_to_css_variable_name(&mut text);

          text
        }))
      },
    };

    (plan, chain_text)
  };

  match plan {
    Fallbacks::Reversed => {
      let elems = args
        .into_iter()
        .rev()
        .map(|arg| Some(create_expr_or_spread(arg)))
        .collect();

      create_array_expression(elems)
    },
    Fallbacks::Chain(_) => create_string_expr(&chain_text),
    Fallbacks::ChainAndRest(_, rest) => {
      let chain = create_string_expr(&chain_text);
      let elems = std::iter::once(chain)
        .chain(rest.iter().map(|&index| args[index].clone()))
        .map(|expr| Some(create_expr_or_spread(expr)))
        .collect();

      create_array_expression(elems)
    },
  }
}
