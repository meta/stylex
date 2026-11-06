//! Tests for the lightweight key/value `Pair` helper.

use std::borrow::Cow;

use crate::pair::{Pair, PairCow};

#[test]
fn pair_new_converts_into_owned_strings() {
  let pair = Pair::new("left", String::from("right"));
  assert_eq!(pair.key, "left");
  assert_eq!(pair.value, "right");
}

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
fn pair_declares_only_when_both_halves_spell_something() {
  assert_eq!(
    Pair::new("top", "10px").as_css_text(),
    Some("top:10px;".to_string())
  );
  assert_eq!(
    Pair::new("z-index", "0").as_css_text(),
    Some("z-index:0;".to_string())
  );
  assert_eq!(
    Pair::new("content", "\"\"").as_css_text(),
    Some("content:\"\";".to_string())
  );

  assert_eq!(Pair::new("top", "").as_css_text(), None);
  assert_eq!(Pair::new("top", " ").as_css_text(), None);
  assert_eq!(Pair::new("top", "  \t ").as_css_text(), None);
  assert_eq!(Pair::new("", "10px").as_css_text(), None);
}

/// Asserts the borrow too: a copying pair would still give the right text.
#[test]
fn pair_cow_declares_without_owning_its_halves() {
  let declaring = Pair::new("top", "10px");
  let blank = Pair::new("top", "  ");

  let borrowed = PairCow::borrowed(&declaring);

  assert_eq!(borrowed.key.as_ptr(), declaring.key.as_ptr());
  assert_eq!(borrowed.value.as_ptr(), declaring.value.as_ptr());

  assert_eq!(borrowed.as_css_text(), Some("top:10px;".to_string()));
  assert_eq!(PairCow::borrowed(&blank).as_css_text(), None);
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
