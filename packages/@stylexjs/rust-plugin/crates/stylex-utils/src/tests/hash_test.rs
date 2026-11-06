//! Shared helpers for the cases below.
//!
//! `parse_expr` and the tower builder are used by more than one of the modules
//! in this file, and the counting hasher is how two of them ask a question the
//! public entry point does not answer: how much of the tree the walk touched,
//! and whether it took its fallback arm.

#[cfg(test)]
use std::hash::Hasher;

#[cfg(test)]
use crate::test_support;
#[cfg(test)]
use swc_core::{
  common::{DUMMY_SP, SyntaxContext},
  ecma::ast::{BinExpr, BinaryOp, Expr, Ident, Lit, Number},
};

/// A [`Hasher`] that records how much the walk fed it and hashes nothing.
///
/// Bytes rather than nodes, because the walk writes through the `Hash`
/// implementations of `Atom`, `SyntaxContext` and the operator enums as well as
/// its own discriminants, and all of that is work the key pays for. It is
/// proportional to the node count for a fixed shape, which is all the cases
/// below compare.
///
/// `finish` is never called: the walk only writes, and the real
/// `stable_hash_unspanned` finishes its own hasher. It answers zero rather than
/// a digest so that a case reading it would notice.
#[cfg(test)]
#[derive(Default)]
struct ByteCounter {
  bytes: u64,
}

#[cfg(test)]
impl Hasher for ByteCounter {
  fn write(&mut self, bytes: &[u8]) {
    self.bytes += bytes.len() as u64;
  }

  fn finish(&self) -> u64 {
    0
  }
}

/// The bytes one whole-subtree hash of `expr` spends -- what a single memo
/// lookup costs the fold at one level.
///
/// Panics if the walk declines the shape, because a declined shape is measuring
/// the deep clone instead of the walk. Use [`hashed_in_place`] to ask which arm
/// runs.
#[cfg(test)]
fn key_cost(expr: &Expr) -> u64 {
  let mut counter = ByteCounter::default();

  assert!(
    super::hash_expr_unspanned(expr, &mut counter),
    "measuring the cost of a shape the in-place walk declines"
  );

  counter.bytes
}

/// Whether the in-place walk covered `expr`, or handed it to the deep-clone
/// fallback.
///
/// The arm is not observable from the hash -- both arms are span-insensitive and
/// neither reports which ran -- so the cases that care ask the walk directly.
#[cfg(test)]
fn hashed_in_place(expr: &Expr) -> bool {
  let mut counter = ByteCounter::default();

  super::hash_expr_unspanned(expr, &mut counter)
}

#[cfg(test)]
fn number(value: f64) -> Expr {
  Expr::Lit(Lit::Num(Number {
    span: DUMMY_SP,
    value,
    raw: None,
  }))
}

/// `MY_CONST` under `depth` levels of `+ 1` -- the shape the evaluation-depth
/// cases are measured against, and the one the fold descends a straight left
/// spine of.
#[cfg(test)]
fn arithmetic_tower(depth: usize) -> Expr {
  let mut expr = Expr::Ident(Ident::new(
    "MY_CONST".into(),
    DUMMY_SP,
    SyntaxContext::empty(),
  ));

  for _ in 0..depth {
    expr = Expr::Bin(BinExpr {
      span: DUMMY_SP,
      op: BinaryOp::Add,
      left: Box::new(expr),
      right: Box::new(number(1.0)),
    });
  }

  expr
}

/// Parses one expression out of source, in the grammar the compiler reads.
///
/// Every case here reads the same grammar, so the syntax is supplied once.
#[cfg(test)]
fn parse_expr(source: &str) -> Expr {
  test_support::parse_expr(source, test_support::es_jsx())
}

#[cfg(test)]
mod create_hash_tests {
  use crate::hash::{create_authored_or_hashed_key, create_hash, create_key_hash};

  #[test]
  fn returns_consistent_hash() {
    let hash1 = create_hash("hello");
    let hash2 = create_hash("hello");
    assert_eq!(hash1, hash2);
  }

  #[test]
  fn different_inputs_produce_different_hashes() {
    assert_ne!(create_hash("hello"), create_hash("world"));
  }

  #[test]
  fn returns_non_empty_string() {
    assert!(!create_hash("test").is_empty());
  }

  #[test]
  fn handles_empty_string() {
    let hash = create_hash("");
    assert!(!hash.is_empty());
  }

  #[test]
  fn handles_unicode_input() {
    let hash = create_hash("日本語");
    assert!(!hash.is_empty());
  }

  #[test]
  fn handles_long_input() {
    let long = "a".repeat(10000);
    let hash = create_hash(&long);
    assert!(!hash.is_empty());
  }

  #[test]
  fn matches_radix_fmt_base36_output() {
    // ASCII only: this pins `to_base36` against `radix_fmt`, and ASCII is the
    // range where the raw murmur2-over-bytes value is still the hashed value.
    // Non-ASCII parity is covered by `matches_upstream_hash_for_non_ascii`.
    for input in ["", "hello", "world", "a very long input string"] {
      let raw = murmur2::murmur2(input.as_bytes(), 1);
      assert_eq!(create_hash(input), radix_fmt::radix(raw, 36).to_string());
    }
  }

  /// Golden vectors produced by running `murmurhash2_32_gc` from
  /// `@stylexjs/babel-plugin@0.19.0`'s `src/shared/hash.js` (`hash`, i.e. seed
  /// 1, base 36) over each input.
  ///
  /// These are the values a class name must be built from for the two compilers
  /// to be interchangeable. The `content` pair is the reproduction from
  /// https://github.com/Dwlad90/stylex-swc-plugin/issues/1248, where both
  /// compilers emitted byte-identical CSS under different class names.
  #[test]
  fn matches_upstream_hash_for_non_ascii() {
    for (input, expected) in [
      ("<>content\"•\"null", "e0tt08"),
      ("<>content'•'null", "wywlkd"),
      ("•", "19lqsls"),
      ("日本語", "csni84"),
      ("<>font-family\"日本語\"null", "1v1enns"),
      ("--épaisseur", "xirl07"),
      // Astral scalars hash as their surrogate halves, matching `charCodeAt`.
      ("🎉", "yd2se2"),
      ("<>content\"🎉\"null", "w4zyq6"),
      ("a🎉b", "yp8u9f"),
    ] {
      assert_eq!(create_hash(input), expected, "input: {:?}", input);
    }
  }

