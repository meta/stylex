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

/// Runs murmur2 over the string's UTF-16 code units, each masked to its low
/// byte.
///
/// The block count comes from the string's length in UTF-16 code units, and
/// each byte read is a code unit masked to its low byte, so the hash is defined
/// over UTF-16 code units rather than UTF-8 bytes. The two encodings only
/// coincide while the
/// input is ASCII; past that, hashing UTF-8 bytes yields a different class name
/// for byte-identical CSS — which is visible in `content` values, non-ASCII
/// `font-family` names, unicode custom properties, and export ids derived from
/// a non-ASCII file path.
///
/// Masking each code unit to its low byte is lossy by definition: `\u{1F389}`
/// hashes as its two surrogate halves, `0xD83C` and `0xDF89`, contributing
/// `0x3C` and `0x89`.
///
/// Limitation: a `&str` cannot hold an *unpaired* surrogate, while a JS string
/// literal can (`Str::value` is a `Wtf8Atom`). An input that reached this
/// function has therefore already lost any lone surrogate, and no masking
/// choice here can recover the code unit it would have contributed. Such
/// literals are vanishingly rare in style values, and are the one input class
/// whose hash is not fully determined by the authored source.
#[inline]
fn murmur2_code_units(value: &str) -> u32 {
  // Every ASCII scalar is a single UTF-16 code unit below `0x80`, so its low
  // byte is already its UTF-8 byte — hash in place, without a buffer.
  if value.is_ascii() {
    return murmur2::murmur2(value.as_bytes(), 1);
  }

  // A non-ASCII scalar always costs at least as many UTF-8 bytes as UTF-16 code
  // units (2 or 3 bytes for one unit, 4 for a surrogate pair), so the UTF-8
  // length is an upper bound that sizes the buffer in one allocation —
  // `EncodeUtf16`'s own lower-bound hint would under-reserve and reallocate.
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

/// The name a constant or a variable is written under in the stylesheet.
///
/// A name the author spelled as a CSS custom property is kept as it was
/// written, without the two leading dashes. Every other name is hashed and
/// carries `prefix` in front of it.
///
/// `defineConsts` and `defineVars` both name their keys this way. The rule is
/// held one time, so the two cannot disagree about it.
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

/// Writes `value` in the radix implied by `digits`, least-significant digit
/// first from the back of `buf`, and collects the populated suffix.
///
/// A zero value writes no digits at all, leaving the representation of zero to
/// the caller — the two callers disagree about it, which is the whole reason
/// they stay separate wrappers.
///
/// `buf` must be wide enough for the largest `value` the caller admits; a
/// narrower buffer panics rather than truncating silently — the `debug_assert`
/// names the requirement, and in release the index of the wrapped `idx` still
/// panics instead of writing a wrong digit.
///
/// The assertion carries no formatted message on purpose: its arguments would be
/// evaluated only on a failure no caller can reach, leaving regions that no test
/// can ever cover.
fn to_radix(mut value: u32, digits: &[u8], buf: &mut [u8]) -> String {
  let radix = digits.len() as u32;
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

  // `digits` holds only ASCII alphanumerics, so the populated suffix is valid
  // UTF-8 by construction. Validating it is a single pass over at most 7 bytes
  // and copies once, where widening each byte to a `char` would re-encode digit
  // by digit; `unwrap_or_default` keeps the check without `unsafe` and without
  // an `expect` the project forbids.
  std::str::from_utf8(&buf[idx..])
    .map(str::to_owned)
    .unwrap_or_default()
}

/// `u32::MAX` in base-36 is `"1z141z3"`, so 7 digits covers every input.
///
/// Zero is `"0"`, spelled explicitly because the digit loop would otherwise
/// leave the buffer empty.
fn to_base36(value: u32) -> String {
  if value == 0 {
    return "0".to_owned();
  }

  to_radix(value, BASE36_DIGITS, &mut [0u8; 7])
}

/// `62u32.pow(5) - 1` in base-62 is `"zzzzz"`, so 5 digits covers every value
/// `create_short_hash` reduces into range.
///
/// Zero is the empty string: `to_radix` loops `while value > 0` and so yields
/// `""` rather than `"0"`. That is reachable — the murmur2 value lands on a
/// multiple of `62^5` for roughly one input in 916 million — and it is left
/// uncorrected on purpose. This feeds class-name hashing, where changing the
/// spelling of any digit silently renames every class it reaches.
fn to_base62(value: u32) -> String {
  to_radix(value, BASE62_DIGITS, &mut [0u8; 5])
}

/// Deterministic hash using `DefaultHasher` (SipHash-based).
pub fn stable_hash<T: Hash>(t: &T) -> u64 {
  let mut hasher = DefaultHasher::new();
  t.hash(&mut hasher);
  hasher.finish()
}

/// The structural key's hasher: xxh3, taken 128 bits wide.
///
/// Two properties are needed at once, and the pairing of a width with a speed is
/// the whole reason this is not `DefaultHasher`.
///
/// **128 bits, because two consumers do not confirm a hit.** The evaluator's memo
/// returns a cached fold, and the before-declaration injection slot splices
/// metadata, on the strength of the key alone. At 64 bits and ten thousand
/// distinct expressions in a file that is a collision every `1e-12` files -- a
/// wrong folded value or a misplaced injection, silently, with no diagnostic. At
/// 128 it is past `1e-31`.
///
/// **In one pass, because the obvious way to get 128 bits costs two.** Two
/// salted `DefaultHasher` states fed by the same walk were measured at +49% on
/// the key and +5.8% on a whole production transform of a 400-`create` file --
/// paying that forever to remove a failure that arrives once per `1e4` years is
/// the wrong trade. xxh3 emits 128 bits from a single pass, and is enough faster
/// than SipHash that the wider key is also the cheaper one.
///
/// Nothing depends on the values. No consumer persists a key, none derives a
/// class name from one, and output order comes from source order -- which is what
/// makes changing the algorithm a contained decision rather than a rename of
/// every class in every project. `DefaultHasher` was never stable across Rust
/// releases either, so nothing could have depended on it and been correct.
///
/// `Xxh3Default` rather than `Xxh3`: the latter carries a seed and a 192-byte
/// custom secret, and copies the secret into every instance. One instance is
/// built per key, which makes that copy the largest fixed cost of a small key.
/// The two digest identically at the default seed and secret, and no consumer
/// needs a seed.
///
/// Only `write` is implemented. Every typed `Hasher` method defaults to routing
/// through it, so the stream is identical whatever a `Hash` implementation calls.
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

  /// xxh3's 64-bit digest, present only because `Hasher` requires it.
  ///
  /// It is **not** the low half of [`WideHasher::finish_wide`]: xxh3's 64-bit and
  /// 128-bit digests are separate constructions over the same stream, so the two
  /// answers are unrelated numbers. Nothing here reads this, and a caller that
  /// reached for the familiar `finish` would get a different, narrower key
  /// without being told -- which is the whole hazard the width exists to remove.
  /// Pinned in `wide_hasher_tests`.
  fn finish(&self) -> u64 {
    self.state.digest()
  }
}

