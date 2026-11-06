// pub static IMPORT_FILE_PARSING_ERROR: &str = r#"There was error when
// attempting to parse the imported file. Please ensure that the 'babelrc' file
// is configured to be able to parse this file."#;

/// A reference that resolves to a *default* import specifier. A theme file is
/// read through its named exports, so a default binding names a value from a
/// file this compiler never evaluates -- there is nothing to fold, and folding
/// the theme reference a named specifier would have answered with would invent
/// a variable the theme file does not define.
///
/// Says "error evaluating the imported file" rather than "cannot use default
/// imports" because that is the reference implementation's text for this input.
/// It has a message of its own for a default import -- `DEFAULT_IMPORT`, still
/// commented out below -- and does not reach it from here; the reference chain
/// refuses a default specifier with this one instead, and the two compilers
/// agreeing on the text is what makes a diagnostic portable between them.
pub static IMPORT_FILE_EVAL_ERROR: &str = concat!(
  "There was an error when attempting to evaluate the imported file.\n",
  "Please ensure that the imported file is self-contained and does not rely ",
  "on dynamic behavior.\n"
);

// pub static DEFAULT_IMPORT: &str = r#"Error: Cannot use default imports.

// Please define your styles without depending on values imported from other
// files.

// You *may* use named imports to use variables defined with `defineVars` in a
// file with `.stylex.js` or `.stylex.ts` file. See: https://stylexjs.com/docs/learn/theming/defining-variables/#rules-when-defining-variables for more information."#;

pub static PATH_WITHOUT_NODE: &str = r#"Unexpected error:
Could not resolve the code being evaluated."#;

pub static UNEXPECTED_MEMBER_LOOKUP: &str = r#"Unexpected error:
Could not determine the property being accessed."#;

pub static IMPORT_PATH_RESOLUTION_ERROR: &str = r#"Could not resolve the path to the imported file.
Please ensure that the theme file has a .stylex.js or .stylex.ts extension and follows the
rules for defining variables:

https://stylexjs.com/docs/learn/theming/defining-variables/#rules-when-defining-variables
"#;

pub static NON_CONSTANT: &str = "Referenced value is not a constant.\n\n";

/// A reference whose binding is declared later in the module. Declarations are
/// collected module-wide, so the initializer is reachable from a use site that
/// precedes it — and inlining it there emits CSS for a value the program does
/// not hold yet.
pub static USED_BEFORE_DECLARATION: &str = "Referenced value is used before declaration.\n\n";

/// A reference to one of `undefined` / `Infinity` / `NaN` that names a binding
/// rather than the global. The three are ordinary binding names to the
/// language, so anything in scope can take one over -- a `const`, a function
/// parameter, a catch binding -- and the global is then not what the reference
/// reads. There is no folded value to answer with either, so the fold refuses.
///
/// Says "not initialized" rather than "shadowed" because that is the reference
/// implementation's text, and it is the text for the same reason: the step is
/// reached only where the binding carries no value the evaluator holds.
pub static UNINITIALIZED_CONST: &str = "Referenced constant is not initialized.\n\n";

pub static UNDEFINED_CONST: &str = "Referenced constant is not defined.";

pub static OBJECT_METHOD: &str = "Unsupported object method.\n\n";

/// An author's own arrow was applied and its body answered no value, with every
/// argument bound. So the body is what has nothing to give: it read something
/// with no compile-time value, or it answered one of this compiler's own values
/// -- a whole token group, the injected function map -- which a stylesheet
/// cannot hold.
///
/// Names the body rather than the call, because the call is fine and the body is
/// the line an author has to change. The neighbouring refusal names the argument
/// instead, and which of the two fires is decided by whether every argument
/// bound.
pub static FUNCTION_BODY_WITHOUT_VALUE: &str = "The function's body has no compile-time value.\n\n";