  /// The ASCII fast path skips the UTF-16 buffer on the claim that an ASCII
  /// scalar's low code-unit byte *is* its UTF-8 byte. Pin that claim against the
  /// general path rather than trusting it, since every ASCII class name in the
  /// codebase is produced by the branch this test is the only check on.
  #[test]
  fn ascii_fast_path_agrees_with_the_utf16_path() {
    for input in [
      "",
      "a",
      "ab",
      "abc",
      "hello",
      "<>content\"x\"null",
      "a very long input string",
    ] {
      let code_units: Vec<u8> = input
        .encode_utf16()
        .map(|unit| (unit & 0xff) as u8)
        .collect();

      assert_eq!(
        create_hash(input),
        radix_fmt::radix(murmur2::murmur2(&code_units, 1), 36).to_string(),
        "input: {:?}",
        input
      );
    }
  }

  /// A hash of exactly zero renders as `"0"` in base 36 — `(0).toString(36)` —
  /// where the same value renders as the empty string in base 62. The two
  /// wrappers disagree deliberately, so each side needs a reachable case;
  /// `murmur2("k4127446806") == 0` is this one, and
  /// `matches_upstream_empty_short_hash_when_value_is_a_multiple_of_62_pow_5`
  /// covers the other.
  #[test]
  fn matches_upstream_hash_when_value_is_zero() {
    assert_eq!(create_hash("k4127446806"), "0");
  }

  /// The ASCII fast path and the UTF-16 path must agree wherever both apply.
  #[test]
  fn matches_upstream_hash_for_ascii() {
    for (input, expected) in [
      ("", "ph554m"),
      ("hello", "1a4283y"),
      ("world", "ck8emq"),
      ("a very long input string", "1ida9zx"),
    ] {
      assert_eq!(create_hash(input), expected, "input: {:?}", input);
    }
  }

  #[test]
  fn create_key_hash_matches_joined_key_hash() {
    assert_eq!(
      create_key_hash("Button.stylex", "root"),
      create_hash("Button.stylex.root")
    );
  }

  #[test]
  fn an_authored_custom_property_keeps_its_name_without_the_dashes() {
    assert_eq!(
      create_authored_or_hashed_key("x", "Theme.stylex", "--brand-color"),
      "brand-color"
    );
    // The prefix and the namespace say nothing about an authored name.
    assert_eq!(
      create_authored_or_hashed_key("", "", "--brand-color"),
      "brand-color"
    );
    // Only the two leading dashes are removed, and only once.
    assert_eq!(create_authored_or_hashed_key("x", "n", "----a"), "--a");
  }

  #[test]
  fn every_other_name_is_hashed_behind_the_prefix() {
    let expected = format!("x{}", create_key_hash("Theme.stylex", "brandColor"));

    assert_eq!(
      create_authored_or_hashed_key("x", "Theme.stylex", "brandColor"),
      expected
    );

    // One dash is not a custom property, so the name is hashed.
    assert!(create_authored_or_hashed_key("x", "n", "-a").starts_with('x'));
    // An empty key is a name like any other and is hashed too.
    assert!(!create_authored_or_hashed_key("x", "n", "").is_empty());
  }
}

#[cfg(test)]
mod stable_hash_tests {
  use crate::hash::{stable_hash, stable_hash_unspanned};
  use swc_core::{
    common::{BytePos, Span, SyntaxContext},
    ecma::ast::{Expr, Ident, IdentName, MemberExpr, MemberProp},
  };

  #[test]
  fn returns_consistent_hash_for_same_value() {
    assert_eq!(stable_hash(&42u64), stable_hash(&42u64));
  }

  #[test]
  fn different_values_produce_different_hashes() {
    assert_ne!(stable_hash(&1u64), stable_hash(&2u64));
  }

  #[test]
  fn works_with_strings() {
    assert_eq!(stable_hash(&"test"), stable_hash(&"test"));
    assert_ne!(stable_hash(&"a"), stable_hash(&"b"));
  }

  #[test]
  fn works_with_tuples() {
    assert_eq!(stable_hash(&(1, 2)), stable_hash(&(1, 2)));
    assert_ne!(stable_hash(&(1, 2)), stable_hash(&(2, 1)));
  }

  #[test]
  fn unspanned_expr_hash_ignores_nested_spans() {
    let expr_a = member_expr("foo", 1, "bar", 4);
    let expr_b = member_expr("foo", 10, "bar", 40);

    assert_eq!(
      stable_hash_unspanned(&expr_a),
      stable_hash_unspanned(&expr_b)
    );
  }

  #[test]
  fn unspanned_expr_hash_preserves_structure() {
    let expr_a = member_expr("foo", 1, "bar", 4);
    let expr_b = member_expr("foo", 1, "baz", 4);

    assert_ne!(
      stable_hash_unspanned(&expr_a),
      stable_hash_unspanned(&expr_b)
    );
  }

  fn member_expr(obj: &str, obj_start: u32, prop: &str, prop_start: u32) -> Expr {
    Expr::Member(MemberExpr {
      span: span(obj_start, prop_start + 3),
      obj: Box::new(Expr::Ident(ident(obj, obj_start))),
      prop: MemberProp::Ident(IdentName::new(
        prop.into(),
        span(prop_start, prop_start + 3),
      )),
    })
  }

  fn ident(sym: &str, start: u32) -> Ident {
    Ident {
      span: span(start, start + sym.len() as u32),
      ctxt: SyntaxContext::empty(),
      sym: sym.into(),
      optional: false,
    }
  }

  fn span(start: u32, end: u32) -> Span {
    Span::new(BytePos(start), BytePos(end))
  }
}

#[cfg(test)]
mod create_short_hash_tests {
  use crate::hash::create_short_hash;

  #[test]
  fn returns_consistent_hash() {
    assert_eq!(create_short_hash("hello"), create_short_hash("hello"));
  }

  #[test]
  fn different_inputs_produce_different_hashes() {
    assert_ne!(create_short_hash("hello"), create_short_hash("world"));
  }

  #[test]
  fn returns_non_empty_string() {
    assert!(!create_short_hash("test").is_empty());
  }

  #[test]
  fn produces_short_output() {
    // base62 encoded, mod 62^5, should be at most 5 chars
    assert!(create_short_hash("test").len() <= 5);
  }

  /// Golden vectors produced by running `createShortHash` from
  /// `@stylexjs/babel-plugin@0.19.0`'s `src/shared/hash.js` over each input.
  #[test]
  fn matches_upstream_short_hash() {
    for (input, expected) in [
      ("", "gFYqE"),
      ("hello", "2hHSQ"),
      ("a very long input string", "aTxoT"),
      ("<>content\"•\"null", "vNlxw"),
      ("<>content'•'null", "AuiFx"),
      ("日本語", "qMRvw"),
      ("--épaisseur", "DAgHH"),
      ("🎉", "GcIck"),
      // Fewer than 5 chars whenever the value needs fewer base-62 digits.
      ("•", "cBiC"),
    ] {
      assert_eq!(create_short_hash(input), expected, "input: {:?}", input);
    }
  }

