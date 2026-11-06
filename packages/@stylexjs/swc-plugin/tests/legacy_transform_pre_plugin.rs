mod test_utils;

use stylex_swc_plugin::{RuntimeInjectionOption, StyleXTransformOptions};
use test_utils::{assert_transform_code_snapshot, snapshot};

#[test]
fn transforms_stylex_create_output_from_pre_plugin_rewrite() {
    let mut options = StyleXTransformOptions {
        runtime_injection: RuntimeInjectionOption::Bool(true),
        ..Default::default()
    };
    options.additional_options.insert(
        "unstable_moduleResolution".to_owned(),
        serde_json::json!({
            "type": "haste",
        }),
    );

    assert_transform_code_snapshot(
        &snapshot(
            r#"
                import stylex from 'stylex';

                function Demo() {
                  return (
                    <div>
                      <button {...stylex.props(
                        styles.default,
                        styles.$0
                      )}>
                        Hello
                      </button>
                    </div>
                  );
                }

                const styles = stylex.create({
                  default: {
                    appearance: 'none',
                    borderWidth: '0',
                    borderStyle: 'none',
                  },
                  $0: {
                    backgroundColor: 'pink',
                    color: 'white',
                  },
                });
            "#,
        ),
        "fixture.jsx",
        &options,
        &snapshot(
            r#"
                import _inject from "@stylexjs/stylex/lib/stylex-inject";
                var _inject2 = _inject;
                import stylex from 'stylex';

                function Demo() {
                  return <div>
                      <button className="xjyslct xc342km xng3xce x6tqnqi x1awj2ng">
                        Hello
                      </button>
                    </div>;
                }

                _inject2({
                  ltr: ".xjyslct{appearance:none}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".xc342km{border-width:0}",
                  priority: 2000,
                });
                _inject2({
                  ltr: ".xng3xce{border-style:none}",
                  priority: 2000,
                });
                _inject2({
                  ltr: ".x6tqnqi{background-color:pink}",
                  priority: 3000,
                });
                _inject2({
                  ltr: ".x1awj2ng{color:white}",
                  priority: 3000,
                });
            "#,
        ),
    );
}
