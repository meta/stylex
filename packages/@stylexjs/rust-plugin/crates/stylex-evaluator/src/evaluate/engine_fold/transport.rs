//! What crosses into the engine, and what it costs to carry.
//!
//! A name the guard resolved becomes a parameter of the printed arrow and its
//! value an argument to it, so a value travels *beside* the printed source
//! rather than inside it. This is that carriage: the shapes a value crosses as,
//! the two counts that bound them, and the last step into the engine's own
//! values, taken once there is an engine to build them in.
//!
//! Everything that can refuse refuses while the value is being measured, before
//! anything is printed or built — so the conversion with the engine in hand only
//! builds. Two walks, therefore, and one walk written once: the first produces
//! nothing at all and so copies nothing, the second produces the engine's own
//! values. What the two would otherwise have to agree about — which shapes the
//! bridge carries — is the kind of agreement this module exists to remove, so
//! they are one traversal under two [carriages](Carriage).

use std::hash::{Hash, Hasher};

use rustc_hash::{FxHashMap, FxHashSet};

use boa_engine::{
  Context, JsObject, JsString, JsValue, NativeFunction,
  native_function::NativeFunctionPointer,
  object::builtins::{JsArray, JsFunction},
  property::PropertyDescriptor,
};
use swc_core::{
  atoms::{Atom, Wtf8Atom},
  common::DUMMY_SP,
  ecma::ast::{
    ArrayLit, AssignPat, Expr, ExprOrSpread, KeyValueProp, Lit, ObjectLit, Pat, Prop, PropName,
    PropOrSpread,
  },
};

use crate::growable_stack::grown_per_level;
use stylex_ast::ast::convertors::{atom_utf16_length, is_js_undefined};
use stylex_ast::ast::factories::{create_binding_ident, create_ident};
use stylex_constants::constants::evaluation_errors::{
  bound_value_has_too_many_entries, bound_value_too_large,
};
use stylex_utils::hash::{stable_hash_unspanned, stable_hash_wide};
use stylex_utils::number::to_js_string;

use super::super::engine_stylex_functions::EngineCallable;
use super::engine::read;
use super::theme::var_group;
use super::{Ceilings, Decline, Depth, Totals};
use stylex_state::{evaluate_result_value::EvaluateResultValue, theme_ref::ThemeRef};
/// The one property name that is not a property when it is written as one.
///
/// `{ __proto__: x }` sets the object's prototype, so the object the source
/// describes carries no own property called `__proto__`. This evaluator's own
/// object form keeps it as an ordinary key, which is why the two directions of
/// this bridge have to agree on it explicitly: an expression written out reaches
/// the engine as text and the language drops it, and a value the guard resolved
/// is built property by property and would keep it.
const PROTOTYPE_KEY: &str = "__proto__";

/// The names the guard resolved and the values it resolved them to, ready to
/// cross the bridge inward.
///
/// The value travels *beside* the printed source rather than inside it: the
/// expression is printed as an arrow taking these names as parameters, and the
/// values are passed as arguments. Substituting a literal into the text instead
/// would reprint and reparse the whole value at every use site, and a value with
/// no literal spelling could not cross at all — where an argument keeps the
/// printed text the size of the expression however large the value is.
///
/// The author's own name is the parameter name. Nothing has to be rewritten for
/// that, and a callback parameter of the same name shadows it in the printed
/// arrow exactly as it does in the module the author wrote.
///
/// Chosen over registering the names on the engine because the engine is one
/// leaked instance per thread, shared by every file that thread compiles: a name
/// left behind or shadowed there would be a cross-file correctness bug.
pub(super) struct Transport {
  params: Vec<Atom>,
  values: Vec<Crossing>,
  totals: Totals,
  /// Whether anything that crossed was a theme reference.
  ///
  /// The cheap half of the question [`fold`](super::fold) asks of its answer: a
  /// group standing where a value belongs has to be handed back, and only a fold
  /// that carried one can produce one. Kept so an ordinary answer never pays for
  /// asking the engine.
  pub(super) read_a_theme_reference: bool,
}

