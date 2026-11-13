// pub static IMPORT_FILE_PARSING_ERROR: &str = r#"There was error when
// attempting to parse the imported file. Please ensure that the 'babelrc' file
// is configured to be able to parse this file."#;

/// A reference resolving to a default import specifier, which theme files never export.
/// Matches Babel's text for this input (not its unreached `DEFAULT_IMPORT`).
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

/// A reference whose binding is declared later in the module.
pub static USED_BEFORE_DECLARATION: &str = "Referenced value is used before declaration.\n\n";

/// A reference to `undefined`/`Infinity`/`NaN` that names a shadowing binding, not the global.
/// Matches Babel's text.
pub static UNINITIALIZED_CONST: &str = "Referenced constant is not initialized.\n\n";

pub static UNDEFINED_CONST: &str = "Referenced constant is not defined.";

pub static OBJECT_METHOD: &str = "Unsupported object method.\n\n";


/// A call to a global or StyleX function with an argument the engine bridge cannot carry.
pub fn uncoercible_value(callee: &str) -> String {
  format!(
    "Cannot coerce this value at compile time.\nOnly static values can be passed to {}().\n\n",
    callee
  )
}

/// A folded array longer than the fold will materialise.
pub fn array_length_too_large(limit: u64) -> String {
  format!(
    concat!(
      "Array length is too large to evaluate at compile time.\n",
      "At most {} elements are supported.\n\n"
    ),
    limit
  )
}

/// A methods-only global called as a function, e.g. `Math(x)`.
pub fn not_a_function(callee: &str) -> String {
  format!(
    "{} is not a function.\nOnly its methods can be called.\n\n",
    callee
  )
}

/// Every spread in a value position; matches Babel's `UNSUPPORTED_EXPRESSION(path.node.type)`.
pub const SPREAD_ELEMENT: &str = "Unsupported expression: SpreadElement\n\n";

pub fn unsupported_operator(op: &str) -> String {
  format!("Unsupported operator: {}\n\n", op)
}

pub fn unsupported_expression(type_: &str) -> String {
  format!("Unsupported expression: {}\n\n", type_)
}

/// An expression nested past the fold's recursion budget (counts fold steps, not source levels).
pub fn expression_too_deep(limit: usize) -> String {
  format!(
    "Expression is too deeply nested to evaluate at compile time.\n\
     At most {} levels of nested evaluation are supported.\n\n",
    limit
  )
}

/// Source nested deeper than printing and parsing can carry (counts source levels).
pub fn nesting_too_deep_to_carry(limit: usize) -> String {
  format!(
    "Expression is nested too deeply to fold at compile time.\n\
     At most {} levels of nested expressions can be carried.\n\n",
    limit
  )
}

/// Shared shape for call refusals: the call, then the reason.
fn cannot_fold(call: &str, reason: &str) -> String {
  format!("Cannot fold '{}' at compile time.\n{}\n\n", call, reason)
}

/// A method whose answer depends on locale data the compiler does not carry.
pub fn locale_sensitive_method(method: &str) -> String {
  cannot_fold(
    method,
    "Its answer depends on locale data the compiler does not carry.",
  )
}

/// A refused static on an engine-owned global (`Math.random`, `Object.freeze`, …); names
/// `Receiver.method`.
pub fn unfoldable_static(callee: &str, method: &str) -> String {
  cannot_fold(
    &format!("{}.{}", callee, method),
    "A fold has to answer from the source alone, and this call does not.",
  )
}

/// A method called on a numeric literal; matches Babel, which applies it without a receiver.
pub fn numeric_literal_receiver(method: &str) -> String {
  format!(
    concat!(
      "Cannot call '{}' on a number literal.\n",
      "Only a number a fold produced can be a method receiver.\n\n"
    ),
    method
  )
}

/// Shared shape for refusals of length-amplifying calls.
fn cannot_bound(built: &str, call: &str, reason: &str) -> String {
  format!(
    "Cannot bound the {} '{}' would build.\n{}\n\n",
    built, call, reason
  )
}

/// A length-amplifying call whose result length could not be read.
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

/// An `Array(n)` whose length reached the engine without being bounded.
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

/// A length-amplifying call whose result is too large; `repeats` is how many times it is
/// evaluated (named only when more than one).
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

