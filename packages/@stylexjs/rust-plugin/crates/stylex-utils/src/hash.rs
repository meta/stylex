use std::{
  collections::hash_map::DefaultHasher,
  hash::{Hash, Hasher},
  mem::{Discriminant, discriminant},
  sync::LazyLock,
};

use xxhash_rust::xxh3::Xxh3Default;

use swc_core::{
  common::{DUMMY_SP, SyntaxContext},
  ecma::{
    ast::{
      ArrayLit, ArrowExpr, ArrowFunctionBody, AwaitExpr, BigInt, BinExpr, Bool, CallExpr, Callee,
      ComputedPropName, CondExpr, Expr, ExprOrSpread, Ident, IdentName, Import, Invalid, Lit,
      MemberExpr, MemberProp, MetaPropExpr, NewExpr, Null, Number, ObjectLit, OptCall,
      OptChainBase, OptChainExpr, ParenExpr, Pat, PrivateName, Prop, PropName, PropOrSpread, Regex,
      SeqExpr, Str, Super, SuperProp, SuperPropExpr, TaggedTpl, ThisExpr, Tpl, TplElement,
      UnaryExpr, UpdateExpr, YieldExpr,
    },
    utils::drop_span,
  },
};

const MAX_UNSPANNED_HASH_COLLECTION_LEN: usize = 128;
const BASE36_DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
const BASE62_DIGITS: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// Hashes a float value by converting to its bit representation first.
pub fn hash_f64(value: f64) -> u64 {
  let bits = value.to_bits();
  let mut hasher = DefaultHasher::new();
  bits.hash(&mut hasher);
  hasher.finish()
}

/// Runs murmur2 over the string's UTF-16 code units, each masked to its low byte, to match JS.
/// A lone surrogate from a JS string literal cannot survive into a `&str`, so it is not hashed.
#[inline]
fn murmur2_code_units(value: &str) -> u32 {
  // ASCII code units already equal their UTF-8 bytes.
  if value.is_ascii() {
    return murmur2::murmur2(value.as_bytes(), 1);
  }

  // The UTF-8 length is an upper bound on the UTF-16 length, so this reserves once.
  let mut code_units = Vec::with_capacity(value.len());
  code_units.extend(value.encode_utf16().map(|unit| (unit & 0xff) as u8));

  murmur2::murmur2(&code_units, 1)
}

/// Creates a base-36 hash of a string using murmur2.
#[inline]
pub fn create_hash(value: &str) -> String {
  to_base36(murmur2_code_units(value))
}

/// Creates a StyleX key hash without allocating through `format!`.
#[inline]
pub fn create_key_hash(namespace: &str, key: &str) -> String {
  let mut value = String::with_capacity(namespace.len() + 1 + key.len());
  value.push_str(namespace);
  value.push('.');
  value.push_str(key);

  create_hash(&value)
}

/// Returns an authored `--name` without its dashes, or else `prefix` plus the key hash.
/// Shared by `defineConsts` and `defineVars`.
#[inline]
pub fn create_authored_or_hashed_key(prefix: &str, namespace: &str, key: &str) -> String {
  match key.strip_prefix("--") {
    Some(authored) => authored.to_string(),
    None => {
      let hash = create_key_hash(namespace, key);
      let mut name = String::with_capacity(prefix.len() + hash.len());
      name.push_str(prefix);
      name.push_str(&hash);
      name
    },
  }
}

/// Writes `value` in the radix of `digits`; zero writes no digits, leaving it to the caller.
/// Panics if `buf` is too narrow for `value`.
fn to_radix<const RADIX: usize>(mut value: u32, digits: &[u8; RADIX], buf: &mut [u8]) -> String {
  let radix = RADIX as u32;
  let mut idx = buf.len();

  debug_assert!(
    u64::from(radix)
      .checked_pow(buf.len() as u32)
      .is_none_or(|capacity| capacity > u64::from(value))
  );

  while value > 0 {
    idx -= 1;
    buf[idx] = digits[(value % radix) as usize];
    value /= radix;
  }

  // `digits` is ASCII, so this never fails and avoids `unsafe` and `expect`.
  std::str::from_utf8(&buf[idx..])
    .map(str::to_owned)
    .unwrap_or_default()
}

