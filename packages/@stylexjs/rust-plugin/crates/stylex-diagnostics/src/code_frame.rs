use anyhow::Error;
use log::{debug, warn};
use smallvec::SmallVec;
use std::{
  cell::Cell,
  fs,
  panic::{self, AssertUnwindSafe, UnwindSafe},
  path::Path,
  sync::{Arc, Once, OnceLock},
};
use stylex_ast::ast::convertors::{convert_concat_to_tpl_expr, convert_simple_tpl_to_str_expr};
use stylex_macros::{panic_macros::__stylex_panic, stylex_error::StyleXError, stylex_panic};
use swc_compiler_base::{PrintArgs, SourceMapsConfig, TransformOutput, parse_js, print};
use swc_config::is_module::IsModule;
use swc_core::{
  atoms::Atom,
  common::{
    DUMMY_SP, EqIgnoreSpan, FileName, Mark, SourceFile, SourceMap, Span, Spanned, SyntaxContext,
    errors::{Handler, *},
    util::take::Take,
  },
  ecma::{ast::*, codegen::Config, transforms::typescript::strip, utils::DropSpan, visit::*},
};

use crate::{declaration_span::find_declaration_span, state::DiagnosticState};
use stylex_regex::regex::URL_REGEX;
use stylex_state_index::key_span_index::{CallLookup, NamespaceKeyQuery};
use stylex_utils::{hash::stable_hash_wide, swc::syntax_for_file};

/// One diagnostic's view of a file: the source map that the frame quotes from,
/// and the handler that draws the caret.
///
/// Every frame shares one process-wide source map, which is why a source goes
/// in through `register_source_once` and not through
/// `new_source_file`. Read that method for the cost of a second registration.
pub struct CodeFrame {
  source_map: Arc<SourceMap>,
  handler: Handler,
}

static SOURCE_MAP: OnceLock<Arc<SourceMap>> = OnceLock::new();
static DIAGNOSTIC_PANIC_HOOK: Once = Once::new();

thread_local! {
  static SUPPRESS_DIAGNOSTIC_PANIC_HOOK: Cell<bool> = const { Cell::new(false) };
}

fn install_diagnostic_panic_hook() {
  DIAGNOSTIC_PANIC_HOOK.call_once(|| {
    let previous_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
      let suppress = SUPPRESS_DIAGNOSTIC_PANIC_HOOK.with(Cell::get);
      if !suppress {
        previous_hook(panic_info);
      }
    }));
  });
}

fn catch_diagnostic_unwind<F, T>(operation: F) -> std::thread::Result<T>
where
  F: FnOnce() -> T + UnwindSafe,
{
  install_diagnostic_panic_hook();

  let previous_suppression = SUPPRESS_DIAGNOSTIC_PANIC_HOOK.with(|suppress| suppress.replace(true));
  let result = panic::catch_unwind(operation);
  SUPPRESS_DIAGNOSTIC_PANIC_HOOK.with(|suppress| suppress.set(previous_suppression));

  result
}

/// The process-global map every span in this compiler is resolved against.
///
/// Reached on its own so printing can have it without a [`CodeFrame`]: building
/// one constructs a diagnostic handler with a boxed emitter and probes the
/// terminal for colour support, none of which printing uses.
fn shared_source_map() -> Arc<SourceMap> {
  SOURCE_MAP
    .get_or_init(|| Arc::new(SourceMap::default()))
    .clone()
}

impl CodeFrame {
  pub(crate) fn new() -> Self {
    let source_map = shared_source_map();

    let handler =
      Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(source_map.clone()));

