//! Tests for the lightweight key/value `Pair` helper.

use std::borrow::Cow;

use crate::pair::{Pair, PairCow};

#[test]
fn pair_cow_borrows_and_converts_back_to_owned_pair() {
  let pair = Pair::new("display", "block");
  let borrowed = PairCow::borrowed(&pair);

  assert_eq!(borrowed, pair);
  assert!(matches!(borrowed.key, Cow::Borrowed("display")));
  assert!(matches!(borrowed.value, Cow::Borrowed("block")));

  let owned = borrowed.into_owned();
  assert_eq!(owned, pair);
}

#[test]
fn an_owning_pair_cow_declares_what_a_borrowing_one_declares() {
  let declaring = PairCow {
    key: Cow::Owned("top".to_string()),
    value: Cow::Owned("10px".to_string()),
  };
  let blank = PairCow {
    key: Cow::Owned("top".to_string()),
    value: Cow::Owned("  ".to_string()),
  };

  assert_eq!(declaring.as_css_text(), Some("top:10px;".to_string()));
  assert_eq!(blank.as_css_text(), None);
}