impl Transport {
  /// An empty transport, holding the ceilings every value it carries is counted
  /// against.
  pub(super) fn new(ceilings: Ceilings) -> Self {
    Self {
      params: Vec::new(),
      values: Vec::new(),
      totals: Totals::new(ceilings),
      read_a_theme_reference: false,
    }
  }

  /// Records what `name` resolves to, or does nothing where the name is already
  /// carried.
  ///
  /// One parameter per name however often the expression reads it, because a
  /// repeated parameter is a syntax error in the arrow this is printed into. The
  /// second reading resolves to the same value — the evaluator memoises it — so
  /// dropping it loses nothing.
  ///
  /// The value is *measured* here rather than when the engine is in hand, so a
  /// value past a bound refuses before anything is printed or built. Measuring
  /// produces nothing, so what is kept is the value the evaluator already
  /// answered: it is built into the engine's own values once, in
  /// [`arguments`](Transport::arguments), rather than copied into a shape of
  /// this module's on the way there.
  pub(super) fn bind(
    &mut self,
    name: &Atom,
    value: EvaluateResultValue,
    depth: Depth,
  ) -> Result<(), Decline> {
    // Asked before the measurement rather than after, so a name read twice is
    // not measured twice — and does not count twice against what one fold may
    // carry.
    if self.holds(name) {
      return Ok(());
    }

    let mut measure = Measure {
      name,
      totals: &mut self.totals,
      read_a_theme_reference: &mut self.read_a_theme_reference,
    };

    cross(&mut measure, &value, depth.restart())?;

    self.carry(name, Crossing::Value(value));

    Ok(())
  }

  /// Records something the bridge already holds in its own shapes, under the
  /// name the printed source reads it through.
  ///
  /// Nothing here can refuse, which is the whole difference from [`bind`]: what
  /// it takes was not copied out of the module, so there is no text and no entry
  /// count to compare a ceiling to. A [StyleX function the engine may
  /// call](super::super::engine_stylex_functions) and a resolved arrow are what
  /// arrive this way.
  ///
  /// [`bind`]: Transport::bind
  pub(super) fn carry(&mut self, name: &Atom, crossing: Crossing) {
    if self.holds(name) {
      return;
    }

    self.params.push(name.clone());
    self.values.push(crossing);
  }

  /// The names as the parameters of the printed arrow.
  ///
  /// A value's name is a bare parameter and its value arrives as an argument. A
  /// function's is a parameter with the expression it was declared from as its
  /// default, because a function has no argument form to arrive as — so the
  /// printed source carries it and the call passes nothing.
  ///
  /// A default is evaluated where the parameter stands, so the order the names
  /// were carried in is the order they have to be printed in: a name a default
  /// reads is carried before the name whose default reads it, which is what
  /// walking a declaration before recording it buys.
  pub(super) fn parameters(&self) -> Vec<Pat> {
    self
      .printed_parameters()
      .map(|(name, default)| {
        let bound = Pat::Ident(create_binding_ident(create_ident(name)));

        match default {
          Some(source) => Pat::Assign(AssignPat {
            span: DUMMY_SP,
            left: Box::new(bound),
            right: Box::new(source.clone()),
          }),
          None => bound,
        }
      })
      .collect()
  }

  /// Each parameter as the two things it is printed from: the name it binds, and
  /// the declaration printed as its default where it has one.
  ///
  /// One walk, because two read it — the parameters themselves and the
  /// [key](Transport::parameters_key) the fold memo shares a compiled script on.
  /// A second copy would be a second answer to "what does this print", and the
  /// memo would hand one fold the script of another the day the two disagreed.
  ///
  /// A source crossing is the only one with a default: it carries a function,
  /// which has no argument form to arrive as. Everything else arrives as an
  /// argument, so its name is the whole of its parameter. Named rather than left
  /// to a wildcard, so the next crossing added has to say which of the two it is.
  fn printed_parameters(&self) -> impl Iterator<Item = (&Atom, Option<&Expr>)> {
    self
      .params
      .iter()
      .zip(&self.values)
      .map(|(name, crossing)| match crossing {
        Crossing::Source(source) => (name, Some(&**source)),
        Crossing::Value(_) | Crossing::Function(_) | Crossing::Namespace => (name, None),
      })
  }

