//! One engine per thread, observed rather than assumed.
//!
//! Most of what the engine's design rests on is an argument about threads. The
//! engine is leaked because its collector lives in a thread-local of undefined
//! drop order; the [fold memo](../../../../../CONTEXT.md) is leaked with it; and
//! the resolved values cross as transport arguments rather than as names
//! registered on the engine *specifically* because one leaked instance is shared
//! by every file a thread compiles, so a name left behind would be read by a
//! later fold in another file.
//!
//! Every existing case folds on one thread in sequence, where a leaked global
//! and a correct fold answer the same thing and neither can be told from the
//! other. These run concurrently and give each thread an answer only it may
//! reach, so a value that arrived from somewhere else is a wrong value rather
//! than an indistinguishable right one — and where an answer cannot tell, the
//! engine's global object is asked directly.

use std::sync::Barrier;
use std::thread;

use super::source_evaluation::*;

/// How many threads fold at once.
///
/// Eight because the split below needs an even number and because it is above
/// the core count of an ordinary machine — threads that are descheduled mid-fold
/// interleave more ways than threads that each get a core.
const THREADS: usize = 8;

/// How many times each thread folds.
///
/// Enough that a thread is still folding while the others start and finish, so
/// the folds overlap rather than merely being started concurrently.
const FOLDS_PER_THREAD: usize = 50;

/// The name every thread binds, to a different value.
const MARKER: &str = "marker";

/// A name the engine has bound before any fold reaches it, as the control for
/// [`super::engine_fold::holds_a_global`].
///
/// Without it, a probe that answered "not bound" for everything would satisfy
/// the leak assertions while seeing nothing at all.
const A_NAME_THE_LANGUAGE_BINDS: &str = "Object";

/// One thread's fold: the source it hands the evaluator, the value it binds the
/// [marker](MARKER) to, and the answer only that thread may reach.
struct Fold {
  call: &'static str,
  marker: String,
  expected: String,
}

/// The fold thread `index` runs.
///
/// Two receivers, split evenly by parity: a string that concatenates the marker
/// and an array that joins it. Half and half because the receiver is what the
/// guard resolves first, and two threads holding different receiver shapes
/// exercise more of the transport at once than eight holding one.
///
/// The marker is bound to a name rather than written into the call, which is
/// what makes the case about threads at all: every thread prints the same source
/// for its half and differs only in the value travelling beside it, so an answer
/// carrying another thread's marker could only have come from another thread's
/// engine.
fn the_fold_for(index: usize) -> Fold {
  let call = match index.is_multiple_of(2) {
    true => "\"a\".concat(marker)",
    false => "[\"a\", marker].join(\"\")",
  };

  Fold {
    call,
    marker: format!("\"{index}\""),
    expected: format!("a{index}"),
  }
}

/// The fold running on the even thread beside the odd thread `index` — the one
/// binding a marker while `index` binds nothing.
fn the_neighbour_of(index: usize) -> Fold {
  the_fold_for(index - 1)
}

/// Folds thread `index`'s own source against its own marker and asserts it got
/// its own answer back.
#[track_caller]
fn assert_own_fold(index: usize) {
  let fold = the_fold_for(index);

  assert_eq!(
    folded_in_a_module_binding(MARKER, &fold.marker, fold.call),
    fold.expected,
    "thread {index} answered a value that is not its own"
  );
}

/// Runs `case` on each of [`THREADS`] threads at once, releasing them together
/// so the folds overlap, and re-raising any panic on the thread that started
/// them.
///
/// Released through a barrier rather than simply spawned, because spawning eight
/// threads in a loop can let the first finish before the last starts — which is
/// eight sequential folds wearing the shape of a concurrent one.
///
/// Scoped, so the barrier and the case are borrowed rather than reference
/// counted: nothing started here outlives the call.
fn on_every_thread(case: impl Fn(usize) + Sync) {
  let released = Barrier::new(THREADS);

  thread::scope(|scope| {
    let threads: Vec<_> = (0..THREADS)
      .map(|index| {
        // Borrowed rather than moved, so every thread reads the one barrier and
        // the one case.
        let released = &released;
        let case = &case;

        scope.spawn(move || {
          released.wait();
          case(index);
        })
      })
      .collect();

    for thread in threads {
      joined(thread);
    }
  });
}

