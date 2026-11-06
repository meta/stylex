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
  // The reference implementation keeps `||`, `&&` and `??` on a node of their
  // own, evaluated over values and tested ahead of this one. SWC folds all
  // three into `BinExpr`, so the split happens here instead — before either of
  // the paths below, which coerce their operands and so cannot answer for an
  // operator that returns one of them unchanged.
  if let Some(op) = logical_expression::LogicalOp::of(bin.op) {
    return logical_expression::evaluate(op, bin, state, traversal_state, fns);
  }

  folded_expr(fold_binary_expr(bin, state, traversal_state, fns)).map(EvaluateResultValue::Expr)
}

/// The expression a folded binary is written as, or `None` for a fold that
/// answered nothing at all.
///
/// One reading for the two places that ask, so the spelling of a folded value
/// cannot come to differ between the whole expression and the left side of a
/// `+` that carries it.
fn folded_expr(folded: BinaryExprType) -> Option<Expr> {
  match folded {
    BinaryExprType::Number(number) => Some(create_number_expr(number)),
    BinaryExprType::Boolean(value) => Some(create_bool_expr(value)),
    BinaryExprType::String { text, .. } => Some(create_string_expr(&text)),
    BinaryExprType::Null => None,
  }
}

/// `ToNumber` over a boolean: the number a comparison result becomes when the
/// surrounding expression is being evaluated as a number.
#[inline]
fn convert_bool_to_number(value: bool) -> f64 {
  if value { 1.0 } else { 0.0 }
}

/// Whether an already-evaluated operand *is* a string, rather than whether it
/// converts to one. An evaluated string always arrives as a string literal, so
/// nothing else can answer yes here — a number does not become a string by
/// having a spelling.
#[inline]
fn is_string(expr: &Expr) -> bool {
  matches!(expr, Expr::Lit(Lit::Str(_)))
}

/// One side of a `+`, reduced the way the operator reads it: its own value
/// where it is already a primitive, and the value its own conversion method
/// answers where it is an object.
///
/// `+` is the one operator that applies `ToPrimitive` with no hint, which asks
/// an object for its `valueOf` before its `toString` -- so
/// `({ valueOf: () => 2 }) + 1` is `3` rather than the concatenation of the
/// default text. Every other operator either compares the two values as they
/// stand, where reducing would make `({ valueOf: () => 1 }) === 1` true, or
/// coerces each side to a number through a bridge that applies the same
/// reduction itself.
///
/// An object that keeps the `Object.prototype` pair reduces to itself here and
/// reaches the string path, which writes the default text it converts to.
///
/// Rebuilt rather than borrowed, because the reduction borrows from the value
/// it reduces -- and only for an object, which is the one value the two
/// readings can part company over. Every other value is handed back exactly as
/// it arrived.
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

/// The reason each side records when it cannot be read as what the path it is
/// on needs. Named rather than built at the call site so the operand helpers
/// stay at one argument per thing they actually need.
const LEFT_NOT_A_NUMBER: &str = "Left expression is not a number";
const RIGHT_NOT_A_NUMBER: &str = "Right expression is not a number";
const LEFT_NOT_A_STRING: &str = "Left expression is not a string";
const RIGHT_NOT_A_STRING: &str = "Right expression is not a string";

/// The reasons the `+` dispatch records, which runs before either path has
/// claimed the operator and so cannot borrow either one's wording: an operand
/// of `+` is on its way to concatenation as readily as to addition, and naming
/// it a failed number would describe a coercion nothing was about to perform.
const LEFT_HAS_NO_VALUE: &str = "Left expression could not be evaluated";
const RIGHT_HAS_NO_VALUE: &str = "Right expression could not be evaluated";