  /// The parameter list as a hash, for the [fold memo](super::engine::FoldKey).
  ///
  /// Stands for the parameters [`parameters`](Transport::parameters) would
  /// print, without printing or building them: two folds whose calls hash alike
  /// share a compiled script only if this agrees too.
  ///
  /// A name alone is not enough for the crossing that prints a default, since
  /// the default is a whole declaration and two modules can bind the same name
  /// to different ones — so a source crossing contributes its expression as well
  /// as its name.
  pub(super) fn parameters_key(&self) -> u128 {
    stable_hash_wide(&PrintedParameters(self))
  }

  /// Whether a value is already travelling under `name`.
  ///
  /// A scan rather than a set, because an expression carries a handful of names
  /// and hashing them would cost more than comparing them.
  pub(super) fn holds(&self, name: &Atom) -> bool {
    self.params.contains(name)
  }

  /// The carried values as the arguments the printed arrow is called with.
  ///
  /// Built with the engine in hand, because an array and an object are engine
  /// objects and there is no way to make one without it. Everything that could
  /// refuse on size was answered when the value was measured, so this walk only
  /// builds — and it descends the very tree that measurement bounded, which is
  /// why it spends the same nesting budget rather than a second one of its own.
  pub(super) fn arguments(
    &self,
    engine: &mut Context,
    method: &Atom,
    depth: Depth,
    var_group: &JsFunction,
    dotted_prefixes: &FxHashMap<Atom, FxHashSet<Atom>>,
  ) -> Result<Vec<JsValue>, Decline> {
    let mut arguments = Vec::with_capacity(self.values.len());

    // Zipped rather than walked alone, because what a group has to nest is a
    // property of the name it travels under and not of the fold.
    for (name, crossing) in self.params.iter().zip(&self.values) {
      arguments.push(match crossing {
        Crossing::Value(value) => {
          let mut build = Build {
            engine,
            method,
            var_group,
            dotted_prefixes: dotted_prefixes.get(name),
          };

          cross(&mut build, value, depth.restart())?
        },
        // Built per fold rather than kept, because a function object belongs to
        // the realm that made it and costs one allocation — where keeping one
        // would tie a value's lifetime to the engine's, which is the arrangement
        // the memo beside it already had to be written around.
        Crossing::Function(call) => native_function(*call, engine),
        Crossing::Namespace => namespace(engine),
        // A function travels in the printed source as the parameter's default,
        // so passing nothing for it is what makes the default the value.
        Crossing::Source(_) => JsValue::undefined(),
      });
    }

    Ok(arguments)
  }
}

/// What travels under one parameter name of the printed arrow.
///
/// A value and a function cross by different routes, and the routes are not
/// interchangeable. A value is measured, kept, and built into the engine once
/// there is one, which keeps the printed text the size of the expression however
/// large the value is. A function of this compiler's own is handed over for the
/// engine to call rather than to read. A function the *module* declared has
/// neither form — there is no engine value a resolved arrow could be built into
/// before the engine exists, and nothing an argument could carry — so what
/// crosses is the source it was declared from, printed back where the parameter
/// stands.
///
/// Printing that one as a *default* rather than substituting it at each reading
/// is what keeps shadowing the language's answer instead of this walk's: a
/// callback parameter of the same name shadows the default exactly as it
/// shadowed the module binding, and a substitution would have had to work that
/// out for itself.
pub(super) enum Crossing {
  /// A value the evaluator answered, measured on the way in and built once.
  ///
  /// Refused in both directions, as stated rules rather than omissions: function
  /// configurations, callbacks, the environment object, an unresolved theme
  /// reference, and the AST-keyed map variants. A *resolved* theme reference
  /// crosses as the stand-in the engine reads members off — see
  /// [`theme`](super::theme) — because resolving it is what mutates compiler
  /// state and that happens before the bridge.
  Value(EvaluateResultValue),
  /// One of this compiler's own functions, reached by its own name.
  Function(NativeFunctionPointer),
  /// The whole callable surface of the StyleX namespace, reached as a property
  /// of it — see [`Reached`](super::super::engine_stylex_functions::Reached).
  Namespace,
  /// A function the module declared, printed back as the parameter's default.
  Source(Box<Expr>),
}

