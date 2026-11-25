use rustc_hash::FxHashSet;

use super::number_properties::NUMBER_PROPERTY_SUFFIXIES;

pub fn get_time_units() -> FxHashSet<String> {
  NUMBER_PROPERTY_SUFFIXIES
    .keys()
    .map(|key| key.to_string())
    .collect()
}
