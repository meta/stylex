// The vendored StyleX core (crates/stylex-transform) compiles the module; this
// root parses it with the bounded native frontend, runs the core, and emits the
// result through the source-preserving emitter.
mod core_options;
mod fast_path;
mod options;
mod render;

use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use stylex_structures::import_resolver::{DebugFileNamer, ImportResolver};
use stylex_structures::plugin_pass::PluginPass;
use stylex_structures::stylex_env::{EnvEntry, HostEnvironment};
use stylex_transform::StyleXTransform;
use stylex_types::enums::data_structures::injectable_style::InjectableStyleBaseKind;
use swc_core::common::{FileName, Mark, GLOBALS};
use swc_core::ecma::transforms::base::resolver;
use swc_core::ecma::visit::VisitMutWith;

use crate::shared::{StyleXTransformOptions, TransformOutput};
use crate::stylesheet::{process_stylex_rules, ProcessStylexRulesConfig, RuleEntry, RuleFields};
use crate::utils::parser::{parse_module_with_options, syntax_for_file, SourceType};

use options::{normalize_options, TransformState};
use render::emit_module_with_origins;

/// Stack every later pass may use: the parser bounds input nesting, but the
/// core, rendering, postflight checks and AST destruction all recurse over the
/// accepted tree.
const TRANSFORM_STACK_SIZE: usize = 512 * 1024 * 1024;

pub fn transform_source(
    source: &str,
    filename: &str,
    options: &StyleXTransformOptions,
) -> Result<TransformOutput> {
    transform_on_large_stack(source, filename, options, Host::default())
}

/// The same transform with the no-change fast path disabled, so the ordinary
/// pipeline runs whatever the module holds. For the fast path's equivalence
/// tests only: it is not reachable through any option, the Node addon or the
/// Babel plugin, so nothing a caller configures can select it.
#[doc(hidden)]
pub fn transform_source_without_fast_path(
    source: &str,
    filename: &str,
    options: &StyleXTransformOptions,
) -> Result<TransformOutput> {
    transform_on_large_stack(
        source,
        filename,
        options,
        Host {
            ordinary_pipeline: true,
            ..Host::default()
        },
    )
}

/// The Node addon's entry point: `options` are the raw serialized options and
/// `env` holds the host's live `env` values.
#[cfg_attr(not(feature = "node"), allow(dead_code))]
pub(crate) fn transform_with_host(
    source: &str,
    filename: &str,
    options: &serde_json::Value,
    host: Option<HostEnvironment>,
    env: IndexMap<String, EnvEntry>,
    resolver: Option<Rc<dyn ImportResolver>>,
    debug_file_namer: Option<Rc<dyn DebugFileNamer>>,
) -> Result<TransformOutput> {
    let options: StyleXTransformOptions = serde_json::from_value(options.clone())
        .map_err(|error| anyhow!("invalid StyleX options: {error}"))?;
    transform_on_large_stack(
        source,
        filename,
        &options,
        Host {
            environment: host,
            env,
            resolver,
            debug_file_namer,
            ordinary_pipeline: false,
        },
    )
}

/// The Rust-API counterpart of [`transform_with_host`] for embedders (and
/// tests) that provide their own host: `env` holds the host's `env` values and
/// `resolver` resolves theme imports the way the host's configuration does.
pub fn transform_source_with_host(
    source: &str,
    filename: &str,
    options: &StyleXTransformOptions,
    host: HostEnvironment,
    env: IndexMap<String, EnvEntry>,
    resolver: Option<Rc<dyn ImportResolver>>,
) -> Result<TransformOutput> {
    transform_on_large_stack(
        source,
        filename,
        options,
        Host {
            environment: Some(host),
            env,
            resolver,
            debug_file_namer: None,
            ordinary_pipeline: false,
        },
    )
}