/// Formats as base-36 in 7 digits, which covers `u32::MAX`.
fn to_base36(value: u32) -> String {
  if value == 0 {
    return "0".to_owned();
  }

  to_radix(value, BASE36_DIGITS, &mut [0u8; 7])
}

/// Formats as base-62 in 5 digits; zero yields `""`, kept because changing it renames classes.
fn to_base62(value: u32) -> String {
  to_radix(value, BASE62_DIGITS, &mut [0u8; 5])
}

/// Deterministic hash using `DefaultHasher` (SipHash-based).
pub fn stable_hash<T: Hash>(t: &T) -> u64 {
  let mut hasher = DefaultHasher::new();
  t.hash(&mut hasher);
  hasher.finish()
}

/// The structural key's hasher: xxh3 with a 128-bit digest.
/// The width matters because some consumers act on a key hit without confirming it.
struct WideHasher {
  state: Xxh3Default,
}

impl WideHasher {
  fn new() -> Self {
    Self {
      state: Xxh3Default::new(),
    }
  }

  fn finish_wide(&self) -> u128 {
    self.state.digest128()
  }
}

impl Hasher for WideHasher {
  fn write(&mut self, bytes: &[u8]) {
    self.state.update(bytes);
  }

  /// The 64-bit xxh3 digest, which is unrelated to the low half of `finish_wide`.
  fn finish(&self) -> u64 {
    self.state.digest()
  }
}

/// [`stable_hash`] over 128 bits, for caches that act on a hit without confirming it.
pub fn stable_hash_wide<T: Hash>(t: &T) -> u128 {
  let mut hasher = WideHasher::new();
  t.hash(&mut hasher);
  hasher.finish_wide()
}

/// Hashes an expression into a 128-bit structural key that ignores spans.
#[inline]
pub fn stable_hash_unspanned(path: &Expr) -> u128 {
  let mut hasher = WideHasher::new();

  if hash_expr_unspanned(path, &mut hasher) {
    hasher.finish_wide()
  } else {
    stable_hash_wide(&drop_span(path.clone()))
  }
}

/// Same as `stable_hash_unspanned(&Expr::Call(call.clone()))`, without the clone.
#[inline]
pub fn stable_hash_unspanned_call(call: &CallExpr) -> u128 {
  let mut hasher = WideHasher::new();

  // The discriminant ignores contents, so an allocation-free placeholder call suffices.
  let call_variant = Expr::Call(CallExpr {
    span: DUMMY_SP,
    ctxt: SyntaxContext::empty(),
    callee: Callee::Super(Super { span: DUMMY_SP }),
    args: Vec::new(),
    type_args: None,
  });
  discriminant(&call_variant).hash(&mut hasher);

  if hash_call_expr_unspanned(call, &mut hasher) {
    hasher.finish_wide()
  } else {
    stable_hash_wide(&drop_span(Expr::Call(call.clone())))
  }
}

/// Same as `stable_hash_unspanned(&Expr::Member(member.clone()))`, without the clone.
#[inline]
pub fn stable_hash_unspanned_member(member: &MemberExpr) -> u128 {
  // Computed once, because a placeholder `MemberExpr` would allocate its boxed `obj`.
  static MEMBER_DISCRIMINANT: LazyLock<Discriminant<Expr>> = LazyLock::new(|| {
    discriminant(&Expr::Member(MemberExpr {
      span: DUMMY_SP,
      obj: Box::new(Expr::Invalid(Invalid { span: DUMMY_SP })),
      prop: MemberProp::PrivateName(PrivateName {
        span: DUMMY_SP,
        name: "".into(),
      }),
    }))
  });

  let mut hasher = WideHasher::new();

  MEMBER_DISCRIMINANT.hash(&mut hasher);

  if hash_member_expr_unspanned(member, &mut hasher) {
    hasher.finish_wide()
  } else {
    stable_hash_wide(&drop_span(Expr::Member(member.clone())))
  }
}

/// Creates a short base-62 hash of a string using murmur2.
pub fn create_short_hash(value: &str) -> String {
  let hash = murmur2_code_units(value) % (62u32.pow(5));
  to_base62(hash)
}