  /// `toBase62` loops `while (_num > 0)`, so a value of zero yields the empty
  /// string rather than `"0"`. `murmur2("k580145052") == 2748398496 == 3 * 62^5`,
  /// making this the reachable case rather than a hypothetical one.
  ///
  /// The empty string is what upstream emits, so it is reproduced rather than
  /// corrected — diverging here would reintroduce the class of mismatch this
  /// module exists to close.
  #[test]
  fn matches_upstream_empty_short_hash_when_value_is_a_multiple_of_62_pow_5() {
    assert_eq!(create_short_hash("k580145052"), "");
  }
}

#[cfg(test)]
mod hash_f64_tests {
  use crate::hash::hash_f64;

  #[test]
  fn returns_consistent_hash_for_same_value() {
    assert_eq!(hash_f64(1.23456), hash_f64(1.23456));
  }

  #[test]
  fn different_values_produce_different_hashes() {
    assert_ne!(hash_f64(1.0), hash_f64(2.0));
  }

  #[test]
  fn zero_and_neg_zero_differ() {
    // In IEEE 754, 0.0 and -0.0 have different bit patterns
    assert_ne!(hash_f64(0.0), hash_f64(-0.0));
  }

  #[test]
  fn handles_special_values() {
    let _ = hash_f64(f64::INFINITY);
    let _ = hash_f64(f64::NEG_INFINITY);
    let _ = hash_f64(f64::NAN);
  }
}

#[cfg(test)]
mod unspanned_fast_path_tests {
  use super::super::{
    create_hash, stable_hash_unspanned, stable_hash_unspanned_call, stable_hash_unspanned_member,
    stable_hash_wide, to_base36,
  };
  use swc_core::{
    common::{DUMMY_SP, SyntaxContext},
    ecma::{
      ast::{
        ArrayLit, ArrowExpr, ArrowFunctionBody, AssignProp, AwaitExpr, BigInt, BinExpr, BinaryOp,
        Bool, Callee, ComputedPropName, CondExpr, Expr, ExprOrSpread, Ident, IdentName, Import,
        ImportPhase, JSXText, KeyValueProp, Lit, MemberExpr, MemberProp, MetaPropExpr,
        MetaPropKind, NewExpr, Null, Number, ObjectLit, OptCall, OptChainBase, OptChainExpr,
        ParenExpr, Pat, PrivateName, Prop, PropName, PropOrSpread, Regex, SeqExpr, SpreadElement,
        Str, Super, SuperProp, SuperPropExpr, TaggedTpl, ThisExpr, Tpl, TplElement, UnaryExpr,
        UnaryOp, UpdateExpr, UpdateOp, YieldExpr,
      },
      utils::drop_span,
    },
  };

  use super::parse_expr;

  #[test]
  fn base36_handles_zero_directly() {
    assert_eq!(to_base36(0), "0");
    assert_eq!(create_hash("Button.root"), create_hash("Button.root"));
  }