/// One side of the expression, evaluated.
///
/// An operand that answered nothing refuses here, whether or not the walk is
/// still confident, and the reason names the side.
///
/// One answer rather than two, because the two would be the same answer: an
/// operand that recorded its own refusal and one that came back empty out of
/// the memo both leave this path with nothing to coerce. Before this the second
/// stopped the build with a panic, where the first deopted.
fn evaluate_operand(
  operand: &Expr,
  reason: &str,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> Result<EvaluateResultValue, anyhow::Error> {
  evaluate_cached(operand, state, traversal_state, fns).ok_or_else(|| anyhow!("{}", reason))
}

/// The left operand of a binary expression, as much of it as the level above
/// can use.
///
/// A `+` whose left side is another `+` is the one shape that carries more than
/// a value: the text below was measured while it was grown, and the level above
/// appends to it. Every other operand is just the value the evaluator resolved.
enum LeftOperand {
  /// A concatenation this same path folded one level down, still carrying the
  /// count it was measured to.
  Measured { text: String, units: usize },
  /// Every other operand.
  Value(EvaluateResultValue),
}

/// A measured left side is remembered as the string literal it spells, which is
/// all the tree can hold, and read back as an ordinary value -- so a chain
/// answered from the memo is measured where it lands, exactly as it was before
/// the count started travelling.
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

/// The left side of a `+`, evaluated -- folding a `+` chain here rather than
/// through the evaluator's own dispatch, so a concatenation keeps the count it
/// was measured to.
///
/// Only `+` reaches this. Every other operator calls [`evaluate_operand`]
/// directly, because only `+` can arrive with a measured left side.
///
/// The dispatch hands a folded `+` back as a plain string literal, which has
/// nowhere to carry a length. Measured again one level up, and copied into a
/// fresh buffer beside it, a chain spends the length of everything already
/// joined once per remaining link -- the square of its text rather than its
/// length. Folded here, the buffer and the count travel together and each
/// operand is read exactly once.
///
/// It is the *same* level of the ceiling and the *same* memo either way, which
/// is what [`folded_once`] is for: a chain refuses at the link it always
/// refused at, and a subtree written twice still answers from the first
/// reading. Only the measurement is extra, and only on the way down -- a
/// remembered answer is a literal like any other, and is measured.
///
/// The mutation check the dispatch makes is not repeated: it asks about an
/// assignment, an update and a `delete`, and this arm has already matched a
/// binary expression.
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
        // The measured text is adopted rather than re-read, which is what the
        // count on it is for. Every other answer is read as the value it is.
        BinaryExprType::String { text, units } => Some(LeftOperand::Measured { text, units }),
        folded => {
          folded_expr(folded).map(|expr| LeftOperand::Value(EvaluateResultValue::Expr(expr)))
        },
      },
    );

    // The same sentence the plain reading below records, because both answers
    // are the same thing: the left side of a `+` had no value.
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
///
/// Three readings for four operators, because `!=` and `!==` take the same one.
/// That is the one row the reference implementation writes differently from the
/// language: it reads `!=` as `!==`, so the two answers differ on exactly the
/// comparisons `==` coerces. This compiler follows the compiler an author's
/// stylesheet is compared against rather than the specification.
#[derive(Clone, Copy)]
enum EqualityReading {
  /// `==`, which coerces before it compares.
  Loose,
  /// `===`, which compares the type before the value.
  Strict,
  /// `!=` and `!==`, which answer the negation of `===`.
  ///
  /// `!=` sits with the strict pair on purpose. The language reads it as the
  /// negation of `==`, so `1 != '1'` is false there; the reference
  /// implementation answers true, and so does this. Measured on the three pairs
  /// the two readings part on -- `1 != '1'`, `null != undefined` and `0 != ''`
  /// -- and the reference implementation takes the strict answer for all three.
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
///
/// Three degrees of freedom rather than four cases: which side goes first, and
/// whether the answer is negated. `x <= y` is `!(y < x)`, and the `undefined`
/// a `NaN` operand produces is false under all four -- negation included.
#[derive(Clone, Copy)]
enum RelationalReading {
  Lt,
  LtEq,
  Gt,
  GtEq,
}

impl RelationalReading {
  /// What the operator answers for these two primitives, or `None` for a text
  /// this compiler cannot read and so cannot order.
  fn answers(self, left: &coercions::Primitive, right: &coercions::Primitive) -> Option<bool> {
    let (first, second, negated) = match self {
      Self::Lt => (left, right, false),
      Self::Gt => (right, left, false),
      Self::LtEq => (right, left, true),
      Self::GtEq => (left, right, true),
    };

    match coercions::js_less_than(first, second)? {
      Some(less) => Some(less != negated),
      // The `undefined` a `NaN` operand produces, which is false under all four
      // operators -- so the negation does not reach it.
      None => Some(false),
    }
  }
}

/// How one of the eight comparison operators reads its two sides.
///
/// One reading rather than two, because both halves want the same thing of the
/// operands: two primitives, compared as the language compares them rather than
/// as two numbers. Read as two numbers instead, `'10' < '9'` answered `false`
/// where the language answers `true`, exactly as `1 != '1'` did before the
/// equality pair was moved here.
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

