// Functions that generate messages with parameters
pub fn illegal_argument_length(fn_name: &str, arg_length: usize) -> String {
  let plural = if arg_length == 1 { "" } else { "s" };
  format!(
    "{}() should have {} argument{}.",
    fn_name, arg_length, plural
  )
}

pub fn non_static_value(fn_name: &str) -> String {
  format!(
    "Only static values are allowed inside of {} {}() call.",
    indefinite_article(fn_name),
    fn_name
  )
}

/// `"a"` or `"an"` by the first spelled letter of `name` (fine for StyleX API names).
fn indefinite_article(name: &str) -> &'static str {
  match name.chars().next() {
    Some(c) => match c.to_ascii_lowercase() {
      'a' | 'e' | 'i' | 'o' | 'u' => "an",
      _ => "a",
    },
    None => "a",
  }
}

pub fn non_style_object(fn_name: &str) -> String {
  format!("{}() can only accept an object.", fn_name)
}

pub fn non_export_named_declaration(fn_name: &str) -> String {
  format!(
    "The return value of {}() must be bound to a named export.",
    fn_name
  )
}

pub fn unbound_call_value(fn_name: &str) -> String {
  format!("{}() calls must be bound to a bare variable.", fn_name)
}

/// Rust-only: the printer drops parentheses around TS assertions, so
/// `(create({…}) as Styles).root` would print as `create() as Styles.root`.
pub fn type_asserted_call_value(fn_name: &str) -> String {
  format!(
    "{}() cannot be written inside a type assertion. Bind the call to a variable and assert the variable.",
    fn_name
  )
}

pub fn cannot_generate_hash(fn_name: &str) -> String {
  format!(
    "Unable to generate hash for {}(). Check that the file has a valid extension and that unstable_moduleResolution is configured.",
    fn_name
  )
}

// Static constants
pub static DUPLICATE_CONDITIONAL: &str =
  "The same pseudo selector or at-rule cannot be used more than once.";

pub static ILLEGAL_PROP_VALUE: &str = "A style value can only contain an array, string or number.";

pub static ILLEGAL_PROP_ARRAY_VALUE: &str =
  "A style array value can only contain strings or numbers.";

pub static ILLEGAL_NAMESPACE_VALUE: &str = "A StyleX namespace must be an object.";

/// Matches the runtime's `TypeError` word for word, which Babel surfaces via `Object.keys`.
pub static NULLISH_TO_OBJECT: &str = "Cannot convert undefined or null to object";

pub static INVALID_PSEUDO_OR_AT_RULE: &str = "Invalid pseudo or at-rule.";

pub static LINT_UNCLOSED_FUNCTION: &str = "Rule contains an unclosed function";

pub static LINT_UNCLOSED_STRING: &str = "Rule contains an unclosed string";

pub static LINT_UNCLOSED_COMMENT: &str = "Rule contains an unclosed comment";

pub static LINT_RULE_BREAKING_TOKEN: &str =
  "Rule contains a `{`, `}` or `;` outside of a string or comment";

pub static LINT_VALUE_NESTED_TOO_DEEPLY: &str =
  "Rule contains a value nested more deeply than the compiler supports";

pub static LINT_VALUE_HAS_NO_TOKENS: &str = "Rule contains a value with nothing to normalize";

/// An `!important` Babel's handler cannot remove cleanly (it may be nested, not misplaced).
pub static LINT_IMPORTANT_NOT_LAST: &str =
  "Rule contains an importance annotation the compiler cannot remove from the value";

pub static NON_OBJECT_KEYFRAME: &str = "Every frame within a keyframes() call must be an object.";

pub static NON_CONTIGUOUS_VARS: &str =
  "All variables passed to firstThatWorks() must be contiguous.";

pub static NO_OBJECT_SPREADS: &str = "Object spreads are not allowed in create() calls.";

pub static ONLY_NAMED_PARAMETERS_IN_DYNAMIC_STYLE_FUNCTIONS: &str = "Only named parameters are allowed in Dynamic Style functions. Destructuring, spreading or default values are not allowed.";

pub static UNPREFIXED_CUSTOM_PROPERTIES: &str = "Unprefixed custom properties";


pub static THEME_IMPORT_KEY_AS_OBJECT_KEY: &str =
  "Theme import keys cannot be used as object keys. Please use a valid object key.";

