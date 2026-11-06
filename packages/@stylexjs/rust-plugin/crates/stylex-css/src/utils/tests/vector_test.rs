use crate::utils::vector::get_intersection;

#[test]
fn intersection_of_empty_vectors() {
  let v1: Vec<i32> = vec![];
  let v2: Vec<i32> = vec![];
  let result = get_intersection(&v1, &v2);
  assert!(result.is_empty());
}

#[test]
fn intersection_first_empty() {
  let v1: Vec<i32> = vec![];
  let v2 = vec![1, 2, 3];
  let result = get_intersection(&v1, &v2);
  assert!(result.is_empty());
}

#[test]
fn intersection_second_empty() {
  let v1 = vec![1, 2, 3];
  let v2: Vec<i32> = vec![];
  let result = get_intersection(&v1, &v2);
  assert!(result.is_empty());
}

#[test]
fn intersection_no_overlap() {
  let v1 = vec![1, 2, 3];
  let v2 = vec![4, 5, 6];
  let result = get_intersection(&v1, &v2);
  assert!(result.is_empty());
}

#[test]
fn intersection_full_overlap() {
  let v1 = vec![1, 2, 3];
  let v2 = vec![1, 2, 3];
  let result = get_intersection(&v1, &v2);
  assert_eq!(result, vec![1, 2, 3]);
}

#[test]
fn intersection_partial_overlap() {
  let v1 = vec![1, 2, 3, 4];
  let v2 = vec![2, 4, 6];
  let result = get_intersection(&v1, &v2);
  assert_eq!(result, vec![2, 4]);
}

#[test]
fn intersection_preserves_order_from_first() {
  let v1 = vec![3, 1, 2];
  let v2 = vec![2, 3];
  let result = get_intersection(&v1, &v2);
  assert_eq!(result, vec![3, 2]);
}

#[test]
fn intersection_with_strings() {
  let v1 = vec!["a".to_string(), "b".to_string(), "c".to_string()];
  let v2 = vec!["b".to_string(), "d".to_string()];
  let result = get_intersection(&v1, &v2);
  assert_eq!(result, vec!["b".to_string()]);
}

#[test]
fn intersection_single_element_match() {
  let v1 = vec![42];
  let v2 = vec![42];
  let result = get_intersection(&v1, &v2);
  assert_eq!(result, vec![42]);
}

#[test]
fn intersection_single_element_no_match() {
  let v1 = vec![42];
  let v2 = vec![99];
  let result = get_intersection(&v1, &v2);
  assert!(result.is_empty());
}
