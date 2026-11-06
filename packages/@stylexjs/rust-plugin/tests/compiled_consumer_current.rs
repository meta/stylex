//! Ordered compiled-style cancellation and unknown-namespace provenance.
use serde_json::json;
use stylex_swc_plugin::{StyleXTransformOptions, transform_source};
#[path = "support/current_babel.rs"]
mod current_babel;

fn compare(source: &str, options: serde_json::Value) -> stylex_swc_plugin::TransformOutput {
  let options: StyleXTransformOptions = serde_json::from_value(options).unwrap();
  current_babel::transform(source, "/tmp/compiled-consumer.js", &options)
}
#[test]
fn compiled_style_runtime_conditional_retains_null_cancellation_fields() {
  for (callee, arguments) in [
    ("stylex", "styles.red,isActive&&styles.blue"),
    ("stylex.props", "[styles.red,isActive&&styles.blue]"),
  ] {
    let source = format!(
      "import stylex from 'stylex';const styles=stylex.create({{red:{{color:'red'}},blue:{{color:null}}}});{callee}({arguments});"
    );
    let result = compare(
      &source,
      json!({"enableInlinedConditionalMerge":false,"runtimeInjection":true}),
    );
    assert!(
      result.code.contains("kMwMTN: null"),
      "compiled cancellation was pruned: {}",
      result.code
    );
  }
}
#[test]
fn compiled_style_unknown_namespace_keeps_the_runtime_read_and_empty_declaration() {
  for callee in ["stylex", "stylex.props"] {
    let source = format!(
      "import * as stylex from '@stylexjs/stylex';const styles=stylex.create({{tileHeading:{{marginRight:12}}}});{callee}(styles.unknown);"
    );
    let result = compare(
      &source,
      json!({"dev":true,"enableDevClassNames":false,"runtimeInjection":true}),
    );
    assert!(result.code.contains("const styles"));
    assert!(result.code.contains("styles.unknown"));
  }
}
#[test]
fn compiled_style_runtime_prefix_retains_only_required_shorthand_nulls() {
  let result = compare(
    "import stylex from 'stylex';const styles=stylex.create({foo:{padding:5},bar:{paddingBlock:10},baz:{paddingTop:7}});stylex(styles.foo);stylex(styles.foo,styles.bar);stylex(styles.bar,styles.foo);stylex(styles.foo,styles.bar,styles.baz);stylex(styles.baz,styles.foo,somethingElse);",
    json!({"styleResolution":"application-order","runtimeInjection":true}),
  );
  assert!(
    result.code.contains("kLKAdn: null"),
    "required prefix cancellation disappeared"
  );
}
#[test]
fn ordinary_missing_properties_and_explicit_null_values_keep_javascript_semantics() {
  let source = "import * as stylex from '@stylexjs/stylex';const data={};const absent=data.missing;const styles=stylex.create({root:{width:absent==null?1:2,color:null}});use(styles);";
  compare(source, json!({}));
  // This is a deliberate Id-safe invariant, not a compiler output normalizer.
  let authored = "const data={};const absent=data.missing;export {absent};";
  let result = transform_source(
    authored,
    "/tmp/compiled-consumer.js",
    &StyleXTransformOptions::default(),
  )
  .unwrap();
  current_babel::assert_authored(authored, "/tmp/compiled-consumer.js", &result);
}