    Self {
      source_map,
      handler,
    }
  }

  pub(crate) fn create_error<'a>(&'a self, span: Span, message: &str) -> DiagnosticBuilder<'a> {
    let prefixed_message = format!("[StyleX] {}", message);
    let mut diagnostic = self.handler.struct_span_err(span, &prefixed_message);

    let urls = URL_REGEX
      .find_iter(message)
      .filter_map(|m| m.ok().map(|m| m.as_str()))
      .collect::<Vec<_>>();

    let note = format!("\n{}", urls.join("\n"));

    diagnostic.warn("Line number isn't real, it's just a placeholder, Please check the actual line number in your editor.");

    diagnostic.note(note.as_str());

    diagnostic
  }

  /// Registers `source` for `file_name` unless the shared source map already
  /// holds it. Returns `Err` only when producing the source itself failed.
  ///
  /// The closure is only called on a miss, so a caller whose source is expensive
  /// to produce -- a clone of the module's text, or a read of the file -- pays
  /// for it once per module rather than once per lookup.
  ///
  /// The guard is a linear scan of the source map's file list. That list grows by
  /// one entry per distinct *content* a process registers, which for a file that
  /// is not being edited is one -- see `memoize_module`, which is the
  /// registration this guard has to agree with. That is the trade this makes: a
  /// string compare per registered module, against the copy of the module's text
  /// it replaces. A long-lived process transforming five thousand modules pays
  /// fifty thousand name comparisons where it used to accumulate fifty thousand
  /// source files.
  /// The file already registered for `file_name` whose text is `source`, if the
  /// shared map holds one.
  ///
  /// `get_source_file` answers with the first file registered under the name,
  /// which after a candidate failed to parse is a text nothing quotes. Asking
  /// for the content as well is what keeps a watch-mode process from appending
  /// one copy of such a file per save. The scan is the same one
  /// `get_source_file` makes, with a content compare on the names that match.
  fn registered_source_file(&self, file_name: &FileName, source: &str) -> Option<Arc<SourceFile>> {
    self
      .source_map
      .files()
      .iter()
      .find(|file| *file.name == *file_name && file.src.as_str() == source)
      .cloned()
  }

  fn register_source_once(&self, file_name: &FileName, source: &str) {
    if self.source_map.get_source_file(file_name).is_some() {
      return;
    }

    self
      .source_map
      .new_source_file(file_name.clone().into(), source.to_owned());
  }

  /// Same as [`Self::register_source_once`], for a source that has to be made
  /// first and can fail in the making. Returns `Err` only when the making
  /// failed.
  ///
  /// The guard here is the only guard. It registers the file itself rather
  /// than calling `register_source_once`, which would scan the same list a
  /// second time on the path that has already paid for one scan.
  fn register_produced_source_once(
    &self,
    file_name: &FileName,
    source: impl FnOnce() -> Result<String, Error>,
  ) -> Result<(), Error> {
    if self.source_map.get_source_file(file_name).is_some() {
      return Ok(());
    }

    self
      .source_map
      .new_source_file(file_name.clone().into(), source()?);

    Ok(())
  }

  pub(crate) fn get_span_line_number(&self, span: Span) -> usize {
    self.source_map.lookup_char_pos(span.lo).line
  }

  /// Emits the diagnostic behind a panic boundary: the code frame is a
  /// best-effort aid, so a source-map lookup panic (e.g. a span whose byte
  /// offsets fall inside a multi-byte character) must never replace the error
  /// being reported.
  pub(crate) fn emit_error(&self, span: Span, message: &str) {
    let emitted = catch_diagnostic_unwind(AssertUnwindSafe(|| {
      self.create_error(span, message).emit();
    }));

    if emitted.is_err() {
      warn!("Failed to emit the code frame for error: {}", message);
    }
  }

  /// Like `get_span_line_number`, but behind the same panic boundary as
  /// `emit_error` and `None` for dummy spans ("location unknown").
  pub fn try_get_span_line_number(&self, span: Span) -> Option<usize> {
    if span.is_dummy() {
      return None;
    }

    catch_diagnostic_unwind(AssertUnwindSafe(|| self.get_span_line_number(span))).ok()
  }
}

fn read_source_file(file_name: &FileName) -> Result<String, std::io::Error> {
  match file_name {
    FileName::Real(path) => fs::read_to_string(path),
    FileName::Custom(path) => fs::read_to_string(path),
    FileName::Url(url) => fs::read_to_string(Path::new(url.path())),
    _ => Err(std::io::Error::other("Unsupported file name type")),
  }
}

pub fn build_code_frame_error<'a>(
  wrapped_expression: &'a Expr,
  fault_expression: &'a Expr,
  error_message: &'a str,
  state: &mut impl DiagnosticState,
) -> &'a str {
  match get_span_from_source_code(wrapped_expression, fault_expression, state) {
    Ok((code_frame, span)) => {
      code_frame.emit_error(span, error_message);
    },
    Err(error) => warn_no_code_frame(&error, state.get_filename(), fault_expression),
  }

  error_message
}

/// Reports that the code frame itself could not be built.
///
/// The full expression is only worth printing to somebody who turned debug
/// logging on; everyone else gets the file and a pointer to that switch.
fn warn_no_code_frame(error: &Error, filename: &str, fault_expression: &Expr) {
  if log::log_enabled!(log::Level::Debug) {
    debug!(
      "Failed to generate code frame error: {:?}. File: {}. Expression: {:?}.",
      error, filename, fault_expression,
    );
  } else {
    warn!(
      "Failed to generate code frame error: {:?}. File: {}. For more information enable debug logging.",
      error, filename,
    );
  }
}