/// A global called as a function was given a value the bridge cannot carry into
/// the engine — the environment object, a theme reference that has not resolved,
/// or a name with no compile-time value at all.
///
/// A refusal rather than a shape handed back, because the fold owns every call
/// to an unbound global: nothing below it folds one, so handing the call back
/// would end it at the catch-all's `Unsupported expression` with the reason
/// lost. The callee is named because which conversion refused is the first thing
/// an author needs.
///
/// Also the sentence for a StyleX function the engine may call, whose argument
/// the bridge cannot carry, reached inside a callback — where nothing below the
/// fold can answer for the same reason. One sentence rather than two nearly
/// identical ones: what the author has to do is the same, and the callee it names
/// is the one they wrote.
pub fn uncoercible_value(callee: &str) -> String {
  format!(
    "Cannot coerce this value at compile time.\nOnly static values can be passed to {}().\n\n",
    callee
  )
}

/// An array came back from a fold longer than the fold will materialise: every
/// element costs the width of an evaluated value, so a length JavaScript accepts
/// can still be an allocation the compiler does not survive.
pub fn array_length_too_large(limit: u64) -> String {
  format!(
    concat!(
      "Array length is too large to evaluate at compile time.\n",
      "At most {} elements are supported.\n\n"
    ),
    limit
  )
}

/// A global that only contributes methods was called as a function — `Math(x)`.
///
/// It reaches the fold at all because its methods are foldable, so the callee
/// is recognised and the call is not. Naming the callee says what the author
/// wrote wrong.
pub fn not_a_function(callee: &str) -> String {
  format!(
    "{} is not a function.\nOnly its methods can be called.\n\n",
    callee
  )
}

/// The one answer every spread in a value position earns.
///
/// A spread is refused wherever it appears — an array element, a call argument,
/// at any nesting — because the reference implementation evaluates elements and
/// arguments as *paths*, so a spread reaches its terminal
/// `UNSUPPORTED_EXPRESSION(path.node.type)` arm whatever it spreads and before
/// the operand is looked at.
///
/// A constant rather than an `unsupported_expression("SpreadElement")` at each
/// of the eleven sites that need it: the node kind is the same string every
/// time, and a site that spelled it differently would be a divergence no test
/// names.
pub const SPREAD_ELEMENT: &str = "Unsupported expression: SpreadElement\n\n";

pub fn unsupported_operator(op: &str) -> String {
  format!("Unsupported operator: {}\n\n", op)
}

pub fn unsupported_expression(type_: &str) -> String {
  format!("Unsupported expression: {}\n\n", type_)
}

/// An expression nested past the fold's recursion budget.
///
/// Without a bound of its own the evaluator's real limit is the thread's stack,
/// and its failure a process abort rather than a diagnostic. The budget makes
/// the limit a number the compiler owns, and the refusal an ordinary one.
///
/// Says *nested evaluation* rather than nested expressions, because the budget
/// counts the fold's own steps and a source level is not always one of them: a
/// member read descends to its object and to the value it lands on, a spread
/// descends to the object it copies, and a parenthesis is unwrapped before the
/// fold is asked at all. Naming source levels would put a number in the message
/// that the input beside it contradicts.
///
/// Shaped after [`array_length_too_large`]: a fold that gives up on size says
/// so, and says how much it was willing to do.
pub fn expression_too_deep(limit: usize) -> String {
  format!(
    "Expression is too deeply nested to evaluate at compile time.\n\
     At most {} levels of nested evaluation are supported.\n\n",
    limit
  )
}

/// Source nested deeper than the fold can carry the printing and the parsing of
/// it.
///
/// The sibling of [`expression_too_deep`] and deliberately not the same
/// sentence, because the two count different things. That one is spent per step
/// of the fold's own walk, and a step is not a source level. This one is source
/// levels exactly: an operand a short circuit never reaches is printed and
/// parsed whole without the walk entering it, so what bounds it is how deeply
/// the text nests and nothing else. Quoting one number through the other's
/// wording would tell an author to shorten something they had not written.
pub fn nesting_too_deep_to_carry(limit: usize) -> String {
  format!(
    "Expression is nested too deeply to fold at compile time.\n\
     At most {} levels of nested expressions can be carried.\n\n",
    limit
  )
}