/// The printed parameter list, as something a hasher can read.
///
/// A borrowed view rather than a collected list, so taking the key allocates
/// nothing — which matters because the key is taken on every fold, including the
/// ones that go on to hit the memo and print nothing at all.
struct PrintedParameters<'a>(&'a Transport);

impl Hash for PrintedParameters<'_> {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.0.params.len().hash(state);

    for (name, default) in self.0.printed_parameters() {
      name.hash(state);
      // Hashed rather than walked into this stream, because a default is an
      // expression and the span-insensitive hash is what reads one.
      default.map(stable_hash_unspanned).hash(state);
    }
  }
}

/// What one walk over a resolved value does with it.
///
/// A value crosses in two walks and this is the difference between them. The
/// first runs before there is an engine — every bound is answered there, so
/// nothing oversized ever reaches one — and produces nothing at all, which is
/// what lets it measure a value without copying it. The second runs with the
/// engine in hand and produces the engine's own values directly.
///
/// One trait rather than two walks because the shapes the bridge carries are
/// exactly what two walks would have to agree about, and a table that has to
/// agree with another table is what this module exists to remove.
trait Carriage {
  /// What one value comes to: nothing while measuring, an engine value while
  /// building.
  type Value;
  /// What an object is assembled in, so neither walk has to collect its
  /// properties into a list of its own first.
  type Object;

  /// Counts one string — a value or a key — against what this walk may carry.
  fn counted_text(&mut self, text: &Wtf8Atom) -> Result<(), Decline>;
  /// Counts the elements or properties about to be walked.
  fn counted(&mut self, entries: usize) -> Result<(), Decline>;
  /// Carries a `defineVars` group, and records that one crossed — which is a
  /// fact about the whole expression as well as about this value.
  fn theme_reference(&mut self, theme: &ThemeRef) -> Result<Self::Value, Decline>;

  fn string(&mut self, text: &Wtf8Atom) -> Self::Value;
  fn number(&mut self, number: f64) -> Self::Value;
  fn boolean(&mut self, truth: bool) -> Self::Value;
  fn null(&mut self) -> Self::Value;
  fn undefined(&mut self) -> Self::Value;
  fn list(&mut self, items: Vec<Self::Value>) -> Self::Value;

  fn object(&mut self) -> Self::Object;
  /// Records one own property on the object under assembly.
  ///
  /// Total in both directions: the measuring walk has nothing to record, and the
  /// building walk writes into an object it has just made.
  fn property(&mut self, object: &mut Self::Object, key: &Wtf8Atom, value: Self::Value);
  fn built(&mut self, object: Self::Object) -> Self::Value;
}

/// The walk that only measures: what a value would cost to copy in, counted
/// against one running total, with nothing built.
///
/// The name is carried so a bound can say which binding to shorten — the size
/// belongs to what the name holds, and the same call on a smaller value folds.
struct Measure<'a> {
  name: &'a Atom,
  totals: &'a mut Totals,
  read_a_theme_reference: &'a mut bool,
}