/// Finds the span (source location) of a target expression within the source
/// code. Uses caching to avoid redundant AST traversals for the same
/// expression.
///
/// # Arguments
/// * `wrapped_expression` - The parent expression containing the target
/// * `target_expression` - The specific expression to locate
/// * `state` - Mutable reference to the state manager (for caching)
///
/// # Returns
/// A tuple of (CodeFrame, Span) where CodeFrame contains the source map for
/// error display
pub fn get_span_from_source_code(
  wrapped_expression: &Expr,
  target_expression: &Expr,
  state: &mut impl DiagnosticState,
) -> Result<(CodeFrame, Span), Error> {
  // Panic boundary: locating a span re-reads, re-prints, and re-parses the
  // module purely to improve diagnostics; a panic anywhere in there must
  // degrade to "no code frame", never abort the compilation.
  locate_span_with_panic_boundary(|| {
    get_span_from_source_code_impl(wrapped_expression, target_expression, state)
  })
}

/// Runs a span-locating closure behind the diagnostic panic boundary,
/// degrading a panic to a regular "no code frame" error.
fn locate_span_with_panic_boundary(
  locate: impl FnOnce() -> Result<(CodeFrame, Span), Error>,
) -> Result<(CodeFrame, Span), Error> {
  catch_diagnostic_unwind(AssertUnwindSafe(locate)).unwrap_or_else(|payload| {
    let detail = payload
      .downcast_ref::<String>()
      .cloned()
      .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_owned()))
      .unwrap_or_else(|| "<no message>".to_owned());
    Err(anyhow::anyhow!(
      "Panicked while locating the source span for a diagnostic: {detail}"
    ))
  })
}

fn get_span_from_source_code_impl(
  wrapped_expression: &Expr,
  target_expression: &Expr,
  state: &mut impl DiagnosticState,
) -> Result<(CodeFrame, Span), Error> {
  // A refusal about a binding is reported against that binding's declaration --
  // see `frame_declaration_of` -- and the two answers for one expression are
  // different positions, so they are cached under different keys.
  //
  // Hashed once rather than once per branch. The expression's key is the input
  // to both the framed-declaration lookup and to whichever cache key comes out
  // of it, and hashing it twice is a whole-subtree walk -- paid on every
  // annotation in the module as soon as one refusal has been recorded, which a
  // non-fatal deopt does routinely. A build that recorded none never hashes here
  // at all, which is what `has_framed_declarations` was for.
  let memo = state.diagnostic_memo();
  let expression_key = memo
    .has_framed_declarations()
    .then(|| compute_cache_key(target_expression));
  let framed_declaration = expression_key.and_then(|key| memo.framed_declaration(key).cloned());
  let cache_key = match (expression_key, framed_declaration.as_ref()) {
    (Some(key), Some(name)) => compute_declaration_cache_key(key, name),
    (Some(key), None) => key,
    (None, _) => compute_cache_key(target_expression),
  };

  let cached_span = memo.cached_span(cache_key);
  let file_name = FileName::Custom(state.get_filename().to_owned());

  // Check what a previous lookup remembered first -- it saves the AST work below.
  if let Some(cached_span) = cached_span {
    let code_frame = load_code_frame_from_cache_for_state(&file_name, state)?;
    return Ok((code_frame, cached_span));
  }

  let code_frame = CodeFrame::new();

  let span = with_memoized_module(
    wrapped_expression,
    target_expression,
    state,
    &file_name,
    &code_frame,
    |module| match framed_declaration.as_ref() {
      // A name the module does not declare after all falls back to the
      // expression: the declaration search reads the *re-parsed* module, whose
      // text can be the compiled module printed back out rather than the file
      // the reference was resolved against.
      Some(name) => match find_declaration_span(module, name) {
        declaration if declaration.is_dummy() => find_expression_span(module, target_expression),
        declaration => declaration,
      },
      None => find_expression_span(module, target_expression),
    },
  )?;

  // Cache the result for future lookups
  state
    .diagnostic_memo_mut()
    .insert_cached_span(cache_key, span);

  Ok((code_frame, span))
}

/// Records that the refusal raised on `fault_expression` is about the binding
/// `name`, so [`get_span_from_source_code`] frames that binding's declaration.
///
/// The declaration's own span cannot be handed over instead. It indexes the
/// compiler's source map, and the frame's positions are in its own -- see
/// `declaration_span`, which is where the name is turned back into a position.
///
/// Recorded against the expression rather than as a single "current refusal",
/// because a refusal is not always the end of a build: a dynamic style's value
/// falls through to an inline style, and a later diagnostic about something else
/// must not be pointed at this binding.
///
/// The key is [`compute_cache_key`], which hashes the expression *including its
/// span* -- so what reaches the frame has to be the node the refusal was raised
/// on, and not a copy of it. It is: a refusal stores the expression on the
/// evaluation state and every caller hands that same value to the frame. A
/// mismatch is not silent corruption but a silent no-op, and the diagnostic
/// falls back to naming the read, which is what it named before any of this.
pub fn frame_declaration_of(
  name: &Atom,
  fault_expression: &Expr,
  state: &mut impl DiagnosticState,
) {
  state
    .diagnostic_memo_mut()
    .frame_declaration(compute_cache_key(fault_expression), name.clone());
}

