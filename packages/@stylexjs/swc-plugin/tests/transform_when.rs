mod test_utils;

use pretty_assertions::assert_eq;
use stylex_swc_plugin::{RuleEntry, RuleFields, StyleXTransformOptions, transform_source};
use test_utils::{assert_transform_code_snapshot, snapshot};

#[test]
fn transforms_when_namespace_imports() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const styles = stylex.create({
                  container: {
                    backgroundColor: {
                      default: 'blue',
                      [stylex.when.ancestor(':hover')]: 'red',
                      [stylex.when.siblingBefore(':focus')]: 'green',
                      [stylex.when.anySibling(':active')]: 'yellow',
                      [stylex.when.siblingAfter(':focus')]: 'purple',
                      [stylex.when.descendant(':focus')]: 'orange',
                    },
                  },
                });

                console.log(styles.container);
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';

                const styles = {container:{kWkggS:"x1t391ir x148kuu xpijypl xoev4mv x1v1vkh3 x9zntq3",$$css:true}};

                console.log(styles.container);
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "x1t391ir".to_owned(),
                RuleFields {
                    ltr: ".x1t391ir{background-color:blue}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
            RuleEntry(
                "x148kuu".to_owned(),
                RuleFields {
                    ltr: ".x148kuu.x148kuu:where(.x-default-marker:hover *){background-color:red}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3011.3,
            ),
            RuleEntry(
                "xpijypl".to_owned(),
                RuleFields {
                    ltr: ".xpijypl.xpijypl:where(.x-default-marker:focus ~ *){background-color:green}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3031.5,
            ),
            RuleEntry(
                "xoev4mv".to_owned(),
                RuleFields {
                    ltr: ".xoev4mv.xoev4mv:where(.x-default-marker:active ~ *, :has(~ .x-default-marker:active)){background-color:yellow}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3021.7,
            ),
            RuleEntry(
                "x1v1vkh3".to_owned(),
                RuleFields {
                    ltr: ".x1v1vkh3.x1v1vkh3:where(:has(~ .x-default-marker:focus)){background-color:purple}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3041.5,
            ),
            RuleEntry(
                "x9zntq3".to_owned(),
                RuleFields {
                    ltr: ".x9zntq3.x9zntq3:where(:has(.x-default-marker:focus)){background-color:orange}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3016.5,
            ),
        ]
    );
}

#[test]
fn transforms_when_named_import_ancestor() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import { when, create } from '@stylexjs/stylex';

                const styles = create({
                  container: {
                    backgroundColor: {
                      default: 'blue',
                      [when.ancestor(':hover')]: 'red',
                    },
                  },
                });

                console.log(styles.container);
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import { when, create } from '@stylexjs/stylex';

                const styles = {container:{kWkggS:"x1t391ir x148kuu",$$css:true}};

                console.log(styles.container);
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "x1t391ir".to_owned(),
                RuleFields {
                    ltr: ".x1t391ir{background-color:blue}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
            RuleEntry(
                "x148kuu".to_owned(),
                RuleFields {
                    ltr: ".x148kuu.x148kuu:where(.x-default-marker:hover *){background-color:red}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3011.3,
            ),
        ]
    );
}

#[test]
fn transforms_when_attribute_selectors() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import { when, create } from '@stylexjs/stylex';

                const styles = create({
                  container: {
                    backgroundColor: {
                      default: 'blue',
                      [when.ancestor('[data-panel-state="open"]')]: 'red',
                      [when.descendant('[data-panel-state="open"]')]: 'green',
                    },
                  },
                });

                console.log(styles.container);
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import { when, create } from '@stylexjs/stylex';

                const styles = {container:{kWkggS:"x1t391ir x11omtej x1doj7mj",$$css:true}};

                console.log(styles.container);
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "x1t391ir".to_owned(),
                RuleFields {
                    ltr: ".x1t391ir{background-color:blue}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
            RuleEntry(
                "x11omtej".to_owned(),
                RuleFields {
                    ltr: ".x11omtej.x11omtej:where(.x-default-marker[data-panel-state=\"open\"] *){background-color:red}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3040.0,
            ),
            RuleEntry(
                "x1doj7mj".to_owned(),
                RuleFields {
                    ltr: ".x1doj7mj.x1doj7mj:where(:has(.x-default-marker[data-panel-state=\"open\"])){background-color:green}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3040.0,
            ),
        ]
    );
}

#[test]
fn transforms_when_aliased_imports() {
    let output = assert_transform_code_snapshot(
        &snapshot(
            r#"
                import { when as w, create } from '@stylexjs/stylex';

                const styles = create({
                  container: {
                    backgroundColor: {
                      default: 'blue',
                      [w.ancestor(':hover')]: 'red',
                      [w.siblingBefore(':focus')]: 'green',
                    },
                  },
                });

                console.log(styles.container);
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
        &snapshot(
            r#"
                import { when as w, create } from '@stylexjs/stylex';

                const styles = {container:{kWkggS:"x1t391ir x148kuu xpijypl",$$css:true}};

                console.log(styles.container);
            "#,
        ),
    );

    assert_eq!(
        output.metadata_stylex,
        vec![
            RuleEntry(
                "x1t391ir".to_owned(),
                RuleFields {
                    ltr: ".x1t391ir{background-color:blue}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
            RuleEntry(
                "x148kuu".to_owned(),
                RuleFields {
                    ltr: ".x148kuu.x148kuu:where(.x-default-marker:hover *){background-color:red}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3011.3,
            ),
            RuleEntry(
                "xpijypl".to_owned(),
                RuleFields {
                    ltr: ".xpijypl.xpijypl:where(.x-default-marker:focus ~ *){background-color:green}".to_owned(),
                    rtl: None,
                    const_key: None,
                    const_val: None,
                },
                3031.5,
            ),
        ]
    );
}

#[test]
fn rejects_when_pseudo_elements() {
    let error = transform_source(
        &snapshot(
            r#"
                import { when, create } from '@stylexjs/stylex';

                const styles = create({
                  container: {
                    backgroundColor: {
                      default: 'blue',
                      [when.ancestor('::before')]: 'red',
                    },
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
    )
    .expect_err("expected transform failure");

    assert_eq!(error.to_string(), "Pseudo selector cannot start with \"::\"");
}

#[test]
fn rejects_invalid_attribute_selector_format() {
    let error = transform_source(
        &snapshot(
            r#"
                import { when, create } from '@stylexjs/stylex';

                const styles = create({
                  container: {
                    backgroundColor: {
                      default: 'blue',
                      [when.ancestor('[data-state=\"open\"')]: 'red',
                    },
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
    )
    .expect_err("expected transform failure");

    assert_eq!(error.to_string(), "Attribute selector must end with \"]\"");
}

#[test]
fn validates_when_selector_format() {
    let error = transform_source(
        &snapshot(
            r#"
                import { when, create } from '@stylexjs/stylex';

                const styles = create({
                  container: {
                    backgroundColor: {
                      default: 'blue',
                      [when.ancestor('hover')]: 'red',
                    },
                  },
                });
            "#,
        ),
        "fixture.js",
        &StyleXTransformOptions::default(),
    )
    .expect_err("expected transform failure");

    assert_eq!(
        error.to_string(),
        "Pseudo selector must start with \":\" or \"[\""
    );
}
