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
mod stable_hash_tests {
  use crate::hash::stable_hash;

  #[test]
  fn returns_consistent_hash_for_same_value() {
    assert_eq!(stable_hash(&42u64), stable_hash(&42u64));
  }
}

#[cfg(test)]
mod unspanned_fast_path_tests {
  use super::super::{create_hash, stable_hash_unspanned, stable_hash_unspanned_member, to_base36};
  use swc_core::{
    common::{DUMMY_SP, SyntaxContext},
    ecma::ast::{
      ArrayLit, ArrowExpr, ArrowFunctionBody, AssignProp, AwaitExpr, BigInt, BinExpr, BinaryOp,
      Bool, Callee, ComputedPropName, CondExpr, Expr, ExprOrSpread, Ident, IdentName, Import,
      ImportPhase, JSXText, KeyValueProp, Lit, MemberExpr, MemberProp, MetaPropExpr, MetaPropKind,
      NewExpr, Null, Number, ObjectLit, OptCall, OptChainBase, OptChainExpr, ParenExpr, Pat,
      PrivateName, Prop, PropName, PropOrSpread, Regex, SeqExpr, SpreadElement, Str, Super,
      SuperProp, SuperPropExpr, TaggedTpl, ThisExpr, Tpl, TplElement, UnaryExpr, UnaryOp,
      UpdateExpr, UpdateOp, YieldExpr,
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

#[cfg(test)]
mod wide_hasher_tests {
  use std::hash::Hasher;

  use super::super::WideHasher;

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
}

/// Edge cases for the structural key: span-insensitivity, the fallback boundary and odd input.
#[cfg(test)]
mod key_edge_case_tests {
  use swc_core::{
    common::DUMMY_SP,
    ecma::ast::{
      ArrayLit, Expr, ExprOrSpread, KeyValueProp, ObjectLit, Prop, PropName, PropOrSpread,
    },
  };

  use super::super::{MAX_UNSPANNED_HASH_COLLECTION_LEN, stable_hash_unspanned};
  use super::{arithmetic_tower, hashed_in_place, key_cost, number};

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
