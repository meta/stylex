//! A Rust-evaluated arrow passed to an actual JavaScript environment function.
//! Own the lexical snapshot; never retain a borrowed or mutably aliased hook.
use super::*;
use crate::utils::host::register_closure;

pub(super) fn register(
    parent: &Evaluator<'_>,
    function: EvalValue,
    scope: OwnedEvaluationScope,
) -> String {
    let EvalValue::Function(original) = &function else {
        unreachable!("only Rust-evaluated functions use the compiler closure registry");
    };
    let identity = Rc::as_ptr(original) as usize;
    if let Some(id) = parent
        .closure_ids
        .borrow()
        .get(&identity)
        .map(|(_, id)| id.clone())
    {
        return id;
    }
    // The strong value in the cache prevents Rc address reuse before the
    // transform ends; structurally equal but separately allocated arrows stay distinct.
    let keep_alive = function.clone();
    let closure_ids = parent.closure_ids.clone();
    let options = parent.options.clone();
    let handles = parent.handles.clone();
    let js_handles = parent.js_handles.clone();
    let effects = parent.effects.clone();
    let self_proxies = parent.self_proxies.clone();
    let theme_import_ids = parent.theme_import_ids.clone();
    let seen_import_reads = parent.seen_import_reads.clone();
    let label = parent.label.clone();
    let track_dependencies = parent.dependencies.is_some();
    let registered = register_closure(Rc::new(move |_this, arguments| {
        // These empty fields are not the evaluation environment: scope.call()
        // installs the exact owned maps, overlays, safety and original budget.
        let expressions = HashMap::new();
        let safety = BindingSafety::default();
        let mut hooks = Evaluator {
            options: &options,
            expressions: &expressions,
            safety: &safety,
            values: HashMap::new(),
            handles: handles.clone(),
            js_handles: js_handles.clone(),
            rules: IndexMap::new(),
            warnings: Vec::new(),
            failure: None,
            last_deopt: None,
            self_proxies: self_proxies.clone(),
            self_overlay: HashMap::new(),
            dependencies: track_dependencies.then(Vec::new),
            label: label.clone(),
            theme_import_ids: theme_import_ids.clone(),
            seen_import_reads: seen_import_reads.clone(),
            import_reads: Vec::new(),
            effects: effects.clone(),
            closure_ids: closure_ids.clone(),
        };
        let values = arguments
            .iter()
            .map(|value| hooks.decode_host(value.clone(), HostValue::Undefined))
            .collect::<Result<Vec<_>>>()?;
        let result = scope.call(&function, values, &mut hooks);
        let result = if let Some(error) = hooks.failure.take() {
            Err(error)
        } else {
            result.map_err(EvaluationFailure::into_error)
        };
        let encoded = result.and_then(|value| hooks.encode_host(&value, Some(&scope)));
        let mut journal = effects.borrow_mut();
        journal.rules.extend(hooks.rules);
        journal.warnings.extend(hooks.warnings);
        journal.import_reads.extend(hooks.import_reads);
        journal
            .dependencies
            .extend(hooks.dependencies.unwrap_or_default());
        encoded
    }));
    parent
        .closure_ids
        .borrow_mut()
        .insert(identity, (keep_alive, registered.clone()));
    registered
}
