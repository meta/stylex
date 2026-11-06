//! The stack every walk that recurses on the bare thread stack runs on.
//!
//! Several descents recurse without a frame budget of their own: the evaluator's
//! own, the guard's walk into an expression on its way to the engine, the
//! carriage of a resolved value inward, the conversion of an engine value back,
//! the printing of the source the engine is handed, and the engine's parse of
//! it. Overflowing any of them aborts the process from inside an evaluation
//! whose whole contract is that it may fail, so none of them may run on whatever
//! the thread happened to have left over.
//!
//! Two ways of asking, and one rule that decides which a descent gets: **a
//! descent that can ask again at the next level does**. Every walk this compiler
//! owns can, so each spends a level and claims room for the one after it, and a
//! walk that stops early pays for the levels it actually descended. The two that
//! cannot are the ones this compiler does not write: SWC's printer clones the
//! expression and writes it out, and the engine's parser reads it back, and both
//! descend through a nested literal on whatever stack they were handed. So the
//! fold claims their whole descent up front, around them and around nothing
//! else, and sizes that claim from how deep the text they are handed actually
//! nests — see [`nesting_of`].

use stylex_structures::evaluation_depth::MAX_EVALUATION_DEPTH_LIMIT;
use swc_core::ecma::{
  ast::{Expr, Pat, Stmt},
  visit::{Visit, VisitWith},
};

/// Room to keep in front of a walk that asks again at the next level.
///
/// One level is not one frame and the frames are not the same size: a nested
/// `Math.max` call descends through argument collection and the callee
/// dispatch, and a debug build keeps every local of a long arm alive across the
/// recursive call. Measured against the most expensive arm, a debug level costs
/// tens of kilobytes, so the zone is sized in megabytes rather than in the
/// hundreds of kilobytes a uniform walk would need.
const HEADROOM_PER_LEVEL: usize = 1024 * 1024;

/// How much stack a walk that asks by the level is given when it is grown.
///
/// Sized to carry a few hundred levels of the most expensive arm in a debug
/// build in a single segment, so even a ceiling raised well past the default
/// allocates once rather than repeatedly.
const SEGMENT: usize = 16 * 1024 * 1024;

/// How much stack one nesting level of the print, the parse and the evaluation
/// is given.
///
/// Only those, because only those run on this claim — every walk this compiler
/// owns asks for its own room, and the print has unwound before the parse
/// begins, so what is sized here is the deeper of the two rather than the sum.
/// Measured on a debug build, which is the expensive one, by running the descent
/// on a thread too small to skip the claim, so that what carries it is the claim
/// and nothing else: four megabytes carry a hundred and forty-eight levels of
/// nested array literal and not a hundred and fifty-two, so a level costs about
/// twenty-eight kilobytes. Sixty-four kilobytes is more than twice that, which
/// is the margin between a ceiling that refuses and a ceiling that aborts.
///
/// **The measurement moves with the engine, and it has.** A level cost about
/// twenty kilobytes under the engine's 0.21 line and about twenty-eight under
/// its 0.22 one, for the same input — the parser's frames grew. That is why
/// `a_dead_operand_deeper_than_the_ceiling_is_never_entered` runs on a thread
/// far too small to skip the claim: an engine bump that outgrows this margin
/// fails there rather than in somebody's build.
const BYTES_PER_LEVEL: usize = 64 * 1024;

/// The deepest nesting [`grown_for_depth`] will carry.
///
/// The evaluation ceiling is clamped to this same number, so a claim made from
/// the ceiling always fits and only text nesting past what any walk spends can
/// ask for more. One name for both because they are one thing: how far down this
/// compiler is prepared to go.
pub const DEEPEST_CARRIED: usize = MAX_EVALUATION_DEPTH_LIMIT;

/// The largest claim [`grown_for_depth`] will ever make.
///
/// It is a consequence rather than a setting: [`DEEPEST_CARRIED`] levels at the
/// cost of a level each. Asserted rather than described, because the numbers it
/// multiplies live in two crates and the sentence that says what they come to is
/// not something a compiler checks.
const LARGEST_CLAIM: usize = 512 * 1024 * 1024;

const _: () = assert!(DEEPEST_CARRIED * BYTES_PER_LEVEL == LARGEST_CLAIM);

/// Runs `work` with room for a few more levels, for a caller that will ask
/// again at the next one.
///
/// A panic unwinding out of `work` — a StyleX diagnostic, which is how a
/// refusal in a position requiring a static value is reported — crosses the
/// boundary safely: `stacker` catches it on the grown stack and resumes the
/// unwind on the original one, so the payload the caller matches on survives.
pub fn grown_per_level<R>(work: impl FnOnce() -> R) -> R {
  stacker::maybe_grow(HEADROOM_PER_LEVEL, SEGMENT, work)
}