/// Waits for `thread` and hands back what it answered, re-raising anything it
/// panicked with.
///
/// Re-raised rather than swallowed, so an assertion that failed over there reads
/// as a failure of the case that started it rather than as a join whose error
/// nobody looked at.
fn joined<T>(thread: thread::ScopedJoinHandle<'_, T>) -> T {
  match thread.join() {
    Ok(answer) => answer,
    Err(panic) => std::panic::resume_unwind(panic),
  }
}

/// Every thread answers its own fold, every time, while the others are folding.
///
/// This is the invariant the whole design rests on stated as an observation: a
/// `Context` reachable from two threads, or a memo entry keyed so two
/// differently bound values collide, would show up here as one thread reporting
/// another's marker.
#[test]
fn every_thread_answers_its_own_fold_while_the_others_fold() {
  on_every_thread(|index| {
    for _ in 0..FOLDS_PER_THREAD {
      assert_own_fold(index);
    }
  });
}

/// A fold leaves no name on the engine for the next fold to read.
///
/// The answer alone cannot tell — a leaked name and a name that was never
/// written fold to the same value — so the global object is asked directly,
/// which is the whole reason [`super::engine_fold::holds_a_global`] exists. It
/// is asked while seven other threads are folding markers of their own, so a
/// name that arrived from one of them is caught as well as one this thread left
/// behind itself.
///
/// The probe is controlled by a name the language does bind, because a probe
/// that answered "not bound" to everything would pass this while seeing nothing.
///
/// The other half is the guard: the threads that bind nothing refuse both the
/// source their neighbour is folding and the one they would fold themselves.
/// That refusal is what keeps a free name from ever being printed, so the two
/// halves together say a leaked name could neither be written nor read.
#[test]
fn a_fold_leaves_no_name_on_the_engine_for_another_fold_to_read() {
  on_every_thread(|index| {
    for _ in 0..FOLDS_PER_THREAD {
      if !index.is_multiple_of(2) {
        assert_deopts(the_neighbour_of(index).call);
        assert_deopts(the_fold_for(index).call);

        continue;
      }

      assert_own_fold(index);

      assert_eq!(
        super::engine_fold::holds_a_global(A_NAME_THE_LANGUAGE_BINDS),
        Some(true),
        "the probe cannot see a name the engine does bind, so it can see none"
      );

      assert_eq!(
        super::engine_fold::holds_a_global(MARKER),
        Some(false),
        "thread {index} left `{MARKER}` bound on its engine after folding"
      );
    }
  });
}

/// A thread that has folded nothing holds no engine, however many engines other
/// threads have already built.
///
/// The direct reading of "one per thread": the slot is a thread-local, so a
/// thread arriving after eight others have each built an engine still starts
/// from empty, and builds its own on the first fold that needs one. A slot that
/// had drifted into being shared would answer this the other way before it
/// answered anything else the wrong way.
///
/// The eight fold first, and each is asserted to hold an engine, because the
/// claim is about a thread arriving after engines exist: where nothing has been
/// built anywhere, an empty slot says nothing about whose it is.
#[test]
fn a_thread_that_has_folded_nothing_holds_no_engine() {
  on_every_thread(|index| {
    assert_own_fold(index);

    assert!(
      super::engine_fold::holds_an_engine(),
      "thread {index} folded without building an engine"
    );
  });

  thread::scope(|scope| {
    let observing = scope.spawn(|| {
      let held_before = super::engine_fold::holds_an_engine();

      assert_own_fold(0);

      (held_before, super::engine_fold::holds_an_engine())
    });

    let (held_before, held_after) = joined(observing);

    assert!(
      !held_before,
      "a thread that has folded nothing found an engine another thread built"
    );

    assert!(
      held_after,
      "a fold on a fresh thread built no engine of its own"
    );
  });
}
