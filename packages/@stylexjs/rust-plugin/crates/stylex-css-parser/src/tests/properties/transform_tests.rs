use super::*;

#[test]
fn test_transform_empty() {
  let transform = Transform::new(vec![]);
  assert_eq!(transform.to_string(), "");
}
