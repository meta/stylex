pub static DEFAULT_INJECT_PATH: &str = "@stylexjs/stylex/lib/stylex-inject";
use once_cell::sync::Lazy;
use phf::phf_set;

// Using MDN data as a source of truth to populate the above sets
// by group in alphabetical order:

pub static VALID_CALLEES: phf::Set<&'static str> = phf_set! {

  "String", "Number", "Math", "Object", "Array"
};

/// The globals the fold recognises only where their *name* is written.
///
/// `Boolean` is never a callee — the reference implementation does not fold
/// `Boolean(x)` — but `[…].filter(Boolean)` is how an author reaches for it, so
/// it has to be recognised somewhere or it reads as a constant nothing
/// declared. Beside [`VALID_CALLEES`] because a value position claims both sets
/// and a reader looking for one will find the other.
pub static VALUE_ONLY_GLOBALS: phf::Set<&'static str> = phf_set! {
  "Boolean"
};

/// The static methods a fold may call on the globals in [`VALID_CALLEES`].
///
/// An allowlist and not a denylist, because `Object` carries reflective
/// methods -- `getPrototypeOf`, `getOwnPropertyDescriptor`, `create` -- that
/// hand back an object from the prototype chain. Any one of them is a step
/// from a plain object to `Function`, and so to arbitrary code inside the
/// compiler. A denylist over that surface can never be complete: the method
/// nobody listed is the next way out. Only methods that answer plain data are
/// listed.
///
/// One set per global rather than one shared set, so that a method of one
/// global cannot be called on another.
pub static VALID_STRING_METHODS: phf::Set<&'static str> = phf_set! {
  "fromCharCode",
  "fromCodePoint",
  "raw",
};

/// The `Number` statics a fold may call. See [`VALID_STRING_METHODS`].
pub static VALID_NUMBER_METHODS: phf::Set<&'static str> = phf_set! {
  "isFinite",
  "isInteger",
  "isNaN",
  "isSafeInteger",
  "parseFloat",
  "parseInt",
};

/// The `Math` statics a fold may call. See [`VALID_STRING_METHODS`].
///
/// `random` is absent on purpose: a build has to answer the same value every
/// time it reads the same source.
///
/// `f16round` is listed although no test names it. The engine the fold runs
/// carries it and Node 22 does not, so a case written against it would answer
/// one way here and another way there. It is listed because the surface is the
/// language's, not because this compiler exercises it.
pub static VALID_MATH_METHODS: phf::Set<&'static str> = phf_set! {
  "abs",
  "acos",
  "acosh",
  "asin",
  "asinh",
  "atan",
  "atan2",
  "atanh",
  "cbrt",
  "ceil",
  "clz32",
  "cos",
  "cosh",
  "exp",
  "expm1",
  "f16round",
  "floor",
  "fround",
  "hypot",
  "imul",
  "log",
  "log10",
  "log1p",
  "log2",
  "max",
  "min",
  "pow",
  "round",
  "sign",
  "sin",
  "sinh",
  "sqrt",
  "tan",
  "tanh",
  "trunc",
};

/// The `Object` statics a fold may call. See [`VALID_STRING_METHODS`].
///
/// Every one of these answers plain data: own values, own names, or a boolean.
/// Left out are the mutating statics (`assign`, `defineProperty`, `freeze`,
/// `seal`, `preventExtensions`) and the reflective ones (`create`,
/// `getPrototypeOf`, `setPrototypeOf`, `getOwnPropertyDescriptor`,
/// `getOwnPropertyDescriptors`), which hand back an object from the prototype
/// chain.
pub static VALID_OBJECT_METHODS: phf::Set<&'static str> = phf_set! {
  "entries",
  "fromEntries",
  "getOwnPropertyNames",
  "getOwnPropertySymbols",
  "groupBy",
  "hasOwn",
  "is",
  "isExtensible",
  "isFrozen",
  "isSealed",
  "keys",
  "values",
};

/// The `Array` statics a fold may call. See [`VALID_STRING_METHODS`].
pub static VALID_ARRAY_METHODS: phf::Set<&'static str> = phf_set! {
  "from",
  "isArray",
  "of",
};

/// Array methods that change their receiver.
///
/// These drive binding disqualification -- a name whose value one of them is
/// called on stops being a constant -- which is a different rule from the
/// static allowlists above. Some names appear in both, and that is a
/// coincidence of the language rather than a duplicate list.
pub static MUTATING_ARRAY_METHODS: phf::Set<&'static str> = phf_set! {
  "push",
  "pop",
  "shift",
  "unshift",
  "splice",
  "sort",
  "reverse",
  "fill",
  "copyWithin",
};

/// `Object` statics that change the object they are handed.
///
/// Read by binding disqualification, like [`MUTATING_ARRAY_METHODS`], and not
/// by the static allowlists above.
pub static MUTATING_OBJECT_METHODS: phf::Set<&'static str> = phf_set! {
  "assign",
  "defineProperty",
  "defineProperties",
  "setPrototypeOf",
};

pub static COMPILED_KEY: &str = "$$css";

pub static SPLIT_TOKEN: &str = "__$$__";

