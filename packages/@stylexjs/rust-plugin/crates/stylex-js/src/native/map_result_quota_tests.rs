//! Result-entry guards run after executed callbacks, before the crossing append.
use super::*;
struct ReturnReceiver {
  calls: usize,
}
impl Callbacks for ReturnReceiver {
  fn call(&mut self, _: &FunctionRef, args: Vec<Value>) -> Option<Value> {
    self.calls += 1;
    args.get(2).cloned()
  }
}
#[test]
fn map_result_quota_stops_only_at_the_first_executed_crossing() {
  with_isolated_evaluation(|| {
    with_evaluation(|| {
      set_result_limits(100, 4);
      let input = Value::Array(Rc::new(vec![
        Value::Number(1.0),
        Value::Number(2.0),
        Value::Number(3.0),
      ]));
      let function = Value::Function(FunctionRef {
        identity: 1,
        value: Rc::new(()),
      });
      let mut callbacks = ReturnReceiver { calls: 0 };
      assert!(apply(None, "map", Some(&input), &[function], &mut callbacks).is_none());
      assert_eq!(
        callbacks.calls, 1,
        "the successful callback crossing the quota executes, but no later callback does"
      );
      assert_eq!(
        quota_refusal(),
        Some(QuotaRefusal {
          kind: QuotaKind::ArrayEntries,
          limit: 4,
          observed: 6
        })
      );
    })
  });
}
#[test]
fn map_result_quota_accepts_the_same_aliases_inside_the_exact_bound() {
  with_isolated_evaluation(|| {
    with_evaluation(|| {
      set_result_limits(100, 12);
      let input = Value::Array(Rc::new(vec![
        Value::Number(1.0),
        Value::Number(2.0),
        Value::Number(3.0),
      ]));
      let function = Value::Function(FunctionRef {
        identity: 1,
        value: Rc::new(()),
      });
      let mut callbacks = ReturnReceiver { calls: 0 };
      let value = apply(None, "map", Some(&input), &[function], &mut callbacks)
        .expect("exactly 12 entries including output slots");
      assert_eq!(callbacks.calls, 3);
      let Value::Array(values) = value else {
        panic!("map output")
      };
      assert_eq!(values.len(), 3);
      assert!(
        values
          .iter()
          .all(|value| matches!(value, Value::Array(items) if items.len()==3))
      );
      assert!(quota_refusal().is_none());
    })
  });
}
