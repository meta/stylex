use super::*;

/// A SubString advanced past its end, so `start_index > end_index`.
fn exhausted_substring() -> SubString {
  let mut substr = SubString::new("ab");
  substr.start_index = 1;
  substr.end_index = 0;
  substr
}

#[test]
fn first_returns_none_when_start_index_past_end_index() {
  let substr = exhausted_substring();
  assert_eq!(substr.first(), None);
}

#[test]
fn get_returns_none_when_absolute_index_past_end_index() {
  let substr = exhausted_substring();
  // absolute_index = start_index(1) + relative_index(0) = 1 > end_index(0)
  assert_eq!(substr.get(0), None);
}

#[test]
fn into_string_returns_empty_when_start_index_past_end_index() {
  let substr = exhausted_substring();
  assert_eq!(substr.into_string(), String::new());
}