/// The total across `repeats` evaluations, or empty when there is only one.
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
pub fn object_size_too_large(limit: u64) -> String {
  format!(
    "Object is too large to evaluate at compile time.\nAt most {} properties are supported.\n\n",
    limit
  )
}

/// A folded value with no carried form; `kind` is the JavaScript `typeof`.
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

/// The compile-time engine could not be prepared (unreachable in practice).
pub fn engine_did_not_start(message: &str) -> String {
  format!(
    "The compile-time JavaScript engine could not start.\n{}\n\n",
    message
  )
}

/// A call the engine threw on, in the engine's words; names the method because the engine's
/// message may not.
pub fn engine_threw(method: &str, message: &str) -> String {
  cannot_fold(method, message)
}

/// A read of `__proto__` or `prototype` (literal keys are fine).
/// Text and line breaks match Babel's; do not re-wrap.
pub static BLOCKED_PROPERTY_ACCESS: &str = concat!(
  "Access to this property is not allowed during compilation.\n",
  "Accessing prototype-chain properties such as 'constructor', '__proto__', or 'prototype'\n",
  "is blocked to prevent arbitrary code execution.\n"
);



/// A read of `constructor`, `call`, `apply` or `bind`, which leads to the function graph.
pub fn escaping_property(property: &str) -> String {
  format!(
    "Cannot fold a read of '{}' at compile time.\n\
     It leads off the value that was written and onto the language's own function graph.\n\n",
    property
  )
}

/// A length-amplifying call inside a callback whose receiver's element count cannot be read.
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

/// A statement kind (e.g. a loop) the fold does not read inside a callback body.
pub fn unfoldable_statement(kind: &str) -> String {
  format!(
    "Cannot fold a callback whose body uses a {}.\n\
     Only a declaration, a branch, a block and a return are read inside a callback body.\n\n",
    kind
  )
}

/// A folded string longer than the fold will carry back.
pub fn folded_string_too_large(limit: u64) -> String {
  format!(
    "Folded string is too large to evaluate at compile time.\n\
     At most {} characters are supported.\n\n",
    limit
  )
}

/// Expression names used by [`grown_string_too_large`].
pub const CONCATENATION: &str = "concatenation";
pub const TEMPLATE_LITERAL: &str = "template literal";
pub const NUMERIC_CONVERSION: &str = "numeric conversion";
pub const STRING_CONVERSION: &str = "string conversion";

/// A string the evaluator itself (`+`, templates, conversions) grew past the folded-string limit.
pub fn grown_string_too_large(kind: &str, limit: u64) -> String {
  format!(
    "This {} builds a string too large to evaluate at compile time.\n\
     At most {} characters are supported.\n\n",
    kind, limit
  )
}

/// A binding whose resolved value has too many characters to copy into a fold.
pub fn bound_value_too_large(name: &str, limit: u64) -> String {
  format!(
    "Cannot carry the value of '{}' into a fold.\nAt most {} characters are supported.\n\n",
    name, limit
  )
}

/// A binding whose resolved value has too many elements and properties to copy into a fold.
pub fn bound_value_has_too_many_entries(name: &str, limit: u64) -> String {
  format!(
    concat!(
      "Cannot carry the value of '{}' into a fold.\n",
      "At most {} elements and properties are supported.\n\n"
    ),
    name, limit
  )
}

/// A function binding not declared as a carryable expression-bodied arrow.
pub fn unfoldable_function(name: &str) -> String {
  format!(
    "Cannot carry the function '{}' into a fold.\n\
     Its declaration is not one the compiler can evaluate.\n\n",
    name
  )
}

/// A global's name used as a value, e.g. `[…].filter(Boolean)`.
pub fn global_as_a_value(name: &str) -> String {
  format!(
    "Cannot carry the global '{}' into a fold.\n\
     The compiler folds what a global answers, not the global itself.\n\n",
    name
  )
}

/// A printed fold that did not evaluate to a function (an invariant; refused, not asserted).
pub fn uncallable_printed_fold(method: &str) -> String {
  cannot_fold(
    method,
    "The printed expression did not compile to a function.",
  )
}

/// A declined method call whose receiver was fine.
pub fn unfoldable_call(method: &str) -> String {
  cannot_fold(
    method,
    "Its receiver or one of its arguments is not in a form the compiler can evaluate.",
  )
}