/// Runs `work` with room for a descent `levels` deep, for a caller that will not
/// ask again.
///
/// SWC's printer and the engine's parser are those callers: each recurses
/// through a nested literal without ever asking for room, so each gets only what
/// was there when it started. Claiming the whole descent up front is what turns "however
/// much this thread happened to have left" into a number, and claiming in
/// proportion to how deep the text nests is what ties that number to the descent
/// it has to carry: an input that nests further gets the stack its nesting
/// needs, and an ordinary one claims a couple of megabytes it almost always
/// already has, so nothing is allocated at all.
///
/// `levels` has to be one [`carriable`] answers for — a caller with deeper text
/// than that refuses instead of asking for a stack no allocation can satisfy.
/// Nothing is allocated when a segment that size is already underfoot, which is
/// what keeps a fold nested inside another fold from claiming twice.
///
/// **What it costs, and who pays.** Only a call the guard admitted: the claim is
/// made after the walk and around the printing and the engine's own work, so a
/// call expression declined for any reason never reaches it. That matters because the guard runs
/// on every call expression the evaluator visits and almost none of them fold —
/// wrapped around the walk as well, this mapped and unmapped a segment per
/// declined call, on a file that folds nothing at all. What is left is a cost on
/// folds, which is where the fold's cost belongs, and it buys the thing the
/// claim exists for: outgrowing it is a diagnostic instead of an abort.
pub fn grown_for_depth<R>(levels: usize, work: impl FnOnce() -> R) -> R {
  let claim = claim_for(levels);

  stacker::maybe_grow(claim, claim, work)
}

/// Whether a descent `levels` deep is one [`grown_for_depth`] can be asked for.
///
/// Asked here rather than by the caller comparing against the constant, so what
/// the claim can cover is decided in the module that decides what the claim is.
pub fn carriable(levels: usize) -> bool {
  levels <= DEEPEST_CARRIED
}

/// The stack [`grown_for_depth`] asks for at `levels`.
///
/// Named rather than written into the call, so what the claim comes to can be
/// asserted against instead of restated — a test that spelled the arithmetic out
/// again would agree with itself whichever number was wrong.
pub(crate) fn claim_for(levels: usize) -> usize {
  levels * BYTES_PER_LEVEL
}

/// How deeply `expr` nests, counted in the levels the printer and the parser
/// each recurse through.
///
/// The guard's walk spends a level on every expression it enters, so what it
/// walked is already covered by the ceiling it walked under. What is not is the
/// operand a short circuit never reaches: the engine decides the short circuit
/// itself, so a dead operand is printed and parsed whole without the walk ever
/// having spent a level on it. Measuring it is what turns the old margin — twice
/// the ceiling, enough until it was not — into a number the input answers for.
///
/// Counted at the three node kinds that nest without bound — an expression, a
/// statement and a binding pattern. Everything between two of them is a fixed
/// number of frames, so counting the three counts the descent: a callback whose
/// body is a thousand nested blocks costs the parser a thousand levels, and a
/// count that read only expressions would have priced it at one.
///
/// Stops descending once the count passes [`DEEPEST_CARRIED`], since the exact
/// depth of a tree nobody can carry changes nothing: the caller refuses either
/// way. And it asks for room at every level, as every walk this compiler owns
/// does, so measuring a deep tree cannot itself overflow.
pub fn nesting_of(expr: &Expr) -> usize {
  let mut nesting = Nesting {
    standing: 0,
    deepest: 0,
  };

  expr.visit_with(&mut nesting);

  nesting.deepest
}

/// The descent [`nesting_of`] measures with: how deep it currently is, and the
/// deepest it has been.
struct Nesting {
  standing: usize,
  deepest: usize,
}

impl Nesting {
  /// One level of the descent, and the room the level under it will need.
  ///
  /// One body for all three node kinds, so a kind added to the count cannot come
  /// to spend its level differently from the two beside it.
  fn one_level_of<N: VisitWith<Self> + ?Sized>(&mut self, node: &N) {
    self.standing += 1;
    self.deepest = self.deepest.max(self.standing);

    if carriable(self.standing) {
      grown_per_level(|| node.visit_children_with(self));
    }

    self.standing -= 1;
  }
}

impl Visit for Nesting {
  fn visit_expr(&mut self, expr: &Expr) {
    self.one_level_of(expr);
  }

  fn visit_stmt(&mut self, stmt: &Stmt) {
    self.one_level_of(stmt);
  }

  fn visit_pat(&mut self, pat: &Pat) {
    self.one_level_of(pat);
  }
}
