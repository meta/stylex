#[cfg(test)]
mod candidate_index {

  use crate::candidate_index::CandidateIndex;

  const KEY: u128 = 0x1234_5678_9abc_def0_1234_5678_9abc_def0;
  const OTHER_KEY: u128 = 1;

  #[test]
  fn moving_an_entry_that_was_never_under_the_key_it_leaves_still_joins_the_new_one() {
    let mut index = CandidateIndex::default();

    index.record(OTHER_KEY, 2usize);
    index.move_entry(Some(KEY), Some(OTHER_KEY), 1usize);

    assert_eq!(index.candidates(|| OTHER_KEY), [2, 1]);
  }
}