  #[test]
  fn unspanned_hash_fast_path_covers_supported_expression_shapes() {
    let expressions = vec![
      Expr::This(ThisExpr { span: DUMMY_SP }),
      Expr::Array(ArrayLit {
        span: DUMMY_SP,
        elems: vec![
          Some(expr_or_spread(ident("a"))),
          Some(ExprOrSpread {
            spread: Some(DUMMY_SP),
            expr: Box::new(ident("b")),
          }),
          None,
        ],
      }),
      Expr::Object(ObjectLit {
        span: DUMMY_SP,
        props: vec![
          PropOrSpread::Spread(SpreadElement {
            dot3_token: DUMMY_SP,
            expr: Box::new(ident("spread")),
          }),
          prop(Prop::Shorthand(ident_pat("short"))),
          prop(Prop::KeyValue(KeyValueProp {
            key: PropName::Ident(IdentName::new("ident".into(), DUMMY_SP)),
            value: Box::new(number(1.0)),
          })),
          prop(Prop::KeyValue(KeyValueProp {
            key: PropName::Str(Str {
              span: DUMMY_SP,
              value: "str".into(),
              raw: None,
            }),
            value: Box::new(string("value")),
          })),
          prop(Prop::KeyValue(KeyValueProp {
            key: PropName::Num(Number {
              span: DUMMY_SP,
              value: 1.0,
              raw: None,
            }),
            value: Box::new(bool_expr(true)),
          })),
          prop(Prop::KeyValue(KeyValueProp {
            key: PropName::Computed(ComputedPropName {
              span: DUMMY_SP,
              expr: Box::new(ident("computed")),
            }),
            value: Box::new(null()),
          })),
          prop(Prop::KeyValue(KeyValueProp {
            key: PropName::BigInt(BigInt {
              span: DUMMY_SP,
              value: Box::new(7u32.into()),
              raw: None,
            }),
            value: Box::new(regex("x", "g")),
          })),
          prop(Prop::Assign(AssignProp {
            span: DUMMY_SP,
            key: ident_pat("assigned"),
            value: Box::new(string("assigned-value")),
          })),
        ],
      }),
      Expr::Unary(UnaryExpr {
        span: DUMMY_SP,
        op: UnaryOp::Bang,
        arg: Box::new(bool_expr(false)),
      }),
      Expr::Update(UpdateExpr {
        span: DUMMY_SP,
        op: UpdateOp::PlusPlus,
        prefix: true,
        arg: Box::new(ident("counter")),
      }),
      Expr::Bin(BinExpr {
        span: DUMMY_SP,
        op: BinaryOp::Add,
        left: Box::new(number(1.0)),
        right: Box::new(number(2.0)),
      }),
      Expr::Member(MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(ident("obj")),
        prop: MemberProp::Ident(IdentName::new("prop".into(), DUMMY_SP)),
      }),
      Expr::Member(MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(ident("obj")),
        prop: MemberProp::PrivateName(PrivateName {
          span: DUMMY_SP,
          name: "secret".into(),
        }),
      }),
      Expr::Member(MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(ident("obj")),
        prop: MemberProp::Computed(ComputedPropName {
          span: DUMMY_SP,
          expr: Box::new(string("prop")),
        }),
      }),
      Expr::SuperProp(SuperPropExpr {
        span: DUMMY_SP,
        obj: Super { span: DUMMY_SP },
        prop: SuperProp::Ident(IdentName::new("x".into(), DUMMY_SP)),
      }),
      Expr::SuperProp(SuperPropExpr {
        span: DUMMY_SP,
        obj: Super { span: DUMMY_SP },
        prop: SuperProp::Computed(ComputedPropName {
          span: DUMMY_SP,
          expr: Box::new(number(0.0)),
        }),
      }),
      Expr::Cond(CondExpr {
        span: DUMMY_SP,
        test: Box::new(ident("test")),
        cons: Box::new(string("yes")),
        alt: Box::new(string("no")),
      }),
      Expr::Call(swc_core::ecma::ast::CallExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        callee: Callee::Expr(Box::new(ident("fn"))),
        args: vec![expr_or_spread(number(1.0))],
        type_args: None,
      }),
      Expr::Call(swc_core::ecma::ast::CallExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        callee: Callee::Super(Super { span: DUMMY_SP }),
        args: vec![],
        type_args: None,
      }),
      Expr::Call(swc_core::ecma::ast::CallExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        callee: Callee::Import(Import {
          span: DUMMY_SP,
          phase: ImportPhase::Evaluation,
        }),
        args: vec![expr_or_spread(string("./dep"))],
        type_args: None,
      }),
      Expr::New(NewExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        callee: Box::new(ident("Ctor")),
        args: Some(vec![expr_or_spread(number(1.0))]),
        type_args: None,
      }),
      Expr::New(NewExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        callee: Box::new(ident("Ctor")),
        args: None,
        type_args: None,
      }),
      Expr::Seq(SeqExpr {
        span: DUMMY_SP,
        exprs: vec![Box::new(ident("a")), Box::new(ident("b"))],
      }),
      ident("id"),
      string("str"),
      bool_expr(true),
      null(),
      number(1.0),
      Expr::Lit(Lit::BigInt(BigInt {
        span: DUMMY_SP,
        value: Box::new(10u32.into()),
        raw: None,
      })),
      regex("x", "gi"),
      Expr::Lit(Lit::JSXText(JSXText {
        span: DUMMY_SP,
        value: "jsx".into(),
        raw: "jsx".into(),
      })),
      tpl(vec![ident("value")]),
      Expr::TaggedTpl(TaggedTpl {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        tag: Box::new(ident("tag")),
        type_params: None,
        tpl: Box::new(tpl_node(vec![ident("value")])),
      }),
      Expr::Arrow(ArrowExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        params: vec![Pat::Ident(ident_pat("arg").into())],
        body: Box::new(ArrowFunctionBody::Expr(Box::new(ident("arg")))),
        is_async: false,
        is_generator: false,
        type_params: None,
        return_type: None,
      }),
      Expr::Arrow(ArrowExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        params: vec![Pat::Expr(Box::new(ident("expr_pat")))],
        body: Box::new(ArrowFunctionBody::Expr(Box::new(ident("expr_pat")))),
        is_async: false,
        is_generator: false,
        type_params: None,
        return_type: None,
      }),
      Expr::Yield(YieldExpr {
        span: DUMMY_SP,
        arg: Some(Box::new(ident("yielded"))),
        delegate: false,
      }),
      Expr::Yield(YieldExpr {
        span: DUMMY_SP,
        arg: None,
        delegate: true,
      }),
      Expr::MetaProp(MetaPropExpr {
        span: DUMMY_SP,
        kind: MetaPropKind::ImportMeta,
      }),
      Expr::Await(AwaitExpr {
        span: DUMMY_SP,
        arg: Box::new(ident("promise")),
      }),
      Expr::Paren(ParenExpr {
        span: DUMMY_SP,
        expr: Box::new(ident("inner")),
      }),
      Expr::OptChain(OptChainExpr {
        span: DUMMY_SP,
        optional: true,
        base: Box::new(OptChainBase::Member(MemberExpr {
          span: DUMMY_SP,
          obj: Box::new(ident("obj")),
          prop: MemberProp::Ident(IdentName::new("prop".into(), DUMMY_SP)),
        })),
      }),
      Expr::OptChain(OptChainExpr {
        span: DUMMY_SP,
        optional: true,
        base: Box::new(OptChainBase::Call(OptCall {
          span: DUMMY_SP,
          ctxt: SyntaxContext::empty(),
          callee: Box::new(ident("maybeFn")),
          args: vec![expr_or_spread(number(1.0))],
          type_args: None,
        })),
      }),
    ];

    for expr in expressions {
      assert_eq!(stable_hash_unspanned(&expr), stable_hash_unspanned(&expr));
    }
  }

  #[test]
  fn unspanned_hash_falls_back_for_unsupported_shapes_without_changing_result() {
    let unsupported = vec![
      parse_expr("function named() {}"),
      parse_expr("class Foo {}"),
      parse_expr("({ get value() { return 1; } })"),
      parse_expr("(value) => { return value; }"),
      parse_expr("([value]) => value"),
      Expr::Array(ArrayLit {
        span: DUMMY_SP,
        elems: (0..129)
          .map(|_| Some(expr_or_spread(number(1.0))))
          .collect(),
      }),
      Expr::Object(ObjectLit {
        span: DUMMY_SP,
        props: (0..129)
          .map(|idx| {
            prop(Prop::KeyValue(KeyValueProp {
              key: PropName::Ident(IdentName::new(format!("key{idx}").into(), DUMMY_SP)),
              value: Box::new(number(idx as f64)),
            }))
          })
          .collect(),
      }),
    ];

    for expr in unsupported {
      assert_eq!(
        stable_hash_unspanned(&expr),
        stable_hash_wide(&drop_span(expr))
      );
    }
  }

  #[test]
  fn unspanned_call_hash_matches_whole_expr_and_ignores_spans() {
    let expr = parse_expr("foo(bar, 1)");
    let Expr::Call(call) = &expr else {
      panic!("expected a call expression");
    };

    // The dedicated call hasher must produce exactly the same key as hashing
    // the call wrapped in a whole `Expr` (the form used on the insertion side).
    assert_eq!(
      stable_hash_unspanned_call(call),
      stable_hash_unspanned(&expr)
    );

    // ...and it must stay span-insensitive: the same call at a different source
    // position hashes identically.
    let shifted = parse_expr("      foo(bar, 1)");
    let Expr::Call(shifted_call) = &shifted else {
      panic!("expected a call expression");
    };
    assert_eq!(
      stable_hash_unspanned_call(call),
      stable_hash_unspanned_call(shifted_call)
    );
  }

  #[test]
  fn unspanned_member_hash_matches_whole_expr() {
    let expr = parse_expr("foo.bar");
    let Expr::Member(member) = &expr else {
      panic!("expected a member expression");
    };

    // The dedicated member hasher must produce exactly the same key as hashing
    // the member wrapped in a whole `Expr`, for the reason its call-shaped
    // counterpart must: the two sides of a lookup reach the key differently.
    assert_eq!(
      stable_hash_unspanned_member(member),
      stable_hash_unspanned(&expr)
    );

    // ...and it must stay span-insensitive.
    let shifted = parse_expr("      foo.bar");
    let Expr::Member(shifted_member) = &shifted else {
      panic!("expected a member expression");
    };
    assert_eq!(
      stable_hash_unspanned_member(member),
      stable_hash_unspanned_member(shifted_member)
    );

    // Different members must not collide on the discriminant prefix alone.
    let other = parse_expr("foo.baz");
    let Expr::Member(other_member) = &other else {
      panic!("expected a member expression");
    };
    assert_ne!(
      stable_hash_unspanned_member(member),
      stable_hash_unspanned_member(other_member)
    );
  }

  #[test]
  fn unspanned_member_hash_fallback_matches_whole_expr() {
    // A computed key holding a function expression is a shape the in-place
    // hasher does not cover, so `stable_hash_unspanned_member` takes its
    // fallback branch. On that branch it must still agree with
    // `stable_hash_unspanned` over the whole `Expr::Member`, which falls back
    // identically -- keeping the insertion-side and lookup-side keys aligned
    // for exotic members too.
    let expr = parse_expr("foo[function () {}]");
    let Expr::Member(member) = &expr else {
      panic!("expected a member expression");
    };

    assert_eq!(
      stable_hash_unspanned_member(member),
      stable_hash_unspanned(&expr)
    );

    let shifted = parse_expr("      foo[function () {}]");
    let Expr::Member(shifted_member) = &shifted else {
      panic!("expected a member expression");
    };
    assert_eq!(
      stable_hash_unspanned_member(member),
      stable_hash_unspanned_member(shifted_member)
    );
  }

  #[test]
  fn unspanned_call_hash_fallback_matches_whole_expr() {
    // A function-expression argument is a shape the in-place call hasher does
    // not cover, so `stable_hash_unspanned_call` takes its fallback branch
    // (delegating to `stable_hash(&Expr::Call(..))`). On that branch it must
    // still produce the same key as `stable_hash_unspanned` over the whole
    // `Expr::Call`, which falls back identically — keeping the insertion-side
    // and lookup-side keys aligned for exotic calls too.
    let expr = parse_expr("foo(function () {})");
    let Expr::Call(call) = &expr else {
      panic!("expected a call expression");
    };

    assert_eq!(
      stable_hash_unspanned_call(call),
      stable_hash_unspanned(&expr)
    );

    let shifted = parse_expr("      foo(function () {})");
    let Expr::Call(shifted_call) = &shifted else {
      panic!("expected a call expression");
    };
    assert_eq!(
      stable_hash_unspanned_call(call),
      stable_hash_unspanned_call(shifted_call)
    );
  }

  fn ident(sym: &str) -> Expr {
    Expr::Ident(ident_pat(sym))
  }

  fn ident_pat(sym: &str) -> Ident {
    Ident {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      sym: sym.into(),
      optional: false,
    }
  }

  fn number(value: f64) -> Expr {
    Expr::Lit(Lit::Num(Number {
      span: DUMMY_SP,
      value,
      raw: None,
    }))
  }

  fn string(value: &str) -> Expr {
    Expr::Lit(Lit::Str(Str {
      span: DUMMY_SP,
      value: value.into(),
      raw: None,
    }))
  }

  fn bool_expr(value: bool) -> Expr {
    Expr::Lit(Lit::Bool(Bool {
      span: DUMMY_SP,
      value,
    }))
  }

  fn null() -> Expr {
    Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))
  }

  fn regex(exp: &str, flags: &str) -> Expr {
    Expr::Lit(Lit::Regex(Regex {
      span: DUMMY_SP,
      exp: exp.into(),
      flags: flags.into(),
    }))
  }

  fn tpl(exprs: Vec<Expr>) -> Expr {
    Expr::Tpl(tpl_node(exprs))
  }

  fn tpl_node(exprs: Vec<Expr>) -> Tpl {
    Tpl {
      span: DUMMY_SP,
      exprs: exprs.into_iter().map(Box::new).collect(),
      quasis: vec![TplElement {
        span: DUMMY_SP,
        tail: true,
        cooked: Some("text".into()),
        raw: "text".into(),
      }],
    }
  }

  fn expr_or_spread(expr: Expr) -> ExprOrSpread {
    ExprOrSpread {
      spread: None,
      expr: Box::new(expr),
    }
  }

  fn prop(prop: Prop) -> PropOrSpread {
    PropOrSpread::Prop(Box::new(prop))
  }
}

