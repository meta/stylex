//! Closed HostEnv adoption contracts, independent of the compiler's evaluator.
use super::*;
use stylex_structures::stylex_env::Adoptable;

fn host(owner: u64) -> FixtureHost {
  FixtureHost {
    owner,
    callback: Rc::new(|id, _| Err(anyhow!("unexpected fixture callback {id}"))),
    data: RefCell::new(HashMap::new()),
    closures: RefCell::new(HashMap::new()),
    causal_closures: RefCell::new(HashMap::new()),
    causes: RefCell::new(IndexMap::new()),
    last_failure: RefCell::new(None),
    next: RefCell::new(0),
  }
}

#[test]
fn adoption_issues_fresh_owner_bound_refs_and_preserves_nested_identity() {
  let host = host(101);
  let child = host
    .adopt(Adoptable::Object {
      null_prototype: false,
      entries: vec![("color".to_owned(), NativeValue::String("red".to_owned()))],
    })
    .expect("adopt a source object");
  let other = host
    .adopt(Adoptable::Object {
      null_prototype: false,
      entries: vec![("color".to_owned(), NativeValue::String("red".to_owned()))],
    })
    .expect("adopt another source object");
  assert_eq!(child.owner, host.owner());
  assert_eq!(child.kind, HostKind::PlainObject);
  assert_ne!(
    child.id, other.id,
    "separate objects retain separate identities"
  );

  let parent = host
    .adopt(Adoptable::Object {
      null_prototype: false,
      entries: vec![
        ("first".to_owned(), NativeValue::Ref(child.clone())),
        ("second".to_owned(), NativeValue::Ref(child.clone())),
      ],
    })
    .expect("adopt an object that holds existing live refs");
  let first = host.get(&parent, "first").unwrap();
  let second = host.get(&parent, "second").unwrap();
  assert_eq!(first, NativeValue::Ref(child));
  assert!(host.same_value(&first, &second).unwrap());
  assert!(!host.same_value(&first, &NativeValue::Ref(other)).unwrap());
}

#[test]
fn adoption_preserves_array_holes_separately_from_explicit_undefined() {
  let host = host(102);
  let array = host
    .adopt(Adoptable::Array(vec![
      None,
      Some(NativeValue::Undefined),
      Some(NativeValue::String("two".to_owned())),
      None,
    ]))
    .expect("adopt a sparse source array");
  assert_eq!(array.owner, host.owner());
  assert_eq!(array.kind, HostKind::Array);
  assert!(host.is_array(&array).unwrap());
  assert_eq!(
    host.get(&array, "length").unwrap(),
    NativeValue::Number(4.0)
  );
  assert_eq!(host.get(&array, "0").unwrap(), NativeValue::Undefined);
  assert_eq!(host.get(&array, "1").unwrap(), NativeValue::Undefined);
  assert_eq!(
    host.get(&array, "2").unwrap(),
    NativeValue::String("two".to_owned())
  );
  assert_eq!(host.get(&array, "02").unwrap(), NativeValue::Undefined);
  assert_eq!(
    host.own_entries(&array).unwrap(),
    vec![
      ("1".to_owned(), NativeValue::Undefined),
      ("2".to_owned(), NativeValue::String("two".to_owned())),
    ],
    "holes are absent keys; explicit undefined remains an own key"
  );
  assert_eq!(
    host
      .static_builtin("Object", "keys", &[NativeValue::Ref(array.clone())])
      .unwrap(),
    NativeValue::Array(vec![
      NativeValue::String("1".to_owned()),
      NativeValue::String("2".to_owned())
    ])
  );
  let values = vec![
    NativeValue::Undefined,
    NativeValue::Undefined,
    NativeValue::String("two".to_owned()),
    NativeValue::Undefined,
  ];
  assert_eq!(host.iterate(&array).unwrap(), values);
  assert_eq!(
    host.materialize(&array).unwrap(),
    Some(Materialized::Array(values))
  );
}

#[test]
fn adoption_preserves_null_prototype_and_ordinary_data_keys() {
  let host = host(103);
  let entries = vec![
    ("10".to_owned(), NativeValue::String("ten".to_owned())),
    ("2".to_owned(), NativeValue::String("two".to_owned())),
    (
      "__proto__".to_owned(),
      NativeValue::String("own proto data".to_owned()),
    ),
    (
      "constructor".to_owned(),
      NativeValue::String("own constructor".to_owned()),
    ),
    (
      "prototype".to_owned(),
      NativeValue::String("own prototype".to_owned()),
    ),
    ("length".to_owned(), NativeValue::Number(7.0)),
  ];
  let object = host
    .adopt(Adoptable::Object {
      null_prototype: true,
      entries,
    })
    .unwrap();
  assert_eq!(object.kind, HostKind::PlainObject);
  let expected = vec![
    ("2".to_owned(), NativeValue::String("two".to_owned())),
    ("10".to_owned(), NativeValue::String("ten".to_owned())),
    (
      "__proto__".to_owned(),
      NativeValue::String("own proto data".to_owned()),
    ),
    (
      "constructor".to_owned(),
      NativeValue::String("own constructor".to_owned()),
    ),
    (
      "prototype".to_owned(),
      NativeValue::String("own prototype".to_owned()),
    ),
    ("length".to_owned(), NativeValue::Number(7.0)),
  ];
  assert_eq!(host.own_entries(&object).unwrap(), expected);
  assert_eq!(
    host.get(&object, "__proto__").unwrap(),
    NativeValue::String("own proto data".to_owned())
  );
  assert_eq!(
    host.get(&object, "length").unwrap(),
    NativeValue::Number(7.0)
  );
  assert_eq!(
    host.materialize(&object).unwrap(),
    Some(Materialized::Object {
      null_prototype: true,
      entries: expected
    })
  );
}

#[test]
fn adoption_keeps_last_data_value_at_the_first_string_key_position() {
  let host = host(104);
  let object = host
    .adopt(Adoptable::Object {
      null_prototype: false,
      entries: vec![
        ("color".to_owned(), NativeValue::String("red".to_owned())),
        ("gap".to_owned(), NativeValue::Number(4.0)),
        ("color".to_owned(), NativeValue::String("blue".to_owned())),
      ],
    })
    .unwrap();
  assert_eq!(
    host.own_entries(&object).unwrap(),
    vec![
      ("color".to_owned(), NativeValue::String("blue".to_owned())),
      ("gap".to_owned(), NativeValue::Number(4.0)),
    ]
  );
}

#[test]
fn adoption_rejects_foreign_nested_refs_without_allocating_a_value() {
  let host = host(105);
  let foreign = HostRef {
    owner: 106,
    id: Rc::from("foreign"),
    kind: HostKind::PlainObject,
  };
  let result = host.adopt(Adoptable::Array(vec![Some(NativeValue::Object(vec![(
    "nested".to_owned(),
    NativeValue::Ref(foreign),
  )]))]));
  assert!(result.is_err());
  let result = host.adopt(Adoptable::Object {
    null_prototype: false,
    entries: vec![(
      "fn".to_owned(),
      NativeValue::Closure(HostFunctionRef {
        owner: 106,
        id: Rc::from("foreign-fn"),
      }),
    )],
  });
  assert!(result.is_err());
  assert!(host.data.borrow().is_empty());
  assert_eq!(
    *host.next.borrow(),
    0,
    "a rejected adoption allocates no handle"
  );
}