/// Runs `work` with the compiler's per-thread state of its own, restoring the
/// caller's afterwards. A host callback can start a transform, or call back
/// into compiled code, while another transform is running on the same thread;
/// neither may see or change the other's error, warning or evaluation state.
/// Every transform runs isolated; hosts wrap their re-entrant operations too.
pub(crate) fn isolated<R>(work: impl FnOnce() -> R) -> R {
    // Kept aside and restored (on unwind too): the evaluation budget, the span
    // of the last reported error, and the warning collection. The stderr
    // suppression guard restores the caller's flag itself, and the uid
    // generators the core uses belong to its per-file state.
    stylex_evaluator::with_isolated_evaluation(|| {
        stylex_macros::stylex_error::with_isolated_error_state(|| warnings::isolated(work))
    })
}

/// Whether an import source names a theme file (`*.stylex`, with or without a
/// code extension), as the Babel plugin's `matchesFileSuffix('.stylex')` does.
pub(super) fn is_theme_import(source: &str) -> bool {
    const SUFFIXES: [&str; 7] = ["", ".js", ".ts", ".tsx", ".jsx", ".mjs", ".cjs"];
    SUFFIXES
        .iter()
        .any(|extension| source.ends_with(&format!(".stylex{extension}")))
}

/// The Babel plugin's `rewriteAliases`: at the end of the transform, a theme
/// import is rewritten to the relative path it resolves to through the
/// configured aliases.
fn rewrite_alias_imports(
    module: &mut swc_core::ecma::ast::Module,
    filename: &str,
    rewriter: &dyn ImportResolver,
) -> Result<()> {
    use swc_core::ecma::ast::{ModuleDecl, ModuleItem, Str};
    for item in &mut module.body {
        let ModuleItem::ModuleDecl(ModuleDecl::Import(import)) = item else {
            continue;
        };
        let Some(source) = import.src.value.as_str().map(str::to_owned) else {
            continue;
        };
        if !is_theme_import(&source) {
            continue;
        }
        let rewritten = rewriter
            .rewrite_alias_import(&source, filename)
            .map_err(|error| anyhow!(error))?;
        if let Some(rewritten) = rewritten.filter(|rewritten| *rewritten != source) {
            // Replace only the authored specifier literal; named and bare
            // imports retain the same source-owned declaration and trivia.
            *import.src = Str {
                span: import.src.span,
                value: rewritten.into(),
                raw: None,
            };
        }
    }
    Ok(())
}

/// What an embedding host contributes to a transform.
#[derive(Default)]
struct Host {
    environment: Option<HostEnvironment>,
    env: IndexMap<String, EnvEntry>,
    resolver: Option<Rc<dyn ImportResolver>>,
    debug_file_namer: Option<Rc<dyn DebugFileNamer>>,
    /// Skip the no-change fast path (equivalence tests only).
    ordinary_pipeline: bool,
}

fn transform_on_large_stack(
    source: &str,
    filename: &str,
    options: &StyleXTransformOptions,
    host: Host,
) -> Result<TransformOutput> {
    // Embedders call this from threads of any size (a 512 KiB worker thread is
    // common), so the whole transform, including dropping its ASTs, runs on a
    // stack known to fit the deepest input the parser accepts. On a thread that
    // already has that much room (the Node addon's compiler thread) this is a
    // no-op. The Node addon marshals JavaScript calls to the JS thread, so no
    // JS frame ever runs on this stack.
    stacker::maybe_grow(TRANSFORM_STACK_SIZE / 2, TRANSFORM_STACK_SIZE, || {
        isolated(|| transform_on_stack(source, filename, options, host))
    })
}

fn parse_options(
    options: &StyleXTransformOptions,
    filename: &str,
) -> Result<(swc_ecma_parser::Syntax, SourceType)> {
    // Integrations that already chose a grammar or source type for a file (a
    // bundler's extracted `<script lang="ts">`, Babel's `sourceType`) pass it.
    let syntax = match options.additional_options.get("__syntax") {
        None => syntax_for_file(filename),
        Some(syntax) => match syntax.as_str() {
            Some("flow") => syntax_for_file("file.js"),
            Some("typescript") => syntax_for_file("file.ts"),
            Some("tsx") => syntax_for_file("file.tsx"),
            _ => {
                return Err(anyhow!(
                    "__syntax must be \"flow\", \"typescript\" or \"tsx\""
                ))
            }
        },
    };
    let source_type = match options.additional_options.get("__sourceType") {
        None => SourceType::Module,
        Some(source_type) => match source_type.as_str() {
            Some("module") => SourceType::Module,
            Some("script") => SourceType::Script,
            Some("unambiguous") => SourceType::Unambiguous,
            _ => {
                return Err(anyhow!(
                    "__sourceType must be \"module\", \"script\" or \"unambiguous\""
                ))
            }
        },
    };
    Ok((syntax, source_type))
}