/// What one memo key costs, and why the evaluator's cost is quadratic in the
/// depth of the expression it folds.
///
/// [`stable_hash_unspanned`](super::stable_hash_unspanned) is the key of the
/// evaluator's memo, and the evaluator asks for one at *every* level of a nested
/// expression. The key is a hash of the whole remaining subtree, so a fold that
/// descends `n` levels hashes `n + (n-1) + (n-2)` … nodes: the memo that exists
/// to avoid repeated work pays for the subtree to decide whether it can avoid
/// it.
///
/// These cases pin that curve, deterministically rather than by timing. The
/// walk's cost is counted in the bytes it feeds its hasher, which is
/// proportional to the nodes it visits and does not vary with the machine, the
/// build profile, or the load. A change to the key that keeps the curve reports
/// as the same numbers; one that flattens it -- an incremental key composed from
/// its children's -- reports as a changed law, which is exactly what a future
/// reader needs to see. Nothing here asserts a *hash value*: only how much of
/// the tree the walk touched.
///
/// Why the key is a whole-subtree hash is
/// `stylex-evaluator/docs/adr/0005-the-memo-key-is-a-whole-subtree-hash.md`.
/// The incremental key was built against these cases and measured slower on
/// every real fixture, which is
/// `stylex-evaluator/docs/adr/0006-an-incremental-memo-key-was-built-and-measured-slower.md`;
/// a reader restating this module in nodes rather than bytes should start from
/// the patch that ADR points at.
#[cfg(test)]
mod key_cost_scaling_tests {
  use super::{arithmetic_tower, key_cost};
  use swc_core::{
    common::DUMMY_SP,
    ecma::ast::{ArrayLit, Expr, ExprOrSpread},
  };

  /// The bytes one key spends on `MY_CONST` alone -- the tower's leaf, and the
  /// only part of a key that does not grow with depth.
  const LEAF: u64 = 21;