/// The one shape every refusal of a *call* takes: the call this compiler could
/// not fold, then the reason it could not.
///
/// Five rules end here, and each of them names a method rather than a node kind,
/// because the method is what an author has to change. One function rather than
/// the same `format!` five times: the first line is what a reader learns to
/// recognise, and a site that spelled it differently would read as a different
/// class of failure without being one.
fn cannot_fold(call: &str, reason: &str) -> String {
  format!("Cannot fold '{}' at compile time.\n{}\n\n", call, reason)
}

/// A method whose answer depends on locale data the compiler does not carry.
///
/// The engine resolves these against the root locale, so
/// `"i".toLocaleUpperCase("tr")` comes back `I` where the language says `İ`.
/// Folding would write a wrong value into the stylesheet, which is worse than
/// writing none, so the fold declines and says which method it declined.
pub fn locale_sensitive_method(method: &str) -> String {
  cannot_fold(
    method,
    "Its answer depends on locale data the compiler does not carry.",
  )
}

/// A static of one of the globals whose surface the engine owns, named in the
/// set the compiler refuses by name.
///
/// The statics a fold may call are listed per global, and a call this sentence
/// answers for is one the list does not hold. Every static outside the list
/// breaks a property a fold rests on: `Math.random` answers a different number
/// each time it is asked, so a class name hashed from it would move between
/// builds, and `Object.freeze` and the rest answer by changing the value they
/// were handed rather than by computing one. Folding either writes a
/// declaration the source does not describe.
///
/// The reflective statics -- `getPrototypeOf`, `create` and the rest -- read
/// the same sentence. They answer the same thing on every build, so "does not
/// answer from the source alone" is a loose fit for them; one sentence is
/// carried anyway, because a second would be a second table to keep.
///
/// Names the receiver with the method — `Object.assign` rather than `assign` —
/// because on a static the receiver is the half that disambiguates: `assign` and
/// `freeze` are `Object`'s, `random` is `Math`'s, and a message naming only the
/// method would read the same for a method call on a value.
///
/// Named as a refusal rather than left to fall through, because the whole static
/// surface folds now: a call this set holds back is the only one an author can
/// write on these globals and not get an answer for, so it has to say why.
pub fn unfoldable_static(callee: &str, method: &str) -> String {
  cannot_fold(
    &format!("{}.{}", callee, method),
    "A fold has to answer from the source alone, and this call does not.",
  )
}

/// A method called on a number written into the source as a literal.
///
/// Every `Number.prototype` method throws on one of those in the reference
/// implementation, which applies the method without a receiver. Refusing keeps
/// both compilers rejecting the same input. A number a fold *produced* is a
/// different shape and folds in both.
pub fn numeric_literal_receiver(method: &str) -> String {
  format!(
    concat!(
      "Cannot call '{}' on a number literal.\n",
      "Only a number a fold produced can be a method receiver.\n\n"
    ),
    method
  )
}

/// The one shape every refusal of a length-amplifying call takes: what the call
/// would build, then why it cannot be built.
///
/// The engine bounds loop iterations, recursion and stack, but not allocation:
/// growth inside a native builtin is not a counted loop. So the length has to be
/// readable, and under the ceiling, before the call is evaluated at all -- two
/// ways to fail one question, which is why they share a first line.
///
/// `built` is the noun rather than a fixed word because a call amplifies in one
/// of the two units a fold spends, and the two do not stand in for each other: a
/// string that repeats builds characters, and an array length declares elements.
/// Naming both through one helper keeps that first line the sentence a reader
/// learns to recognise.
fn cannot_bound(built: &str, call: &str, reason: &str) -> String {
  format!(
    "Cannot bound the {} '{}' would build.\n{}\n\n",
    built, call, reason
  )
}