pub static ROOT_FONT_SIZE: i8 = 16;

pub static VAR_GROUP_HASH_KEY: &str = "__varGroupHash__";

pub static COLOR_FUNCTION_LISTED_NORMALIZED_PROPERTY_VALUES: Lazy<[&str; 9]> = Lazy::new(|| {
  [
    "oklch",
    "lch",
    "oklab",
    "hsla",
    "radial-gradient",
    "hwb",
    "lab",
    "clamp",
    "hsl",
  ]
});

/// CSS color functions that support relative color syntax, e.g.
/// `rgb(from red r g b)`. SWC's CSS parser cannot parse the relative form, so
/// values using it are normalized via spacing only (like the functions in
/// `COLOR_FUNCTION_LISTED_NORMALIZED_PROPERTY_VALUES`).
pub static COLOR_RELATIVE_VALUE_FUNCTIONS: Lazy<[&str; 10]> = Lazy::new(|| {
  [
    "rgb", "rgba", "hsl", "hsla", "hwb", "lab", "lch", "oklab", "oklch", "color",
  ]
});

pub static CSS_CONTENT_FUNCTIONS: Lazy<[&str; 7]> = Lazy::new(|| {
  [
    "attr(",
    "counter(",
    "counters(",
    "url(",
    "linear-gradient(",
    "image-set(",
    "var(--",
  ]
});

pub static CSS_CONTENT_KEYWORDS: Lazy<[&str; 11]> = Lazy::new(|| {
  [
    "normal",
    "none",
    "open-quote",
    "close-quote",
    "no-open-quote",
    "no-close-quote",
    "inherit",
    "initial",
    "revert",
    "revert-layer",
    "unset",
  ]
});

// TODO: Once we have a reliable validator, these property checks should be
// replaced with validators that can also validate the values.
pub static VALID_POSITION_TRY_PROPERTIES: Lazy<[&str; 40]> = Lazy::new(|| {
  [
    // anchor Properties
    "anchorName",
    // position Properties
    "positionAnchor",
    "positionArea",
    // inset Properties
    "top",
    "right",
    "bottom",
    "left",
    "inset",
    "insetBlock",
    "insetBlockEnd",
    "insetBlockStart",
    "insetInline",
    "insetInlineEnd",
    "insetInlineStart",
    // margin Properties
    "margin",
    "marginBlock",
    "marginBlockEnd",
    "marginBlockStart",
    "marginInline",
    "marginInlineEnd",
    "marginInlineStart",
    "marginTop",
    "marginBottom",
    "marginLeft",
    "marginRight",
    // size properties
    "width",
    "height",
    "minWidth",
    "minHeight",
    "maxWidth",
    "maxHeight",
    "blockSize",
    "inlineSize",
    "minBlockSize",
    "minInlineSize",
    "maxBlockSize",
    "maxInlineSize",
    // self alignment properties
    "alignSelf",
    "justifySelf",
    "placeSelf",
  ]
});

// Validation of `stylex.viewTransitionClass` function call
pub static VALID_VIEW_TRANSITION_CLASS_PROPERTIES: Lazy<[&str; 4]> =
  Lazy::new(|| ["group", "imagePair", "old", "new"]);

pub static CONSTS_FILE_EXTENSION: &str = ".const";

/// CSS custom property used in the logical float/clear value polyfill system.
/// Represents the logical "start" direction for float/clear, which maps to the
/// physical direction "left" in left-to-right (LTR) text direction and "right"
/// in right-to-left (RTL). This variable should be defined on the root or
/// relevant container elements, typically by a polyfill or runtime logic that
/// sets its value based on the current text direction.
///
/// This ensures that logical float/clear values behave correctly in both LTR
/// and RTL contexts.
pub static LOGICAL_FLOAT_START_VAR: &str = "--stylex-logical-start";

/// CSS custom property used in the logical float/clear value polyfill system.
/// Represents the logical "end" direction for float/clear, which maps to the
/// physical direction "right" in left-to-right (LTR) text direction and "left"
/// in right-to-left (RTL). This variable should be defined on the root or
/// relevant container elements, typically by a polyfill or runtime logic that
/// sets its value based on the current text direction.
///
/// This ensures that logical float/clear values behave correctly in both LTR
/// and RTL contexts.
pub static LOGICAL_FLOAT_END_VAR: &str = "--stylex-logical-end";

/// List of valid JSX runtime call names to check against when determining if a
/// call expression is a JSX runtime call. This includes both the classic
/// React.createElement and the newer jsx/jsxs calls, as well as their DEV
/// variants.
///
/// Supported:
/// - React runtime: `_jsx`, `_jsxs`
/// - React classic: `createElement`, `React.createElement`
/// - Vue runtime: `_createElementBlock`, `_createElementVNode`, `_createVNode`
///
///   The DEV variants (e.g., `_jsxDEV`) are also included by checking for a
///   "DEV" suffix,   which is commonly used in development builds of React to
///   provide additional debugging information.
pub static RUNTIME_JSX_CALL_NAMES: &[&str] = &[
  "jsx",
  "jsxs",
  "createElement",
  "createElementBlock",
  "createElementVNode",
  "createVNode",
];
