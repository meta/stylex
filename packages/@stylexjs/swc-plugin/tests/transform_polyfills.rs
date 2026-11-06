mod test_utils;

use stylex_swc_plugin::StyleXTransformOptions;
use test_utils::assert_transform_metadata_snapshot;

#[ignore = "Babel suite marks lineClamp polyfill test as skipped"]
#[test]
fn line_clamp_polyfill_metadata() {
    assert_transform_metadata_snapshot(
        r#"
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({ x: { lineClamp: 3 } });
        "#,
        "fixture.js",
        &StyleXTransformOptions::default(),
        r#"{"stylex":[]}"#,
    );
}

#[ignore = "Babel suite marks pointerEvents polyfill test as skipped"]
#[test]
fn pointer_events_polyfill_metadata() {
    assert_transform_metadata_snapshot(
        r#"
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              a: { pointerEvents: 'auto' },
              b: { pointerEvents: 'box-none' },
              c: { pointerEvents: 'box-only' },
              d: { pointerEvents: 'none' }
            });
        "#,
        "fixture.js",
        &StyleXTransformOptions::default(),
        r#"{"stylex":[]}"#,
    );
}

#[ignore = "Babel suite marks scrollbarWidth polyfill test as skipped"]
#[test]
fn scrollbar_width_polyfill_metadata() {
    assert_transform_metadata_snapshot(
        r#"
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({ x: { scrollbarWidth: 'none' } });
        "#,
        "fixture.js",
        &StyleXTransformOptions::default(),
        r#"{"stylex":[]}"#,
    );
}