fn metadata_entry(rule: &stylex_types::structures::meta_data::MetaData) -> RuleEntry {
    let (ltr, rtl, const_key, const_val) = match rule.get_style() {
        InjectableStyleBaseKind::Regular(style) => {
            (style.ltr.clone(), style.rtl.clone(), None, None)
        }
        InjectableStyleBaseKind::Const(style) => (
            style.ltr.clone(),
            style.rtl.clone(),
            Some(style.const_key.clone()),
            Some(
                serde_json::from_str(&style.const_value)
                    .unwrap_or_else(|_| serde_json::Value::String(style.const_value.clone())),
            ),
        ),
    };
    RuleEntry(
        rule.get_class_name().to_owned(),
        RuleFields {
            ltr,
            rtl,
            const_key,
            const_val,
        },
        *rule.get_priority(),
    )
}

/// Warnings the core logs where the Babel plugin calls `console.warn`. They are
/// collected per transform and returned as `warnings`; the host prints them.
mod warnings {
    use std::cell::RefCell;

    // Modules whose warnings the Babel plugin reports to the user (as
    // `[stylex] <message>`). Other core log output is diagnostic only.
    const REPORTED_TARGETS: &[&str] =
        &["stylex_transform::shared::utils::core::flat_map_expanded_shorthands"];

    thread_local! {
        static COLLECTED: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
    }

    struct Collector;

    impl log::Log for Collector {
        fn enabled(&self, metadata: &log::Metadata) -> bool {
            metadata.level() <= log::Level::Warn
        }

        fn log(&self, record: &log::Record) {
            if record.level() > log::Level::Warn
                || !REPORTED_TARGETS
                    .iter()
                    .any(|target| record.target().starts_with(target))
            {
                return;
            }
            COLLECTED.with(|collected| {
                if let Some(warnings) = collected.borrow_mut().as_mut() {
                    warnings.push(format!("[stylex] {}", record.args()));
                }
            });
        }

        fn flush(&self) {}
    }

    /// Runs `work`, returning what it produced and the warnings it reported.
    pub(super) fn collect<T>(work: impl FnOnce() -> T) -> (T, Vec<String>) {
        static INSTALLED: std::sync::Once = std::sync::Once::new();
        INSTALLED.call_once(|| {
            // An embedder that installed its own logger keeps it; warnings then
            // go there instead of into the result.
            if log::set_logger(&Collector).is_ok() {
                log::set_max_level(log::LevelFilter::Warn);
            }
        });
        let (result, warnings) = with_collector(Some(Vec::new()), work);
        (result, warnings.unwrap_or_default())
    }

    /// Runs `work` with `collector` in place of the caller's collection, which
    /// is restored afterwards, on unwind too. Returns what `work` produced and
    /// what the collector gathered.
    fn with_collector<T>(
        collector: Option<Vec<String>>,
        work: impl FnOnce() -> T,
    ) -> (T, Option<Vec<String>>) {
        struct Restore(Option<Option<Vec<String>>>);
        impl Drop for Restore {
            fn drop(&mut self) {
                if let Some(previous) = self.0.take() {
                    COLLECTED.with(|collected| *collected.borrow_mut() = previous);
                }
            }
        }
        let mut restore = Restore(Some(
            COLLECTED.with(|collected| collected.replace(collector)),
        ));
        let result = work();
        let previous = restore.0.take().unwrap_or_default();
        let gathered = COLLECTED.with(|collected| collected.replace(previous));
        (result, gathered)
    }

    /// Runs `work` with no warning collection of the caller's in reach: what
    /// an operation nested in another transform reports is not the outer
    /// transform's warning.
    pub(super) fn isolated<T>(work: impl FnOnce() -> T) -> T {
        with_collector(None, work).0
    }
}