/// A length-amplifying call whose result length could not be read.
///
/// A count and, for `repeat`, the receiver it multiplies are both needed before
/// the product can be compared to anything. Either may be written out or named,
/// and a call is the one receiver deliberately left unread: its own answer is
/// bounded per link, so reading it is what would let two allowed lengths
/// multiply into one that is not.
pub fn unbounded_amplified_length(method: &str, limit: u64) -> String {
  cannot_bound(
    "string",
    method,
    &format!(
      concat!(
        "Its length must resolve to a number of at most {}, ",
        "on a receiver whose own length can be read."
      ),
      limit
    ),
  )
}

/// An `Array(n)` whose length reached the conversion behind the fold.
///
/// A length is read and bounded in front of the engine, on the whole call. A
/// call the fold handed back was never bounded there, so the elements are
/// refused rather than allocated on a number nothing checked.
///
/// Shares its first line with the other length refusals, because it is the same
/// question failed one step later.
pub fn unbounded_declared_length(call: &str) -> String {
  cannot_bound(
    "array",
    call,
    concat!(
      "Write the elements out, or keep the rest of the expression foldable ",
      "so its length can be checked."
    ),
  )
}

/// A length-amplifying call whose result length was read and is too large.
///
/// Names the length asked for beside the limit, because the two together are
/// what say whether the call is a typo or a project that has outgrown the
/// ceiling -- and a limit alone leaves an author guessing which.
///
/// `count` and `built` are the same number for a call that pads, and differ for
/// one that repeats: `'xx'.repeat(600000)` builds 1200000 characters, and 600000
/// is the only one of those an author can find in what they wrote. So both are
/// named where they differ, and the one number is named once where they do not.
///
/// `repeats` is how many times the call is evaluated -- one outside a callback,
/// and the receiver's element count inside one. Named only where it is more than
/// one, because that is where the total is a number the author cannot find
/// anywhere in what they wrote and the call alone reads as being inside the
/// limit.
pub fn amplified_length_too_large(
  method: &str,
  count: u64,
  built: u64,
  repeats: u64,
  limit: u64,
) -> String {
  let asked = match built == count {
    true => format!("{} characters", built),
    false => format!(
      "{} copies of the value it is called on, which is {} characters",
      count, built
    ),
  };

  cannot_bound(
    "string",
    method,
    &format!(
      "It asks for {}{}, and at most {} are supported.",
      asked,
      per_element(built, repeats, "characters"),
      limit
    ),
  )
}

/// What a bound read once comes to across every evaluation of the call, or
/// nothing at all where there is only one.
///
/// Only where there is more than one, because that is where the total is a number
/// the author cannot find anywhere in what they wrote and the call alone reads as
/// being inside the limit. One evaluation says nothing about repeats, and a
/// clause saying so would be noise on the common refusal.
///
/// `built` is what one evaluation comes to and `unit` names it, for the reason
/// [`cannot_bound`] takes a noun: a call amplifies in one of the two units a fold
/// spends, and both refusals need the same sentence in their own.
fn per_element(built: u64, repeats: u64, unit: &str) -> String {
  match repeats > 1 {
    true => format!(
      concat!(
        " once per element of the receiver it is written inside, ",
        "which is {} evaluations and {} {} in all"
      ),
      repeats,
      built.saturating_mul(repeats),
      unit
    ),
    false => String::new(),
  }
}

/// A call declaring more array elements than the fold's entry budget.
///
/// The sibling of [`amplified_length_too_large`] in the other unit. It reads a
/// length rather than a count of copies, so there is one number to name and not
/// two: `Array(100000000)` says what it will build in the only place an author
/// can change it.
///
/// Named for the call rather than for the array, unlike
/// [`array_length_too_large`], which is the same size measured after the fact.
/// The difference is when: this one arrives before anything is allocated, so
/// what it names is the argument to edit.
///
/// `repeats` is the sibling of the one [`amplified_length_too_large`] takes, and
/// is named on the same terms: only where it is more than one, since that is
/// where the declaration alone reads as being inside the limit.
pub fn amplified_entries_too_large(call: &str, declared: u64, repeats: u64, limit: u64) -> String {
  cannot_bound(
    "array",
    call,
    &format!(
      "It declares a length of {} elements{}, and at most {} are supported.",
      declared,
      per_element(declared, repeats, "elements"),
      limit
    ),
  )
}

