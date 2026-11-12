mod test_utils;

use stylex_swc_plugin::StyleXTransformOptions;
use test_utils::{assert_transform_metadata_snapshot, snapshot};

fn application_order_options() -> StyleXTransformOptions {
    StyleXTransformOptions::default()
}

fn legacy_options(enable_logical_styles_polyfill: bool) -> StyleXTransformOptions {
    let mut options = StyleXTransformOptions::default();
    options.additional_options.insert(
        "styleResolution".to_owned(),
        serde_json::json!("legacy-expand-shorthands"),
    );
    options.additional_options.insert(
        "enableLogicalStylesPolyfill".to_owned(),
        serde_json::json!(enable_logical_styles_polyfill),
    );
    options
}

macro_rules! metadata_case {
    ($name:ident, $opts:expr, $input:expr, $expected:expr) => {
        #[test]
        fn $name() {
            assert_transform_metadata_snapshot(
                &snapshot($input),
                "fixture.js",
                &$opts,
                &snapshot($expected),
            );
        }
    };
}

metadata_case!(
    application_order_end_alias,
    application_order_options(),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { end: 5 } });
    "#,
    r#"
        {
          "stylex": [
            [
              "xceh6e4",
              {
                "ltr": ".xceh6e4{inset-inline-end:5px}",
                "rtl": null
              },
              3000
            ]
          ]
        }
    "#
);

metadata_case!(
    application_order_margin_end_alias,
    application_order_options(),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { marginEnd: 0 } });
    "#,
    r#"
        {
          "stylex": [
            [
              "x14z9mp",
              {
                "ltr": ".x14z9mp{margin-inline-end:0}",
                "rtl": null
              },
              3000
            ]
          ]
        }
    "#
);

metadata_case!(
    application_order_margin_horizontal_alias,
    application_order_options(),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { marginHorizontal: 0 } });
    "#,
    r#"
        {
          "stylex": [
            [
              "xrxpjvj",
              {
                "ltr": ".xrxpjvj{margin-inline:0}",
                "rtl": null
              },
              2000
            ]
          ]
        }
    "#
);

metadata_case!(
    application_order_margin_start_alias,
    application_order_options(),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { marginStart: 0 } });
    "#,
    r#"
        {
          "stylex": [
            [
              "x1lziwak",
              {
                "ltr": ".x1lziwak{margin-inline-start:0}",
                "rtl": null
              },
              3000
            ]
          ]
        }
    "#
);

metadata_case!(
    application_order_margin_vertical_alias,
    application_order_options(),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { marginVertical: 0 } });
    "#,
    r#"
        {
          "stylex": [
            [
              "x10im51j",
              {
                "ltr": ".x10im51j{margin-block:0}",
                "rtl": null
              },
              2000
            ]
          ]
        }
    "#
);

metadata_case!(
    application_order_padding_end_alias,
    application_order_options(),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { paddingEnd: 0 } });
    "#,
    r#"
        {
          "stylex": [
            [
              "xyri2b",
              {
                "ltr": ".xyri2b{padding-inline-end:0}",
                "rtl": null
              },
              3000
            ]
          ]
        }
    "#
);

metadata_case!(
    application_order_padding_horizontal_alias,
    application_order_options(),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { paddingHorizontal: 0 } });
    "#,
    r#"
        {
          "stylex": [
            [
              "xnjsko4",
              {
                "ltr": ".xnjsko4{padding-inline:0}",
                "rtl": null
              },
              2000
            ]
          ]
        }
    "#
);

metadata_case!(
    application_order_padding_start_alias,
    application_order_options(),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { paddingStart: 0 } });
    "#,
    r#"
        {
          "stylex": [
            [
              "x1c1uobl",
              {
                "ltr": ".x1c1uobl{padding-inline-start:0}",
                "rtl": null
              },
              3000
            ]
          ]
        }
    "#
);

metadata_case!(
    application_order_padding_vertical_alias,
    application_order_options(),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { paddingVertical: 0 } });
    "#,
    r#"
        {
          "stylex": [
            [
              "xt970qd",
              {
                "ltr": ".xt970qd{padding-block:0}",
                "rtl": null
              },
              2000
            ]
          ]
        }
    "#
);

metadata_case!(
    application_order_start_alias,
    application_order_options(),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { start: 5 } });
    "#,
    r#"
        {
          "stylex": [
            [
              "x1fb7gu6",
              {
                "ltr": ".x1fb7gu6{inset-inline-start:5px}",
                "rtl": null
              },
              3000
            ]
          ]
        }
    "#
);

