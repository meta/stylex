//! What a case can ask about the thread's engine that no fold answers.
//!
//! Each reading below is the observable half of a claim the engine's design
//! rests on — built on first use, compiled once and reused, and leaving the
//! global object as it found it. None of them can be read out of what a fold
//! answers: a leaked name and a name that was never written produce the same
//! value, and a memo hit and a fresh compile produce the same value too, which
//! is the whole point of the memo and also why it needs a witness of its own.
//!
//! Beside the engine rather than inside it, because it is scaffolding: it runs
//! under a test and nowhere else, and a broken invariant here is a fault of
//! this compiler rather than a fold refusing — so each reading panics where the
//! engine will not answer, which no shipped path may do.

use boa_engine::JsString;

use super::ENGINE;

/// Whether this thread is holding an engine — the observable half of "built on
/// first use and never before".
///
/// Reads the slot rather than counting constructions, because what the claim is
/// about is whether an engine exists after an input the fold declined. Paired
/// with [`forget_engine`], since a case asserting an engine was *not* built has
/// to start from a thread that has none.
pub(in crate::evaluate) fn holds_an_engine() -> bool {
  ENGINE.with_borrow(|slot| slot.is_some())
}

/// How many distinct expressions this thread's engine has compiled, or none
/// where it holds no engine.
pub(in crate::evaluate) fn compiled_expressions() -> Option<usize> {
  ENGINE.with_borrow(|slot| slot.as_ref().map(|engine| engine.memo.len()))
}

/// Whether this thread's engine has `name` bound on its global object, or none
/// where it holds no engine.
///
/// The direct reading of the claim the transport was chosen for: a resolved
/// name crosses as an argument to a printed arrow rather than as a property
/// written onto the engine, so a fold leaves the global object exactly as it
/// found it.
///
/// Own properties rather than the whole prototype chain, because what is being
/// asked is whether a *fold* wrote something, and the names the language brings
/// with it are not that.
pub(in crate::evaluate) fn holds_a_global(name: &str) -> Option<bool> {
  ENGINE.with_borrow_mut(|slot| {
    slot.as_mut().map(|engine| {
      let global = engine.context.global_object();

      match global.has_own_property(JsString::from(name), &mut engine.context) {
        Ok(held) => held,
        // A global object that will not answer whether it holds a name is a
        // broken invariant rather than a fold refusing.
        Err(error) => panic!("the engine would not say whether `{name}` is bound: {error}"),
      }
    })
  })
}

/// Drops this thread's engine reference without dropping the engine, which is
/// what the slot's `ManuallyDrop` already does at thread exit and for the same
/// reason: the collector lives in a thread-local of its own and the drop order
/// between the two is not defined.
pub(in crate::evaluate) fn forget_engine() {
  ENGINE.with_borrow_mut(|slot| {
    slot.take();
  });
}