  /// What the operator answers for these two primitives, or `None` where the
  /// pair has no answer this compiler can give.
  fn answers(self, left: &coercions::Primitive, right: &coercions::Primitive) -> Option<bool> {
    match self {
      Self::Equality(reading) => Some(reading.answers(left, right)),
      Self::Relational(reading) => reading.answers(left, right),
    }
  }
}

/// The primitive an already-evaluated operand is, where it is one.
///
/// `None` covers both ways a side can fail to be one: a value with no
/// expression form at all, and an expression the language compares by
/// reference. Neither has an answer this evaluator can give, and both fall to
/// the numeric coercion, which names what it could not read.
fn primitive_of(value: &EvaluateResultValue) -> Option<coercions::Primitive<'_>> {
  value.as_expr().and_then(coercions::to_js_primitive)
}

/// A binary expression folded to its value rather than to an expression: the
/// number-or-string path, falling back to the string path.
///
/// The same two-step [`evaluate`] performs, minus the boxing back into the
/// tree, so the descent above reads a measured string where `evaluate` would
/// hand back a string literal. `Null` is the refusal, as it is there.
fn fold_binary_expr(
  binary_expr: &BinExpr,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> BinaryExprType {
  // No confidence guard. Every caller folds from a walk that is still
  // confident, and `Null` is what this answers for a chain that refused, so a
  // guard would answer the same thing one step earlier. What it would buy is a
  // short circuit rather than a different answer, and no measurement asks for
  // one.
  binary_expr_to_num_or_str(binary_expr, state, traversal_state, fns).unwrap_or_else(|num_error| {
    binary_expr_to_string(binary_expr, state, traversal_state, fns).unwrap_or_else(|str_error| {
      debug!("Binary expression to string error: {}", str_error);
      debug!("Binary expression to number error: {}", num_error);

      BinaryExprType::Null
    })
  })
}

/// Whether `op` applies `ToNumber` to each side. Then an array or an object
/// reads as the number of its primitive, as under a unary operator.
///
/// `+` does not: it concatenates as soon as a primitive is a string, and the
/// string path reads an array in that way. A relational comparison compares two
/// strings as strings, and an equality compares two objects by reference, so a
/// comparison with an object refuses (see its tests). `in` asks about a key and
/// `instanceof` about a prototype. A number would give each of these a wrong
/// answer.
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
///
/// An operator that reads each side as a number reads it through
/// [`operand_to_js_number`], as the unary operators do. Every other operator
/// keeps the narrow reading, which refuses a side with no expression form, such
/// as an array, with `not_an_expression`.
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

/// Every binary operator but the three logical ones, folded to whichever of a
/// number and a string its operands decide.
///
/// Named for both because `+` returns either: it is the one operator whose
/// result type is not the path's to choose, and reading the name as a promise
/// of a number is how a caller comes to treat `'1' + 2` as arithmetic. The two
/// callers outside this node want a number specifically and refuse the other
/// answer themselves.
pub(crate) fn binary_expr_to_num_or_str(
  binary_expr: &BinExpr,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> Result<BinaryExprType, anyhow::Error> {
  let op = binary_expr.op;

  // `+` is the one operator whose result type its operands decide rather than
  // the path that claimed it: JavaScript concatenates as soon as either side is
  // a string, and only adds when neither is. Asked after the numeric coercion
  // instead, `'1' + 2` would answer `3` — both sides coerce, so the number path
  // would never yield to the string one.
  //
  // It is therefore also the only operator that asks anything of its right side
  // before its left has coerced. The rest are left in the order they had, so a
  // left side with no numeric form goes on refusing there rather than deopting
  // on a right side the refusal never needed.
  //
  // It is also the only operator whose left side can arrive measured, so the
  // two shapes part here and the numeric path below holds a plain value.

  // The right side of a `+` that adds, and the right side of a comparison the
  // reading had no answer for, are the same slot: a side this path has already
  // evaluated and the numeric read below would otherwise ask for a second
  // time. A second read would be a memo hit rather than a fold, but a hit
  // costs a structural hash of the whole subtree and a deep clone of what it
  // remembered -- and for a `+` it would hand back the object unreduced.
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

      // Concatenated here rather than by handing the expression back to the
      // string path, which would evaluate both operands a second time -- and
      // would drop the left side's measurement on the way, since only a value
      // can carry one.
      //
      // One arm rather than two for the concatenation, so the side is handed on
      // as it arrived. A measured side is always a string, which is the whole
      // of what makes the `+` a concatenation whatever the right side holds; an
      // ordinary side concatenates when either side is one.
      //
      // Each side is read through its own conversion before the question is
      // asked, because that is the order the operator applies: an object whose
      // own `toString` answers a string concatenates, and one whose own
      // `valueOf` answers a number adds. A measured side is already a string
      // and has nothing to reduce.
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

  // The eight comparison operators compare two values, and only fall to the
  // numeric coercion below when one of them is not a primitive. Asked after the
  // coercion instead, `1 != '1'` answered `false` and `'10' < '9'` answered
  // `false` where the language and the reference implementation both answer
  // `true`, and `'a' == 'a'` refused because neither side has a number.
  //
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
    // An operator whose result is a number is read in `stylex-js`, beside the
    // coercions it applies -- the arithmetic and the six bitwise ones. One home
    // for them, because the transform reads a binary expression through the
    // same function and the two readings have to answer alike.
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
    // `in` and `instanceof` ask a question about an object, which this path has
    // already coerced away to a number. What they answer here is therefore not
    // the operator's meaning; it is left as found, because nothing real StyleX
    // source can write reaches either arm.
    BinaryOp::In => convert_bool_to_number(right_num == 0.0),
    BinaryOp::InstanceOf => convert_bool_to_number(right_num == 0.0),
    // Unreachable, on two grounds. The three logical operators are dispatched to
    // their own node before this path can run, and there they return an operand
    // rather than a number. The eight comparison operators are answered by the
    // reading above for every side that is a primitive -- and a side that is
    // not one has no number either, so it refuses at the coercion above and
    // never arrives here. Both are refused on the same terms as any other
    // operator this path has no answer for, rather than coerced to one.
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

/// `+` over operands at least one of which is a string: `ToString` of each
/// side, joined.
///
/// Only `+` has a string result. Every other operator reaches here through the
/// number path's fallback, having already refused there, and is refused again
/// so the caller deopts rather than failing the build — which is also what the
/// language asks for, since `'a' * 'b'` is a value rather than an error.
///
/// Reached for a `+` only when the number path refused one — an operand with no
/// numeric reading and no string either side, such as an array. The `+` the
/// number path *did* claim concatenates through [`concatenate`] directly,
/// keeping the operands it has already evaluated.
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

  // Both operands through the plain reading, not the measured one: a left side
  // that folds to a string never arrives here, because the number-or-string
  // path claims that `+` and concatenates it itself. What reaches this fallback
  // is a `+` that path refused -- an operand with no numeric form and no string
  // either side, such as an array -- so a descent here could only pay for a
  // measurement nothing would carry.
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
///
/// Both sides grow the same buffer, which measures every piece before it lands
/// -- so a chain of doublings is refused at the append that passes the ceiling
/// rather than after the next one has allocated, and an operand that is an
/// array is refused at the element that passes it rather than after its whole
/// join.
///
/// A left side this path folded one level down is *adopted* rather than copied
/// in: it is the same text and the same count, already checked against the same
/// ceiling, so re-reading it would spend the length of everything joined so far
/// once per remaining link. Any other left side is written through the coercion
/// like the right one, because an array's join is only measurable while it is
/// being written.
///
/// Each side is taken through `ToString`, the coercion the rest of the evaluator
/// already shares from `stylex_js`. This arm used to keep a second, weaker one
/// of its own, which read a string, a number and a big integer and refused the
/// rest -- so `'x' + true` failed to fold where JavaScript says `"xtrue"`. The
/// shared one answers for the whole falsy list, for arrays and for objects, and
/// refuses only where no compile-time string exists at all.
///
/// It is also more permissive than the reference implementation on two
/// operands, and deliberately left that way: a big integer and a regular
/// expression both have a string here, where upstream refuses either literal
/// outright with an unsupported-expression diagnostic. The folded strings are
/// what the language says, so the disagreement costs nothing but a build that
/// succeeds where the other fails.
fn concatenate(
  binary_expr: &BinExpr,
  left: LeftOperand,
  right: &EvaluateResultValue,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
) -> Result<BinaryExprType, anyhow::Error> {
  // The sentence for an operand with no string at all names which side it was;
  // a ceiling refusal carries the buffer's own, which names the concatenation
  // rather than a side, because either side reaching the ceiling is the same
  // expression growing too large.
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