metadata_case!(
    application_order_clear_end_value,
    application_order_options(),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { clear: 'end' } });
    "#,
    r#"
        {
          "stylex": [
            [
              "xodj72a",
              {
                "ltr": ".xodj72a{clear:right}",
                "rtl": ".xodj72a{clear:left}"
              },
              3000
            ]
          ]
        }
    "#
);

metadata_case!(
    application_order_clear_start_value,
    application_order_options(),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { clear: 'start' } });
    "#,
    r#"
        {
          "stylex": [
            [
              "x390i0x",
              {
                "ltr": ".x390i0x{clear:left}",
                "rtl": ".x390i0x{clear:right}"
              },
              3000
            ]
          ]
        }
    "#
);

metadata_case!(
    application_order_float_end_value,
    application_order_options(),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { float: 'end' } });
    "#,
    r#"
        {
          "stylex": [
            [
              "x1guec7k",
              {
                "ltr": ".x1guec7k{float:right}",
                "rtl": ".x1guec7k{float:left}"
              },
              3000
            ]
          ]
        }
    "#
);

metadata_case!(
    application_order_float_start_value,
    application_order_options(),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { float: 'start' } });
    "#,
    r#"
        {
          "stylex": [
            [
              "xrbpyxo",
              {
                "ltr": ".xrbpyxo{float:left}",
                "rtl": ".xrbpyxo{float:right}"
              },
              3000
            ]
          ]
        }
    "#
);

metadata_case!(
    legacy_true_end_alias,
    legacy_options(true),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { end: 5 } });
    "#,
    r#"
        {
          "stylex": [
            [
              "xceh6e4",
              {
                "ltr": ".xceh6e4{right:5px}",
                "rtl": ".xceh6e4{left:5px}"
              },
              3000
            ]
          ]
        }
    "#
);

metadata_case!(
    legacy_true_margin_end_alias,
    legacy_options(true),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { marginEnd: 0 } });
    "#,
    r#"
        {
          "stylex": [
            [
              "x14z9mp",
              {
                "ltr": ".x14z9mp{margin-right:0}",
                "rtl": ".x14z9mp{margin-left:0}"
              },
              3000
            ]
          ]
        }
    "#
);

metadata_case!(
    legacy_true_margin_horizontal_alias,
    legacy_options(true),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { marginHorizontal: 0 } });
    "#,
    r#"
        {
          "stylex": [
            [
              "x1lziwak",
              {
                "ltr": ".x1lziwak{margin-left:0}",
                "rtl": ".x1lziwak{margin-right:0}"
              },
              3000
            ],
            [
              "x14z9mp",
              {
                "ltr": ".x14z9mp{margin-right:0}",
                "rtl": ".x14z9mp{margin-left:0}"
              },
              3000
            ]
          ]
        }
    "#
);

metadata_case!(
    legacy_true_margin_start_alias,
    legacy_options(true),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { marginStart: 0 } });
    "#,
    r#"
        {
          "stylex": [
            [
              "x1lziwak",
              {
                "ltr": ".x1lziwak{margin-left:0}",
                "rtl": ".x1lziwak{margin-right:0}"
              },
              3000
            ]
          ]
        }
    "#
);

metadata_case!(
    legacy_true_margin_vertical_alias,
    legacy_options(true),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { marginVertical: 0 } });
    "#,
    r#"
        {
          "stylex": [
            [
              "xdj266r",
              {
                "ltr": ".xdj266r{margin-top:0}",
                "rtl": null
              },
              4000
            ],
            [
              "xat24cr",
              {
                "ltr": ".xat24cr{margin-bottom:0}",
                "rtl": null
              },
              4000
            ]
          ]
        }
    "#
);

metadata_case!(
    legacy_true_padding_end_alias,
    legacy_options(true),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { paddingEnd: 0 } });
    "#,
    r#"
        {
          "stylex": [
            [
              "xyri2b",
              {
                "ltr": ".xyri2b{padding-right:0}",
                "rtl": ".xyri2b{padding-left:0}"
              },
              3000
            ]
          ]
        }
    "#
);

