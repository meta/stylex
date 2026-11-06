//! Tests for the hashing helpers, with shared parsing, tower-building and byte-counting helpers.

#[cfg(test)]
use std::hash::Hasher;

#[cfg(test)]
use crate::test_support;
#[cfg(test)]
use swc_core::{
  common::{DUMMY_SP, SyntaxContext},
  ecma::ast::{BinExpr, BinaryOp, Expr, Ident, Lit, Number},
};

/// A [`Hasher`] that counts the bytes the walk feeds it and hashes nothing.
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

/// The bytes one whole-subtree hash of `expr` spends; panics if the walk falls back to cloning.
#[cfg(test)]
fn key_cost(expr: &Expr) -> u64 {
  let mut counter = ByteCounter::default();

  assert!(
    super::hash_expr_unspanned(expr, &mut counter),
    "measuring the cost of a shape the in-place walk declines"
  );

  counter.bytes
}

/// Whether the in-place walk covered `expr` rather than handing it to the deep-clone fallback.
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

/// `MY_CONST` under `depth` levels of `+ 1`.
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

/// Parses one expression in the grammar the compiler reads.
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
    // ASCII only, where raw murmur2 over bytes still equals the hashed value.
    for input in ["", "hello", "world", "a very long input string"] {
      let raw = murmur2::murmur2(input.as_bytes(), 1);
      assert_eq!(create_hash(input), radix_fmt::radix(raw, 36).to_string());
    }
  }

  /// Golden vectors from `@stylexjs/babel-plugin@0.19.0`'s `hash` (`murmurhash2_32_gc`, seed 1).
  /// The `content` pair reproduces https://github.com/Dwlad90/stylex-swc-plugin/issues/1248.
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

  /// `murmur2("k4127446806") == 0`, which base 36 renders as `"0"` like `(0).toString(36)`.
  #[test]
  fn matches_upstream_hash_when_value_is_zero() {
    assert_eq!(create_hash("k4127446806"), "0");
  }

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
    assert_eq!(
      create_authored_or_hashed_key("", "", "--brand-color"),
      "brand-color"
    );
    assert_eq!(create_authored_or_hashed_key("x", "n", "----a"), "--a");
  }

  #[test]
  fn every_other_name_is_hashed_behind_the_prefix() {
    let expected = format!("x{}", create_key_hash("Theme.stylex", "brandColor"));

    assert_eq!(
      create_authored_or_hashed_key("x", "Theme.stylex", "brandColor"),
      expected
    );

    assert!(create_authored_or_hashed_key("x", "n", "-a").starts_with('x'));
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
    assert!(create_short_hash("test").len() <= 5);
  }

  /// Golden vectors from `@stylexjs/babel-plugin@0.19.0`'s `createShortHash`.
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
      ("•", "cBiC"),
    ] {
      assert_eq!(create_short_hash(input), expected, "input: {:?}", input);
    }
  }

  /// Upstream `toBase62` yields `""` for zero, and `murmur2("k580145052")` is `3 * 62^5`.
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

    // Must match the key of the call wrapped in a whole `Expr`, as the insertion side hashes it.
    assert_eq!(
      stable_hash_unspanned_call(call),
      stable_hash_unspanned(&expr)
    );

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

    // Must match the key of the member wrapped in a whole `Expr`.
    assert_eq!(
      stable_hash_unspanned_member(member),
      stable_hash_unspanned(&expr)
    );

    let shifted = parse_expr("      foo.bar");
    let Expr::Member(shifted_member) = &shifted else {
      panic!("expected a member expression");
    };
    assert_eq!(
      stable_hash_unspanned_member(member),
      stable_hash_unspanned_member(shifted_member)
    );

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
    // A function-expression key forces the fallback, which must still match the whole `Expr`.
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
    // A function-expression argument forces the fallback, which must still match the whole `Expr`.
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

/// Pins how much of the tree each memo key hashes, counted in bytes rather than timed.
/// See `stylex-evaluator/docs/adr/0005` and `0006` for why the key is a whole-subtree hash.
#[cfg(test)]
mod key_cost_scaling_tests {
  use super::{arithmetic_tower, key_cost};
  use swc_core::{
    common::DUMMY_SP,
    ecma::ast::{ArrayLit, Expr, ExprOrSpread},
  };

  /// The bytes one key spends on the `MY_CONST` leaf.
  const LEAF: u64 = 21;

  /// The bytes each `+ 1` level adds to a key, as measured for SWC's current layout.
  const PER_LEVEL: u64 = 48;

