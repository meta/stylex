pub static DEFAULT_INJECT_PATH: &str = "@stylexjs/stylex/lib/stylex-inject";
use once_cell::sync::Lazy;
use phf::phf_set;

// Using MDN data as a source of truth to populate the above sets
// by group in alphabetical order:

pub static VALID_CALLEES: phf::Set<&'static str> = phf_set! {

  "String", "Number", "Math", "Object", "Array"
};

/// Globals recognised only as a value, e.g. `[…].filter(Boolean)`; Babel never folds `Boolean(x)`.
pub static VALUE_ONLY_GLOBALS: phf::Set<&'static str> = phf_set! {
  "Boolean"
};

/// The `String` statics a fold may call. Per-global allowlists, not denylists: reflective
/// statics lead from plain data to `Function`.
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

/// The `Math` statics a fold may call; `random` is excluded so builds are deterministic.
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

/// The `Object` statics a fold may call; mutating and reflective statics are excluded.
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

/// Array methods that mutate their receiver, disqualifying the binding as a constant.
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

/// `Object` statics that mutate their argument; used like [`MUTATING_ARRAY_METHODS`].
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

/// Color functions supporting relative syntax (`rgb(from red r g b)`); SWC cannot parse it,
/// so these values are only re-spaced.
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

/// Custom property for the logical float/clear polyfill's start direction (`left` in LTR).
pub static LOGICAL_FLOAT_START_VAR: &str = "--stylex-logical-start";

/// Custom property for the logical float/clear polyfill's end direction (`right` in LTR).
pub static LOGICAL_FLOAT_END_VAR: &str = "--stylex-logical-end";

/// JSX runtime call names (React classic and automatic runtime, Vue); `DEV` variants are
/// matched by suffix.
pub static RUNTIME_JSX_CALL_NAMES: &[&str] = &[
  "jsx",
  "jsxs",
  "createElement",
  "createElementBlock",
  "createElementVNode",
  "createVNode",
];
