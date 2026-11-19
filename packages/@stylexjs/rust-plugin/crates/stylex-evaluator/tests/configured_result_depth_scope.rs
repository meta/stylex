#[test]
fn data_depth_guards_restore_without_refunding_shared_work() {
  use stylex_js::native::*;
  with_evaluation(|| {
    set_result_depth_limit(2);
    let first = enter_value_depth().unwrap();
    let second = enter_value_depth().unwrap();
    assert!(enter_value_depth().is_none());
    drop(second);
    drop(first);
    assert!(budget_exhausted());
    set_result_depth_limit(256);
    assert_eq!(result_depth_limit(), 2);
    assert!(
      charge_work(1).is_none(),
      "a depth refusal must remain terminal"
    );
    with_isolated_evaluation(|| {
      with_evaluation(|| {
        assert!(!budget_exhausted());
        set_result_depth_limit(200);
        assert_eq!(result_depth_limit(), 200);
        assert!(charge_work(1).is_some());
      })
    });
    assert!(budget_exhausted());
  });
  with_evaluation(|| {
    set_result_depth_limit(200);
    assert!(charge_work(MAX_FUEL - 1).is_some());
    set_result_depth_limit(256);
    assert!(
      charge_work(2).is_none(),
      "configuration must not replenish spent fuel"
    );
  });
}