/// Leaves a panic the transform catches silent (the default hook would print
/// every StyleX error to stderr before it is reported as an exception).
fn install_quiet_panic_hook() {
    static INSTALLED: std::sync::Once = std::sync::Once::new();
    INSTALLED.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if !stylex_macros::stylex_error::is_panic_stderr_suppressed() {
                previous(info);
            }
        }));
    });
}

/// The message the core reported, without its `[StyleX]` prefix, the object
/// key path it prepends, or the location line it appends: the Babel plugin
/// reports the message alone and the host adds the file name and code frame.
fn core_error(
    panic: &Box<dyn std::any::Any + Send>,
    module: &swc_core::ecma::ast::Module,
) -> anyhow::Error {
    // The message exactly as the core raised it (trailing blank lines are part
    // of some of the Babel plugin's messages); otherwise the first line of the
    // panic text without its prefix and key path.
    let text = stylex_macros::stylex_error::format_panic_message(panic);
    // A message recorded for an error the core caught itself is not this one.
    let recorded = stylex_macros::stylex_error::take_panic_message()
        .filter(|message| text.contains(message.trim_end()));
    let message = recorded.unwrap_or_else(|| {
        let first = text.lines().next().unwrap_or_default();
        let message = first
            .strip_prefix(stylex_constants::logger::STYLEX_LOG_PREFIX)
            .unwrap_or(first)
            .trim_start();
        message.rsplit(" > ").next().unwrap_or(message).to_owned()
    });
    let message = message.as_str();
    // An error the Babel plugin throws as a plain `Error` has no code frame.
    let plain = stylex_macros::stylex_error::take_panic_plain();
    if plain {
        let _ = stylex_macros::stylex_error::take_panic_span();
        let _ = stylex_macros::stylex_error::take_panic_declaration();
        return crate::utils::diagnostics::Diagnostic::plain(message.to_owned()).into_error();
    }
    // A refusal about a binding is framed at the binding's declaration in the
    // authored module, as the Babel plugin frames `binding.path`.
    let declaration = stylex_macros::stylex_error::take_panic_declaration()
        .map(|name| {
            stylex_diagnostics::declaration_span::find_declaration_span(module, &name.into())
        })
        .filter(|span| !span.is_dummy());
    // The node the core reported the error at, when it named one.
    let recorded_span = stylex_macros::stylex_error::take_panic_span();
    let span = recorded_span
        .map(|(lo, hi)| {
            swc_core::common::Span::new(
                swc_core::common::BytePos(lo),
                swc_core::common::BytePos(hi),
            )
        })
        .unwrap_or(swc_core::common::DUMMY_SP);
    let span = declaration.unwrap_or(span);
    if span.is_dummy() && recorded_span.is_none() {
        return crate::utils::diagnostics::Diagnostic::plain(message.to_owned()).into_error();
    }
    let message = message
        .strip_suffix('\n')
        .filter(|body| {
            !span.is_dummy()
                && *body == stylex_constants::constants::evaluation_errors::PATH_WITHOUT_NODE
        })
        .unwrap_or(message);
    crate::utils::diagnostics::Diagnostic::syntax(message.to_owned(), span).into_error()
}

