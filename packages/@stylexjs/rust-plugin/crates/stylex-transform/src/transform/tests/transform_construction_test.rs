//! How the transform is built, and what it reads a call as. The compiler uses
//! [`StyleXTransform::new`], which the fixture suite's test builder never exercises.

use swc_core::common::{GLOBALS, Globals};

use stylex_structures::stylex_options::StyleXOptionsParams;

use crate::transform::tests::prelude::test_transform;

/// Read off the built transform: the point is only that each setting reaches the state.
#[test]
fn the_builder_carries_an_options_record_and_the_debug_data_prop() {
  GLOBALS.set(&Globals::default(), || {
    let mut config = StyleXOptionsParams {
      class_name_prefix: Some("zz".to_string()),
      ..Default::default()
    };

    let transform = test_transform(|builder| {
      builder
        .with_options(&mut config)
        .with_enable_debug_data_prop(true)
    });

    assert_eq!(transform.state.options.class_name_prefix, "zz");
    assert!(transform.state.options.enable_debug_data_prop);
  });
}
