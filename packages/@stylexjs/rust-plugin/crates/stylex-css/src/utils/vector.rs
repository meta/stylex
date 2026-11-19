pub fn get_intersection<T: PartialEq + Clone>(vec1: &[T], vec2: &[T]) -> Vec<T> {
  vec1
    .iter()
    .filter(|item| vec2.contains(item))
    .cloned()
    .collect()
}