fn hash_expr_unspanned<H: Hasher>(expr: &Expr, state: &mut H) -> bool {
  discriminant(expr).hash(state);

  match expr {
    Expr::This(this_expr) => hash_this_expr_unspanned(this_expr, state),
    Expr::Array(array) => hash_array_lit_unspanned(array, state),
    Expr::Object(object) => hash_object_lit_unspanned(object, state),
    Expr::Unary(unary) => hash_unary_expr_unspanned(unary, state),
    Expr::Update(update) => hash_update_expr_unspanned(update, state),
    Expr::Bin(bin) => hash_bin_expr_unspanned(bin, state),
    Expr::Member(member) => hash_member_expr_unspanned(member, state),
    Expr::SuperProp(super_prop) => hash_super_prop_expr_unspanned(super_prop, state),
    Expr::Cond(cond) => hash_cond_expr_unspanned(cond, state),
    Expr::Call(call) => hash_call_expr_unspanned(call, state),
    Expr::New(new_expr) => hash_new_expr_unspanned(new_expr, state),
    Expr::Seq(seq) => hash_seq_expr_unspanned(seq, state),
    Expr::Ident(ident) => hash_ident_unspanned(ident, state),
    Expr::Lit(lit) => hash_lit_unspanned(lit, state),
    Expr::Tpl(tpl) => hash_tpl_unspanned(tpl, state),
    Expr::TaggedTpl(tagged_tpl) => hash_tagged_tpl_unspanned(tagged_tpl, state),
    Expr::Arrow(arrow) => hash_arrow_expr_unspanned(arrow, state),
    Expr::Yield(yield_expr) => hash_yield_expr_unspanned(yield_expr, state),
    Expr::MetaProp(meta_prop) => hash_meta_prop_expr_unspanned(meta_prop, state),
    Expr::Await(await_expr) => hash_await_expr_unspanned(await_expr, state),
    Expr::Paren(paren) => hash_paren_expr_unspanned(paren, state),
    Expr::OptChain(opt_chain) => hash_opt_chain_expr_unspanned(opt_chain, state),
    Expr::Assign(_)
    | Expr::Fn(_)
    | Expr::Class(_)
    | Expr::JSXMember(_)
    | Expr::JSXNamespacedName(_)
    | Expr::JSXEmpty(_)
    | Expr::JSXElement(_)
    | Expr::JSXFragment(_)
    | Expr::TsTypeAssertion(_)
    | Expr::TsConstAssertion(_)
    | Expr::TsNonNull(_)
    | Expr::TsAs(_)
    | Expr::TsInstantiation(_)
    | Expr::TsSatisfies(_)
    | Expr::PrivateName(_)
    | Expr::Invalid(_) => false,
  }
}

fn hash_this_expr_unspanned<H: Hasher>(_this_expr: &ThisExpr, _state: &mut H) -> bool {
  true
}

fn hash_array_lit_unspanned<H: Hasher>(array: &ArrayLit, state: &mut H) -> bool {
  if array.elems.len() > MAX_UNSPANNED_HASH_COLLECTION_LEN {
    return false;
  }

  hash_slice_option_with(&array.elems, state, hash_expr_or_spread_unspanned)
}

fn hash_object_lit_unspanned<H: Hasher>(object: &ObjectLit, state: &mut H) -> bool {
  if object.props.len() > MAX_UNSPANNED_HASH_COLLECTION_LEN {
    return false;
  }

  hash_slice_with(&object.props, state, hash_prop_or_spread_unspanned)
}

fn hash_unary_expr_unspanned<H: Hasher>(unary: &UnaryExpr, state: &mut H) -> bool {
  unary.op.hash(state);
  hash_expr_unspanned(&unary.arg, state)
}

fn hash_update_expr_unspanned<H: Hasher>(update: &UpdateExpr, state: &mut H) -> bool {
  update.op.hash(state);
  update.prefix.hash(state);
  hash_expr_unspanned(&update.arg, state)
}

fn hash_bin_expr_unspanned<H: Hasher>(bin: &BinExpr, state: &mut H) -> bool {
  bin.op.hash(state);
  hash_expr_unspanned(&bin.left, state) && hash_expr_unspanned(&bin.right, state)
}

fn hash_member_expr_unspanned<H: Hasher>(member: &MemberExpr, state: &mut H) -> bool {
  hash_expr_unspanned(&member.obj, state) && hash_member_prop_unspanned(&member.prop, state)
}

fn hash_super_prop_expr_unspanned<H: Hasher>(super_prop: &SuperPropExpr, state: &mut H) -> bool {
  hash_super_unspanned(&super_prop.obj, state) && hash_super_prop_unspanned(&super_prop.prop, state)
}

