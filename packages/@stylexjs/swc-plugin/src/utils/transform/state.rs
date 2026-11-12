use indexmap::IndexMap;
use std::collections::{HashMap, HashSet};
use swc_ecma_ast::Expr;

use crate::core::{RuleEntry, StyleValue};

#[derive(Clone)]
pub(super) struct CreateNamespace {
    pub(super) properties: IndexMap<String, Option<String>>,
    pub(super) property_names: IndexMap<String, String>,
    pub(super) css_marker: Option<String>,
}

pub(super) type CreateMap = IndexMap<String, CreateNamespace>;

pub(super) struct PendingRuntimeInjection {
    pub(super) item_index: usize,
    pub(super) rules: Vec<RuleEntry>,
}

#[derive(Default)]
pub(super) struct TransformState {
    pub(super) create_vars: HashMap<String, CreateMap>,
    pub(super) exported_create_vars: HashSet<String>,
    pub(super) runtime_placeholder_exports: HashSet<String>,
    pub(super) hoisted_create_ident_counts: HashMap<String, usize>,
    pub(super) local_exprs: HashMap<String, Expr>,
    pub(super) local_values: HashMap<String, StyleValue>,
    pub(super) metadata: Vec<RuleEntry>,
    pub(super) pending_runtime_injections: Vec<PendingRuntimeInjection>,
    pub(super) errors: Vec<String>,
    pub(super) fatal_error: Option<String>,
    pub(super) warnings: Vec<String>,
}