impl Carriage for Measure<'_> {
  // Producing nothing is what measuring is, so a value is the unit type — and a
  // list of them allocates nothing, since a vector of zero-sized values never
  // reaches the allocator.
  type Value = ();
  type Object = ();

  fn counted_text(&mut self, text: &Wtf8Atom) -> Result<(), Decline> {
    self
      .totals
      .count_characters(atom_utf16_length(text) as u64)
      .map_err(|ceiling| Decline::rule(bound_value_too_large(self.name, ceiling)))
  }

  fn counted(&mut self, entries: usize) -> Result<(), Decline> {
    self
      .totals
      .count_entries(entries as u64)
      .map_err(|ceiling| Decline::rule(bound_value_has_too_many_entries(self.name, ceiling)))
  }

  fn theme_reference(&mut self, theme: &ThemeRef) -> Result<(), Decline> {
    *self.read_a_theme_reference = true;

    // The group's own hash is the one text of it that is written whatever the
    // expression reads, so it is what a group costs to carry. A member is
    // derived inside the engine as it is read, and what comes back is counted
    // against the outward total like every other answer.
    self.counted_text(&Wtf8Atom::from(theme.to_string_value().as_str()))
  }

  fn string(&mut self, _: &Wtf8Atom) {}
  fn number(&mut self, _: f64) {}
  fn boolean(&mut self, _: bool) {}
  fn null(&mut self) {}
  fn undefined(&mut self) {}
  fn list(&mut self, _: Vec<()>) {}

  fn object(&mut self) {}
  fn property(&mut self, _: &mut (), _: &Wtf8Atom, _: ()) {}
  fn built(&mut self, _: ()) {}
}

/// The walk that builds: the same shapes as the engine's own values, with
/// nothing left to refuse on size.
struct Build<'a> {
  engine: &'a mut Context,
  method: &'a Atom,
  var_group: &'a JsFunction,
  /// The dotted paths a group carried under this name has to answer with a
  /// stand-in rather than a variable, read off the source by the guard's walk.
  /// `None` where the walk read no chain through it, which is every name that
  /// holds something other than a group.
  dotted_prefixes: Option<&'a FxHashSet<Atom>>,
}

impl Carriage for Build<'_> {
  type Value = JsValue;
  type Object = JsObject;

  // Both counts were spent when the value was measured, and nothing has been
  // added to it since.
  fn counted_text(&mut self, _: &Wtf8Atom) -> Result<(), Decline> {
    Ok(())
  }

  fn counted(&mut self, _: usize) -> Result<(), Decline> {
    Ok(())
  }

  // A fact the measuring walk recorded, so this half only builds.
  fn theme_reference(&mut self, theme: &ThemeRef) -> Result<JsValue, Decline> {
    read(self.method, || {
      var_group(self.var_group, theme, self.dotted_prefixes, self.engine)
    })
  }

  fn string(&mut self, text: &Wtf8Atom) -> JsValue {
    JsValue::from(carry_string(text))
  }

  fn number(&mut self, number: f64) -> JsValue {
    JsValue::from(number)
  }

  fn boolean(&mut self, truth: bool) -> JsValue {
    JsValue::from(truth)
  }

  fn null(&mut self) -> JsValue {
    JsValue::null()
  }

  fn undefined(&mut self) -> JsValue {
    JsValue::undefined()
  }

  fn list(&mut self, items: Vec<JsValue>) -> JsValue {
    JsValue::from(JsArray::from_iter(items, self.engine))
  }

  fn object(&mut self) -> JsObject {
    JsObject::with_object_proto(self.engine.intrinsics())
  }

  fn property(&mut self, object: &mut JsObject, key: &Wtf8Atom, value: JsValue) {
    define(object, carry_string(key), value);
  }

  fn built(&mut self, object: JsObject) -> JsValue {
    JsValue::from(object)
  }
}

/// One value the evaluator answered, walked under `carriage`, or the reason it
/// does not cross.
///
/// A shape the bridge does not carry is not a refusal. The dispatch below the
/// fold owns those values and answers for them, and a rule fired here would stop
/// it ever being asked.
fn cross<C: Carriage>(
  carriage: &mut C,
  value: &EvaluateResultValue,
  depth: Depth,
) -> Result<C::Value, Decline> {
  // Room for the next level asked for at this one, as every walk this module
  // owns does. See `growable_stack`.
  grown_per_level(|| nested_value(carriage, value, depth))
}