fn hash_cond_expr_unspanned<H: Hasher>(cond: &CondExpr, state: &mut H) -> bool {
  hash_expr_unspanned(&cond.test, state)
    && hash_expr_unspanned(&cond.cons, state)
    && hash_expr_unspanned(&cond.alt, state)
}

fn hash_call_expr_unspanned<H: Hasher>(call: &CallExpr, state: &mut H) -> bool {
  call.ctxt.hash(state);

  hash_callee_unspanned(&call.callee, state)
    && hash_slice_with(&call.args, state, hash_expr_or_spread_unspanned)
    && hash_none(&call.type_args, state)
}

fn hash_new_expr_unspanned<H: Hasher>(new_expr: &NewExpr, state: &mut H) -> bool {
  new_expr.ctxt.hash(state);

  hash_expr_unspanned(&new_expr.callee, state)
    && hash_option_slice_with(&new_expr.args, state, hash_expr_or_spread_unspanned)
    && hash_none(&new_expr.type_args, state)
}

fn hash_seq_expr_unspanned<H: Hasher>(seq: &SeqExpr, state: &mut H) -> bool {
  hash_slice_with(&seq.exprs, state, |expr, state| {
    hash_expr_unspanned(expr, state)
  })
}

fn hash_lit_unspanned<H: Hasher>(lit: &Lit, state: &mut H) -> bool {
  discriminant(lit).hash(state);

  match lit {
    Lit::Str(str_lit) => hash_str_unspanned(str_lit, state),
    Lit::Bool(bool_lit) => hash_bool_unspanned(bool_lit, state),
    Lit::Null(null_lit) => hash_null_unspanned(null_lit, state),
    Lit::Num(number) => hash_number_unspanned(number, state),
    Lit::BigInt(big_int) => hash_big_int_unspanned(big_int, state),
    Lit::Regex(regex) => hash_regex_unspanned(regex, state),
    Lit::JSXText(_) => false,
  }
}

fn hash_tpl_unspanned<H: Hasher>(tpl: &Tpl, state: &mut H) -> bool {
  hash_slice_with(&tpl.exprs, state, |expr, state| {
    hash_expr_unspanned(expr, state)
  }) && hash_slice_with(&tpl.quasis, state, hash_tpl_element_unspanned)
}

fn hash_tagged_tpl_unspanned<H: Hasher>(tagged_tpl: &TaggedTpl, state: &mut H) -> bool {
  tagged_tpl.ctxt.hash(state);

  hash_expr_unspanned(&tagged_tpl.tag, state)
    && hash_none(&tagged_tpl.type_params, state)
    && hash_tpl_unspanned(&tagged_tpl.tpl, state)
}

fn hash_arrow_expr_unspanned<H: Hasher>(arrow: &ArrowExpr, state: &mut H) -> bool {
  arrow.ctxt.hash(state);
  arrow.is_async.hash(state);
  arrow.is_generator.hash(state);

  hash_slice_with(&arrow.params, state, hash_pat_unspanned)
    && hash_arrow_function_body_unspanned(&arrow.body, state)
    && hash_none(&arrow.type_params, state)
    && hash_none(&arrow.return_type, state)
}

fn hash_yield_expr_unspanned<H: Hasher>(yield_expr: &YieldExpr, state: &mut H) -> bool {
  yield_expr.delegate.hash(state);
  hash_option_with(&yield_expr.arg, state, |expr, state| {
    hash_expr_unspanned(expr, state)
  })
}

fn hash_meta_prop_expr_unspanned<H: Hasher>(meta_prop: &MetaPropExpr, state: &mut H) -> bool {
  meta_prop.kind.hash(state);
  true
}

fn hash_await_expr_unspanned<H: Hasher>(await_expr: &AwaitExpr, state: &mut H) -> bool {
  hash_expr_unspanned(&await_expr.arg, state)
}

fn hash_paren_expr_unspanned<H: Hasher>(paren: &ParenExpr, state: &mut H) -> bool {
  hash_expr_unspanned(&paren.expr, state)
}

fn hash_opt_chain_expr_unspanned<H: Hasher>(opt_chain: &OptChainExpr, state: &mut H) -> bool {
  opt_chain.optional.hash(state);
  hash_opt_chain_base_unspanned(&opt_chain.base, state)
}