  /// The summed key cost of folding a tower of `depth`, which hashes each level of its left spine.
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
    // The sum of `LEAF + PER_LEVEL * k` over k in 0..=depth has this closed form.
    for depth in [0, 1, 16, 32, 64, 128, 256] {
      let expected = (depth as u64 + 1) * (LEAF + PER_LEVEL / 2 * depth as u64);

      assert_eq!(
        spine_key_cost(depth),
        expected,
        "the keys bought while folding a {depth}-level tower"
      );
    }

    for depth in [16usize, 32, 64, 128] {
      let ratio = spine_key_cost(depth * 2) as f64 / spine_key_cost(depth) as f64;

      assert!(
        (3.5..4.0).contains(&ratio),
        "{depth} -> {} levels multiplied the key cost by {ratio}, not by ~4",
        depth * 2
      );
    }
  }

  /// The `ArrayLit` discriminant, slot count and per-slot `Option` tags, excluding the elements.
  const TWO_ELEMENT_ARRAY: u64 = 20;

  #[test]
  fn two_copies_of_one_subtree_cost_twice() {
    // The walk does not recognise a subtree it has already hashed.
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

#[cfg(test)]
mod wide_hasher_tests {
  use std::hash::Hasher;

  use super::super::{WideHasher, stable_hash_unspanned, stable_hash_wide};
  use super::parse_expr;

  #[test]
  fn finish_is_a_different_number_from_either_half_of_the_wide_one() {
    // xxh3's 64-bit digest is a separate construction from its 128-bit one.
    let mut hasher = WideHasher::new();
    hasher.write(b"stylex");

    let wide = hasher.finish_wide();
    let narrow = u128::from(hasher.finish());

    assert_ne!(narrow, wide & u128::from(u64::MAX));
    assert_ne!(narrow, wide >> 64);

    let mut again = WideHasher::new();
    again.write(b"stylex");

    assert_eq!(hasher.finish(), again.finish());
    assert_eq!(wide, again.finish_wide());
  }

  #[test]
  fn a_wide_hash_uses_the_whole_width() {
    // Catches a `finish_wide` that drops the high half.
    let key = stable_hash_unspanned(&parse_expr("styles.root"));

    assert_ne!(key >> 64, 0);
    assert_ne!(key & u128::from(u64::MAX), 0);
  }

  #[test]
  fn the_wide_hash_is_deterministic_across_hashers() {
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

/// Edge cases for the structural key: span-insensitivity, the fallback boundary and odd input.
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
    let limit = MAX_UNSPANNED_HASH_COLLECTION_LEN;

    assert!(hashed_in_place(&array_of(limit)));
    assert!(hashed_in_place(&object_of(limit)));
    assert!(!hashed_in_place(&array_of(limit + 1)));
    assert!(!hashed_in_place(&object_of(limit + 1)));

    // An empty collection is what a `stylex.create({})` argument has.
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
    // The fallback arm is chosen per level, so each over-limit level pays for a deep clone.
    let limit = MAX_UNSPANNED_HASH_COLLECTION_LEN;
    let mut chain = object_of(limit + 1);

    for level in 0..4 {
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
      // A parser may produce an invalid node for source it could not understand.
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
    // `Atom` hashes bytes, so this checks the walk does not truncate non-ASCII text.
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
    // The key is over bytes and must not normalize Unicode.
    assert_ne!(
      stable_hash_unspanned(&parse_expr("'\u{00e9}'")),
      stable_hash_unspanned(&parse_expr("'e\u{0301}'"))
    );
  }

  #[test]
  fn a_span_is_never_part_of_the_key() {
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

    assert_eq!(
      stable_hash_unspanned(&parse_expr("MY_CONST + 1")),
      stable_hash_unspanned(&parse_expr("                    MY_CONST + 1"))
    );
  }

  #[test]
  fn a_literal_the_parser_wrote_and_one_the_compiler_built_are_different_keys() {
    // Literals keep their raw text in the key, so a parsed `1` and a synthesized `1` differ.
    assert_ne!(
      stable_hash_unspanned(&parse_expr("1")),
      stable_hash_unspanned(&number(1.0))
    );

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
    // Same-named bindings in different scopes must not share a memo entry.
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
    // A key one level shorter differs, which shows the walk reached the bottom.
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
    // Over the limit, the fallback runs once for the whole collection, not once per element.
    let half = key_cost(&array_of(MAX_UNSPANNED_HASH_COLLECTION_LEN / 2));
    let full = key_cost(&array_of(MAX_UNSPANNED_HASH_COLLECTION_LEN));

    assert!(
      (1.9..2.1).contains(&((full - 2) as f64 / (half - 2) as f64)),
      "doubling the width cost {full} rather than about twice {half}"
    );

    let wide = array_of(100_000);
    assert!(!hashed_in_place(&wide));

    assert!(hashed_in_place(&number(0.0)));
    assert_eq!(stable_hash_unspanned(&wide), stable_hash_unspanned(&wide));
  }
}