/// One value, on the room [`cross`] asked for, and reached only through it — a
/// direct call would descend on no room at all.
fn nested_value<C: Carriage>(
  carriage: &mut C,
  value: &EvaluateResultValue,
  depth: Depth,
) -> Result<C::Value, Decline> {
  match value {
    EvaluateResultValue::Expr(expr) => cross_expr(carriage, expr, depth),
    // The evaluator answers an array either as a list of its own or as the array
    // literal it was written as. Both are one array here, which is the whole of
    // why the two dispatch arms that carried the array methods could disagree
    // about which names they knew: they were answering for the same value in two
    // shapes.
    EvaluateResultValue::Vec(items) => {
      let inner = depth.descend()?;

      carriage.counted(items.len())?;

      let mut list = Vec::with_capacity(items.len());

      for item in items {
        list.push(cross(carriage, item, inner)?);
      }

      Ok(carriage.list(list))
    },
    // A `defineVars` group, carried as the stand-in the engine reads members off
    // — see [`theme`](super::theme). It is the one of this compiler's own values
    // with a JavaScript form to cross as, and the form is a proxy rather than a
    // string because the group answers a member it never stored.
    EvaluateResultValue::ThemeRef(theme) => carriage.theme_reference(theme),
    _ => Err(Decline::NotACandidate),
  }
}

/// One evaluated expression, walked under `carriage`.
///
/// One level of the budget per node, leaves included, exactly as the guard's own
/// walk spends it — the two walk the same shapes and would otherwise disagree
/// about how deep the same value is.
fn cross_expr<C: Carriage>(
  carriage: &mut C,
  expr: &Expr,
  depth: Depth,
) -> Result<C::Value, Decline> {
  grown_per_level(|| nested_expr(carriage, expr, depth))
}

/// One expression, on the room [`cross_expr`] asked for, and reached only
/// through it — a direct call would descend on no room at all.
fn nested_expr<C: Carriage>(
  carriage: &mut C,
  expr: &Expr,
  depth: Depth,
) -> Result<C::Value, Decline> {
  let inner = depth.descend()?;

  match expr {
    Expr::Lit(Lit::Str(text)) => {
      carriage.counted_text(&text.value)?;

      Ok(carriage.string(&text.value))
    },
    Expr::Lit(Lit::Num(number)) => Ok(carriage.number(number.value)),
    Expr::Lit(Lit::Bool(truth)) => Ok(carriage.boolean(truth.value)),
    Expr::Lit(Lit::Null(_)) => Ok(carriage.null()),
    // The one value the grammar has no literal for, so an author spells it as a
    // name and the evaluator hands it back as one. A binding of that name never
    // arrives here — the evaluator refuses a shadowed `undefined` ahead of every
    // reader of this shape — and `void 0` is the same value under a different
    // spelling, which the evaluator has already answered as this identifier.
    Expr::Ident(ident) if is_js_undefined(ident) => Ok(carriage.undefined()),
    Expr::Array(ArrayLit { elems, .. }) => {
      carriage.counted(elems.len())?;

      let mut list = Vec::with_capacity(elems.len());

      for elem in elems {
        // A hole is `undefined` and a spread was refused where it was written, so
        // neither reaches a fold: the reference compiler refuses a method call on
        // both, and folding a hole as anything at all would write a value the
        // source does not describe.
        let Some(ExprOrSpread { spread: None, expr }) = elem else {
          return Err(Decline::NotACandidate);
        };

        list.push(cross_expr(carriage, expr, inner)?);
      }

      Ok(carriage.list(list))
    },
    Expr::Object(ObjectLit { props, .. }) => {
      carriage.counted(props.len())?;

      let mut object = carriage.object();

      for prop in props {
        let PropOrSpread::Prop(prop) = prop else {
          return Err(Decline::NotACandidate);
        };

        let Prop::KeyValue(KeyValueProp { key, value }) = prop.as_ref() else {
          return Err(Decline::NotACandidate);
        };

        let Some(key) = property_name(key) else {
          return Err(Decline::NotACandidate);
        };

        carriage.counted_text(&key)?;

        // `__proto__` written as a plain key sets the prototype rather than a
        // member, so the object the source describes has no own property of that
        // name. The evaluator keeps it as one, so it is dropped here — where an
        // expression written out reaches the engine as text and the language
        // drops it for us. Both paths then answer alike, and alike is what the
        // reference compiler answers.
        if key == PROTOTYPE_KEY {
          continue;
        }

        let value = cross_expr(carriage, value, inner)?;

        carriage.property(&mut object, &key, value);
      }

      Ok(carriage.built(object))
    },
    _ => Err(Decline::NotACandidate),
  }
}

