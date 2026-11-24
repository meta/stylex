# StyleX core: front-end boundary

This is the contract between a StyleX front end (SWC, oxc, anything that parses JS) and the
parser-independent core in `crates/stylex-core`. The code is real and tested
(`crates/stylex-core/src/tests.rs`, values copied from the Babel plugin's snapshots).
`crates/stylex-swc-adapter-sketch` shows #2008's SWC side calling it.

## The split

The front end owns everything that needs an AST:

- finding StyleX imports and calls, scopes, bindings, `env`, cross-file imports
- evaluating each call's arguments (#2008's `stylex-evaluator`)
- validation that reports source spans
- writing results back, hoisting, runtime injection, dev class names and `data-style-src`
- the `cond && styles.a` fan-out of `props`/`attrs`

The core owns everything that only needs values:

- flattening style objects, shorthand expansion, value normalization, class-name hashing
- CSS rule text, RTL variants, priorities
- variable, constant, theme and marker naming
- `keyframes`, `positionTry`, `viewTransitionClass` names and rules
- styleq merging for static `props`/`attrs`
- `processStylexRules`

The line sits where #2008 already has it: after `evaluate_*` returns and before the
`shared/transformers/*` functions run. Those functions read the evaluated `Expr` only as data.

## Types

```rust
pub enum Value {            // crates/stylex-value
  Undefined, Null, Bool(bool), Number(f64), String(String),
  Array(Vec<Value>),
  Object(IndexMap<String, Value>),   // JS own-property order; see object_from_js_entries
  Opaque(OpaqueId),                  // an expression the core carries but cannot read
}
pub struct OpaqueId(pub u32);        // index into a table the front end owns
```

`Opaque` is how the core stays AST-free while still handling dynamic styles. The front end
puts non-static leaves in its own table and hands the core their indexes. The core copies
indexes into its output. It never looks inside.

Errors come back as `CoreError { message }` with #2008's message text. The vendored crates
report errors by panicking (`stylex_panic!`); the boundary catches that, so callers get a
`Result`. Spans are the front end's job.

Rules come back as `RuleEntry(key, RuleFields { ltr, rtl, constKey, constVal }, priority)`,
the same type `processStylexRules` takes and the same shape as Babel's `metadata.stylex`.

## API

One `StyleXCore` per file. It holds `StyleXStateOptions`, a per-file value cache and the
rules registered so far (first registration of a key wins, as in #2008).

```rust
impl StyleXCore {
  fn new(options: StyleXStateOptions) -> Self;
  fn rules(&self) -> Vec<RuleEntry>;                       // all rules so far, in order
  fn export_id(file_name: &str, export_name: &str) -> String; // "file//export"

  // stylex.create
  fn create(&mut self, namespaces: &IndexMap<String, Namespace>) -> CoreResult<CreateOutput>;

  // variables and themes
  fn define_vars(&mut self, export_id: &str, vars: &Value) -> CoreResult<ObjectOutput>;
  fn define_consts(&mut self, export_id: &str, consts: &Value) -> CoreResult<ObjectOutput>;
  fn create_theme(&mut self, theme_vars: &Value, overrides: &Value) -> CoreResult<ObjectOutput>;
  fn define_marker(&self, export_id: &str) -> ObjectOutput;

  // named rules
  fn keyframes(&mut self, frames: &Value) -> CoreResult<NamedRule>;
  fn position_try(&mut self, styles: &Value) -> CoreResult<NamedRule>;
  fn view_transition_class(&mut self, styles: &Value) -> CoreResult<NamedRule>;
}

fn props(args: &[PropsArg]) -> Object;   // { className, style, data-style-src }
fn attrs(args: &[PropsArg]) -> Object;   // { class, style: "a:b;", data-style-src }
fn process_stylex_rules(rules: &[RuleEntry], config: Option<&ProcessStylexRulesConfig>) -> String;
```

### stylex.create

```rust
pub enum Namespace {
  Static(Object),
  Dynamic { params: Vec<String>, body: Object },
}
pub struct CreateOutput {
  pub namespaces: IndexMap<String, CompiledNamespace>,
  pub rules: Vec<RuleEntry>,          // new rules from this call
}
pub struct CompiledNamespace {
  pub classes: IndexMap<String, Option<String>>, // key -> "x1 x2" | null, then "$$css"
  pub class_paths: IndexMap<String, Vec<String>>, // class -> authored key path
  pub dynamic: Option<Vec<DynamicLeaf>>,
}
pub struct DynamicLeaf {
  pub var_name: String,               // "--x-color" or "--x-<hash>"
  pub path: Vec<String>,
  pub value: OpaqueId,
  pub unit_suffix: Option<&'static str>, // "px", "ms"... for numbers at runtime
}
```

Static namespaces: evaluate the object, pass it. `CompiledNamespace::to_value()` is the object
to write back (`{ kMwMTN: "xju2f9n", $$css: true }`). Keys are minified per
`enableMinifiedKeys`.

Dynamic namespaces (`root: (color) => ({ color })`): evaluate the arrow's object body with the
parameters unbound. Every leaf the evaluator cannot fold becomes `Value::Opaque`. A leaf is a
whole property value, as in #2008's `evaluate_partial_object_recursively`. The core swaps each
opaque leaf for `var(--x-…)`, compiles the result like a static namespace, adds the
`@property --x-… { syntax: "*"; inherits: … }` rules, and lists the leaves. The front end then
emits the runtime function:

```js
(color) => [compiled, { "--x-color": color != null ? color : undefined }]
```

with `(v => typeof v === "number" ? v + unit : v != null ? v : undefined)(expr)` when
`unit_suffix` is set. The Babel plugin on main also guards each class by the leaf that controls it
(`color != null ? "x14rh7hd" : ""`) and hoists the static part.
`CompiledNamespace::controlling_leaf(class)` gives that leaf. Whether a guard is needed
depends on the expression (`is_safe_to_skip_null_check`), so the front end decides.

Nested `keyframes()`/`positionTry()` calls inside a `create` argument: the front end's
evaluator calls `core.keyframes(...)` when it meets one and folds it to the returned name.
The rule is registered in the same `StyleXCore`, before the create rules, which is #2008's
order (`take_nested_rules_before`).

### defineVars, defineConsts, createTheme, defineMarker

`export_id` is `StyleXCore::export_id(file_name_for_hashing, export_name)`. The file name is
the front end's job (`unstable_moduleResolution`, see #2008's `get_filename_for_hashing`).