pub static POSITION_TRY_INVALID_PROPERTY: &str = "Invalid property in `positionTry()` call. It may only contain, positionAnchor, positionArea, inset properties (top, left, insetInline etc.), margin properties, size properties (height, inlineSize, etc.), and self-alignment properties (alignSelf, justifySelf, placeSelf)";

pub static VIEW_TRANSITION_CLASS_INVALID_PROPERTY: &str = "Invalid property in `viewTransitionClass()` call. It may only contain group, imagePair, old, and new properties";

pub static INVALID_MEDIA_QUERY_SYNTAX: &str = "Invalid media query syntax.";

pub static SPREAD_NOT_SUPPORTED: &str =
  "The spread operator (...) is not supported in this context. Declare each property explicitly.";

/// A spread whose operand's own properties cannot be read at compile time (e.g. an astral
/// string).
pub static SPREAD_PROPERTIES_UNREADABLE: &str =
  "The spread argument's properties could not be read at compile time.";

pub static EXPRESSION_IS_NOT_A_STRING: &str =
  "Expected a string value but received a non-string expression.";

/// A computed key with no string form at compile time (a function, a lone surrogate, a
/// compiler-internal value).
pub static KEY_HAS_NO_NAME: &str = "The key has no name at compile time.";

pub static VALUES_MUST_BE_OBJECT: &str = "The values argument must be a plain object.";

pub static ONLY_OVERRIDE_DEFINE_VARS: &str =
  "Can only override variables theme created with defineVars().";
pub static ONLY_OVERRIDE_DEFINE_VARS_NESTED: &str =
  "Can only override variables theme created with unstable_defineVarsNested().";

pub static MEMBER_NOT_RESOLVED: &str =
  "Could not resolve the member expression. Ensure the object and property are statically known.";

pub static PROPERTY_NOT_FOUND: &str =
  "Property not found on the object. Ensure the key exists and is spelled correctly.";

pub static EXPECTED_OBJECT_EXPRESSION: &str = "Expected an object expression.";

pub static VALUE_MUST_BE_STRING: &str = "Expected a string value but received a different type.";

pub static VALUE_MUST_BE_LITERAL: &str =
  "Expected a static literal value (string, number, or boolean).";

pub static ARGUMENT_NOT_EXPRESSION: &str = "Function argument must be a static expression.";

pub static EXPORT_ID_NOT_SET: &str =
  "Export identifier is not set. The variable must be bound to a named export.";

pub static INVALID_UTF8: &str = "String value contains invalid UTF-8 encoding.";

pub static EXPECTED_CSS_VAR: &str = "Expected a CSS custom property (variable) reference.";

pub static KEY_MUST_EVAL_TO_STRING: &str = "Style property key must evaluate to a string.";

pub static VALUE_NOT_EXPRESSION: &str = "Style value must evaluate to a static expression.";

pub static EVAL_RESULT_EXPECTED: &str = "Expected a value from evaluation result.";

pub static VAR_DECL_INIT_REQUIRED: &str = "Variable declaration must have an initializer.";

pub static OBJECT_KEY_MUST_BE_IDENT: &str = "Object key must be a static identifier.";

pub static THEME_VARS_MUST_BE_OBJECT: &str = "Theme variables must be defined as a plain object.";

pub static RULE_SET_EMPTY: &str = "The style rule set is empty. At least one rule is required.";

pub fn expected_call_expression(fn_name: &str) -> String {
  format!(
    "{}(): Expected a call expression. Ensure the value is a direct function call.",
    fn_name
  )
}

/// An object variable without a `default` key; matches Babel's text, naming the top-level
/// variable.
pub fn missing_default_value(key: &str) -> String {
  format!("Default value is not defined for {} variable.", key)
}

pub fn invalid_define_vars_function_value() -> String {
  "Function values in defineVars() must be zero-argument and return a static value supported by defineVars().".to_string()
}

pub fn cyclic_define_vars_reference(cycle: &str) -> String {
  format!(
    "Cyclic same-group references in defineVars() are not allowed: {}.",
    cycle
  )
}

pub fn unknown_define_vars_reference(key: &str, dependency: &str) -> String {
  format!(
    "Unknown same-group reference \"{}\" found while resolving \"{}\" in defineVars().",
    dependency, key
  )
}