/// A property name as the string the language reads it as, or `None` where it is
/// not a name the bridge carries.
fn property_name(key: &PropName) -> Option<Wtf8Atom> {
  match key {
    PropName::Ident(name) => Some(Wtf8Atom::from(&*name.sym)),
    PropName::Str(name) => Some(name.value.clone()),
    // A numeric key names the property its own string form spells, read by the
    // conversion every other number-to-string in this compiler uses rather than
    // by a spelling of its own.
    PropName::Num(number) => Some(Wtf8Atom::from(to_js_string(number.value).as_str())),
    // A computed key was evaluated before it reached a value, and a BigInt is not
    // a value this bridge carries in any position.
    _ => None,
  }
}

/// One of this compiler's own functions as an engine value.
fn native_function(call: NativeFunctionPointer, engine: &mut Context) -> JsValue {
  JsValue::from(NativeFunction::from_fn_ptr(call).to_js_function(engine.realm()))
}

/// The StyleX namespace as the object a printed `stylex.firstThatWorks(…)` reads
/// its function off.
///
/// Every callable function of the namespace rather than the one this call named,
/// so a fold naming two of them carries one object holding both — see
/// [`Reached::AsAProperty`](super::super::engine_stylex_functions::Reached).
fn namespace(engine: &mut Context) -> JsValue {
  let object = JsObject::with_object_proto(engine.intrinsics());

  for (property, call) in EngineCallable::namespace_properties() {
    let value = native_function(call, engine);

    define(&object, JsString::from(property), value);
  }

  JsValue::from(object)
}

/// One own property on an object this module just made.
///
/// Written into the object's own properties rather than defined through the
/// language's own step. Both objects the bridge builds are fresh and ordinary,
/// so the two agree on every question that step asks: the object is extensible,
/// and every property written here is writable and configurable, so a key
/// written a second time replaces the value in the slot the first writing made.
/// What the direct write removes is the answer to a question with one answer —
/// a throw that cannot happen, asked on every property of every object that
/// crosses.
///
/// A key written twice therefore keeps the place of its first writing and the
/// value of its last, which is what the language says and what the reference
/// implementation writes.
fn define(object: &JsObject, key: JsString, value: JsValue) {
  object.insert_property(
    key,
    PropertyDescriptor::builder()
      .value(value)
      .writable(true)
      .enumerable(true)
      .configurable(true),
  );
}

/// One resolved string as the engine's own string type.
///
/// Two readings of the same atom, because a JavaScript string literal can hold
/// an unpaired surrogate and no Rust `str` can. The engine's strings are UTF-16,
/// so the ill-formed reading carries such a value across *exactly* — where the
/// outward direction has to substitute the replacement character, since a
/// `Lit::Str` is what it has to land in. A valid value takes the direct reading
/// rather than being taken apart into code units it would only be rebuilt from.
fn carry_string(value: &Wtf8Atom) -> JsString {
  match value.as_str() {
    Some(text) => JsString::from(text),
    None => JsString::from(&value.to_ill_formed_utf16().collect::<Vec<u16>>()[..]),
  }
}

#[cfg(test)]
#[path = "tests/carried_shape_tests.rs"]
mod carried_shape_tests;