metadata_case!(
    legacy_true_padding_horizontal_alias,
    legacy_options(true),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { paddingHorizontal: 0 } });
    "#,
    r#"
        {
          "stylex": [
            [
              "x1c1uobl",
              {
                "ltr": ".x1c1uobl{padding-left:0}",
                "rtl": ".x1c1uobl{padding-right:0}"
              },
              3000
            ],
            [
              "xyri2b",
              {
                "ltr": ".xyri2b{padding-right:0}",
                "rtl": ".xyri2b{padding-left:0}"
              },
              3000
            ]
          ]
        }
    "#
);

metadata_case!(
    legacy_true_padding_start_alias,
    legacy_options(true),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { paddingStart: 0 } });
    "#,
    r#"
        {
          "stylex": [
            [
              "x1c1uobl",
              {
                "ltr": ".x1c1uobl{padding-left:0}",
                "rtl": ".x1c1uobl{padding-right:0}"
              },
              3000
            ]
          ]
        }
    "#
);

metadata_case!(
    legacy_true_padding_vertical_alias,
    legacy_options(true),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { paddingVertical: 0 } });
    "#,
    r#"
        {
          "stylex": [
            [
              "xexx8yu",
              {
                "ltr": ".xexx8yu{padding-top:0}",
                "rtl": null
              },
              4000
            ],
            [
              "x18d9i69",
              {
                "ltr": ".x18d9i69{padding-bottom:0}",
                "rtl": null
              },
              4000
            ]
          ]
        }
    "#
);

metadata_case!(
    legacy_true_start_alias,
    legacy_options(true),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { start: 5 } });
    "#,
    r#"
        {
          "stylex": [
            [
              "x1fb7gu6",
              {
                "ltr": ".x1fb7gu6{left:5px}",
                "rtl": ".x1fb7gu6{right:5px}"
              },
              3000
            ]
          ]
        }
    "#
);

metadata_case!(
    legacy_true_clear_end_value,
    legacy_options(true),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { clear: 'end' } });
    "#,
    r#"
        {
          "stylex": [
            [
              "x1wx8h15",
              {
                "ltr": ".x1wx8h15{clear:var(--stylex-logical-end)}",
                "rtl": null
              },
              3000
            ]
          ]
        }
    "#
);

metadata_case!(
    legacy_true_clear_start_value,
    legacy_options(true),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { clear: 'start' } });
    "#,
    r#"
        {
          "stylex": [
            [
              "x1fhb4fj",
              {
                "ltr": ".x1fhb4fj{clear:var(--stylex-logical-start)}",
                "rtl": null
              },
              3000
            ]
          ]
        }
    "#
);

metadata_case!(
    legacy_true_float_end_value,
    legacy_options(true),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { float: 'end' } });
    "#,
    r#"
        {
          "stylex": [
            [
              "xtrg13t",
              {
                "ltr": ".xtrg13t{float:var(--stylex-logical-end)}",
                "rtl": null
              },
              3000
            ]
          ]
        }
    "#
);

metadata_case!(
    legacy_true_float_start_value,
    legacy_options(true),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { float: 'start' } });
    "#,
    r#"
        {
          "stylex": [
            [
              "xj87blo",
              {
                "ltr": ".xj87blo{float:var(--stylex-logical-start)}",
                "rtl": null
              },
              3000
            ]
          ]
        }
    "#
);

metadata_case!(
    legacy_false_end_alias,
    legacy_options(false),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { end: 5 } });
    "#,
    r#"
        {
          "stylex": [
            [
              "xceh6e4",
              {
                "ltr": ".xceh6e4{inset-inline-end:5px}",
                "rtl": null
              },
              3000
            ]
          ]
        }
    "#
);

metadata_case!(
    legacy_false_margin_end_alias,
    legacy_options(false),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { marginEnd: 0 } });
    "#,
    r#"
        {
          "stylex": [
            [
              "x14z9mp",
              {
                "ltr": ".x14z9mp{margin-inline-end:0}",
                "rtl": null
              },
              3000
            ]
          ]
        }
    "#
);

metadata_case!(
    legacy_false_margin_horizontal_alias,
    legacy_options(false),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { marginHorizontal: 0 } });
    "#,
    r#"
        {
          "stylex": [
            [
              "x1lziwak",
              {
                "ltr": ".x1lziwak{margin-inline-start:0}",
                "rtl": null
              },
              3000
            ],
            [
              "x14z9mp",
              {
                "ltr": ".x14z9mp{margin-inline-end:0}",
                "rtl": null
              },
              3000
            ]
          ]
        }
    "#
);

