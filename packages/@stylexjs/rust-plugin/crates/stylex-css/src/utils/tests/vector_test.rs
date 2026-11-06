use crate::utils::vector::get_intersection;

#[test]
fn intersection_single_element_match() {
  let v1 = vec![42];
  let v2 = vec![42];
  let result = get_intersection(&v1, &v2);
  assert_eq!(result, vec![42]);
}
