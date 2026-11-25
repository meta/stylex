use rustc_hash::FxHashMap;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub enum Aliases {
  String(FxHashMap<String, String>),
  StringVec(FxHashMap<String, Vec<String>>),
}