metadata_case!(
    legacy_false_margin_start_alias,
    legacy_options(false),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { marginStart: 0 } });
    "#,
    r#"
        {
          "stylex": [
            [
              "x1lziwak",
              {
                "ltr": ".x1lziwak{margin-inline-start:0}",
                "rtl": null
              },
              3000
            ]
          ]
        }
    "#
);

metadata_case!(
    legacy_false_margin_vertical_alias,
    legacy_options(false),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { marginVertical: 0 } });
    "#,
    r#"
        {
          "stylex": [
            [
              "xdj266r",
              {
                "ltr": ".xdj266r{margin-top:0}",
                "rtl": null
              },
              4000
            ],
            [
              "xat24cr",
              {
                "ltr": ".xat24cr{margin-bottom:0}",
                "rtl": null
              },
              4000
            ]
          ]
        }
    "#
);

metadata_case!(
    legacy_false_padding_end_alias,
    legacy_options(false),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { paddingEnd: 0 } });
    "#,
    r#"
        {
          "stylex": [
            [
              "xyri2b",
              {
                "ltr": ".xyri2b{padding-inline-end:0}",
                "rtl": null
              },
              3000
            ]
          ]
        }
    "#
);

metadata_case!(
    legacy_false_padding_horizontal_alias,
    legacy_options(false),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { paddingHorizontal: 0 } });
    "#,
    r#"
        {
          "stylex": [
            [
              "x1c1uobl",
              {
                "ltr": ".x1c1uobl{padding-inline-start:0}",
                "rtl": null
              },
              3000
            ],
            [
              "xyri2b",
              {
                "ltr": ".xyri2b{padding-inline-end:0}",
                "rtl": null
              },
              3000
            ]
          ]
        }
    "#
);

metadata_case!(
    legacy_false_padding_start_alias,
    legacy_options(false),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { paddingStart: 0 } });
    "#,
    r#"
        {
          "stylex": [
            [
              "x1c1uobl",
              {
                "ltr": ".x1c1uobl{padding-inline-start:0}",
                "rtl": null
              },
              3000
            ]
          ]
        }
    "#
);

metadata_case!(
    legacy_false_padding_vertical_alias,
    legacy_options(false),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { paddingVertical: 0 } });
    "#,
    r#"
        {
          "stylex": [
            [
              "xexx8yu",
              {
                "ltr": ".xexx8yu{padding-top:0}",
                "rtl": null
              },
              4000
            ],
            [
              "x18d9i69",
              {
                "ltr": ".x18d9i69{padding-bottom:0}",
                "rtl": null
              },
              4000
            ]
          ]
        }
    "#
);

metadata_case!(
    legacy_false_start_alias,
    legacy_options(false),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { start: 5 } });
    "#,
    r#"
        {
          "stylex": [
            [
              "x1fb7gu6",
              {
                "ltr": ".x1fb7gu6{inset-inline-start:5px}",
                "rtl": null
              },
              3000
            ]
          ]
        }
    "#
);

metadata_case!(
    legacy_false_clear_end_value,
    legacy_options(false),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { clear: 'end' } });
    "#,
    r#"
        {
          "stylex": [
            [
              "x1wx8h15",
              {
                "ltr": ".x1wx8h15{clear:var(--stylex-logical-end)}",
                "rtl": null
              },
              3000
            ]
          ]
        }
    "#
);

metadata_case!(
    legacy_false_clear_start_value,
    legacy_options(false),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { clear: 'start' } });
    "#,
    r#"
        {
          "stylex": [
            [
              "x1fhb4fj",
              {
                "ltr": ".x1fhb4fj{clear:var(--stylex-logical-start)}",
                "rtl": null
              },
              3000
            ]
          ]
        }
    "#
);

metadata_case!(
    legacy_false_float_end_value,
    legacy_options(false),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { float: 'end' } });
    "#,
    r#"
        {
          "stylex": [
            [
              "xtrg13t",
              {
                "ltr": ".xtrg13t{float:var(--stylex-logical-end)}",
                "rtl": null
              },
              3000
            ]
          ]
        }
    "#
);

metadata_case!(
    legacy_false_float_start_value,
    legacy_options(false),
    r#"
        import * as stylex from '@stylexjs/stylex';
        export const styles = stylex.create({ x: { float: 'start' } });
    "#,
    r#"
        {
          "stylex": [
            [
              "xj87blo",
              {
                "ltr": ".xj87blo{float:var(--stylex-logical-start)}",
                "rtl": null
              },
              3000
            ]
          ]
        }
    "#
);
