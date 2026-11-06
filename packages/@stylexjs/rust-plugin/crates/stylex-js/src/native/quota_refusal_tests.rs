//! Quota labels come from the failed guard, not a generic error message.
use super::*;

#[test]
fn quota_character_guard_records_configured_limit_and_observed_total() {
  with_isolated_evaluation(|| {
    with_evaluation(|| {
      set_result_limits(2, 10);
      assert!(enforce_result_limits(&Value::String("abc".into())).is_none());
      assert_eq!(
        quota_refusal(),
        Some(QuotaRefusal {
          kind: QuotaKind::Characters,
          limit: 2,
          observed: 3
        })
      );
    })
  });
}
#[test]
fn quota_entry_guards_keep_array_and_object_evidence_distinct() {
  for object in [false, true] {
    with_isolated_evaluation(|| {
      with_evaluation(|| {
        set_result_limits(100, 2);
        let value = if object {
          Value::Object(Rc::new(IndexMap::from_iter([
            ("a".into(), Value::Number(1.0)),
            ("b".into(), Value::Number(2.0)),
            ("c".into(), Value::Number(3.0)),
          ])))
        } else {
          Value::Array(Rc::new(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(3.0),
          ]))
        };
        assert!(enforce_result_limits(&value).is_none());
        assert_eq!(
          quota_refusal(),
          Some(QuotaRefusal {
            kind: if object {
              QuotaKind::ObjectEntries
            } else {
              QuotaKind::ArrayEntries
            },
            limit: 2,
            observed: 3
          })
        );
      })
    });
  }
}
#[test]
fn quota_fuel_and_output_refusals_are_not_character_limits() {
  with_isolated_evaluation(|| {
    with_evaluation(|| {
      assert!(charge_work(MAX_FUEL + 1).is_none());
      assert_eq!(
        quota_refusal(),
        Some(QuotaRefusal {
          kind: QuotaKind::Fuel,
          limit: MAX_FUEL,
          observed: MAX_FUEL + 1
        })
      );
    })
  });
  with_isolated_evaluation(|| {
    with_evaluation(|| {
      assert!(charge_output(MAX_OUTPUT + 1).is_none());
      assert_eq!(
        quota_refusal(),
        Some(QuotaRefusal {
          kind: QuotaKind::Output,
          limit: MAX_OUTPUT,
          observed: MAX_OUTPUT + 1
        })
      );
    })
  });
}
#[test]
fn quota_first_refusal_is_kept_and_a_new_operation_starts_clean() {
  with_isolated_evaluation(|| {
    with_evaluation(|| {
      set_result_limits(2, 10);
      assert!(enforce_result_limits(&Value::String("abc".into())).is_none());
      let first = quota_refusal();
      assert_eq!(
        first,
        Some(QuotaRefusal {
          kind: QuotaKind::Characters,
          limit: 2,
          observed: 3
        })
      );
      assert!(charge_work(MAX_FUEL + 1).is_none());
      assert_eq!(quota_refusal(), first);
    })
  });
  with_isolated_evaluation(|| with_evaluation(|| assert!(quota_refusal().is_none())));
}