  /// The bytes each `+ 1` above it adds to that key: the `BinExpr`
  /// discriminant, its operator, and the `Number` on the right.
  ///
  /// Both constants are the measured layout rather than a derived one. They move
  /// if SWC changes what a `BinExpr` or a `Number` hashes as, and a reader
  /// comparing against a new pair learns that the layout changed rather than
  /// that the curve did -- the curve is the law two cases below, which is stated
  /// in terms of them.
  const PER_LEVEL: u64 = 48;

  /// Every key the fold buys while folding a tower of `depth`, summed.
  ///
  /// The fold hashes the node it is about to evaluate and then descends into
  /// that node's operands, so for this shape the levels that pay are the left
  /// spine: the whole tower, then the tower one shorter, and so on. Each `+ 1`
  /// on the right is a leaf, hashed as part of its parent's subtree and never
  /// asked for a key of its own.
  fn spine_key_cost(depth: usize) -> u64 {
    let mut total = 0;
    let mut level = arithmetic_tower(depth);

    loop {
      total += key_cost(&level);

      let Expr::Bin(bin) = level else {
        break;
      };

      level = *bin.left;
    }

    total
  }

  #[test]
  fn one_key_costs_the_whole_remaining_subtree() {
    // Linear in depth, and exactly linear: a level of `(x + 1)` adds a fixed
    // number of bytes to every key taken above it.
    for depth in [0, 1, 2, 8, 64, 256] {
      assert_eq!(
        key_cost(&arithmetic_tower(depth)),
        LEAF + PER_LEVEL * depth as u64,
        "one memo key over a {depth}-level tower"
      );
    }
  }

  #[test]
  fn the_fold_pays_for_that_key_once_per_level() {
    // The sum of a linear cost over a linear number of levels, which is the
    // quadratic the ticket measured in milliseconds (2.6x to 3.8x per doubling,
    // converging on 4x). Counted in bytes it has a closed form, so it is
    // asserted as an equality rather than as a ratio: summing `LEAF +
    // PER_LEVEL * k` for k in 0..=depth is `(depth + 1) * (LEAF + PER_LEVEL /
    // 2 * depth)`.
    //
    // An incremental key would break this equality, which is the point: the
    // replacement should be a new law, not a deleted case.
    for depth in [0, 1, 16, 32, 64, 128, 256] {
      let expected = (depth as u64 + 1) * (LEAF + PER_LEVEL / 2 * depth as u64);

      assert_eq!(
        spine_key_cost(depth),
        expected,
        "the keys bought while folding a {depth}-level tower"
      );
    }

    // Read as a curve, that law is a doubling of depth for a quadrupling of
    // cost, approached from below.
    for depth in [16usize, 32, 64, 128] {
      let ratio = spine_key_cost(depth * 2) as f64 / spine_key_cost(depth) as f64;

      assert!(
        (3.5..4.0).contains(&ratio),
        "{depth} -> {} levels multiplied the key cost by {ratio}, not by ~4",
        depth * 2
      );
    }
  }

  /// Two elements in an array, with the towers' own cost taken out: the
  /// `ArrayLit` discriminant, the slot count, and an `Option` tag per slot.
  /// Measured, and asserted below to be independent of what the slots hold.
  const TWO_ELEMENT_ARRAY: u64 = 20;

  #[test]
  fn two_copies_of_one_subtree_cost_twice() {
    // There is no memo *on the key itself*, which is the whole of the open
    // question: the walk has no way to recognise a subtree it has already
    // hashed, so an expression holding two structurally-identical towers pays
    // the walk at each of them. An incremental key with a cache would make the
    // second copy nearly free, and this is the case that would report it.
    fn pair_of(depth: usize) -> Expr {
      Expr::Array(ArrayLit {
        span: DUMMY_SP,
        elems: (0..2)
          .map(|_| {
            Some(ExprOrSpread {
              spread: None,
              expr: Box::new(arithmetic_tower(depth)),
            })
          })
          .collect(),
      })
    }

    for depth in [0, 4, 32, 128] {
      assert_eq!(
        key_cost(&pair_of(depth)),
        key_cost(&arithmetic_tower(depth)) * 2 + TWO_ELEMENT_ARRAY,
        "two copies of a {depth}-level tower"
      );
    }
  }
}

/// The hasher behind the structural key.
#[cfg(test)]
mod wide_hasher_tests {
  use std::hash::Hasher;

  use super::super::{WideHasher, stable_hash_unspanned, stable_hash_wide};
  use super::parse_expr;

  #[test]
  fn finish_is_a_different_number_from_either_half_of_the_wide_one() {
    // `Hasher::finish` exists because the trait requires it, and it is xxh3's
    // 64-bit digest -- a separate construction from the 128-bit one over the
    // same stream, so it matches neither half of it. That is the trap worth
    // pinning: a caller reaching for the familiar `finish` gets a narrower key
    // *and* a different one, while the two consumers that act on a bare hit are
    // relying on the full width.
    let mut hasher = WideHasher::new();
    hasher.write(b"stylex");

    let wide = hasher.finish_wide();
    let narrow = u128::from(hasher.finish());

    assert_ne!(narrow, wide & u128::from(u64::MAX));
    assert_ne!(narrow, wide >> 64);

    // Deterministic, though: the same stream answers the same way twice, which
    // is what makes it safe for the trait's other users even if it is wrong for
    // this one.
    let mut again = WideHasher::new();
    again.write(b"stylex");

    assert_eq!(hasher.finish(), again.finish());
    assert_eq!(wide, again.finish_wide());
  }

  #[test]
  fn a_wide_hash_uses_the_whole_width() {
    // Not a distribution test -- one value cannot be one -- but the high half
    // being non-zero on ordinary input is what says the pair is wired up at all.
    // A `finish_wide` that forgot to shift, or an `Xxh3Default` swapped for
    // something 64-bit, reports here.
    let key = stable_hash_unspanned(&parse_expr("styles.root"));

    assert_ne!(key >> 64, 0);
    assert_ne!(key & u128::from(u64::MAX), 0);
  }

  #[test]
  fn the_wide_hash_is_deterministic_across_hashers() {
    // Two instances of the hasher are seeded identically, so the same bytes
    // land on the same key -- the property every cache keyed by this depends on,
    // and the one an accidentally seeded or randomized hasher would break.
    assert_eq!(
      stable_hash_wide(&"styles.root"),
      stable_hash_wide(&"styles.root")
    );
    assert_ne!(
      stable_hash_wide(&"styles.root"),
      stable_hash_wide(&"styles.base")
    );
  }
}