fn transform_on_stack(
    source: &str,
    filename: &str,
    options: &StyleXTransformOptions,
    host: Host,
) -> Result<TransformOutput> {
    let ordinary_pipeline = host.ordinary_pipeline;
    let mut state = TransformState { errors: Vec::new() };
    let options = normalize_options(options, &mut state);
    let (syntax, source_type) = parse_options(&options, filename)?;
    let mut params = core_options::core_params(&options, host.environment, host.env)?;
    let rewriter = options
        .rewrite_aliases
        .then(|| host.resolver.clone())
        .flatten();
    params.import_resolver = host.resolver;
    params.debug_file_namer = host.debug_file_namer;

    let mut parsed = parse_module_with_options(source, filename, syntax, source_type)?;
    // Parsing (including grammar errors/resource bounds) and option validation
    // are never skipped. Only a conservative no-change proof bypasses the core
    // and structural emitter; matching/unused/type-only imports stay slow.
    if !ordinary_pipeline && !fast_path::may_transform(&parsed.module, &options) {
        let input_map = options
            .additional_options
            .get("inputSourceMap")
            .filter(|map| map.is_object());
        let maps = options.bool_option("sourceMaps", false);
        let emitted = render::emit_verbatim(&parsed.file, maps || input_map.is_some())?;
        let emitted = render::compose_input_source_map(emitted, input_map)?;
        return Ok(TransformOutput {
            code: emitted.code,
            metadata_stylex: Vec::new(),
            source_map: maps.then_some(emitted.map),
            errors: state.errors,
            warnings: Vec::new(),
        });
    }
    let globals = Default::default();
    GLOBALS.set(&globals, || -> Result<TransformOutput> {
        let unresolved_mark = Mark::new();
        let top_level_mark = Mark::new();
        let is_typescript = matches!(syntax, swc_ecma_parser::Syntax::Typescript(_));
        parsed.module.visit_mut_with(&mut resolver(
            unresolved_mark,
            top_level_mark,
            is_typescript,
        ));
        // The emitter compares the compiled module with the authored one; both
        // carry the same resolution marks.
        let original_module = parsed.module.clone();

        // The Babel adapter passes `__sourceFileName: ''` for a file Babel has
        // no name for; the core then sees the empty name the Babel plugin
        // works with (`currentFile.opts.filename || ''`).
        let unnamed = options
            .additional_options
            .get("__sourceFileName")
            .and_then(|value| value.as_str())
            == Some("");
        let plugin_pass = PluginPass {
            cwd: std::env::current_dir().ok(),
            filename: FileName::Real(PathBuf::from(if unnamed { "" } else { filename })),
        };
        // The core reports a StyleX error by panicking with its message; the
        // panic is caught here and becomes the error the Babel plugin throws.
        install_quiet_panic_hook();
        let _ = stylex_macros::stylex_error::take_panic_span();
        let _ = stylex_macros::stylex_error::take_panic_plain();
        let _ = stylex_macros::stylex_error::take_panic_message();
        let _ = stylex_macros::stylex_error::take_panic_declaration();
        let (compiled, core_warnings) = warnings::collect(|| {
            let _quiet = stylex_macros::stylex_error::SuppressPanicStderr::new();
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut core = StyleXTransform::new(&parsed.comments, plugin_pass, &mut params);
                core.state.set_input_source_file(parsed.file.clone());
                parsed.module.visit_mut_with(&mut core);
                core.state.replay_queued_evaluation_effects();
                core.state
                    .reported_rules()
                    .iter()
                    .map(metadata_entry)
                    .collect::<Vec<_>>()
            }))
        });
        let metadata_stylex = match compiled {
            Ok(metadata) => metadata,
            Err(panic) => return Err(core_error(&panic, &original_module)),
        };

        if let Some(rewriter) = rewriter.as_deref().filter(|_| !unnamed) {
            rewrite_alias_imports(&mut parsed.module, filename, rewriter)?;
        }

        let emitted = emit_module_with_origins(&parsed, &original_module, &[], &[])?;
        // A map from an earlier transform of this file composes with ours, so
        // the result maps to the original source.
        let emitted = render::compose_input_source_map(
            emitted,
            options
                .additional_options
                .get("inputSourceMap")
                .filter(|map| map.is_object()),
        )?;
        let source_map = options
            .bool_option("sourceMaps", false)
            .then_some(emitted.map);
        Ok(TransformOutput {
            code: emitted.code,
            metadata_stylex,
            errors: state.errors,
            warnings: core_warnings,
            source_map,
        })
    })
}

pub fn transform_file(
    path: impl AsRef<Path>,
    options: &StyleXTransformOptions,
) -> Result<TransformOutput> {
    let path = path.as_ref();
    let source = fs::read_to_string(path)?;
    transform_source(&source, &path.to_string_lossy(), options)
}

pub fn process_metadata_to_css(metadata_stylex: &[RuleEntry]) -> String {
    process_stylex_rules(metadata_stylex, Some(&ProcessStylexRulesConfig::default()))
}

#[cfg(test)]
mod isolation_tests {
    use super::{isolated, warnings};

    const REPORTED: &str = "stylex_transform::shared::utils::core::flat_map_expanded_shorthands";