- `define_vars` returns the tokens object (`{ color: "var(--xwx8imx)", __varGroupHash__: … }`).
  Typed values (`stylex.types.color(...)`) are passed as the `{ syntax, value }` objects the
  `types` functions evaluate to.
- `define_consts` returns the values and registers const rules (`constKey`, `constVal`).
- `create_theme` takes the imported `defineVars` object as the front end resolved it.
  Cross-file resolution of `vars` is front-end work.
- `define_marker` returns `{ [id]: id, $$css: true }` and no rule.

### keyframes, positionTry, viewTransitionClass

Each returns `NamedRule { name, rule }`. Fold the call to `name`.

### props / attrs

```rust
pub enum PropsArg { Style(Object), Nullish }
```

Pass each argument as the compiled namespace object (`to_value()` of a static namespace) or
`Nullish` for `null`, `undefined`, `false`. For conditional arguments, call once per
combination, as #2008's `stylex_merge` does. Dynamic namespaces stay runtime calls.

### processStylexRules

`process_stylex_rules(&core.rules(), Some(&config))` gives the stylesheet. It is #2008's
`src/stylesheet` unchanged, now its own crate.

## How #2008's SWC front end calls it

The adapter is three functions: `expr_to_value(&Expr, &mut Vec<Expr>) -> Value`,
`value_to_expr(&Value, &[Expr]) -> Expr` and the call-site switch. They are in
`crates/stylex-swc-adapter-sketch/src/lib.rs` (580 lines, tests pass against swc_core 84.0.3).

To wire it into #2008 itself, each producer keeps its evaluation and swaps the transformer:

| #2008 call site | today | with the core |
|---|---|---|
| `transform_stylex_keyframes_call.rs` | `stylex_keyframes(&EvaluateResultValue, &mut StateManager)` | `core.keyframes(&expr_to_value(expr))` |
| `transform_stylex_position_try_call.rs` | `stylex_position_try(...)` | `core.position_try(...)` |
| `transform_stylex_view_transition_class_call.rs` | `stylex_view_transition_class(...)` | `core.view_transition_class(...)` |
| `transform_stylex_define_vars_call/mod.rs` | `stylex_define_vars(&value, &mut state)` | `core.define_vars(state.export_id, ...)` |
| `transform_stylex_define_consts_call.rs` | `stylex_define_consts(...)` | `core.define_consts(...)` |
| `transform_stylex_create_theme_call.rs` | `stylex_create_theme(...)` | `core.create_theme(...)` |
| `transform_define_marker_call.rs` | inline hashing | `core.define_marker(export_id)` |
| `transform_stylex_create_call/mod.rs` | `evaluate_stylex_create_arg` then `stylex_create_set` | evaluate, map arrow namespaces to `Namespace::Dynamic`, `core.create(...)` |
| `stylex_merge.rs` (props/attrs) | `props(&[ResolvedArg])` | `stylex_core::props(&[PropsArg])` per combination |
| `node.rs` `processStylexRules` | `crate::stylesheet::...` | `stylex_core::process_stylex_rules` |

`StateManager` keeps owning the rules it emits. It either reads `core.rules()` at the end or
appends each call's `rules` as it goes. The `RuleEntry` type is the one #2008 already uses.

One real conflict: the carved `stylex_structures` changes `EnvEntry::Expr(Expr)` to
`EnvEntry::Value(Value)` and `JSFunction` to `Fn(Vec<Value>) -> Value`. #2008's evaluator
reads env values as `Expr`. It would convert at that read (`value_to_expr`), or the options
would split so `env` lives in a front-end struct. See `reports/core-extract.md`.

## How an oxc front end calls it

The same way. Evaluate with an oxc evaluator to `Value` (or convert oxc's evaluated AST
literals), call the core, emit with oxc's `AstBuilder`. No oxc type crosses the boundary, so
the core does not pin an oxc version.

## Not in the core (yet)

- Validation with spans (`validators.rs`, 975 lines in #2008). The core raises #2008's
  messages for illegal values, but not every structural check (`validate_namespace`).
- Dev class names, `data-style-src` labels and `debug` source-map data (need the variable
  name, file name and lines).
- The `legacy-expand-shorthands` re-keying of dynamic leaves and the explicit-nullish-fallback
  class guard. `controlling_leaf` covers the default mode.
- `createThemeNested`, `defineVarsNested`, `defineConstsNested`, `stylex.when`, atoms,
  `stylex.types` call evaluation (front end builds the `{ syntax, value }` object), test mode.