/// A folded object with more properties than the fold's budget.
///
/// Shaped after [`array_length_too_large`] and bounded by the same number, for
/// the same reason: every property crossing back becomes an AST node, which
/// costs far more as a tree than it did as a value in the engine.
pub fn object_size_too_large(limit: u64) -> String {
  format!(
    "Object is too large to evaluate at compile time.\nAt most {} properties are supported.\n\n",
    limit
  )
}

/// A folded value with no form this evaluator carries — a function, a symbol,
/// a BigInt, or an object that is not a plain one.
///
/// Names the kind rather than the expression, because the expression is what
/// the author wrote and the kind is what the language answered with; only the
/// second says why an otherwise valid call folds to nothing usable.
///
/// Phrased so the kind needs no article in front of it. That is what lets the
/// caller pass the language's own `typeof` straight through instead of keeping
/// a word per kind of its own, and the second line is what tells an author
/// whose value is `object` that theirs is not a plain one.
pub fn unfoldable_fold_result(kind: &str) -> String {
  format!(
    concat!(
      "Cannot carry a folded {} back from the engine.\n",
      "Only strings, numbers, booleans, null, undefined, arrays and plain objects ",
      "can be folded.\n\n"
    ),
    kind
  )
}

/// The engine could not be prepared, so there is nothing to fold with.
///
/// Unreachable in practice — what it reports is the one assignment the fold
/// makes when it builds an engine — and answered rather than asserted, because
/// this runs inside an evaluation whose whole contract is that it may fail,
/// where a panic would abort a build that a refusal only leaves to the runtime.
pub fn engine_did_not_start(message: &str) -> String {
  format!(
    "The compile-time JavaScript engine could not start.\n{}\n\n",
    message
  )
}

/// A call the engine threw on, reported in the engine's own words under this
/// compiler's naming of the call that produced them.
///
/// A throw is an answer rather than a fault of the fold: `[].reduce(f)` and
/// `"a".repeat(-1)` throw in the language too, and the sentence the language
/// wrote says more than any sentence this compiler could substitute for it.
///
/// The method is named alongside it because the language's sentence does not
/// always name it. A call to a method that does not exist reads a property that
/// is `undefined` and then calls it, so the throw is `not a callable function` —
/// which tells an author nothing the code frame has not already shown them.
pub fn engine_threw(method: &str, message: &str) -> String {
  cannot_fold(method, message)
}

/// A read of a property that steps onto the prototype chain.
///
/// `__proto__` and `prototype` reach the language's function graph one step
/// before `constructor` does, so a read of either is refused wherever it is
/// written. `constructor` is named in the sentence as an example, although a
/// read of that name is answered by [`escaping_property`] first -- the two
/// rules overlap on it by design, and the sentence reads as a list of what is
/// blocked rather than as a claim about which rule fired.
///
/// A `__proto__` written as an object-literal *key* is not this: it sets a
/// prototype rather than reading one, and it folds.
///
/// The only sentence here that is not built by [`cannot_fold`], and the only
/// one whose line breaks are written in. Both are deliberate: the reference
/// implementation words this refusal the same way, so the two compilers answer
/// such a read with one text rather than two, and the parity corpus records
/// that agreement. Re-wrapping the lines would end it.
///
/// The same sentence answers a read the printed source makes, where the key is
/// a value rather than a name the syntax spells. The fold's checked reader
/// throws it -- see [`BLOCKED_FUNCTION_CALL`] for the second guard behind it --
/// so an author reads one text for the rule wherever it fires.
pub static BLOCKED_PROPERTY_ACCESS: &str = concat!(
  "Access to this property is not allowed during compilation.\n",
  "Accessing prototype-chain properties such as 'constructor', '__proto__', or 'prototype'\n",
  "is blocked to prevent arbitrary code execution.\n"
);

