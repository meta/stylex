pub const STYLEX_ATTRS: &str = "attrs";
pub const STYLEX_CREATE: &str = "create";
pub const STYLEX_CREATE_THEME: &str = "createTheme";
pub const STYLEX_DEFAULT_MARKER: &str = "defaultMarker";
pub const STYLEX_UNSTABLE_CREATE_THEME_NESTED: &str = "unstable_createThemeNested";
pub const STYLEX_UNSTABLE_CONDITIONAL: &str = "unstable_conditional";
pub const STYLEX_UNSTABLE_DEFINE_CONSTS_NESTED: &str = "unstable_defineConstsNested";
pub const STYLEX_UNSTABLE_DEFINE_VARS_NESTED: &str = "unstable_defineVarsNested";
pub const STYLEX_DEFINE_CONSTS: &str = "defineConsts";
pub const STYLEX_DEFINE_MARKER: &str = "defineMarker";
pub const STYLEX_DEFINE_VARS: &str = "defineVars";
pub const STYLEX_ENV: &str = "env";
pub const STYLEX_FIRST_THAT_WORKS: &str = "firstThatWorks";
pub const STYLEX_KEYFRAMES: &str = "keyframes";
pub const STYLEX_POSITION_TRY: &str = "positionTry";
pub const STYLEX_PROPS: &str = "props";
pub const STYLEX_SX: &str = "sx";
pub const STYLEX_TYPES: &str = "types";
pub const STYLEX_VIEW_TRANSITION_CLASS: &str = "viewTransitionClass";
pub const STYLEX_WHEN: &str = "when";

/// The only key a function config carries in the reference implementation, which
/// spells one as `{ fn }`. Read where a folded function map materializes the
/// object a single config stands for.
pub const FUNCTION_CONFIG_FN_KEY: &str = "fn";
