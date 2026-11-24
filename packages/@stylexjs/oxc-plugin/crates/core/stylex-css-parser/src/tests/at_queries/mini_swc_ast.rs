//! Test-only stand-in for the handful of SWC AST types the media-query transform suites were
//! written against. It keeps those suites byte-for-byte in their assertions while the
//! transform itself is generic ([`MediaQueryProp`]); the impl below mirrors how an SWC front
//! end would implement the trait for `KeyValueProp`.

use super::MediaQueryProp;

pub const DUMMY_SP: () = ();

#[derive(Debug, Clone, PartialEq)]
pub struct Wtf8Atom(String);

impl Wtf8Atom {
  pub fn as_str(&self) -> Option<&str> {
    Some(&self.0)
  }
}

impl From<&str> for Wtf8Atom {
  fn from(value: &str) -> Self {
    Wtf8Atom(value.to_owned())
  }
}

impl From<String> for Wtf8Atom {
  fn from(value: String) -> Self {
    Wtf8Atom(value)
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Str {
  pub span: (),
  pub value: Wtf8Atom,
  pub raw: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Number {
  pub span: (),
  pub value: f64,
  pub raw: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IdentName {
  pub sym: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PropName {
  Str(Str),
  Ident(IdentName),
  Num(Number),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Lit {
  Str(Str),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
  Lit(Lit),
  Object(ObjectLit),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectLit {
  pub span: (),
  pub props: Vec<PropOrSpread>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PropOrSpread {
  Prop(Box<Prop>),
  Spread(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Prop {
  KeyValue(KeyValueProp),
  Shorthand(IdentName),
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeyValueProp {
  pub key: PropName,
  pub value: Box<Expr>,
}

impl MediaQueryProp for KeyValueProp {
  fn key_str(&self) -> Option<&str> {
    match &self.key {
      PropName::Str(s) => s.value.as_str(),
      PropName::Ident(id) => Some(id.sym.as_str()),
      _ => None,
    }
  }

  fn object_children(&self) -> Option<Vec<Self>> {
    let Expr::Object(obj) = &*self.value else {
      return None;
    };
    obj
      .props
      .iter()
      .map(|p| match p {
        PropOrSpread::Prop(p) => match &**p {
          Prop::KeyValue(kv) => Some(kv.clone()),
          _ => None,
        },
        _ => None,
      })
      .collect()
  }

  fn with_object_children(&self, children: Vec<Self>) -> Self {
    KeyValueProp {
      key: self.key.clone(),
      value: Box::new(Expr::Object(ObjectLit {
        span: DUMMY_SP,
        props: children
          .into_iter()
          .map(|kv| PropOrSpread::Prop(Box::new(Prop::KeyValue(kv))))
          .collect(),
      })),
    }
  }

  fn rekeyed(self, key: &str) -> Self {
    KeyValueProp {
      key: PropName::Str(Str {
        span: DUMMY_SP,
        value: Wtf8Atom::from(key),
        raw: None,
      }),
      value: self.value,
    }
  }

  fn set_value_from(&mut self, other: Self) {
    self.value = other.value;
  }
}