fn hash_ident_unspanned<H: Hasher>(ident: &Ident, state: &mut H) -> bool {
  ident.ctxt.hash(state);
  ident.sym.hash(state);
  ident.optional.hash(state);
  true
}

fn hash_ident_name_unspanned<H: Hasher>(ident: &IdentName, state: &mut H) -> bool {
  ident.sym.hash(state);
  true
}

fn hash_private_name_unspanned<H: Hasher>(private_name: &PrivateName, state: &mut H) -> bool {
  private_name.name.hash(state);
  true
}

fn hash_str_unspanned<H: Hasher>(str_lit: &Str, state: &mut H) -> bool {
  str_lit.value.hash(state);
  str_lit.raw.hash(state);
  true
}

fn hash_bool_unspanned<H: Hasher>(bool_lit: &Bool, state: &mut H) -> bool {
  bool_lit.value.hash(state);
  true
}

fn hash_null_unspanned<H: Hasher>(_null_lit: &Null, _state: &mut H) -> bool {
  true
}

fn hash_number_unspanned<H: Hasher>(number: &Number, state: &mut H) -> bool {
  number.value.to_bits().hash(state);
  number.raw.hash(state);
  true
}

fn hash_big_int_unspanned<H: Hasher>(big_int: &BigInt, state: &mut H) -> bool {
  big_int.value.hash(state);
  big_int.raw.hash(state);
  true
}

fn hash_regex_unspanned<H: Hasher>(regex: &Regex, state: &mut H) -> bool {
  regex.exp.hash(state);
  regex.flags.hash(state);
  true
}

fn hash_tpl_element_unspanned<H: Hasher>(tpl_element: &TplElement, state: &mut H) -> bool {
  tpl_element.tail.hash(state);
  tpl_element.cooked.hash(state);
  tpl_element.raw.hash(state);
  true
}

fn hash_expr_or_spread_unspanned<H: Hasher>(expr_or_spread: &ExprOrSpread, state: &mut H) -> bool {
  expr_or_spread.spread.is_some().hash(state);
  hash_expr_unspanned(&expr_or_spread.expr, state)
}

fn hash_prop_or_spread_unspanned<H: Hasher>(prop_or_spread: &PropOrSpread, state: &mut H) -> bool {
  discriminant(prop_or_spread).hash(state);

  match prop_or_spread {
    PropOrSpread::Spread(spread) => hash_expr_unspanned(&spread.expr, state),
    PropOrSpread::Prop(prop) => hash_prop_unspanned(prop, state),
  }
}

fn hash_prop_unspanned<H: Hasher>(prop: &Prop, state: &mut H) -> bool {
  discriminant(prop).hash(state);

  match prop {
    Prop::Shorthand(ident) => hash_ident_unspanned(ident, state),
    Prop::KeyValue(key_value) => {
      hash_prop_name_unspanned(&key_value.key, state)
        && hash_expr_unspanned(&key_value.value, state)
    },
    Prop::Assign(assign) => {
      hash_ident_unspanned(&assign.key, state) && hash_expr_unspanned(&assign.value, state)
    },
    Prop::Getter(_) | Prop::Setter(_) | Prop::Method(_) => false,
  }
}

fn hash_prop_name_unspanned<H: Hasher>(prop_name: &PropName, state: &mut H) -> bool {
  discriminant(prop_name).hash(state);

  match prop_name {
    PropName::Ident(ident) => hash_ident_name_unspanned(ident, state),
    PropName::Str(str_lit) => hash_str_unspanned(str_lit, state),
    PropName::Num(number) => hash_number_unspanned(number, state),
    PropName::Computed(computed) => hash_computed_prop_name_unspanned(computed, state),
    PropName::BigInt(big_int) => hash_big_int_unspanned(big_int, state),
  }
}

fn hash_member_prop_unspanned<H: Hasher>(member_prop: &MemberProp, state: &mut H) -> bool {
  discriminant(member_prop).hash(state);

  match member_prop {
    MemberProp::Ident(ident) => hash_ident_name_unspanned(ident, state),
    MemberProp::PrivateName(private_name) => hash_private_name_unspanned(private_name, state),
    MemberProp::Computed(computed) => hash_computed_prop_name_unspanned(computed, state),
  }
}