/// A call of a function that compiles a string into code.
///
/// The backstop behind [`BLOCKED_PROPERTY_ACCESS`] and the callee allowlist.
/// Both of those refuse a *route* to such a function, and each route is one
/// this compiler already knows about; this refuses the call itself, so a route
/// nobody has found yet still ends in a refusal rather than in arbitrary code.
///
/// Thrown by the fold's checked caller, which compares the callee against the
/// language's own `Function`, against anything inheriting directly from it --
/// the async, generator and async-generator constructors -- and against `eval`.
///
/// Worded as the reference implementation words it, line breaks included, so
/// the two compilers answer such a call with one text rather than two.
pub static BLOCKED_FUNCTION_CALL: &str = concat!(
  "Calling this function is not allowed during compilation.\n",
  "Functions that compile strings into code, such as 'Function' and 'eval', are blocked to\n",
  "prevent arbitrary code execution.\n"
);

/// A name the printed fold reserves for a check of its own.
///
/// The fold prints a reader and a caller into the source it hands the engine,
/// and both are parameters of the printed arrow rather than globals -- a global
/// is a name the source could point somewhere else. A parameter is still a
/// name, so a binding of the same spelling inside the printed source would
/// shadow it, and the check would run the author's function instead of this
/// compiler's. Such a binding is refused here rather than folded past.
///
/// No author writes these names, so the sentence says what they are for rather
/// than how to avoid them: the rename is the whole of the fix.
///
/// Not built by [`cannot_fold`], although it starts the same way. That shape
/// names the *call* a rule refused, and this refuses a binding — there is no
/// method to name, and naming the call around it would point an author at a
/// line they need not change.
pub fn reserved_compiler_name(name: &str) -> String {
  format!(
    "Cannot fold an expression that binds '{}' at compile time.\n\
     The compiler reserves that name for its own use in the source it evaluates.\n\n",
    name
  )
}

/// A named property read that leads off the value the author wrote.
///
/// `constructor` on a literal is `String`, and `String.constructor` is
/// `Function`, which compiles a string into a body — so two reads and a call
/// are arbitrary code inside the compiler, answering differently on every build
/// and able to write to a prototype every later fold in the build shares.
/// `call`, `apply` and `bind` are what turn an unapplied function back into a
/// call, so they are refused with it.
///
/// Names the property rather than the method call it sat in, because the
/// property is the whole of the reason: the same call on any other name folds.
pub fn escaping_property(property: &str) -> String {
  format!(
    "Cannot fold a read of '{}' at compile time.\n\
     It leads off the value that was written and onto the language's own function graph.\n\n",
    property
  )
}

/// A length-amplifying call inside a callback whose evaluations were not counted.
///
/// [`unbounded_amplified_length`] bounds one evaluation, which is what a length
/// written into the source can bound. A callback body is evaluated once per
/// element of its receiver, so the bound the source states is one factor and that
/// element count is the other -- and where the receiver has an element count the
/// guard can read, the product is the bound and this refusal does not arrive.
///
/// It arrives for the remainder: a receiver whose elements nothing measured. So
/// the sentence names the receiver rather than the callback, because the receiver
/// is what an author changes -- writing the elements out, or naming the array
/// they are in, is what makes the count readable.
///
/// `built` is the noun for the reason [`cannot_bound`] takes one: a call
/// amplifies in one of the two units a fold spends. The advice fits both, and is
/// the whole of what makes the refusal cheap to act on -- a repeat of three has
/// the string written out, and an array of a declared length has its elements.
pub fn amplification_inside_a_callback(built: &str, call: &str) -> String {
  format!(
    concat!(
      "Cannot bound the {} '{}' would build inside a callback.\n",
      "The callback's receiver holds an element count that cannot be read here, ",
      "so a length written into the source bounds one evaluation rather than the ",
      "call. Write the receiver's elements out, or write the value out instead.\n\n"
    ),
    built, call
  )
}

