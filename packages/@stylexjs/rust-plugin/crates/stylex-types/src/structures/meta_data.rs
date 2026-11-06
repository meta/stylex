use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize, Serializer};

use crate::{
  enums::data_structures::injectable_style::{InjectableStyleBaseKind, InjectableStyleKind},
  traits::InjectableStylesMap,
};
use stylex_utils::hash::hash_f64;

fn f64_to_int<S>(priority: &f64, serializer: S) -> Result<S::Ok, S::Error>
where
  S: Serializer,
{
  if priority.fract() == 0.0 {
    serializer.serialize_i32(*priority as i32)
  } else {
    serializer.serialize_f64(*priority)
  }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]

pub struct MetaData {
  class_name: String,
  style: InjectableStyleBaseKind,
  #[serde(serialize_with = "f64_to_int")]
  priority: f64,
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Hash for MetaData {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.class_name.hash(state);
    self.style.hash(state);
    hash_f64(self.priority);
  }
}

impl Eq for MetaData {}

impl MetaData {
  pub fn new(class_name: impl Into<String>, injectable_style: InjectableStyleKind) -> Self {
    Self {
      class_name: class_name.into(),
      priority: injectable_style.priority(),
      style: InjectableStyleBaseKind::from(injectable_style),
    }
  }
  pub fn get_style(&self) -> &InjectableStyleBaseKind {
    &self.style
  }

  pub fn get_css(&self) -> &str {
    match &self.style {
      InjectableStyleBaseKind::Regular(style) => style.ltr.as_str(),
      InjectableStyleBaseKind::Const(style) => style.ltr.as_str(),
    }
  }

  pub fn get_const_key(&self) -> Option<&str> {
    match &self.style {
      InjectableStyleBaseKind::Const(style) => Some(style.const_key.as_str()),
      _ => None,
    }
  }
  pub fn get_const_value(&self) -> Option<&str> {
    match &self.style {
      InjectableStyleBaseKind::Const(style) => Some(style.const_value.as_str()),
      _ => None,
    }
  }

  pub fn get_css_rtl(&self) -> Option<&str> {
    match &self.style {
      InjectableStyleBaseKind::Regular(style) => style.rtl.as_deref(),
      InjectableStyleBaseKind::Const(style) => style.rtl.as_deref(),
    }
  }

  pub fn get_class_name(&self) -> &str {
    self.class_name.as_str()
  }

  pub fn get_priority(&self) -> &f64 {
    &self.priority
  }

  pub fn convert_from_injected_styles_map(
    injected_styles_map: &InjectableStylesMap,
  ) -> Vec<MetaData> {
    injected_styles_map
      .iter()
      .map(|(class_name, injectable_style)| {
        MetaData::new(
          class_name.clone().into_string(),
          injectable_style.as_ref().clone(),
        )
      })
      .collect()
  }
}