    #[test]
    fn only_the_unlocated_hole_frame_changes_dummy_fault_error_kind() {
        use stylex_diagnostics::code_frame::{build_code_frame_error, unlocated_hole_fault};
        use swc_core::{
            common::{SyntaxContext, DUMMY_SP},
            ecma::ast::{Expr, Ident, Invalid, Module},
        };
        for (fault, kind) in [
            (unlocated_hole_fault(), "SyntaxError"),
            (Expr::Invalid(Invalid { span: DUMMY_SP }), "Error"),
            (
                Expr::Ident(Ident::new(
                    "missing".into(),
                    DUMMY_SP,
                    SyntaxContext::empty(),
                )),
                "Error",
            ),
        ] {
            isolated(|| {
                let mut state = stylex_state::state_manager::StateManager::default();
                let _quiet = stylex_macros::stylex_error::SuppressPanicStderr::new();
                let message = "unlocated diagnostic\n";
                let payload = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    stylex_macros::stylex_panic!(
                        "{}",
                        build_code_frame_error(&fault, &fault, message, &mut state)
                    );
                }))
                .unwrap_err();
                let module = Module {
                    span: DUMMY_SP,
                    body: Vec::new(),
                    shebang: None,
                };
                let error = super::core_error(&payload, &module);
                let diagnostic = crate::utils::diagnostics::from_error(&error);
                assert_eq!(diagnostic.name, kind);
                assert_eq!(diagnostic.message, message);
                assert!(diagnostic.range.is_none());
            });
        }
    }

    #[test]
    fn a_framed_hole_message_keeps_the_base_separator_only() {
        use stylex_constants::constants::evaluation_errors::PATH_WITHOUT_NODE;
        use stylex_diagnostics::code_frame::build_code_frame_error;
        use swc_core::{
            common::{BytePos, Span},
            ecma::ast::{Expr, Invalid, Module},
        };
        for (message, expected) in [
            (
                format!("{PATH_WITHOUT_NODE}\n"),
                PATH_WITHOUT_NODE.to_owned(),
            ),
            (
                "other diagnostic\n\n".to_owned(),
                "other diagnostic\n\n".to_owned(),
            ),
        ] {
            isolated(|| {
                let fault = Expr::Invalid(Invalid {
                    span: Span::new(BytePos(1), BytePos(2)),
                });
                let mut state = stylex_state::state_manager::StateManager::default();
                let _quiet = stylex_macros::stylex_error::SuppressPanicStderr::new();
                let payload = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    stylex_macros::stylex_panic!(
                        "{}",
                        build_code_frame_error(&fault, &fault, &message, &mut state)
                    );
                }))
                .unwrap_err();
                let module = Module {
                    span: swc_core::common::DUMMY_SP,
                    body: Vec::new(),
                    shebang: None,
                };
                let diagnostic =
                    crate::utils::diagnostics::from_error(&super::core_error(&payload, &module));
                assert_eq!(diagnostic.name, "SyntaxError");
                assert_eq!(diagnostic.message, expected);
                assert!(diagnostic.range.is_some());
            });
        }
    }

    #[test]
    fn a_nested_operation_keeps_the_outer_warnings_and_error_span() {
        let ((), outer) = warnings::collect(|| {
            log::warn!(target: REPORTED, "outer before");
            stylex_macros::stylex_error::record_panic_span(10, 20);
            let unwound = std::panic::catch_unwind(|| {
                isolated(|| {
                    log::warn!(target: REPORTED, "nested");
                    stylex_macros::stylex_error::record_panic_span(1, 2);
                    panic!("nested failure");
                })
            });
            assert!(unwound.is_err());
            log::warn!(target: REPORTED, "outer after");
            assert_eq!(
                stylex_macros::stylex_error::take_panic_span(),
                Some((10, 20))
            );
        });
        assert_eq!(outer, ["[stylex] outer before", "[stylex] outer after"]);
    }

    #[test]
    fn a_nested_transform_collects_its_own_warnings() {
        let ((), outer) = warnings::collect(|| {
            let ((), nested) =
                isolated(|| warnings::collect(|| log::warn!(target: REPORTED, "nested")));
            assert_eq!(nested, ["[stylex] nested"]);
        });
        assert!(outer.is_empty());
    }
}