/// A statement inside a callback body that the fold does not read.
///
/// A callback runs as real JavaScript, so its body may branch, declare and
/// return. A **loop** is the one kind left out for a bound rather than for a
/// shape: the engine's iteration count lives on the call frame, so a callback
/// invoked once per element starts a fresh count every time and the bound is
/// multiplied by an element count the source never states — exactly as a
/// length-amplifying call's is. See [`amplification_inside_a_callback`].
///
/// Names the statement kind, because that is the word an author can look for in
/// the body they wrote -- and says what *is* read, so the next step is a
/// rewrite rather than a guess.
pub fn unfoldable_statement(kind: &str) -> String {
  format!(
    "Cannot fold a callback whose body uses a {}.\n\
     Only a declaration, a branch, a block and a return are read inside a callback body.\n\n",
    kind
  )
}

/// A folded string longer than the fold will carry back.
///
/// [`unbounded_amplified_length`] bounds what one written call may be *asked*
/// to build; this bounds what actually came back, whatever produced it. Shaped
/// after [`array_length_too_large`] and bounded by the same number as the
/// argument is, because it is the same string measured on the other side.
pub fn folded_string_too_large(limit: u64) -> String {
  format!(
    "Folded string is too large to evaluate at compile time.\n\
     At most {} characters are supported.\n\n",
    limit
  )
}

/// The four expressions that make the evaluator write a string, as
/// [`grown_string_too_large`] names them.
///
/// Written here rather than at the call sites so the wording an author reads
/// lives beside the sentence it lands in.
///
/// The first two *grow* one. The last two do not keep a string at all: each
/// reads its answer out of the text its argument renders, and an array renders
/// as the join of its elements — so the ceiling is spent there too, and the
/// sentence names what the author wrote rather than the join inside it.
pub const CONCATENATION: &str = "concatenation";
pub const TEMPLATE_LITERAL: &str = "template literal";
pub const NUMERIC_CONVERSION: &str = "numeric conversion";
pub const STRING_CONVERSION: &str = "string conversion";

/// A string the evaluator grew past what it will hold.
///
/// [`folded_string_too_large`] bounds a string the engine built and handed back,
/// so it is read only where a value crosses a fold. Nothing crosses here: `+` and
/// an interpolation are answered by the evaluator itself, so a chain that doubles
/// its own result was bounded by no number at all -- and a depth budget, which is
/// what stopped it, bounds how far a walk descends rather than how large a value
/// gets.
///
/// Bounded by the same number as a folded string, because it is the same string
/// and the same cost: the ceiling a project raises to fold a long value raises
/// this with it.
///
/// Names which expression grew it, because a doubling chain is innocent one line
/// at a time and the line that passed the ceiling is the one an author has to
/// look at. An array's own join is measured through the same buffer and so
/// carries one of the same two names: what an author has to look at there is the
/// interpolation or the `+` the array was written in, not the join.
pub fn grown_string_too_large(kind: &str, limit: u64) -> String {
  format!(
    "This {} builds a string too large to evaluate at compile time.\n\
     At most {} characters are supported.\n\n",
    kind, limit
  )
}

/// A binding whose resolved value is too large to carry into a fold.
///
/// The transport passes a resolved value as an *argument* rather than printing
/// it into the source, so the printed text stays the size of the expression
/// however large the value is. What that does not shrink is the value itself:
/// it is copied into the engine, so it is bounded on the way in by the same
/// number that bounds a folded string on the way out.
///
/// Names the binding rather than the method, because the size is a property of
/// what the name holds and the same call on a shorter value folds.
pub fn bound_value_too_large(name: &str, limit: u64) -> String {
  format!(
    "Cannot carry the value of '{}' into a fold.\nAt most {} characters are supported.\n\n",
    name, limit
  )
}