fn hash_computed_prop_name_unspanned<H: Hasher>(
  computed: &ComputedPropName,
  state: &mut H,
) -> bool {
  hash_expr_unspanned(&computed.expr, state)
}

fn hash_super_prop_unspanned<H: Hasher>(super_prop: &SuperProp, state: &mut H) -> bool {
  discriminant(super_prop).hash(state);

  match super_prop {
    SuperProp::Ident(ident) => hash_ident_name_unspanned(ident, state),
    SuperProp::Computed(computed) => hash_computed_prop_name_unspanned(computed, state),
  }
}

fn hash_callee_unspanned<H: Hasher>(callee: &Callee, state: &mut H) -> bool {
  discriminant(callee).hash(state);

  match callee {
    Callee::Super(super_expr) => hash_super_unspanned(super_expr, state),
    Callee::Import(import) => hash_import_unspanned(import, state),
    Callee::Expr(expr) => hash_expr_unspanned(expr, state),
  }
}

fn hash_super_unspanned<H: Hasher>(_super_expr: &Super, _state: &mut H) -> bool {
  true
}

fn hash_import_unspanned<H: Hasher>(import: &Import, state: &mut H) -> bool {
  import.phase.hash(state);
  true
}

fn hash_arrow_function_body_unspanned<H: Hasher>(
  arrow_function_body: &ArrowFunctionBody,
  state: &mut H,
) -> bool {
  discriminant(arrow_function_body).hash(state);

  match arrow_function_body {
    ArrowFunctionBody::Expr(expr) => hash_expr_unspanned(expr, state),
    ArrowFunctionBody::FunctionBody(_) => false,
  }
}

fn hash_pat_unspanned<H: Hasher>(pat: &Pat, state: &mut H) -> bool {
  discriminant(pat).hash(state);

  match pat {
    Pat::Ident(ident) => {
      hash_ident_unspanned(&ident.id, state) && hash_none(&ident.type_ann, state)
    },
    Pat::Expr(expr) => hash_expr_unspanned(expr, state),
    Pat::Array(_) | Pat::Rest(_) | Pat::Object(_) | Pat::Assign(_) | Pat::Invalid(_) => false,
  }
}

fn hash_opt_chain_base_unspanned<H: Hasher>(base: &OptChainBase, state: &mut H) -> bool {
  discriminant(base).hash(state);

  match base {
    OptChainBase::Member(member) => hash_member_expr_unspanned(member, state),
    OptChainBase::Call(call) => hash_opt_call_unspanned(call, state),
  }
}

fn hash_opt_call_unspanned<H: Hasher>(call: &OptCall, state: &mut H) -> bool {
  call.ctxt.hash(state);

  hash_expr_unspanned(&call.callee, state)
    && hash_slice_with(&call.args, state, hash_expr_or_spread_unspanned)
    && hash_none(&call.type_args, state)
}

fn hash_slice_with<T, H: Hasher, F>(values: &[T], state: &mut H, mut hash_value: F) -> bool
where
  F: FnMut(&T, &mut H) -> bool,
{
  values.len().hash(state);

  values.iter().all(|value| hash_value(value, state))
}

fn hash_slice_option_with<T, H: Hasher, F>(
  values: &[Option<T>],
  state: &mut H,
  mut hash_value: F,
) -> bool
where
  F: FnMut(&T, &mut H) -> bool,
{
  values.len().hash(state);

  values
    .iter()
    .all(|value| hash_option_with(value, state, &mut hash_value))
}

fn hash_option_slice_with<T, H: Hasher, F>(
  values: &Option<Vec<T>>,
  state: &mut H,
  mut hash_value: F,
) -> bool
where
  F: FnMut(&T, &mut H) -> bool,
{
  hash_option_with(values, state, |values, state| {
    hash_slice_with(values, state, &mut hash_value)
  })
}

fn hash_option_with<T, H: Hasher, F>(value: &Option<T>, state: &mut H, mut hash_value: F) -> bool
where
  F: FnMut(&T, &mut H) -> bool,
{
  value.is_some().hash(state);

  match value {
    Some(value) => hash_value(value, state),
    None => true,
  }
}

fn hash_none<T, H: Hasher>(value: &Option<T>, state: &mut H) -> bool {
  value.is_some().hash(state);
  value.is_none()
}

#[cfg(test)]
#[path = "tests/hash_test.rs"]
mod tests;