/// The structural key at its edges: the boundary that selects its fallback arm,
/// the shapes that have no in-place hashing at all, and the inputs a parser can
/// hand it that no author would write.
///
/// Every case here asks one of two questions about
/// [`stable_hash_unspanned`](super::stable_hash_unspanned): does it stay
/// span-insensitive, and does it answer at all rather than panicking. Nothing
/// asserts a hash *value* -- no consumer of this key persists it, and none of
/// them derives a class name from it, so a case pinning a number would pin
/// something no output depends on.
#[cfg(test)]
mod key_edge_case_tests {
  use swc_core::{
    common::{BytePos, DUMMY_SP, GLOBALS, Globals, Mark, Span, SyntaxContext},
    ecma::{
      ast::{
        ArrayLit, BinExpr, BinaryOp, Expr, ExprOrSpread, Ident, Invalid, KeyValueProp, ObjectLit,
        Prop, PropName, PropOrSpread,
      },
      utils::drop_span,
    },
  };

  use super::super::{MAX_UNSPANNED_HASH_COLLECTION_LEN, stable_hash_unspanned, stable_hash_wide};
  use super::{arithmetic_tower, hashed_in_place, key_cost, number, parse_expr};

  fn array_of(len: usize) -> Expr {
    Expr::Array(ArrayLit {
      span: DUMMY_SP,
      elems: (0..len)
        .map(|index| {
          Some(ExprOrSpread {
            spread: None,
            expr: Box::new(number(index as f64)),
          })
        })
        .collect(),
    })
  }