/// A binding whose resolved value carries more entries than the fold will copy
/// into the engine.
///
/// [`bound_value_too_large`] bounds the text a value holds; this bounds how many
/// elements and properties hold it, because those are two costs rather than one:
/// a thousand empty arrays are no text at all and still a thousand values to
/// build in the engine. Bounded by the number that bounds a folded array on the
/// way out, since it is the same count measured on the other side.
///
/// Names the binding for the reason [`bound_value_too_large`] does: the size
/// belongs to what the name holds, and the same call on a smaller value folds.
pub fn bound_value_has_too_many_entries(name: &str, limit: u64) -> String {
  format!(
    concat!(
      "Cannot carry the value of '{}' into a fold.\n",
      "At most {} elements and properties are supported.\n\n"
    ),
    name, limit
  )
}

/// A binding the fold reached as a function and could not carry.
///
/// A function has no value form to pass beside the printed source, so the one
/// the guard *can* carry is the arrow it was declared from, printed back as the
/// parameter's default. Every other declaration of a function is out: a block
/// body, a destructured or defaulted parameter, a `function` of either
/// spelling, and a binding written to after it was declared.
///
/// Names the binding rather than the call, because the call is fine — an author
/// told that `map` cannot fold would look at `map`, where what has to change is
/// one declaration away. Says the declaration is the place to look rather than
/// listing the shapes, since the list is what the doc comment above is for and a
/// diagnostic that recites it is longer without being clearer.
///
/// Written out rather than routed through [`cannot_fold`], which is the shape of
/// every refusal *of a call*: this one names a binding, so it would have to pass
/// the wrong noun to get the first line it needs. Shaped after
/// [`bound_value_too_large`] instead, which names a binding for the same reason.
pub fn unfoldable_function(name: &str) -> String {
  format!(
    "Cannot carry the function '{}' into a fold.\n\
     Its declaration is not one the compiler can evaluate.\n\n",
    name
  )
}

/// A global whose *name* was written where the fold needs a value.
///
/// `[…].filter(Boolean)` is the shape it arrives as. The bridge carries values
/// and a global is not one: the engine holds the real function, and nothing this
/// side can hand across stands for it.
///
/// Names the global rather than the call around it, for the reason
/// [`unfoldable_function`] names a binding — the call is fine, and the name is
/// what an author has to replace.
///
/// Says what a fold does answer for a global rather than which position would
/// have worked. Naming a position would promise one: `Boolean` is here and
/// `Boolean(x)` does not fold either, so a sentence pointing at the call would
/// send an author to a second refusal.
pub fn global_as_a_value(name: &str) -> String {
  format!(
    "Cannot carry the global '{}' into a fold.\n\
     The compiler folds what a global answers, not the global itself.\n\n",
    name
  )
}

/// A printed fold that did not evaluate to the function it was printed as.
///
/// The fold prints its expression as an arrow and calls it, so the value the
/// engine answers is a function by construction. Answered as a refusal rather
/// than asserted, because a broken invariant inside an evaluation whose whole
/// contract is that it may fail must not abort the build.
pub fn uncallable_printed_fold(method: &str) -> String {
  cannot_fold(
    method,
    "The printed expression did not compile to a function.",
  )
}

/// A method call the fold declined, where the receiver's own value was fine.
///
/// The whole prototype surface folds through the engine, so a call arriving at
/// the older dispatch with a usable receiver was declined for one of two reasons:
/// something in it has no compile-time value, or it is written in a shape the
/// fold's guard does not read. The sentence names neither, because the arm cannot
/// tell them apart and a sentence that named the wrong one would send an author
/// looking in the wrong place. It says what is true of both.
///
/// Names the method, because that is what an author has to look at -- where the
/// node kind would only tell them they wrote a call.
pub fn unfoldable_call(method: &str) -> String {
  cannot_fold(
    method,
    "Its receiver or one of its arguments is not in a form the compiler can evaluate.",
  )
}