/// The binding a refusal on `fault_expression` was recorded against, if one was.
///
/// The read side of [`frame_declaration_of`]: hashing the expression is what the
/// two have to agree on, so neither spells the key. A build that refused nothing
/// answers without hashing at all.
///
/// Public because the pair is: a crate that lets a caller record a fact and not
/// read it back has half an interface, and the write side already has a
/// production caller in the evaluator. What reads this one is that same
/// evaluator's tests, which assert which binding a refusal framed.
///
/// No production caller of its own. `get_span_from_source_code_impl` inlines the
/// same two steps, because it needs the expression's key for the cache key as
/// well, and hashing a whole subtree twice to answer one question was the cost
/// `has_framed_declarations` had been added to avoid.
pub fn framed_declaration_of(
  fault_expression: &Expr,
  state: &impl DiagnosticState,
) -> Option<Atom> {
  let memo = state.diagnostic_memo();

  if !memo.has_framed_declarations() {
    return None;
  }

  memo
    .framed_declaration(compute_cache_key(fault_expression))
    .cloned()
}

/// Computes a cache key for an expression based on its type and structure.
///
/// 128 bits, because the read side acts on a hit alone: `cached_span` returns a
/// span and the caller turns it straight into a `file:line`. Unlike the
/// evaluator's memo, a collision here is *directly observable* -- a style
/// annotated with another style's line number -- so the width is the only thing
/// standing behind it.
fn compute_cache_key(expr: &Expr) -> u128 {
  stable_hash_wide(&(std::mem::discriminant(expr), expr))
}

/// The key for the *declaration* answer about an expression, mixed from the
/// expression's own key and the name being framed.
///
/// Separate from [`compute_cache_key`] so the two answers for one expression
/// cannot overwrite each other: the same read is annotated at its own position
/// on the debug path and at its binding's declaration when it is refused.
fn compute_declaration_cache_key(expression_key: u128, name: &Atom) -> u128 {
  stable_hash_wide(&("stylex-declaration-span:v1", expression_key, name))
}

/// Finds the span of a style namespace by its **key** inside the parsed
/// source, instead of matching the namespace's value expression.
///
/// Object keys are static strings that survive value-level code transforms
/// (e.g. compile-time macro expansion done by an earlier loader), so this
/// locates the original source position even when the compiled AST's values
/// no longer textually match the file on disk — the case where
/// `find_expression_span` has nothing to match against.
///
/// The `call_expr`'s own argument keys are used to disambiguate between
/// multiple objects containing an identically named property: the candidate
/// object sharing the most sibling keys with the compiled call wins.
///
/// Returns a dummy span when the key cannot be located.
pub fn get_key_span_from_source_code(
  lookup: &CallLookup,
  namespace_key: &str,
  state: &mut impl DiagnosticState,
) -> Result<(CodeFrame, Span), Error> {
  // Same panic boundary as `get_span_from_source_code`: locating a span is
  // best-effort and must never abort the compilation.
  locate_span_with_panic_boundary(|| {
    get_key_span_from_source_code_impl(lookup, namespace_key, state)
  })
}

fn get_key_span_from_source_code_impl(
  lookup: &CallLookup,
  namespace_key: &str,
  state: &mut impl DiagnosticState,
) -> Result<(CodeFrame, Span), Error> {
  let query = lookup.query(namespace_key);
  let cache_key = compute_key_span_cache_key(lookup.digest(), &query);
  let file_name = FileName::Custom(state.get_filename().to_owned());

  if let Some(cached_span) = state.diagnostic_memo().cached_span(cache_key) {
    let code_frame = load_code_frame_from_cache_for_state(&file_name, state)?;
    return Ok((code_frame, cached_span));
  }

  let code_frame = CodeFrame::new();

  // The lookup's wrapper, cloned on the first namespace that gets this far and
  // shared by the rest -- this ran once per namespace before, and a call whose
  // namespaces all hit the cache above never builds one at all.
  let wrapped_call = lookup.wrapped();

  memoize_module(wrapped_call, wrapped_call, state, &file_name, &code_frame)?;

  // One index over the whole module, not one walk per namespace key: the debug
  // path asks this question once per style, and the walk it replaces made a
  // `dev` build quadratic in the size of a file that is one long list of them.
  let span = match state.key_span_index() {
    Some(index) => index.resolve(&query),
    None => return Err(no_module_to_quote(state)),
  };

  state
    .diagnostic_memo_mut()
    .insert_cached_span(cache_key, span);

  Ok((code_frame, span))
}