  fn object_of(len: usize) -> Expr {
    Expr::Object(ObjectLit {
      span: DUMMY_SP,
      props: (0..len)
        .map(|index| {
          PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
            key: PropName::Str(format!("key{index}").into()),
            value: Box::new(number(index as f64)),
          })))
        })
        .collect(),
    })
  }

  /// `object_of(len)` with one more property, holding `value`.
  fn object_of_holding(len: usize, value: Expr) -> Expr {
    let Expr::Object(object) = object_of(len) else {
      panic!("object_of builds an object");
    };

    let mut props = object.props;
    props.push(PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
      key: PropName::Str("nested".into()),
      value: Box::new(value),
    }))));

    Expr::Object(ObjectLit {
      span: DUMMY_SP,
      props,
    })
  }

  #[test]
  fn the_collection_boundary_selects_the_arm_and_nothing_else() {
    // One element either side of the limit decides which arm runs, and both
    // arms have to answer the same question the same way. The limit is read
    // from the constant rather than spelled, so raising it moves these cases
    // with it.
    let limit = MAX_UNSPANNED_HASH_COLLECTION_LEN;

    assert!(hashed_in_place(&array_of(limit)));
    assert!(hashed_in_place(&object_of(limit)));
    assert!(!hashed_in_place(&array_of(limit + 1)));
    assert!(!hashed_in_place(&object_of(limit + 1)));

    // An empty collection is the other end of the same boundary, and is the
    // shape a `stylex.create({})` argument actually has.
    assert!(hashed_in_place(&array_of(0)));
    assert!(hashed_in_place(&object_of(0)));

    for expr in [array_of(limit + 1), object_of(limit + 1)] {
      assert_eq!(
        stable_hash_unspanned(&expr),
        stable_hash_wide(&drop_span(expr.clone())),
        "the fallback arm is what the public contract promises for an \
         over-limit collection"
      );
    }
  }

  #[test]
  fn an_over_limit_collection_nested_pays_the_fallback_at_every_level() {
    // The arm is selected per level, not per expression: a chain of over-limit
    // objects hands *each* level to the deep clone, which is the one shape
    // where the clone -- rather than the walk -- is the dominant cost of a
    // fold. Nothing in this workspace's fixtures reaches it; how rarely a real
    // project does is recorded in `docs/adr/0005` in `stylex-evaluator`.
    let limit = MAX_UNSPANNED_HASH_COLLECTION_LEN;
    let mut chain = object_of(limit + 1);

    for level in 0..4 {
      // Every level of the chain is over the limit, so hashing the outermost
      // declines *and* would decline at each level it descended to -- checked
      // by walking back down the chain below.
      chain = object_of_holding(limit + 1, chain);

      let mut inner = &chain;
      let mut descended = 0;

      while let Expr::Object(object) = inner {
        assert!(
          !hashed_in_place(inner),
          "level {descended} of a {level}-deep chain was hashed in place"
        );

        let Some(PropOrSpread::Prop(last)) = object.props.last() else {
          break;
        };
        let Prop::KeyValue(entry) = last.as_ref() else {
          break;
        };

        inner = entry.value.as_ref();
        descended += 1;
      }

      assert_eq!(
        descended,
        level + 2,
        "the chain should be one level deeper than the last iteration"
      );
    }
  }

  #[test]
  fn a_shape_with_no_in_place_hashing_still_answers() {
    // Every arm the walk declines, reached through the public entry point. The
    // claim is only that it answers -- span-insensitively, without panicking --
    // because a caller cannot see which arm ran and must not have to care.
    let declined = vec![
      parse_expr("function named() {}"),
      parse_expr("class Foo { #field = 1; }"),
      parse_expr("({ get value() { return 1; } })"),
      parse_expr("({ set value(next) {} })"),
      parse_expr("({ method() {} })"),
      parse_expr("(value) => { return value; }"),
      parse_expr("([first]) => first"),
      parse_expr("({ key }) => key"),
      parse_expr("(...rest) => rest"),
      parse_expr("(value = 1) => value"),
      parse_expr("(target = 1)"),
      parse_expr("<div a=\"b\">text</div>"),
      parse_expr("<></>"),
      // A parser is entitled to hand the evaluator a node standing for source
      // it could not make sense of; hashing one must not be the thing that
      // panics.
      Expr::Invalid(Invalid { span: DUMMY_SP }),
    ];

    for expr in declined {
      assert!(
        !hashed_in_place(&expr),
        "expected the fallback arm for {expr:?}"
      );
      assert_eq!(
        stable_hash_unspanned(&expr),
        stable_hash_wide(&drop_span(expr.clone()))
      );
    }
  }

  #[test]
  fn a_non_ascii_source_hashes_the_same_wherever_it_is_written() {
    // The same spelling at a different position is one key, whatever alphabet
    // it is written in. An astral-plane scalar and a combining sequence are here
    // because the walk hashes an `Atom`, whose `Hash` is over bytes -- so the
    // case is about the walk not truncating rather than about the encoding.
    for source in [
      "'🎉'",
      "'e\u{0301}'",
      "'\u{00e9}'",
      "({ 'ключ': 'значение' })",
      "ünïcödé",
      "({ 日本語: 1 })",
      "`${'🎉'} tail`",
    ] {
      let here = parse_expr(source);
      let there = parse_expr(&format!("      {source}"));

      assert_eq!(
        stable_hash_unspanned(&here),
        stable_hash_unspanned(&there),
        "{source} hashed differently for being written six columns over"
      );
    }
  }

  #[test]
  fn a_precomposed_and_a_decomposed_spelling_are_different_keys() {
    // `é` and `e` + a combining acute are the same grapheme and different
    // bytes. The key is over bytes, so they are different keys -- which is the
    // correct answer for a *cache*, where two spellings that a browser treats
    // differently must not share an entry, and is worth pinning because the
    // opposite (normalizing) would be a plausible-looking change.
    assert_ne!(
      stable_hash_unspanned(&parse_expr("'\u{00e9}'")),
      stable_hash_unspanned(&parse_expr("'e\u{0301}'"))
    );
  }

  #[test]
  fn a_span_is_never_part_of_the_key() {
    // The property the whole key exists for, asserted at the extremes of the
    // span rather than at a plausible position: the same node with a
    // synthesized span, and with a span at the end of the addressable range.
    // Two structurally-identical expressions have to land on one memo entry
    // however far apart they were written.
    fn one_level_at(span: Span) -> Expr {
      Expr::Bin(BinExpr {
        span,
        op: BinaryOp::Add,
        left: Box::new(Expr::Ident(Ident::new(
          "MY_CONST".into(),
          span,
          SyntaxContext::empty(),
        ))),
        right: Box::new(number(1.0)),
      })
    }

    assert_eq!(
      stable_hash_unspanned(&one_level_at(DUMMY_SP)),
      stable_hash_unspanned(&one_level_at(Span::new(
        BytePos(u32::MAX - 1),
        BytePos(u32::MAX)
      )))
    );

    // The parsed form of the same source is span-insensitive too, at both ends
    // of a line.
    assert_eq!(
      stable_hash_unspanned(&parse_expr("MY_CONST + 1")),
      stable_hash_unspanned(&parse_expr("                    MY_CONST + 1"))
    );
  }

  #[test]
  fn a_literal_the_parser_wrote_and_one_the_compiler_built_are_different_keys() {
    // Span-insensitive is not source-insensitive. A `Number` and a `Str` each
    // carry the *raw* text they were written as, `None` for a literal nothing
    // wrote, and the key covers it -- so `1` read out of a file and `1` the
    // compiler synthesized do not share a memo entry even though they fold to
    // the same value.
    //
    // Pinned rather than fixed. It costs a duplicated entry for a value that
    // was already folded, never a wrong one, and the raw text is exactly what
    // distinguishes `1` from `1.0` -- a key that dropped it would merge inputs
    // whose *authored* form is what a diagnostic quotes back.
    assert_ne!(
      stable_hash_unspanned(&parse_expr("1")),
      stable_hash_unspanned(&number(1.0))
    );

    // Same value, two spellings, two keys -- the same rule read from the other
    // direction, for a number and for a string.
    assert_ne!(
      stable_hash_unspanned(&parse_expr("1")),
      stable_hash_unspanned(&parse_expr("1.0"))
    );
    assert_ne!(
      stable_hash_unspanned(&parse_expr("0x10")),
      stable_hash_unspanned(&parse_expr("16"))
    );
    assert_ne!(
      stable_hash_unspanned(&parse_expr(r"'\u0041\u0042'")),
      stable_hash_unspanned(&parse_expr("'AB'"))
    );
  }

  #[test]
  fn a_syntax_context_is_part_of_the_key() {
    // The counterpart to the case above: what the key *must* keep. Two
    // references spelled the same in different scopes are different bindings,
    // and a memo that merged them would answer for a shadowed parameter with an
    // imported binding's value -- the failure the shadowing work exists to
    // prevent.
    let globals = Globals::default();

    let outer = Expr::Ident(Ident::new("value".into(), DUMMY_SP, SyntaxContext::empty()));
    let inner = GLOBALS.set(&globals, || {
      Expr::Ident(Ident::new(
        "value".into(),
        DUMMY_SP,
        SyntaxContext::empty().apply_mark(Mark::new()),
      ))
    });

    assert_ne!(stable_hash_unspanned(&outer), stable_hash_unspanned(&inner));
  }

  #[test]
  fn a_deep_expression_hashes_the_whole_depth_without_running_out_of_stack() {
    // The key recurses, so its own limit is the thread's stack rather than the
    // evaluator's ceiling -- the ceiling bounds how deep a *fold* goes, and a
    // caller is free to hand this function anything the parser produced. 1024
    // levels is past every ceiling in this workspace and past the depth at
    // which the stages around the fold give out, so a key that could not answer
    // here would be the shallower limit of the two.
    //
    // Answering is not enough: the key has to have *reached* the bottom. One
    // level shorter is a different key, and the byte count is the whole
    // subtree's, which together say the walk did not stop early.
    let deep = arithmetic_tower(1024);
    let shallower = arithmetic_tower(1023);

    assert_ne!(
      stable_hash_unspanned(&deep),
      stable_hash_unspanned(&shallower)
    );
    assert_eq!(
      key_cost(&deep) - key_cost(&shallower),
      key_cost(&arithmetic_tower(1)) - key_cost(&arithmetic_tower(0))
    );
  }

  #[test]
  fn width_costs_linearly_and_takes_the_arm_once() {
    // Width is not depth. Under the limit the walk is linear in the number of
    // elements, and over it the collection takes the fallback arm *once* for
    // the whole collection rather than once per element -- so 100_000 elements
    // cost one clone and one walk, not 100_000 of either. A fallback moved
    // inside the element loop would report here as elements that decline the
    // walk on their own.
    let half = key_cost(&array_of(MAX_UNSPANNED_HASH_COLLECTION_LEN / 2));
    let full = key_cost(&array_of(MAX_UNSPANNED_HASH_COLLECTION_LEN));

    assert!(
      (1.9..2.1).contains(&((full - 2) as f64 / (half - 2) as f64)),
      "doubling the width cost {full} rather than about twice {half}"
    );

    let wide = array_of(100_000);
    assert!(!hashed_in_place(&wide));

    // Each element on its own is a shape the walk covers, which is what makes
    // the decline above a property of the collection's length alone.
    assert!(hashed_in_place(&number(0.0)));
    assert_eq!(stable_hash_unspanned(&wide), stable_hash_unspanned(&wide));
  }
}
