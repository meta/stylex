/// Expectations match the StyleX runtime's `style` attribute spelling (babel-plugin 0.19.0).
#[cfg(test)]
mod kebab_case_tests {

  use crate::string::kebab_case;

  /// A final sigma lowercases to `ς`, which only lowercasing the whole name gets right.
  #[test]
  fn lowercases_a_name_that_is_not_ascii() {
    assert_eq!(kebab_case("Ünicode"), "ünicode");
    assert_eq!(kebab_case("aΣB"), "aς-b");
    assert_eq!(kebab_case("ΑΣ"), "ας");
    assert_eq!(kebab_case("aΣ"), "aς");
    assert_eq!(kebab_case("ΣΑ"), "σα");
  }
}

#[cfg(test)]
mod dashify_tests {
  use crate::string::dashify;

  #[test]
  fn lowercases_a_trailing_sigma_to_its_final_form() {
    assert_eq!(dashify("aBΣ"), "a-bς");
  }
}

#[cfg(test)]
mod utf16_length_tests {}

#[cfg(test)]
mod is_blank_css_text_tests {}

#[cfg(test)]
mod json_stringify_tests {}
