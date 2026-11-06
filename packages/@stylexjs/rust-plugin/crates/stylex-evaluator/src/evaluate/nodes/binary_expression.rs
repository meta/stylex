use super::super::*;
use super::logical_expression;
use anyhow::anyhow;
use stylex_js::operators::evaluate_bin_expr;
use stylex_macros::as_expr_or_err;
use swc_core::ecma::ast::{BinExpr, BinaryOp};

pub(in super::super) fn evaluate(
  bin: &BinExpr,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> Option<EvaluateResultValue> {
  if let Some(op) = logical_expression::LogicalOp::of(bin.op) {
    return logical_expression::evaluate(op, bin, state, traversal_state, fns);
  }

  let host_before = stylex_js::native::host_epoch();
  let mut left = evaluate_cached(&bin.left, state, traversal_state, fns)?;
  let mut right = evaluate_cached(&bin.right, state, traversal_state, fns)?;
  if matches!(
    bin.op,
    BinaryOp::EqEq | BinaryOp::NotEq | BinaryOp::EqEqEq | BinaryOp::NotEqEq
  ) {
    let reference_value = |value: &EvaluateResultValue| {
      matches!(
        value,
        EvaluateResultValue::Object(_)
          | EvaluateResultValue::Vec(_)
          | EvaluateResultValue::SharedArray(_)
          | EvaluateResultValue::NativeArray(_)
          | EvaluateResultValue::Callback(_)
          | EvaluateResultValue::ThemeRef(_)
          | EvaluateResultValue::Expr(Expr::Object(_) | Expr::Array(_))
      )
    };
    if reference_value(&left) && reference_value(&right) {
      let fresh = |expression: &Expr| {
        matches!(
          normalize_expr(expression),
          Expr::Object(_) | Expr::Array(_) | Expr::Arrow(_)
        )
      };
      let distinct_fresh = fresh(&bin.left) && fresh(&bin.right);
      let same_binding = match (normalize_expr(&bin.left), normalize_expr(&bin.right)) {
        (Expr::Ident(left), Expr::Ident(right)) if left.to_id() == right.to_id() => {
          fns.bindings.contains_key(&left.to_id())
            || (traversal_state.declaration_of(left).is_some()
              && !fns.identifiers.contains_key(&left.sym))
        },
        _ => false,
      };
      if distinct_fresh || same_binding {
        let equal = same_binding;
        let value = if matches!(bin.op, BinaryOp::NotEq | BinaryOp::NotEqEq) {
          !equal
        } else {
          equal
        };
        return Some(EvaluateResultValue::Expr(create_bool_expr(value)));
      }
    }
  }
  let strict = matches!(
    bin.op,
    BinaryOp::EqEqEq | BinaryOp::NotEqEq | BinaryOp::In | BinaryOp::InstanceOf
  );
  let loose = matches!(bin.op, BinaryOp::EqEq | BinaryOp::NotEq);
  let reference = |value: &EvaluateResultValue| {
    matches!(
      value,
      EvaluateResultValue::Object(_)
        | EvaluateResultValue::Vec(_)
        | EvaluateResultValue::SharedArray(_)
        | EvaluateResultValue::NativeArray(_)
        | EvaluateResultValue::ThemeRef(_)
        | EvaluateResultValue::Expr(Expr::Object(_) | Expr::Array(_))
    )
  };
  if !(strict || loose && reference(&left) && reference(&right)) {
    let hint = if bin.op == BinaryOp::Add || loose {
      stylex_structures::stylex_env::PrimitiveHint::Default
    } else {
      stylex_structures::stylex_env::PrimitiveHint::Number
    };
    if reference(&left) {
      left = match super::super::primitive::primitive(&left, hint, traversal_state, bin.span) {
        Ok(value) => value,
        Err(error) => return super::super::callback_failure::invocation_failure(error, state),
      };
    }
    if reference(&right) {
      right = match super::super::primitive::primitive(&right, hint, traversal_state, bin.span) {
        Ok(value) => value,
        Err(error) => return super::super::callback_failure::invocation_failure(error, state),
      };
    }
  }
  if matches!(left, EvaluateResultValue::Host(_)) || matches!(right, EvaluateResultValue::Host(_)) {
    return match super::super::host_operators::binary_values(bin.op, &left, &right, traversal_state)
    {
      Ok(value) => Some(value),
      Err(error) => deopt(&Expr::Bin(bin.clone()), state, &error.message),
    };
  }
  if let Some(value) =
    super::super::native_call::primitive_binary(bin.op, &left, &right, traversal_state)
  {
    return Some(value);
  }
  if stylex_js::native::host_epoch() != host_before {
    return deopt(
      &Expr::Bin(bin.clone()),
      state,
      &unsupported_expression("BinaryExpression"),
    );
  }
  folded_expr(fold_binary_expr(bin, state, traversal_state, fns)).map(EvaluateResultValue::Expr)
}

/// The expression a folded binary is written as, or `None` for a fold that answered nothing at all.
fn folded_expr(folded: BinaryExprType) -> Option<Expr> {
  match folded {
    BinaryExprType::Number(number) => Some(create_number_expr(number)),
    BinaryExprType::Boolean(value) => Some(create_bool_expr(value)),
    BinaryExprType::String { text, .. } => Some(create_string_expr(&text)),
    BinaryExprType::Null => None,
  }
}

/// `ToNumber` over a boolean: the number a comparison result becomes when the surrounding expression is being evaluated as a number.
#[inline]
fn convert_bool_to_number(value: bool) -> f64 {
  if value { 1.0 } else { 0.0 }
}

/// Test whether an evaluated operand is a string before coercion.
#[inline]
fn is_string(expr: &Expr) -> bool {
  matches!(expr, Expr::Lit(Lit::Str(_)))
}

/// One side of a `+`, reduced the way the operator reads it: its own value where it is already a primitive, and the value its own conversion method answers where it is an object.
fn reduced_addend(value: EvaluateResultValue) -> EvaluateResultValue {
  let reduced = match value.as_expr() {
    Some(object @ Expr::Object(_)) => coercions::to_js_default_primitive(object).cloned(),
    _ => None,
  };

  match reduced {
    Some(primitive) => EvaluateResultValue::Expr(primitive),
    None => value,
  }
}

/// The reason each side records when it cannot be read as what the path it is on needs.
const LEFT_NOT_A_NUMBER: &str = "Left expression is not a number";
const RIGHT_NOT_A_NUMBER: &str = "Right expression is not a number";
const LEFT_NOT_A_STRING: &str = "Left expression is not a string";
const RIGHT_NOT_A_STRING: &str = "Right expression is not a string";

const LEFT_HAS_NO_VALUE: &str = "Left expression could not be evaluated";
const RIGHT_HAS_NO_VALUE: &str = "Right expression could not be evaluated";

/// One side of the expression, evaluated.
fn evaluate_operand(
  operand: &Expr,
  reason: &str,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> Result<EvaluateResultValue, anyhow::Error> {
  evaluate_cached(operand, state, traversal_state, fns).ok_or_else(|| anyhow!("{}", reason))
}

/// The left operand of a binary expression, as much of it as the level above can use.
enum LeftOperand {
  /// A concatenation this same path folded one level down, still carrying the count it was measured to.
  Measured { text: String, units: usize },
  /// Every other operand.
  Value(EvaluateResultValue),
}

/// A measured left side is remembered as the string literal it spells, which is all the tree can hold, and read back as an ordinary value -- so a chain answered from the memo is measured where it lands, exactly as it was before the count started travelling.
impl Memoized for LeftOperand {
  fn from_memo(remembered: EvaluateResultValue) -> Self {
    Self::Value(remembered)
  }

  fn to_memo(&self) -> EvaluateResultValue {
    match self {
      Self::Measured { text, .. } => EvaluateResultValue::Expr(create_string_expr(text)),
      Self::Value(value) => value.clone(),
    }
  }
}

/// Evaluate a left addend while retaining a concatenation's measured length.
fn evaluate_left_operand(
  binary_expr: &BinExpr,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> Result<LeftOperand, anyhow::Error> {
  if let Expr::Bin(inner) = normalize_expr(&binary_expr.left)
    && matches!(inner.op, BinaryOp::Add)
  {
    let folded = folded_once(
      &binary_expr.left,
      state,
      traversal_state,
      |state, traversal_state| match fold_binary_expr(inner, state, traversal_state, fns) {
        BinaryExprType::String { text, units } => Some(LeftOperand::Measured { text, units }),
        folded => {
          folded_expr(folded).map(|expr| LeftOperand::Value(EvaluateResultValue::Expr(expr)))
        },
      },
    );

    return folded.ok_or_else(|| anyhow!("{}", LEFT_HAS_NO_VALUE));
  }

  evaluate_operand(
    &binary_expr.left,
    LEFT_HAS_NO_VALUE,
    state,
    traversal_state,
    fns,
  )
  .map(LeftOperand::Value)
}

/// How one of the four equality operators compares its two sides.
#[derive(Clone, Copy)]
enum EqualityReading {
  /// `==`, which coerces before it compares.
  Loose,
  /// `===`, which compares the type before the value.
  Strict,
  /// `!=` and `!==`, which answer the negation of `===`.
  NotStrict,
}

impl EqualityReading {
  /// What the operator answers for these two primitives.
  fn answers(self, left: &coercions::Primitive, right: &coercions::Primitive) -> bool {
    match self {
      Self::Loose => coercions::loose_equals(left, right),
      Self::Strict => coercions::strict_equals(left, right),
      Self::NotStrict => !coercions::strict_equals(left, right),
    }
  }
}

/// How each of the four relational operators reads `IsLessThan`.
#[derive(Clone, Copy)]
enum RelationalReading {
  Lt,
  LtEq,
  Gt,
  GtEq,
}

impl RelationalReading {
  /// What the operator answers for these two primitives, or `None` for a text this compiler cannot read and so cannot order.
  fn answers(self, left: &coercions::Primitive, right: &coercions::Primitive) -> Option<bool> {
    let (first, second, negated) = match self {
      Self::Lt => (left, right, false),
      Self::Gt => (right, left, false),
      Self::LtEq => (right, left, true),
      Self::GtEq => (left, right, true),
    };

    match coercions::js_less_than(first, second)? {
      Some(less) => Some(less != negated),

      None => Some(false),
    }
  }
}

/// How one of the eight comparison operators reads its two sides.
#[derive(Clone, Copy)]
enum ComparisonReading {
  Equality(EqualityReading),
  Relational(RelationalReading),
}

impl ComparisonReading {
  /// The reading `op` takes, or `None` for every other operator.
  fn of(op: BinaryOp) -> Option<Self> {
    match op {
      BinaryOp::EqEq => Some(Self::Equality(EqualityReading::Loose)),
      BinaryOp::EqEqEq => Some(Self::Equality(EqualityReading::Strict)),
      BinaryOp::NotEq | BinaryOp::NotEqEq => Some(Self::Equality(EqualityReading::NotStrict)),
      BinaryOp::Lt => Some(Self::Relational(RelationalReading::Lt)),
      BinaryOp::LtEq => Some(Self::Relational(RelationalReading::LtEq)),
      BinaryOp::Gt => Some(Self::Relational(RelationalReading::Gt)),
      BinaryOp::GtEq => Some(Self::Relational(RelationalReading::GtEq)),
      _ => None,
    }
  }

  /// What the operator answers for these two primitives, or `None` where the pair has no answer this compiler can give.
  fn answers(self, left: &coercions::Primitive, right: &coercions::Primitive) -> Option<bool> {
    match self {
      Self::Equality(reading) => Some(reading.answers(left, right)),
      Self::Relational(reading) => reading.answers(left, right),
    }
  }
}

/// The primitive an already-evaluated operand is, where it is one.
fn primitive_of(value: &EvaluateResultValue) -> Option<coercions::Primitive<'_>> {
  value.as_expr().and_then(coercions::to_js_primitive)
}

/// Fold numeric or string binary operations.
fn fold_binary_expr(
  binary_expr: &BinExpr,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> BinaryExprType {
  binary_expr_to_num_or_str(binary_expr, state, traversal_state, fns).unwrap_or_else(|num_error| {
    binary_expr_to_string(binary_expr, state, traversal_state, fns).unwrap_or_else(|str_error| {
      debug!("Binary expression to string error: {}", str_error);
      debug!("Binary expression to number error: {}", num_error);

      BinaryExprType::Null
    })
  })
}

/// Whether `op` applies `ToNumber` to each side.
fn reads_each_side_as_a_number(op: BinaryOp) -> bool {
  matches!(
    op,
    BinaryOp::Sub
      | BinaryOp::Mul
      | BinaryOp::Div
      | BinaryOp::Mod
      | BinaryOp::Exp
      | BinaryOp::LShift
      | BinaryOp::RShift
      | BinaryOp::ZeroFillRShift
      | BinaryOp::BitAnd
      | BinaryOp::BitOr
      | BinaryOp::BitXor
  )
}

/// The number of one evaluated side of `op`.
fn side_number(
  op: BinaryOp,
  side: &EvaluateResultValue,
  not_an_expression: &'static str,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> Result<f64, anyhow::Error> {
  if !reads_each_side_as_a_number(op) {
    let expr = as_expr_or_err!(side, not_an_expression);
    return expr_to_num(expr, state, traversal_state, fns);
  }

  operand_to_js_number(side, state, traversal_state, fns)
    .map_err(|refusal| anyhow!(refusal.reason(traversal_state.character_ceiling())))
}

/// Every binary operator but the three logical ones, folded to whichever of a number and a string its operands decide.
pub(crate) fn binary_expr_to_num_or_str(
  binary_expr: &BinExpr,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> Result<BinaryExprType, anyhow::Error> {
  let op = binary_expr.op;

  // Asked after the numeric coercion instead, `'1' + 2` would answer `3` — both sides coerce, so the number path would never yield to the string one.
  let mut evaluated_right = None;

  let left = match op {
    BinaryOp::Add => {
      let left = evaluate_left_operand(binary_expr, state, traversal_state, fns)?;

      let right = reduced_addend(evaluate_operand(
        &binary_expr.right,
        RIGHT_HAS_NO_VALUE,
        state,
        traversal_state,
        fns,
      )?);

      let left = match left {
        LeftOperand::Value(value) => LeftOperand::Value(reduced_addend(value)),
        measured => measured,
      };

      match left {
        LeftOperand::Value(value)
          if !value.as_expr().is_some_and(is_string) && !right.as_expr().is_some_and(is_string) =>
        {
          evaluated_right = Some(right);

          value
        },
        concatenating => {
          return concatenate(binary_expr, concatenating, &right, state, traversal_state);
        },
      }
    },
    _ => evaluate_operand(
      &binary_expr.left,
      LEFT_NOT_A_NUMBER,
      state,
      traversal_state,
      fns,
    )?,
  };

  if let Some(reading) = ComparisonReading::of(op)
    && let Some(left_value) = primitive_of(&left)
  {
    let right = evaluate_operand(
      &binary_expr.right,
      RIGHT_NOT_A_NUMBER,
      state,
      traversal_state,
      fns,
    )?;

    if let Some(right_value) = primitive_of(&right)
      && let Some(answer) = reading.answers(&left_value, &right_value)
    {
      return Result::Ok(BinaryExprType::Boolean(answer));
    }

    evaluated_right = Some(right);
  }

  let left_num = side_number(
    op,
    &left,
    "Left argument not expression",
    state,
    traversal_state,
    fns,
  )?;

  let right = match evaluated_right {
    Some(right) => right,
    None => evaluate_operand(
      &binary_expr.right,
      RIGHT_NOT_A_NUMBER,
      state,
      traversal_state,
      fns,
    )?,
  };
  let right_num = side_number(
    op,
    &right,
    "Right argument not expression",
    state,
    traversal_state,
    fns,
  )?;

  let result = match &op {
    BinaryOp::Add
    | BinaryOp::Sub
    | BinaryOp::Mul
    | BinaryOp::Div
    | BinaryOp::Mod
    | BinaryOp::Exp
    | BinaryOp::RShift
    | BinaryOp::LShift
    | BinaryOp::BitAnd
    | BinaryOp::BitOr
    | BinaryOp::BitXor
    | BinaryOp::ZeroFillRShift => evaluate_bin_expr(op, left_num, right_num),

    BinaryOp::In => convert_bool_to_number(right_num == 0.0),
    BinaryOp::InstanceOf => convert_bool_to_number(right_num == 0.0),

    BinaryOp::LogicalOr
    | BinaryOp::LogicalAnd
    | BinaryOp::NullishCoalescing
    | BinaryOp::EqEq
    | BinaryOp::NotEq
    | BinaryOp::EqEqEq
    | BinaryOp::NotEqEq
    | BinaryOp::Lt
    | BinaryOp::LtEq
    | BinaryOp::Gt
    | BinaryOp::GtEq => {
      return Result::Err(anyhow!(unsupported_operator(op.as_str())));
    },
  };

  Result::Ok(BinaryExprType::Number(result))
}

/// `+` over operands at least one of which is a string: `ToString` of each side, joined.
fn binary_expr_to_string(
  binary_expr: &BinExpr,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> Result<BinaryExprType, anyhow::Error> {
  let op = binary_expr.op;

  if !matches!(op, BinaryOp::Add) {
    return Result::Err(anyhow!(
      "For string expressions, only addition is supported, got {:?}",
      op
    ));
  }

  let left = evaluate_operand(
    &binary_expr.left,
    LEFT_NOT_A_STRING,
    state,
    traversal_state,
    fns,
  )
  .map(LeftOperand::Value)?;
  let right = evaluate_operand(
    &binary_expr.right,
    RIGHT_NOT_A_STRING,
    state,
    traversal_state,
    fns,
  )?;

  concatenate(binary_expr, left, &right, state, traversal_state)
}

/// Two evaluated operands of a `+`, joined through one measured buffer.
fn concatenate(
  binary_expr: &BinExpr,
  left: LeftOperand,
  right: &EvaluateResultValue,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
) -> Result<BinaryExprType, anyhow::Error> {
  let refusal = |reason: &'static str| {
    move |refused: StringAppend| match refused {
      StringAppend::NoStringForm => anyhow!("{}", reason),
      StringAppend::TooLarge(sentence) => anyhow!("{}", sentence),
    }
  };
  let path = || Expr::Bin(binary_expr.clone());

  let mut joined = match left {
    LeftOperand::Measured { text, units } => GrownString::adopt(text, units, CONCATENATION),
    LeftOperand::Value(value) => {
      let mut joined = GrownString::new(CONCATENATION);

      joined
        .push_string_of(&value, path, state, traversal_state)
        .map_err(refusal(LEFT_NOT_A_STRING))?;

      joined
    },
  };

  joined
    .push_string_of(right, path, state, traversal_state)
    .map_err(refusal(RIGHT_NOT_A_STRING))?;

  let (text, units) = joined.into_measured();

  Result::Ok(BinaryExprType::String { text, units })
}

#[cfg(test)]
#[path = "tests/binary_expression_tests.rs"]
mod tests;
