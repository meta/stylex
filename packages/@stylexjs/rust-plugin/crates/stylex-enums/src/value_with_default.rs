use indexmap::IndexMap;

#[derive(Debug, PartialEq, Clone)]
pub enum ValueWithDefault {
  Number(f64),
  String(String),
  Map(IndexMap<String, ValueWithDefault>),
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl std::hash::Hash for ValueWithDefault {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    match self {
      ValueWithDefault::Number(n) => n.to_bits().hash(state),
      ValueWithDefault::String(s) => s.hash(state),
      ValueWithDefault::Map(map) => {
        for (key, value) in map {
          key.hash(state);
          value.hash(state);
        }
      },
    }
  }
}

impl ValueWithDefault {
  pub fn as_map(&self) -> Option<&IndexMap<String, ValueWithDefault>> {
    match self {
      ValueWithDefault::Map(map) => Some(map),
      _ => None,
    }
  }
}

#[cfg(test)]
#[path = "tests/value_with_default_test.rs"]
mod tests;
