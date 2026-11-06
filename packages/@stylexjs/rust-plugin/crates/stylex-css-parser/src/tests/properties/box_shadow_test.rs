/*!
Box shadow property tests.
*/

use crate::{
  css_types::color::*,
  properties::box_shadow::{BoxShadow, BoxShadowList},
};

#[cfg(test)]
mod test_css_property_box_shadow {
  use super::*;

  #[cfg(test)]
  mod box_shadow {
    use stylex_macros::stylex_panic;

    use super::*;

    #[test]
    fn valid_box_shadow_with_inset_keyword_and_blur_spread_radius() {
      let result = BoxShadow::parser()
        .parse_to_end("10px 5px 15px 8px #ff0000 inset")
        .unwrap();

      assert_eq!(result.offset_x.value, 10.0);
      assert_eq!(result.offset_x.unit, "px");
      assert_eq!(result.offset_y.value, 5.0);
      assert_eq!(result.offset_y.unit, "px");
      assert_eq!(result.blur_radius.value, 15.0);
      assert_eq!(result.blur_radius.unit, "px");
      assert_eq!(result.spread_radius.value, 8.0);
      assert_eq!(result.spread_radius.unit, "px");

      if let Color::Hash(hash) = &result.color {
        assert_eq!(hash.value, "ff0000");
      } else {
        stylex_panic!("Expected Hash color");
      }

      assert!(result.inset);
    }
  }

  #[cfg(test)]
  mod box_shadow_list {}

  #[cfg(test)]
  mod invalid_cases {}
}

#[cfg(test)]
mod test_css_property_box_shadow_list {
  use super::*;

  #[test]
  fn box_shadow_list_complex() {
    let input = "0 1px 3px rgba(0,0,0,0.12), 0 1px 2px rgba(0,0,0,0.24), inset 0 0 0 1px #e0e0e0";
    let shadow_list = BoxShadowList::parser().parse_to_end(input).unwrap();

    assert_eq!(shadow_list.shadows.len(), 3);

    assert!(!shadow_list.shadows[0].inset);
    assert!(!shadow_list.shadows[1].inset);
    assert!(shadow_list.shadows[2].inset);

    let reparsed = BoxShadowList::parser()
      .parse_to_end(&shadow_list.to_string())
      .unwrap();
    assert_eq!(shadow_list.shadows.len(), reparsed.shadows.len());
  }

  #[test]
  fn box_shadow_none_comprehensive() {
    let shadow_list = BoxShadowList::parser().parse_to_end("none").unwrap();
    assert!(shadow_list.shadows.is_empty());
    assert_eq!(shadow_list.to_string(), "none");

    let empty_list = BoxShadowList::new(vec![]);
    assert_eq!(empty_list.to_string(), "none");

    let reparsed = BoxShadowList::parser()
      .parse_to_end(&shadow_list.to_string())
      .unwrap();
    assert!(reparsed.shadows.is_empty());
    assert_eq!(reparsed.to_string(), "none");
  }
}