/// [`stable_hash`] over 128 bits.
///
/// For the fallback arms of the structural key, and for any other cache whose
/// reads act on a hash hit without confirming it -- the span cache in
/// `stylex-transform`'s code-frame lookup is the other one, and a collision
/// there is a wrong `file:line` in the output rather than a slow path.
pub fn stable_hash_wide<T: Hash>(t: &T) -> u128 {
  let mut hasher = WideHasher::new();
  t.hash(&mut hasher);
  hasher.finish_wide()
}

/// Hashes an expression into a stable structural key for the evaluator cache,
/// treating spans as insignificant for the common expression shapes.
///
/// Common shapes are hashed span-insensitively in place (no clone, no
/// span-stripping) — this is the hot path that keeps structurally-equal
/// expressions at different source positions sharing a cache entry. The rare
/// unsupported shapes (functions, classes, JSX, TS-only nodes, oversized
/// collections) fall back to hashing a span-stripped clone so the public
/// contract stays span-insensitive for every expression shape.
///
/// 128 bits wide, because two of its consumers act on a hit without confirming
/// it -- see [`WideHasher`] for what that buys and why it is not two hashes.
#[inline]
pub fn stable_hash_unspanned(path: &Expr) -> u128 {
  let mut hasher = WideHasher::new();

  if hash_expr_unspanned(path, &mut hasher) {
    hasher.finish_wide()
  } else {
    stable_hash_wide(&drop_span(path.clone()))
  }
}

/// Hashes a [`CallExpr`] producing the exact same key as
/// `stable_hash_unspanned(&Expr::Call(call.clone()))` — so a call can be looked
/// up against a map keyed by whole-`Expr` spread hashes — without cloning the
/// call into an owned `Expr` on the common, fully-hashable path.
///
/// It reproduces [`stable_hash_unspanned`]'s layout for the `Expr::Call`
/// variant: the variant discriminant followed by the call body. The owned
/// `Expr::Call` is only materialized on the rare fallback path (a call whose
/// argument has a shape the in-place hasher does not cover), keeping the key
/// identical in every case.
#[inline]
pub fn stable_hash_unspanned_call(call: &CallExpr) -> u128 {
  let mut hasher = WideHasher::new();

  // `discriminant` over the `Expr::Call` variant is independent of the call's
  // contents, so a throwaway stack value (no heap allocation) yields the same
  // discriminant `hash_expr_unspanned` writes for a real `Expr::Call`.
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

/// Hashes a [`MemberExpr`] producing the exact same key as
/// `stable_hash_unspanned(&Expr::Member(member.clone()))`, without cloning it
/// into an owned `Expr` on the common, fully hashable path.
///
/// Nothing relies on that parity today: `callee_members` is the only map keyed
/// this way, and it is written and read through this function alone. Kept
/// identical anyway so a future consumer keyed by whole-`Expr` hashes can probe
/// it without a special case, and pinned by the tests beside it.
///
/// The counterpart of [`stable_hash_unspanned_call`], and it exists for the
/// same reason: the caller holds a borrowed node and the question it asks runs
/// on a hot path, so materializing an owned `Expr` per lookup is the cost the
/// helper removes.
///
/// Unlike the call variant, the `Expr::Member` discriminant cannot be taken
/// from a throwaway stack value: `MemberExpr::obj` is a `Box`, so building one
/// would allocate on every call. It is computed once into a `LazyLock`
/// instead. `Discriminant<T>` is `Send + Sync` for every `T` and depends only
/// on the variant, never on the contents.
#[inline]
pub fn stable_hash_unspanned_member(member: &MemberExpr) -> u128 {
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