/// The per-namespace half of a key-span cache key, mixed with the call digest
/// from [`CallLookup::digest`]. 128 bits for the same reason as
/// [`compute_cache_key`].
///
/// Hashed as one tuple rather than field by field, so the wide hasher is built
/// once and the pieces cannot drift out of the key by being added to the
/// function and forgotten in the digest.
fn compute_key_span_cache_key(siblings_digest: u128, query: &NamespaceKeyQuery) -> u128 {
  stable_hash_wide(&(
    "stylex-key-span:v5",
    siblings_digest,
    query.namespace_key,
    // The version above still reads v5 because the byte stream did not move:
    // `SmallVec` hashes as a slice, exactly as the `Vec` it replaced did, over
    // the same names in the same order.
    sorted_value_keys(query.namespace_value_keys.iter()),
    query.target_offset,
  ))
}

/// How many value keys are ordered on the stack before the sort falls back to
/// the heap.
///
/// One namespace's value is a handful of CSS properties, so this covers what a
/// style object holds while the buffer stays small enough for a function the
/// debug path calls once per namespace of every call.
const INLINE_VALUE_KEYS: usize = 16;

/// `keys` in the one order the cache key may hash them in.
///
/// Sorted, because the keys arrive from a hash set whose iteration order is not
/// part of the identity being keyed -- two namespaces spelling the same
/// properties are the same namespace. Two things keep the ordering off the
/// heap, and both are load-bearing: the buffer is inline up to
/// [`INLINE_VALUE_KEYS`], and the sort is the unstable one, because the stable
/// sort allocates scratch space of its own. Equal elements cannot be told apart
/// here anyway -- they come from a set, so there are none.
fn sorted_value_keys<'keys>(
  keys: impl Iterator<Item = &'keys Atom>,
) -> SmallVec<[&'keys Atom; INLINE_VALUE_KEYS]> {
  let mut sorted: SmallVec<[&'keys Atom; INLINE_VALUE_KEYS]> = keys.collect();
  sorted.sort_unstable();

  sorted
}

/// Loads a CodeFrame with the source file for error display.
fn load_code_frame_from_cache_for_state(
  file_name: &FileName,
  state: &impl DiagnosticState,
) -> Result<CodeFrame, Error> {
  let code_frame = CodeFrame::new();

  // Registered at most once. The source map behind every `CodeFrame` is a
  // process-global `OnceLock`, so re-registering here would append another copy
  // of the module to it on every call -- and the debug-data path calls this once
  // per style. On a 200 KB module with 1 257 styles that was a quarter of a
  // gigabyte of duplicated source and the largest single cost in a `dev` build.
  code_frame.register_produced_source_once(file_name, || {
    state
      .get_seen_module_source_code()
      .and_then(|(_, source_code)| source_code.map(str::to_owned))
      .map(Ok)
      .unwrap_or_else(|| {
        read_source_file(file_name)
          .map_err(|error| anyhow::anyhow!("Failed to read source file: {}", error))
      })
  })?;

  Ok(code_frame)
}

/// Finds the span of a target expression within a program AST
fn find_expression_span(module: &Module, target_expression: &Expr) -> Span {
  let mut finder = ExpressionFinder::new(target_expression);
  module.visit_with(&mut finder);

  if let Some(span) = finder.get_span() {
    return span;
  }

  // Fallback: try finding after template literal conversion
  let mut converted_target = target_expression.clone();
  converted_target.visit_mut_with(&mut TplConverter {});
  let mut fallback_finder = ExpressionFinder::new(&converted_target);
  module.visit_with(&mut fallback_finder);

  // The target expression's own span belongs to the caller's source map, not
  // the code-frame one, so its byte offsets are meaningless here and can even
  // land inside a multi-byte character, panicking on source-map lookups. A
  // dummy span signals "location unknown" instead.
  fallback_finder.get_span().unwrap_or(DUMMY_SP)
}

/// Gets or parses the source code as a Program AST, with memoization.
/// Returns a cleaned and normalized Program that can be used for expression
/// finding.
/// Runs `visit` against the module's parsed, normalized source, parsing and
/// memoizing it first if that has not happened yet.
///
/// Shaped as a closure rather than as "return me the program" for two reasons.
/// The program lives in the state, which is borrowed mutably here, so it cannot
/// be handed back and still leave the caller free to write the resolved span
/// back afterwards -- the closure's borrow ends when this returns. And the
/// previous shape *did* hand it back, as `Program::Module(module.clone())`: a
/// deep clone of the whole module, once per style in a `dev` build, which made
/// the annotation cost grow about quadratically with file size. A bigger module
/// was both more clones and a bigger clone.
fn with_memoized_module<T>(
  wrapped_expression: &Expr,
  target_expression: &Expr,
  state: &mut impl DiagnosticState,
  file_name: &FileName,
  code_frame: &CodeFrame,
  visit: impl FnOnce(&Module) -> T,
) -> Result<T, Error> {
  memoize_module(
    wrapped_expression,
    target_expression,
    state,
    file_name,
    code_frame,
  )?;

  match state.get_seen_module_source_code() {
    Some((module, _)) => Ok(visit(module)),
    None => Err(no_module_to_quote(state)),
  }
}

/// The answer when the frame has no module to quote, named by the file it was
/// looking for.
///
/// Raised where no candidate text parsed, and again where a memoized module is
/// asked for after [`memoize_module`] returned `Ok`. That second case is not
/// reachable in practice -- the call either found a memoized module or stored
/// one -- but it stays an error rather than a panic, because the getters' types
/// cannot say so and a diagnostic aid must never be the reason a build stops.
fn no_module_to_quote(state: &impl DiagnosticState) -> Error {
  anyhow::anyhow!("Failed to parse source file: {}", state.get_filename())
}

/// Parses and memoizes the module's own source on the state, and registers that
/// source with `code_frame`, unless a previous lookup already did.
///
/// Separate from [`with_memoized_module`] because the namespace-key lookup does
/// not want the module itself: it wants the key span index built from it, which
/// the state hands out and which cannot be borrowed out of a closure holding the
/// module.
fn memoize_module(
  wrapped_expression: &Expr,
  target_expression: &Expr,
  state: &mut impl DiagnosticState,
  file_name: &FileName,
  code_frame: &CodeFrame,
) -> Result<(), Error> {
  if let Some((_, Some(source_code))) = state.get_seen_module_source_code() {
    // Registered once, not once per lookup -- see `register_source_once`.
    code_frame.register_source_once(file_name, source_code);

    return Ok(());
  }

  let mut authored_text_found = false;

  for candidate in source_code_candidates(state) {
    // Made here rather than in the list, so a text that a better candidate
    // makes unnecessary is never built. Printing the memoized module is a deep
    // clone of it, which a file read before it usually makes unnecessary.
    let Some(source_code) = candidate.source_code(state, file_name) else {
      continue;
    };

    authored_text_found = true;

    if try_memoize_source(source_code, target_expression, state, file_name, code_frame) {
      return Ok(());
    }
  }

  if authored_text_found {
    return Err(no_module_to_quote(state));
  }

  // No authored text can be read, so the frame quotes a module made of the
  // expression alone. Such a module always parses, which is what keeps an
  // answer available. The outcome is not examined for that reason: a state that
  // kept nothing is reported by the caller that asks for the module back.
  let synthesized = print_module(create_module(wrapped_expression), None);

  let _ = try_memoize_source(synthesized, target_expression, state, file_name, code_frame);

  Ok(())
}

/// Registers `source_code` with `code_frame`, parses it and memoizes the module
/// on `state`. `false` when the text does not parse, so the caller can go on to
/// the next candidate.
fn try_memoize_source(
  source_code: String,
  target_expression: &Expr,
  state: &mut impl DiagnosticState,
  file_name: &FileName,
  code_frame: &CodeFrame,
) -> bool {
  // Through the same reuse `register_source_once` applies, rather than around
  // it. `new_source_file` never deduplicates -- it appends -- and this map is
  // a process-global `OnceLock` that is never cleared, so registering here on
  // every compile is how a watch-mode process accumulated one full copy of
  // each module per save. Comparing the content is the load-bearing part: an
  // edited file still gets a fresh registration, and only an unchanged one is
  // reused.
  let source_file = match code_frame.registered_source_file(file_name, &source_code) {
    Some(existing) => existing,
    None => code_frame
      .source_map
      .new_source_file(Arc::new(file_name.clone()), source_code),
  };

  let program = parse_and_normalize_program(
    &source_file,
    code_frame,
    state.get_filename(),
    target_expression,
  );

  // A candidate that does not parse stays in the map, and the next one is
  // registered beside it. A span is resolved by position, so the file that did
  // parse is still the one a frame is quoted from.
  let Some(program) = program else {
    return false;
  };

  // The state keeps the file the frame registered, so the text it hands back
  // later is that same allocation and not a copy of it.
  state.set_seen_module_source_code(expect_module(&program), Some(source_file));

  true
}

/// The module of a program [`parse_and_normalize_program`] returned.
///
/// It parses with `IsModule::Bool(true)`, so a successful parse is always a
/// module. This stays a panic rather than being dropped because it guards an
/// invariant of that call rather than an input, and the day the call learns a
/// second mode this is where it should stop.
fn expect_module(program: &Program) -> &Module {
  match program.as_module() {
    Some(module) => module,
    None => stylex_panic!("Expected a module program for source code caching."),
  }
}

/// Where the text of the module the frame quotes may come from.
enum SourceCandidate {
  /// The file on disk, as `useRealFileForSource` permits.
  Disk,
  /// The text the compiler was given for the file.
  Given,
  /// The memoized module, printed back out from its AST.
  PrintedModule,
}

impl SourceCandidate {
  /// The text, or `None` where this source cannot supply one -- a file that is
  /// not there, or a name no file can be read from.
  fn source_code(&self, state: &impl DiagnosticState, file_name: &FileName) -> Option<String> {
    match self {
      Self::Disk => read_source_file(file_name).ok(),
      Self::Given => state.input_source_text().map(str::to_owned),
      Self::PrintedModule => {
        let (module, _) = state.get_seen_module_source_code()?;

        Some(print_module(
          module.clone(),
          Some(
            Config::default()
              .with_minify(false)
              .with_omit_last_semi(false)
              .with_reduce_escaped_newline(false)
              .with_inline_script(false),
          ),
        ))
      },
    }
  }
}

/// The sources the module the frame quotes may come from, best first.
///
/// While `useRealFileForSource` is on, the file on disk comes first and the
/// text the compiler was given is the fallback for a file that is not there.
/// Both keep the authored layout, which is the only one a `file:line` may be
/// measured against. When the option is off, no file is opened and a module
/// memoized without its text is printed back out, as the option documents.
///
/// A list rather than one source because `filename` can name a host file whose
/// content is not the JavaScript the compiler was fed -- a single-file
/// component, an `.mdx`, a `?query`-suffixed name that still resolves on disk,
/// or a file edited since the read. [`memoize_module`] takes the first
/// candidate that parses, so such a file degrades to the text the compiler
/// holds instead of leaving the diagnostic with no frame. Where no candidate
/// supplies a text at all, that caller synthesizes a module from the expression
/// itself, which is how an answer stays available.
///
/// The memoized *text* is not a candidate here: the only caller reaches this
/// after finding there is none.
fn source_code_candidates(state: &impl DiagnosticState) -> SmallVec<[SourceCandidate; 3]> {
  let mut candidates = SmallVec::new();

  if state.reads_source_from_disk() {
    candidates.push(SourceCandidate::Disk);
    candidates.push(SourceCandidate::Given);
  }

  // A module printed back out from its AST lays its lines out differently from
  // the authored file, so it is quoted only after every authored text.
  candidates.push(SourceCandidate::PrintedModule);

  candidates
}

/// Parses source code into a Program AST and normalizes it
fn parse_and_normalize_program(
  source_file: &Arc<swc_core::common::SourceFile>,
  code_frame: &CodeFrame,
  filename: &str,
  target_expression: &Expr,
) -> Option<Program> {
  // The syntax the compiler parsed the module with, so a module that parsed
  // there parses here: a `.ts` module holding a generic arrow is read as
  // TypeScript without JSX by both, and its refusal keeps its frame.
  let parse_result = parse_js(
    code_frame.source_map.clone(),
    source_file.clone(),
    &code_frame.handler,
    EsVersion::EsNext,
    syntax_for_file(Path::new(filename)),
    IsModule::Bool(true),
    None,
  );

  match parse_result {
    Ok(program) => {
      let unresolved_mark = Mark::new();
      let top_level_mark = Mark::new();

      // Clean and normalize: remove syntax contexts, convert template literals
      let mut normalized = program.apply(strip(unresolved_mark, top_level_mark));
      normalized.visit_mut_with(&mut TplConverter {});
      Some(normalized)
    },
    Err(error) => {
      warn_unparseable(&error, filename, target_expression);
      None
    },
  }
}

/// Reports that the module's own source could not be parsed, so nothing can be
/// searched for a position. Same split as [`warn_no_code_frame`].
fn warn_unparseable(error: &Error, filename: &str, target: &Expr) {
  if log::log_enabled!(log::Level::Debug) {
    debug!(
      "Failed to parse program: {:?}. File: {}. Expression: {:?}",
      error, filename, target
    );
  } else {
    warn!("Failed to parse program: {:?}. File: {}", error, filename);
  }
}

pub fn print_module(module: Module, codegen_config: Option<Config>) -> String {
  print_program(Program::Module(module), codegen_config)
}

pub(crate) fn print_program(mut program: Program, codegen_config: Option<Config>) -> String {
  // The printed AST carries spans from the compiler's own source map, which
  // are meaningless in the shared code-frame map. The codegen resolves
  // non-dummy spans against its source map (e.g. `span_to_snippet` for
  // trailing-comma detection), so foreign offsets would read unrelated files
  // and can panic mid-character on multi-byte sources.
  program.visit_mut_with(&mut DropSpan {});

  let printed_source_code = print(
    shared_source_map(),
    &program,
    PrintArgs {
      source_map: SourceMapsConfig::Bool(false),
      codegen_config: codegen_config.unwrap_or_default(),
      ..Default::default()
    },
  )
  .unwrap_or_else(nothing_printed);

  printed_source_code.code
}

/// What a print that failed leaves behind: no code, and nothing said about why.
/// The caller is already printing only to *quote* a module back, so an empty
/// quote is the graceful answer.
fn nothing_printed(_: Error) -> TransformOutput {
  TransformOutput {
    code: String::new(),
    map: None,
    output: None,
    diagnostics: Vec::default(),
    extracted_comments: None,
  }
}

pub fn create_module(wrapped_expression: &Expr) -> Module {
  Module {
    span: DUMMY_SP,
    body: vec![ModuleItem::Stmt(Stmt::Expr(ExprStmt {
      span: DUMMY_SP,
      expr: Box::new(wrapped_expression.clone()),
    }))],
    shebang: None,
  }
}

/// Visitor that searches for a specific expression in an AST.
/// Uses discriminant matching for fast filtering before expensive
/// eq_ignore_span checks.
#[derive(Debug)]
struct ExpressionFinder {
  target: Expr,
  target_discriminant: std::mem::Discriminant<Expr>,
  found_span: Option<Span>,
}

/// Visitor that normalizes AST by removing syntax contexts and type
/// annotations. This allows for more reliable expression matching across
/// different parsing contexts.
#[derive(Debug)]
struct Cleaner {}
impl VisitMut for Cleaner {
  noop_visit_mut_type!();

  fn visit_mut_binding_ident(&mut self, node: &mut BindingIdent) {
    node.id.ctxt = SyntaxContext::empty();
    node.type_ann = None;
    node.visit_mut_children_with(self);
  }

  fn visit_mut_ident(&mut self, ident: &mut Ident) {
    ident.ctxt = SyntaxContext::empty();
    ident.visit_mut_children_with(self);
  }
}

impl ExpressionFinder {
  fn new(target: &Expr) -> Self {
    let mut cleaned_target = target.clone();
    cleaned_target.visit_mut_children_with(&mut Cleaner {});
    let target_discriminant = std::mem::discriminant(&cleaned_target);

    Self {
      target: cleaned_target,
      target_discriminant,
      found_span: None,
    }
  }

  fn get_span(&self) -> Option<Span> {
    self.found_span
  }
}

/// Visitor that normalizes template literals and string concatenations.
/// Helps match expressions that may be written differently in source vs AST.
#[derive(Debug)]
struct TplConverter {}

impl VisitMut for TplConverter {
  noop_visit_mut_type!();

  fn visit_mut_expr(&mut self, expr: &mut Expr) {
    let converted = convert_simple_tpl_to_str_expr(convert_concat_to_tpl_expr(expr.take()));
    *expr = converted;
    expr.visit_mut_children_with(self);
  }
}

impl Visit for ExpressionFinder {
  noop_visit_type!();

  fn visit_expr(&mut self, expr: &Expr) {
    if self.found_span.is_some() {
      return;
    }

    // Fast discriminant check filters expressions by variant type
    if std::mem::discriminant(expr) != self.target_discriminant {
      expr.visit_children_with(self);
      return;
    }

    // Expensive structural comparison only for matching variants
    if self.target.eq_ignore_span(expr) {
      self.found_span = Some(Span::new(expr.span_lo(), expr.span_hi()));
      return;
    }

    expr.visit_children_with(self);
  }
}

#[track_caller]
#[cold]
pub fn build_code_frame_error_and_panic(
  wrapped_expression: &Expr,
  fault_expression: &Expr,
  error_message: &str,
  state: &mut impl DiagnosticState,
) -> ! {
  let caller_location = std::panic::Location::caller();

  // Emit the code frame diagnostic to stderr (already [StyleX]-prefixed)
  let (file, line) = match get_span_from_source_code(wrapped_expression, fault_expression, state) {
    Ok((code_frame, span)) => {
      code_frame.emit_error(span, error_message);
      let line_num = code_frame.try_get_span_line_number(span);
      (Some(state.get_filename().to_owned()), line_num)
    },
    Err(error) => {
      warn_no_code_frame(&error, state.get_filename(), fault_expression);
      (Some(state.get_filename().to_owned()), None)
    },
  };

  let err = StyleXError {
    message: error_message.to_string().into(),
    file: file.map(Into::into),
    key_path: None,
    line,
    col: None,
    source_location: Some(format!("{}:{}", caller_location.file(), caller_location.line()).into()),
  };

  __stylex_panic(err)
}

#[track_caller]
#[cold]
pub fn build_code_frame_error_and_panic_at(
  expr: &Expr,
  error_message: &str,
  state: &mut impl DiagnosticState,
) -> ! {
  build_code_frame_error_and_panic(expr, expr, error_message, state)
}

#[cfg(test)]
#[path = "tests/code_frame_test.rs"]
mod tests;
