use std::borrow::Borrow;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, derive_more::Display)]
#[display("{}", _0)]
#[repr(transparent)]
pub struct ClassName(pub String);

impl ClassName {
  #[inline]
  pub fn as_str(&self) -> &str {
    &self.0
  }

  #[inline]
  pub fn into_string(self) -> String {
    self.0
  }
}

impl From<String> for ClassName {
  #[inline]
  fn from(value: String) -> Self {
    Self(value)
  }
}

impl From<&str> for ClassName {
  #[inline]
  fn from(value: &str) -> Self {
    Self(value.to_string())
  }
}

impl AsRef<str> for ClassName {
  #[inline]
  fn as_ref(&self) -> &str {
    self.as_str()
  }
}

impl Borrow<str> for ClassName {
  #[inline]
  fn borrow(&self) -> &str {
    self.as_str()
  }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, derive_more::Display)]
#[display("{}", _0)]
#[repr(transparent)]
pub struct RuleKey(pub String);

impl RuleKey {
  #[inline]
  pub fn as_str(&self) -> &str {
    &self.0
  }

  #[inline]
  pub fn into_string(self) -> String {
    self.0
  }
}

impl From<String> for RuleKey {
  #[inline]
  fn from(value: String) -> Self {
    Self(value)
  }
}

impl From<&str> for RuleKey {
  #[inline]
  fn from(value: &str) -> Self {
    Self(value.to_string())
  }
}

impl AsRef<str> for RuleKey {
  #[inline]
  fn as_ref(&self) -> &str {
    self.as_str()
  }
}

impl Borrow<str> for RuleKey {
  #[inline]
  fn borrow(&self) -> &str {
    self.as_str()
  }
}
