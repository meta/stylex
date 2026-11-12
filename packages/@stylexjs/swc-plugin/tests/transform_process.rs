mod test_utils;

use stylex_swc_plugin::{
    process_stylex_rules, LayersConfig, ProcessStylexRulesConfig, RuntimeInjectionOption,
    RuleEntry, RuleFields, StyleXTransformOptions, UseLayers,
};
use test_utils::{assert_code_matches_snapshot, snapshot};

fn process_options() -> StyleXTransformOptions {
    let mut options = StyleXTransformOptions::default();
    options.runtime_injection = RuntimeInjectionOption::Bool(false);
    options.additional_options.insert(
        "debug".to_owned(),
        serde_json::Value::Bool(true),
    );
    options.additional_options.insert(
        "enableDebugClassNames".to_owned(),
        serde_json::Value::Bool(true),
    );
    options.additional_options.insert(
        "styleResolution".to_owned(),
        serde_json::Value::String("property-specificity".to_owned()),
    );
    options.additional_options.insert(
        "unstable_moduleResolution".to_owned(),
        serde_json::json!({
            "rootDir": "/src/app/",
            "type": "commonJS",
        }),
    );
    options
}

fn transform_compound(
    source: &str,
    options: &StyleXTransformOptions,
) -> (String, Vec<stylex_swc_plugin::RuleEntry>) {
    let tokens = stylex_swc_plugin::transform_source(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';
                export const constants = stylex.defineConsts({
                  YELLOW: 'yellow',
                  ORANGE: 'var(--orange-theme-color)',
                  mediaBig: '@media (max-width: 1000px)',
                  mediaSmall: '@media (max-width: 500px)'
                });
                export const vars = stylex.defineVars({
                  blue: 'blue',
                  marginTokens: {
                    default: "10px",
                    "@media (min-width: 600px)": "20px"
                  },
                  colorTokens: {
                    default: 'red',
                    '@media (prefers-color-scheme: dark)': {
                      default: 'lightblue',
                      '@supports (color: oklab(0 0 0))': 'oklab(0.7 -0.3 -0.4)',
                    }
                  },
                });
            "#,
        ),
        "/src/app/tokens.stylex.js",
        options,
    )
    .expect("transform tokens");

    let other_tokens = stylex_swc_plugin::transform_source(
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';
                export const spacing = stylex.defineVars({
                  small: '2px',
                  medium: '4px',
                  large: '8px'
                });
            "#,
        ),
        "/src/app/otherTokens.stylex.js",
        options,
    )
    .expect("transform other tokens");

    let combined_source = format!(
        "{}\n{}\n{}",
        tokens.code,
        other_tokens
            .code
            .replace("import * as stylex from '@stylexjs/stylex';", ""),
        source.replace("import * as stylex from '@stylexjs/stylex';", "")
    );
    let main = stylex_swc_plugin::transform_source(
        &combined_source,
        "/src/app/main.js",
        options,
    )
    .expect("transform main");

    let mut metadata = tokens.metadata_stylex;
    metadata.extend(other_tokens.metadata_stylex);
    metadata.extend(main.metadata_stylex);
    (main.code, metadata)
}

fn process_css(
    metadata: &[RuleEntry],
    config: ProcessStylexRulesConfig,
) -> String {
    process_stylex_rules(metadata, Some(&config))
}

fn rule(name: &str, ltr: &str, priority: f64) -> RuleEntry {
    RuleEntry(
        name.to_owned(),
        RuleFields {
            ltr: ltr.to_owned(),
            rtl: None,
            const_key: None,
            const_val: None,
        },
        priority,
    )
}

const FIXTURE: &str = r#"
    import * as stylex from '@stylexjs/stylex';
    export const themeColor = stylex.createTheme(vars, {
      blue: 'lightblue'
    });
    export const themeSpacing = stylex.createTheme(spacing, {
      small: '5px',
      medium: '10px',
      large: '20px'
    });
    export const styles = stylex.create({
      root: {
        animationName: stylex.keyframes({
          '0%': {
            boxShadow: '1px 2px 3px 4px red',
            color: constants.YELLOW
          },
          '100%': {
            boxShadow: '10px 20px 30px 40px green',
            color: constants.ORANGE
          }
        }),
        backgroundColor: {
          default: 'red',
          ':hover': 'blue',
          [stylex.when.ancestor(':focus')]: 'green',
          '@media (max-width: 1000px)': {
            default: 'yellow',
            [stylex.when.descendant(':focus')]: 'purple',
            [stylex.when.anySibling(':active')]: 'orange',
          }
        },
        margin: vars.marginTokens,
        borderColor: {
          default: 'green',
          [constants.mediaBig]: {
            default: vars.blue,
            [constants.mediaSmall]: 'yellow',
          }
        },
        outlineColor: vars.colorTokens,
        textShadow: {
          default: '1px 2px 3px 4px red',
          '@media (min-width:320px)': '10px 20px 30px 40px green'
        },
        padding: spacing.large,
        margin: '10px 20px',
        float: 'inline-start'
      },
      overrideColor: {
        [constants.ORANGE]: 'red'
      },
      dynamic: (color) => ({ color })
    });
"#;

#[test]
fn process_no_rules_matches_snapshot() {
    let options = process_options();
    let (code, metadata) = transform_compound("", &options);

    assert_code_matches_snapshot(
        &code,
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';
                export const constants = {
                  YELLOW: "yellow",
                  ORANGE: "var(--orange-theme-color)",
                  mediaBig: "@media (max-width: 1000px)",
                  mediaSmall: "@media (max-width: 500px)",
                };
                export const vars = {
                  blue: "var(--blue-xpqh4lw)",
                  marginTokens: "var(--marginTokens-x8nt2k2)",
                  colorTokens: "var(--colorTokens-xkxfyv)",
                  __varGroupHash__: "xsg933n",
                };
                export const spacing = {
                  small: "var(--small-x19twipt)",
                  medium: "var(--medium-xypjos2)",
                  large: "var(--large-x1ec7iuc)",
                  __varGroupHash__: "xbiwvf9",
                };
            "#,
        ),
        "/src/app/main.js",
    );

    assert_eq!(
        process_stylex_rules(
            &metadata,
            Some(&ProcessStylexRulesConfig {
                use_layers: UseLayers::Bool(false),
                enable_ltr_rtl_comments: false,
                ..Default::default()
            })
        ),
        snapshot(
            r#"
                :root, .xsg933n{--blue-xpqh4lw:blue;--marginTokens-x8nt2k2:10px;--colorTokens-xkxfyv:red;}
                :root, .xbiwvf9{--small-x19twipt:2px;--medium-xypjos2:4px;--large-x1ec7iuc:8px;}
                @media (prefers-color-scheme: dark){:root, .xsg933n{--colorTokens-xkxfyv:lightblue;}}
                @media (min-width: 600px){:root, .xsg933n{--marginTokens-x8nt2k2:20px;}}
                @supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xsg933n{--colorTokens-xkxfyv:oklab(0.7 -0.3 -0.4);}}}"#,
        )
    );
}

#[test]
fn process_all_rules_without_layers_matches_snapshot() {
    let options = process_options();
    let (_code, metadata) = transform_compound(FIXTURE, &options);

    assert_eq!(
        process_stylex_rules(
            &metadata,
            Some(&ProcessStylexRulesConfig {
                use_layers: UseLayers::Bool(false),
                enable_ltr_rtl_comments: false,
                ..Default::default()
            })
        ),
        snapshot(
            r#"
                @property --x-color { syntax: "*"; inherits: false;}
                @keyframes x35atj5-B{0%{box-shadow:1px 2px 3px 4px red;color:yellow;}100%{box-shadow:10px 20px 30px 40px green;color:var(--orange-theme-color);}}
                :root, .xsg933n{--blue-xpqh4lw:blue;--marginTokens-x8nt2k2:10px;--colorTokens-xkxfyv:red;}
                :root, .xbiwvf9{--small-x19twipt:2px;--medium-xypjos2:4px;--large-x1ec7iuc:8px;}
                @media (prefers-color-scheme: dark){:root, .xsg933n{--colorTokens-xkxfyv:lightblue;}}
                @media (min-width: 600px){:root, .xsg933n{--marginTokens-x8nt2k2:20px;}}
                @supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xsg933n{--colorTokens-xkxfyv:oklab(0.7 -0.3 -0.4);}}}
                .x6xqkwy.x6xqkwy, .x6xqkwy.x6xqkwy:root{--blue-xpqh4lw:lightblue;}
                .x57uvma.x57uvma, .x57uvma.x57uvma:root{--large-x1ec7iuc:20px;--medium-xypjos2:10px;--small-x19twipt:5px;}
                .--orange-theme-color-xufgesz{--orange-theme-color:red}
                .margin-xymmreb:not(#\#){margin:10px 20px}
                .padding-xss17vw:not(#\#){padding:var(--large-x1ec7iuc)}
                .borderColor-x1bg2uv5:not(#\#):not(#\#){border-color:green}
                @media (max-width: 1000px){.borderColor-x5ugf7c.borderColor-x5ugf7c:not(#\#):not(#\#){border-color:var(--blue-xpqh4lw)}}
                @media (max-width: 500px){@media (max-width: 1000px){.borderColor-xqiy1ys.borderColor-xqiy1ys.borderColor-xqiy1ys:not(#\#):not(#\#){border-color:yellow}}}
                .animationName-x13ah0pd:not(#\#):not(#\#):not(#\#){animation-name:x35atj5-B}
                .backgroundColor-xrkmrrc:not(#\#):not(#\#):not(#\#){background-color:red}
                .color-x14rh7hd:not(#\#):not(#\#):not(#\#){color:var(--x-color)}
                html:not([dir='rtl']) .float-x1kmio9f:not(#\#):not(#\#):not(#\#){float:left}
                html[dir='rtl'] .float-x1kmio9f:not(#\#):not(#\#):not(#\#){float:right}
                .outlineColor-x184ctg8:not(#\#):not(#\#):not(#\#){outline-color:var(--colorTokens-xkxfyv)}
                .textShadow-x1skrh0i:not(#\#):not(#\#):not(#\#){text-shadow:1px 2px 3px 4px red}
                .backgroundColor-xfy810d.backgroundColor-xfy810d:where(.x-default-marker:focus *):not(#\#):not(#\#):not(#\#){background-color:green}
                .backgroundColor-xbrh7vm:hover:not(#\#):not(#\#):not(#\#){background-color:blue}
                @media (max-width: 1000px){.backgroundColor-xahc4vn.backgroundColor-xahc4vn:not(#\#):not(#\#):not(#\#){background-color:yellow}}
                @media (min-width: 320px){.textShadow-xtj17id.textShadow-xtj17id:not(#\#):not(#\#):not(#\#){text-shadow:10px 20px 30px 40px green}}
                @media (max-width: 1000px){.backgroundColor-x1t4kl4c.backgroundColor-x1t4kl4c.backgroundColor-x1t4kl4c:where(:has(.x-default-marker:focus)):not(#\#):not(#\#):not(#\#){background-color:purple}}
                @media (max-width: 1000px){.backgroundColor-x975j7z.backgroundColor-x975j7z.backgroundColor-x975j7z:where(.x-default-marker:active ~ *, :has(~ .x-default-marker:active)):not(#\#):not(#\#):not(#\#){background-color:orange}}"#,
        )
    );
}

#[test]
fn process_all_rules_with_layers_prefix_before_after_matches_snapshot() {
    let options = process_options();
    let (_code, metadata) = transform_compound(FIXTURE, &options);

    assert_eq!(
        process_stylex_rules(
            &metadata,
            Some(&ProcessStylexRulesConfig {
                use_layers: UseLayers::Config(LayersConfig {
                    prefix: "xds.base".to_owned(),
                    before: vec!["xds.reset".to_owned(), "xds.typography".to_owned()],
                    after: vec!["xds.theme".to_owned()],
                }),
                enable_ltr_rtl_comments: false,
                ..Default::default()
            })
        ),
        ["".to_owned(), snapshot(
            r#"
                @layer xds.reset, xds.typography, xds.base.priority1, xds.base.priority2, xds.base.priority3, xds.base.priority4, xds.theme;
                @property --x-color { syntax: "*"; inherits: false;}
                @keyframes x35atj5-B{0%{box-shadow:1px 2px 3px 4px red;color:yellow;}100%{box-shadow:10px 20px 30px 40px green;color:var(--orange-theme-color);}}
                :root, .xsg933n{--blue-xpqh4lw:blue;--marginTokens-x8nt2k2:10px;--colorTokens-xkxfyv:red;}
                :root, .xbiwvf9{--small-x19twipt:2px;--medium-xypjos2:4px;--large-x1ec7iuc:8px;}
                @media (prefers-color-scheme: dark){:root, .xsg933n{--colorTokens-xkxfyv:lightblue;}}
                @media (min-width: 600px){:root, .xsg933n{--marginTokens-x8nt2k2:20px;}}
                @supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xsg933n{--colorTokens-xkxfyv:oklab(0.7 -0.3 -0.4);}}}
                .x6xqkwy.x6xqkwy, .x6xqkwy.x6xqkwy:root{--blue-xpqh4lw:lightblue;}
                .x57uvma.x57uvma, .x57uvma.x57uvma:root{--large-x1ec7iuc:20px;--medium-xypjos2:10px;--small-x19twipt:5px;}
                .--orange-theme-color-xufgesz{--orange-theme-color:red}
                @layer xds.base.priority2{
                .margin-xymmreb{margin:10px 20px}
                .padding-xss17vw{padding:var(--large-x1ec7iuc)}
                }
                @layer xds.base.priority3{
                .borderColor-x1bg2uv5{border-color:green}
                @media (max-width: 1000px){.borderColor-x5ugf7c.borderColor-x5ugf7c{border-color:var(--blue-xpqh4lw)}}
                @media (max-width: 500px){@media (max-width: 1000px){.borderColor-xqiy1ys.borderColor-xqiy1ys.borderColor-xqiy1ys{border-color:yellow}}}
                }
                @layer xds.base.priority4{
                .animationName-x13ah0pd{animation-name:x35atj5-B}
                .backgroundColor-xrkmrrc{background-color:red}
                .color-x14rh7hd{color:var(--x-color)}
                html:not([dir='rtl']) .float-x1kmio9f{float:left}
                html[dir='rtl'] .float-x1kmio9f{float:right}
                .outlineColor-x184ctg8{outline-color:var(--colorTokens-xkxfyv)}
                .textShadow-x1skrh0i{text-shadow:1px 2px 3px 4px red}
                .backgroundColor-xfy810d.backgroundColor-xfy810d:where(.x-default-marker:focus *){background-color:green}
                .backgroundColor-xbrh7vm:hover{background-color:blue}
                @media (max-width: 1000px){.backgroundColor-xahc4vn.backgroundColor-xahc4vn{background-color:yellow}}
                @media (min-width: 320px){.textShadow-xtj17id.textShadow-xtj17id{text-shadow:10px 20px 30px 40px green}}
                @media (max-width: 1000px){.backgroundColor-x1t4kl4c.backgroundColor-x1t4kl4c.backgroundColor-x1t4kl4c:where(:has(.x-default-marker:focus)){background-color:purple}}
                @media (max-width: 1000px){.backgroundColor-x975j7z.backgroundColor-x975j7z.backgroundColor-x975j7z:where(.x-default-marker:active ~ *, :has(~ .x-default-marker:active)){background-color:orange}}
                }"#,
        )].join("\n")
    );
}

#[test]
fn process_all_rules_with_layers_matches_snapshot() {
    let options = process_options();
    let (_code, metadata) = transform_compound(FIXTURE, &options);

    assert_eq!(
        process_stylex_rules(
            &metadata,
            Some(&ProcessStylexRulesConfig {
                use_layers: UseLayers::Bool(true),
                enable_ltr_rtl_comments: false,
                ..Default::default()
            })
        ),
        ["".to_owned(), snapshot(
            r#"
                @layer priority1, priority2, priority3, priority4;
                @property --x-color { syntax: "*"; inherits: false;}
                @keyframes x35atj5-B{0%{box-shadow:1px 2px 3px 4px red;color:yellow;}100%{box-shadow:10px 20px 30px 40px green;color:var(--orange-theme-color);}}
                :root, .xsg933n{--blue-xpqh4lw:blue;--marginTokens-x8nt2k2:10px;--colorTokens-xkxfyv:red;}
                :root, .xbiwvf9{--small-x19twipt:2px;--medium-xypjos2:4px;--large-x1ec7iuc:8px;}
                @media (prefers-color-scheme: dark){:root, .xsg933n{--colorTokens-xkxfyv:lightblue;}}
                @media (min-width: 600px){:root, .xsg933n{--marginTokens-x8nt2k2:20px;}}
                @supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xsg933n{--colorTokens-xkxfyv:oklab(0.7 -0.3 -0.4);}}}
                .x6xqkwy.x6xqkwy, .x6xqkwy.x6xqkwy:root{--blue-xpqh4lw:lightblue;}
                .x57uvma.x57uvma, .x57uvma.x57uvma:root{--large-x1ec7iuc:20px;--medium-xypjos2:10px;--small-x19twipt:5px;}
                .--orange-theme-color-xufgesz{--orange-theme-color:red}
                @layer priority2{
                .margin-xymmreb{margin:10px 20px}
                .padding-xss17vw{padding:var(--large-x1ec7iuc)}
                }
                @layer priority3{
                .borderColor-x1bg2uv5{border-color:green}
                @media (max-width: 1000px){.borderColor-x5ugf7c.borderColor-x5ugf7c{border-color:var(--blue-xpqh4lw)}}
                @media (max-width: 500px){@media (max-width: 1000px){.borderColor-xqiy1ys.borderColor-xqiy1ys.borderColor-xqiy1ys{border-color:yellow}}}
                }
                @layer priority4{
                .animationName-x13ah0pd{animation-name:x35atj5-B}
                .backgroundColor-xrkmrrc{background-color:red}
                .color-x14rh7hd{color:var(--x-color)}
                html:not([dir='rtl']) .float-x1kmio9f{float:left}
                html[dir='rtl'] .float-x1kmio9f{float:right}
                .outlineColor-x184ctg8{outline-color:var(--colorTokens-xkxfyv)}
                .textShadow-x1skrh0i{text-shadow:1px 2px 3px 4px red}
                .backgroundColor-xfy810d.backgroundColor-xfy810d:where(.x-default-marker:focus *){background-color:green}
                .backgroundColor-xbrh7vm:hover{background-color:blue}
                @media (max-width: 1000px){.backgroundColor-xahc4vn.backgroundColor-xahc4vn{background-color:yellow}}
                @media (min-width: 320px){.textShadow-xtj17id.textShadow-xtj17id{text-shadow:10px 20px 30px 40px green}}
                @media (max-width: 1000px){.backgroundColor-x1t4kl4c.backgroundColor-x1t4kl4c.backgroundColor-x1t4kl4c:where(:has(.x-default-marker:focus)){background-color:purple}}
                @media (max-width: 1000px){.backgroundColor-x975j7z.backgroundColor-x975j7z.backgroundColor-x975j7z:where(.x-default-marker:active ~ *, :has(~ .x-default-marker:active)){background-color:orange}}
                }"#,
        )].join("\n")
    );
}

#[test]
fn process_rules_with_layers_before_matches_snapshot() {
    let options = process_options();
    let (_code, metadata) = transform_compound(FIXTURE, &options);

    assert_eq!(
        process_stylex_rules(
            &metadata,
            Some(&ProcessStylexRulesConfig {
                use_layers: UseLayers::Config(LayersConfig {
                    prefix: "".to_owned(),
                    before: vec!["reset".to_owned(), "typography".to_owned()],
                    after: vec![],
                }),
                enable_ltr_rtl_comments: false,
                ..Default::default()
            })
        ),
        ["".to_owned(), snapshot(
            r#"
                @layer reset, typography, priority1, priority2, priority3, priority4;
                @property --x-color { syntax: "*"; inherits: false;}
                @keyframes x35atj5-B{0%{box-shadow:1px 2px 3px 4px red;color:yellow;}100%{box-shadow:10px 20px 30px 40px green;color:var(--orange-theme-color);}}
                :root, .xsg933n{--blue-xpqh4lw:blue;--marginTokens-x8nt2k2:10px;--colorTokens-xkxfyv:red;}
                :root, .xbiwvf9{--small-x19twipt:2px;--medium-xypjos2:4px;--large-x1ec7iuc:8px;}
                @media (prefers-color-scheme: dark){:root, .xsg933n{--colorTokens-xkxfyv:lightblue;}}
                @media (min-width: 600px){:root, .xsg933n{--marginTokens-x8nt2k2:20px;}}
                @supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xsg933n{--colorTokens-xkxfyv:oklab(0.7 -0.3 -0.4);}}}
                .x6xqkwy.x6xqkwy, .x6xqkwy.x6xqkwy:root{--blue-xpqh4lw:lightblue;}
                .x57uvma.x57uvma, .x57uvma.x57uvma:root{--large-x1ec7iuc:20px;--medium-xypjos2:10px;--small-x19twipt:5px;}
                .--orange-theme-color-xufgesz{--orange-theme-color:red}
                @layer priority2{
                .margin-xymmreb{margin:10px 20px}
                .padding-xss17vw{padding:var(--large-x1ec7iuc)}
                }
                @layer priority3{
                .borderColor-x1bg2uv5{border-color:green}
                @media (max-width: 1000px){.borderColor-x5ugf7c.borderColor-x5ugf7c{border-color:var(--blue-xpqh4lw)}}
                @media (max-width: 500px){@media (max-width: 1000px){.borderColor-xqiy1ys.borderColor-xqiy1ys.borderColor-xqiy1ys{border-color:yellow}}}
                }
                @layer priority4{
                .animationName-x13ah0pd{animation-name:x35atj5-B}
                .backgroundColor-xrkmrrc{background-color:red}
                .color-x14rh7hd{color:var(--x-color)}
                html:not([dir='rtl']) .float-x1kmio9f{float:left}
                html[dir='rtl'] .float-x1kmio9f{float:right}
                .outlineColor-x184ctg8{outline-color:var(--colorTokens-xkxfyv)}
                .textShadow-x1skrh0i{text-shadow:1px 2px 3px 4px red}
                .backgroundColor-xfy810d.backgroundColor-xfy810d:where(.x-default-marker:focus *){background-color:green}
                .backgroundColor-xbrh7vm:hover{background-color:blue}
                @media (max-width: 1000px){.backgroundColor-xahc4vn.backgroundColor-xahc4vn{background-color:yellow}}
                @media (min-width: 320px){.textShadow-xtj17id.textShadow-xtj17id{text-shadow:10px 20px 30px 40px green}}
                @media (max-width: 1000px){.backgroundColor-x1t4kl4c.backgroundColor-x1t4kl4c.backgroundColor-x1t4kl4c:where(:has(.x-default-marker:focus)){background-color:purple}}
                @media (max-width: 1000px){.backgroundColor-x975j7z.backgroundColor-x975j7z.backgroundColor-x975j7z:where(.x-default-marker:active ~ *, :has(~ .x-default-marker:active)){background-color:orange}}
                }"#,
        )].join("\n")
    );
}

#[test]
fn process_rules_with_layers_after_matches_snapshot() {
    let options = process_options();
    let (_code, metadata) = transform_compound(FIXTURE, &options);

    assert_eq!(
        process_stylex_rules(
            &metadata,
            Some(&ProcessStylexRulesConfig {
                use_layers: UseLayers::Config(LayersConfig {
                    prefix: "".to_owned(),
                    before: vec![],
                    after: vec!["overrides".to_owned(), "xds.theme".to_owned()],
                }),
                enable_ltr_rtl_comments: false,
                ..Default::default()
            })
        ),
        ["".to_owned(), snapshot(
            r#"
                @layer priority1, priority2, priority3, priority4, overrides, xds.theme;
                @property --x-color { syntax: "*"; inherits: false;}
                @keyframes x35atj5-B{0%{box-shadow:1px 2px 3px 4px red;color:yellow;}100%{box-shadow:10px 20px 30px 40px green;color:var(--orange-theme-color);}}
                :root, .xsg933n{--blue-xpqh4lw:blue;--marginTokens-x8nt2k2:10px;--colorTokens-xkxfyv:red;}
                :root, .xbiwvf9{--small-x19twipt:2px;--medium-xypjos2:4px;--large-x1ec7iuc:8px;}
                @media (prefers-color-scheme: dark){:root, .xsg933n{--colorTokens-xkxfyv:lightblue;}}
                @media (min-width: 600px){:root, .xsg933n{--marginTokens-x8nt2k2:20px;}}
                @supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xsg933n{--colorTokens-xkxfyv:oklab(0.7 -0.3 -0.4);}}}
                .x6xqkwy.x6xqkwy, .x6xqkwy.x6xqkwy:root{--blue-xpqh4lw:lightblue;}
                .x57uvma.x57uvma, .x57uvma.x57uvma:root{--large-x1ec7iuc:20px;--medium-xypjos2:10px;--small-x19twipt:5px;}
                .--orange-theme-color-xufgesz{--orange-theme-color:red}
                @layer priority2{
                .margin-xymmreb{margin:10px 20px}
                .padding-xss17vw{padding:var(--large-x1ec7iuc)}
                }
                @layer priority3{
                .borderColor-x1bg2uv5{border-color:green}
                @media (max-width: 1000px){.borderColor-x5ugf7c.borderColor-x5ugf7c{border-color:var(--blue-xpqh4lw)}}
                @media (max-width: 500px){@media (max-width: 1000px){.borderColor-xqiy1ys.borderColor-xqiy1ys.borderColor-xqiy1ys{border-color:yellow}}}
                }
                @layer priority4{
                .animationName-x13ah0pd{animation-name:x35atj5-B}
                .backgroundColor-xrkmrrc{background-color:red}
                .color-x14rh7hd{color:var(--x-color)}
                html:not([dir='rtl']) .float-x1kmio9f{float:left}
                html[dir='rtl'] .float-x1kmio9f{float:right}
                .outlineColor-x184ctg8{outline-color:var(--colorTokens-xkxfyv)}
                .textShadow-x1skrh0i{text-shadow:1px 2px 3px 4px red}
                .backgroundColor-xfy810d.backgroundColor-xfy810d:where(.x-default-marker:focus *){background-color:green}
                .backgroundColor-xbrh7vm:hover{background-color:blue}
                @media (max-width: 1000px){.backgroundColor-xahc4vn.backgroundColor-xahc4vn{background-color:yellow}}
                @media (min-width: 320px){.textShadow-xtj17id.textShadow-xtj17id{text-shadow:10px 20px 30px 40px green}}
                @media (max-width: 1000px){.backgroundColor-x1t4kl4c.backgroundColor-x1t4kl4c.backgroundColor-x1t4kl4c:where(:has(.x-default-marker:focus)){background-color:purple}}
                @media (max-width: 1000px){.backgroundColor-x975j7z.backgroundColor-x975j7z.backgroundColor-x975j7z:where(.x-default-marker:active ~ *, :has(~ .x-default-marker:active)){background-color:orange}}
                }"#,
        )].join("\n")
    );
}

#[test]
fn process_rules_with_layers_before_and_after_matches_snapshot() {
    let options = process_options();
    let (_code, metadata) = transform_compound(FIXTURE, &options);

    assert_eq!(
        process_stylex_rules(
            &metadata,
            Some(&ProcessStylexRulesConfig {
                use_layers: UseLayers::Config(LayersConfig {
                    prefix: "".to_owned(),
                    before: vec!["reset".to_owned()],
                    after: vec!["xds.theme".to_owned()],
                }),
                enable_ltr_rtl_comments: false,
                ..Default::default()
            })
        ),
        ["".to_owned(), snapshot(
            r#"
                @layer reset, priority1, priority2, priority3, priority4, xds.theme;
                @property --x-color { syntax: "*"; inherits: false;}
                @keyframes x35atj5-B{0%{box-shadow:1px 2px 3px 4px red;color:yellow;}100%{box-shadow:10px 20px 30px 40px green;color:var(--orange-theme-color);}}
                :root, .xsg933n{--blue-xpqh4lw:blue;--marginTokens-x8nt2k2:10px;--colorTokens-xkxfyv:red;}
                :root, .xbiwvf9{--small-x19twipt:2px;--medium-xypjos2:4px;--large-x1ec7iuc:8px;}
                @media (prefers-color-scheme: dark){:root, .xsg933n{--colorTokens-xkxfyv:lightblue;}}
                @media (min-width: 600px){:root, .xsg933n{--marginTokens-x8nt2k2:20px;}}
                @supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xsg933n{--colorTokens-xkxfyv:oklab(0.7 -0.3 -0.4);}}}
                .x6xqkwy.x6xqkwy, .x6xqkwy.x6xqkwy:root{--blue-xpqh4lw:lightblue;}
                .x57uvma.x57uvma, .x57uvma.x57uvma:root{--large-x1ec7iuc:20px;--medium-xypjos2:10px;--small-x19twipt:5px;}
                .--orange-theme-color-xufgesz{--orange-theme-color:red}
                @layer priority2{
                .margin-xymmreb{margin:10px 20px}
                .padding-xss17vw{padding:var(--large-x1ec7iuc)}
                }
                @layer priority3{
                .borderColor-x1bg2uv5{border-color:green}
                @media (max-width: 1000px){.borderColor-x5ugf7c.borderColor-x5ugf7c{border-color:var(--blue-xpqh4lw)}}
                @media (max-width: 500px){@media (max-width: 1000px){.borderColor-xqiy1ys.borderColor-xqiy1ys.borderColor-xqiy1ys{border-color:yellow}}}
                }
                @layer priority4{
                .animationName-x13ah0pd{animation-name:x35atj5-B}
                .backgroundColor-xrkmrrc{background-color:red}
                .color-x14rh7hd{color:var(--x-color)}
                html:not([dir='rtl']) .float-x1kmio9f{float:left}
                html[dir='rtl'] .float-x1kmio9f{float:right}
                .outlineColor-x184ctg8{outline-color:var(--colorTokens-xkxfyv)}
                .textShadow-x1skrh0i{text-shadow:1px 2px 3px 4px red}
                .backgroundColor-xfy810d.backgroundColor-xfy810d:where(.x-default-marker:focus *){background-color:green}
                .backgroundColor-xbrh7vm:hover{background-color:blue}
                @media (max-width: 1000px){.backgroundColor-xahc4vn.backgroundColor-xahc4vn{background-color:yellow}}
                @media (min-width: 320px){.textShadow-xtj17id.textShadow-xtj17id{text-shadow:10px 20px 30px 40px green}}
                @media (max-width: 1000px){.backgroundColor-x1t4kl4c.backgroundColor-x1t4kl4c.backgroundColor-x1t4kl4c:where(:has(.x-default-marker:focus)){background-color:purple}}
                @media (max-width: 1000px){.backgroundColor-x975j7z.backgroundColor-x975j7z.backgroundColor-x975j7z:where(.x-default-marker:active ~ *, :has(~ .x-default-marker:active)){background-color:orange}}
                }"#,
        )].join("\n")
    );
}

#[test]
fn process_rules_with_layers_prefix_matches_snapshot() {
    let options = process_options();
    let (_code, metadata) = transform_compound(FIXTURE, &options);

    assert_eq!(
        process_css(
            &metadata,
            ProcessStylexRulesConfig {
                use_layers: UseLayers::Config(LayersConfig {
                    prefix: "stylex".to_owned(),
                    before: vec![],
                    after: vec![],
                }),
                enable_ltr_rtl_comments: false,
                ..Default::default()
            }
        ),
        ["".to_owned(), snapshot(
            r#"
                @layer stylex.priority1, stylex.priority2, stylex.priority3, stylex.priority4;
                @property --x-color { syntax: "*"; inherits: false;}
                @keyframes x35atj5-B{0%{box-shadow:1px 2px 3px 4px red;color:yellow;}100%{box-shadow:10px 20px 30px 40px green;color:var(--orange-theme-color);}}
                :root, .xsg933n{--blue-xpqh4lw:blue;--marginTokens-x8nt2k2:10px;--colorTokens-xkxfyv:red;}
                :root, .xbiwvf9{--small-x19twipt:2px;--medium-xypjos2:4px;--large-x1ec7iuc:8px;}
                @media (prefers-color-scheme: dark){:root, .xsg933n{--colorTokens-xkxfyv:lightblue;}}
                @media (min-width: 600px){:root, .xsg933n{--marginTokens-x8nt2k2:20px;}}
                @supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xsg933n{--colorTokens-xkxfyv:oklab(0.7 -0.3 -0.4);}}}
                .x6xqkwy.x6xqkwy, .x6xqkwy.x6xqkwy:root{--blue-xpqh4lw:lightblue;}
                .x57uvma.x57uvma, .x57uvma.x57uvma:root{--large-x1ec7iuc:20px;--medium-xypjos2:10px;--small-x19twipt:5px;}
                .--orange-theme-color-xufgesz{--orange-theme-color:red}
                @layer stylex.priority2{
                .margin-xymmreb{margin:10px 20px}
                .padding-xss17vw{padding:var(--large-x1ec7iuc)}
                }
                @layer stylex.priority3{
                .borderColor-x1bg2uv5{border-color:green}
                @media (max-width: 1000px){.borderColor-x5ugf7c.borderColor-x5ugf7c{border-color:var(--blue-xpqh4lw)}}
                @media (max-width: 500px){@media (max-width: 1000px){.borderColor-xqiy1ys.borderColor-xqiy1ys.borderColor-xqiy1ys{border-color:yellow}}}
                }
                @layer stylex.priority4{
                .animationName-x13ah0pd{animation-name:x35atj5-B}
                .backgroundColor-xrkmrrc{background-color:red}
                .color-x14rh7hd{color:var(--x-color)}
                html:not([dir='rtl']) .float-x1kmio9f{float:left}
                html[dir='rtl'] .float-x1kmio9f{float:right}
                .outlineColor-x184ctg8{outline-color:var(--colorTokens-xkxfyv)}
                .textShadow-x1skrh0i{text-shadow:1px 2px 3px 4px red}
                .backgroundColor-xfy810d.backgroundColor-xfy810d:where(.x-default-marker:focus *){background-color:green}
                .backgroundColor-xbrh7vm:hover{background-color:blue}
                @media (max-width: 1000px){.backgroundColor-xahc4vn.backgroundColor-xahc4vn{background-color:yellow}}
                @media (min-width: 320px){.textShadow-xtj17id.textShadow-xtj17id{text-shadow:10px 20px 30px 40px green}}
                @media (max-width: 1000px){.backgroundColor-x1t4kl4c.backgroundColor-x1t4kl4c.backgroundColor-x1t4kl4c:where(:has(.x-default-marker:focus)){background-color:purple}}
                @media (max-width: 1000px){.backgroundColor-x975j7z.backgroundColor-x975j7z.backgroundColor-x975j7z:where(.x-default-marker:active ~ *, :has(~ .x-default-marker:active)){background-color:orange}}
                }"#,
        )].join("\n")
    );
}

#[test]
fn process_rules_with_empty_layers_before_and_after_matches_snapshot() {
    let options = process_options();
    let (_code, metadata) = transform_compound(FIXTURE, &options);

    assert_eq!(
        process_css(
            &metadata,
            ProcessStylexRulesConfig {
                use_layers: UseLayers::Config(LayersConfig::default()),
                enable_ltr_rtl_comments: false,
                ..Default::default()
            }
        ),
        ["".to_owned(), snapshot(
            r#"
                @layer priority1, priority2, priority3, priority4;
                @property --x-color { syntax: "*"; inherits: false;}
                @keyframes x35atj5-B{0%{box-shadow:1px 2px 3px 4px red;color:yellow;}100%{box-shadow:10px 20px 30px 40px green;color:var(--orange-theme-color);}}
                :root, .xsg933n{--blue-xpqh4lw:blue;--marginTokens-x8nt2k2:10px;--colorTokens-xkxfyv:red;}
                :root, .xbiwvf9{--small-x19twipt:2px;--medium-xypjos2:4px;--large-x1ec7iuc:8px;}
                @media (prefers-color-scheme: dark){:root, .xsg933n{--colorTokens-xkxfyv:lightblue;}}
                @media (min-width: 600px){:root, .xsg933n{--marginTokens-x8nt2k2:20px;}}
                @supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xsg933n{--colorTokens-xkxfyv:oklab(0.7 -0.3 -0.4);}}}
                .x6xqkwy.x6xqkwy, .x6xqkwy.x6xqkwy:root{--blue-xpqh4lw:lightblue;}
                .x57uvma.x57uvma, .x57uvma.x57uvma:root{--large-x1ec7iuc:20px;--medium-xypjos2:10px;--small-x19twipt:5px;}
                .--orange-theme-color-xufgesz{--orange-theme-color:red}
                @layer priority2{
                .margin-xymmreb{margin:10px 20px}
                .padding-xss17vw{padding:var(--large-x1ec7iuc)}
                }
                @layer priority3{
                .borderColor-x1bg2uv5{border-color:green}
                @media (max-width: 1000px){.borderColor-x5ugf7c.borderColor-x5ugf7c{border-color:var(--blue-xpqh4lw)}}
                @media (max-width: 500px){@media (max-width: 1000px){.borderColor-xqiy1ys.borderColor-xqiy1ys.borderColor-xqiy1ys{border-color:yellow}}}
                }
                @layer priority4{
                .animationName-x13ah0pd{animation-name:x35atj5-B}
                .backgroundColor-xrkmrrc{background-color:red}
                .color-x14rh7hd{color:var(--x-color)}
                html:not([dir='rtl']) .float-x1kmio9f{float:left}
                html[dir='rtl'] .float-x1kmio9f{float:right}
                .outlineColor-x184ctg8{outline-color:var(--colorTokens-xkxfyv)}
                .textShadow-x1skrh0i{text-shadow:1px 2px 3px 4px red}
                .backgroundColor-xfy810d.backgroundColor-xfy810d:where(.x-default-marker:focus *){background-color:green}
                .backgroundColor-xbrh7vm:hover{background-color:blue}
                @media (max-width: 1000px){.backgroundColor-xahc4vn.backgroundColor-xahc4vn{background-color:yellow}}
                @media (min-width: 320px){.textShadow-xtj17id.textShadow-xtj17id{text-shadow:10px 20px 30px 40px green}}
                @media (max-width: 1000px){.backgroundColor-x1t4kl4c.backgroundColor-x1t4kl4c.backgroundColor-x1t4kl4c:where(:has(.x-default-marker:focus)){background-color:purple}}
                @media (max-width: 1000px){.backgroundColor-x975j7z.backgroundColor-x975j7z.backgroundColor-x975j7z:where(.x-default-marker:active ~ *, :has(~ .x-default-marker:active)){background-color:orange}}
                }"#,
        )].join("\n")
    );
}

#[test]
fn process_all_rules_with_legacy_disable_layers_matches_snapshot() {
    let options = process_options();
    let (_code, metadata) = transform_compound(FIXTURE, &options);

    assert_eq!(
        process_css(
            &metadata,
            ProcessStylexRulesConfig {
                use_layers: UseLayers::Bool(false),
                enable_ltr_rtl_comments: false,
                legacy_disable_layers: true,
                ..Default::default()
            }
        ),
        snapshot(
            r#"
                @property --x-color { syntax: "*"; inherits: false;}
                @keyframes x35atj5-B{0%{box-shadow:1px 2px 3px 4px red;color:yellow;}100%{box-shadow:10px 20px 30px 40px green;color:var(--orange-theme-color);}}
                :root, .xsg933n{--blue-xpqh4lw:blue;--marginTokens-x8nt2k2:10px;--colorTokens-xkxfyv:red;}
                :root, .xbiwvf9{--small-x19twipt:2px;--medium-xypjos2:4px;--large-x1ec7iuc:8px;}
                @media (prefers-color-scheme: dark){:root, .xsg933n{--colorTokens-xkxfyv:lightblue;}}
                @media (min-width: 600px){:root, .xsg933n{--marginTokens-x8nt2k2:20px;}}
                @supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xsg933n{--colorTokens-xkxfyv:oklab(0.7 -0.3 -0.4);}}}
                .x6xqkwy.x6xqkwy, .x6xqkwy.x6xqkwy:root{--blue-xpqh4lw:lightblue;}
                .x57uvma.x57uvma, .x57uvma.x57uvma:root{--large-x1ec7iuc:20px;--medium-xypjos2:10px;--small-x19twipt:5px;}
                .--orange-theme-color-xufgesz{--orange-theme-color:red}
                .margin-xymmreb{margin:10px 20px}
                .padding-xss17vw{padding:var(--large-x1ec7iuc)}
                .borderColor-x1bg2uv5{border-color:green}
                @media (max-width: 1000px){.borderColor-x5ugf7c.borderColor-x5ugf7c{border-color:var(--blue-xpqh4lw)}}
                @media (max-width: 500px){@media (max-width: 1000px){.borderColor-xqiy1ys.borderColor-xqiy1ys.borderColor-xqiy1ys{border-color:yellow}}}
                .animationName-x13ah0pd{animation-name:x35atj5-B}
                .backgroundColor-xrkmrrc{background-color:red}
                .color-x14rh7hd{color:var(--x-color)}
                html:not([dir='rtl']) .float-x1kmio9f{float:left}
                html[dir='rtl'] .float-x1kmio9f{float:right}
                .outlineColor-x184ctg8{outline-color:var(--colorTokens-xkxfyv)}
                .textShadow-x1skrh0i{text-shadow:1px 2px 3px 4px red}
                .backgroundColor-xfy810d.backgroundColor-xfy810d:where(.x-default-marker:focus *){background-color:green}
                .backgroundColor-xbrh7vm:hover{background-color:blue}
                @media (max-width: 1000px){.backgroundColor-xahc4vn.backgroundColor-xahc4vn{background-color:yellow}}
                @media (min-width: 320px){.textShadow-xtj17id.textShadow-xtj17id{text-shadow:10px 20px 30px 40px green}}
                @media (max-width: 1000px){.backgroundColor-x1t4kl4c.backgroundColor-x1t4kl4c.backgroundColor-x1t4kl4c:where(:has(.x-default-marker:focus)){background-color:purple}}
                @media (max-width: 1000px){.backgroundColor-x975j7z.backgroundColor-x975j7z.backgroundColor-x975j7z:where(.x-default-marker:active ~ *, :has(~ .x-default-marker:active)){background-color:orange}}"#,
        )
    );
}

#[test]
fn process_rules_does_not_mutate_input() {
    let options = process_options();
    let (_code, metadata) = transform_compound(FIXTURE, &options);
    let frozen = metadata.clone();

    let _ = process_stylex_rules(&metadata, None);

    assert_eq!(metadata, frozen);
}

#[test]
fn process_rules_use_legacy_classnames_sort_false_matches_snapshot() {
    let mut options = process_options();
    options.additional_options.insert(
        "enableDebugClassNames".to_owned(),
        serde_json::Value::Bool(false),
    );
    let (_code, metadata) = transform_compound(FIXTURE, &options);

    assert_eq!(
        process_css(
            &metadata,
            ProcessStylexRulesConfig {
                use_layers: UseLayers::Bool(false),
                enable_ltr_rtl_comments: true,
                use_legacy_classnames_sort: false,
                legacy_disable_layers: true,
                ..Default::default()
            }
        ),
        snapshot(
            r#"
                @property --x-color { syntax: "*"; inherits: false;}
                @keyframes x35atj5-B{0%{box-shadow:1px 2px 3px 4px red;color:yellow;}100%{box-shadow:10px 20px 30px 40px green;color:var(--orange-theme-color);}}
                :root, .xbiwvf9{--x19twipt:2px;--xypjos2:4px;--x1ec7iuc:8px;}
                :root, .xsg933n{--xpqh4lw:blue;--x8nt2k2:10px;--xkxfyv:red;}
                @media (min-width: 600px){:root, .xsg933n{--x8nt2k2:20px;}}
                @media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:lightblue;}}
                @supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:oklab(0.7 -0.3 -0.4);}}}
                .x4hn0rr.x4hn0rr, .x4hn0rr.x4hn0rr:root{--x1ec7iuc:20px;--xypjos2:10px;--x19twipt:5px;}
                .x1coplze.x1coplze, .x1coplze.x1coplze:root{--xpqh4lw:lightblue;}
                .xufgesz{--orange-theme-color:red}
                .xymmreb{margin:10px 20px}
                .x1s2izit{padding:var(--x1ec7iuc)}
                .x1bg2uv5{border-color:green}
                @media (max-width: 1000px){.xio2edn.xio2edn{border-color:var(--xpqh4lw)}}
                @media (max-width: 500px){@media (max-width: 1000px){.xqiy1ys.xqiy1ys.xqiy1ys{border-color:yellow}}}
                .x13ah0pd{animation-name:x35atj5-B}
                .xrkmrrc{background-color:red}
                .x14rh7hd{color:var(--x-color)}
                /* @ltr begin */.x1kmio9f{float:left}/* @ltr end */
                /* @rtl begin */.x1kmio9f{float:right}/* @rtl end */
                .x18abd1y{outline-color:var(--xkxfyv)}
                .x1skrh0i{text-shadow:1px 2px 3px 4px red}
                .xfy810d.xfy810d:where(.x-default-marker:focus *){background-color:green}
                .xbrh7vm:hover{background-color:blue}
                @media (max-width: 1000px){.xahc4vn.xahc4vn{background-color:yellow}}
                @media (min-width: 320px){.xtj17id.xtj17id{text-shadow:10px 20px 30px 40px green}}
                @media (max-width: 1000px){.x1t4kl4c.x1t4kl4c.x1t4kl4c:where(:has(.x-default-marker:focus)){background-color:purple}}
                @media (max-width: 1000px){.x975j7z.x975j7z.x975j7z:where(.x-default-marker:active ~ *, :has(~ .x-default-marker:active)){background-color:orange}}"#,
        )
    );
}

#[test]
fn process_rules_use_legacy_classnames_sort_true_matches_snapshot() {
    let mut options = process_options();
    options.additional_options.insert(
        "enableDebugClassNames".to_owned(),
        serde_json::Value::Bool(false),
    );
    let (_code, metadata) = transform_compound(FIXTURE, &options);

    assert_eq!(
        process_css(
            &metadata,
            ProcessStylexRulesConfig {
                use_layers: UseLayers::Bool(false),
                enable_ltr_rtl_comments: true,
                use_legacy_classnames_sort: true,
                legacy_disable_layers: true,
                ..Default::default()
            }
        ),
        snapshot(
            r#"
                @property --x-color { syntax: "*"; inherits: false;}
                @keyframes x35atj5-B{0%{box-shadow:1px 2px 3px 4px red;color:yellow;}100%{box-shadow:10px 20px 30px 40px green;color:var(--orange-theme-color);}}
                :root, .xbiwvf9{--x19twipt:2px;--xypjos2:4px;--x1ec7iuc:8px;}
                :root, .xsg933n{--xpqh4lw:blue;--x8nt2k2:10px;--xkxfyv:red;}
                @media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:lightblue;}}
                @media (min-width: 600px){:root, .xsg933n{--x8nt2k2:20px;}}
                @supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xsg933n{--xkxfyv:oklab(0.7 -0.3 -0.4);}}}
                .x1coplze.x1coplze, .x1coplze.x1coplze:root{--xpqh4lw:lightblue;}
                .x4hn0rr.x4hn0rr, .x4hn0rr.x4hn0rr:root{--x1ec7iuc:20px;--xypjos2:10px;--x19twipt:5px;}
                .xufgesz{--orange-theme-color:red}
                .x1s2izit{padding:var(--x1ec7iuc)}
                .xymmreb{margin:10px 20px}
                .x1bg2uv5{border-color:green}
                @media (max-width: 1000px){.xio2edn.xio2edn{border-color:var(--xpqh4lw)}}
                @media (max-width: 500px){@media (max-width: 1000px){.xqiy1ys.xqiy1ys.xqiy1ys{border-color:yellow}}}
                .x13ah0pd{animation-name:x35atj5-B}
                .x14rh7hd{color:var(--x-color)}
                .x18abd1y{outline-color:var(--xkxfyv)}
                /* @ltr begin */.x1kmio9f{float:left}/* @ltr end */
                /* @rtl begin */.x1kmio9f{float:right}/* @rtl end */
                .x1skrh0i{text-shadow:1px 2px 3px 4px red}
                .xrkmrrc{background-color:red}
                .xfy810d.xfy810d:where(.x-default-marker:focus *){background-color:green}
                .xbrh7vm:hover{background-color:blue}
                @media (max-width: 1000px){.xahc4vn.xahc4vn{background-color:yellow}}
                @media (min-width: 320px){.xtj17id.xtj17id{text-shadow:10px 20px 30px 40px green}}
                @media (max-width: 1000px){.x1t4kl4c.x1t4kl4c.x1t4kl4c:where(:has(.x-default-marker:focus)){background-color:purple}}
                @media (max-width: 1000px){.x975j7z.x975j7z.x975j7z:where(.x-default-marker:active ~ *, :has(~ .x-default-marker:active)){background-color:orange}}"#,
        )
    );
}

#[test]
fn process_rules_sort_is_deterministic_regardless_of_input_order() {
    let rules = vec![
        rule(
            "xMedia1",
            "@media (min-width: 48rem){.xMedia1{display:none}}",
            6000.0,
        ),
        rule(
            "xContainer1",
            "@container card (min-width: 31.25rem){.xContainer1{display:flex}}",
            6000.0,
        ),
        rule("xStarting1", "@starting-style{.xStarting1{opacity:0}}", 6000.0),
        rule(
            "xVar1",
            "var(--x10fi87w){.xVar1.xVar1{grid-template-columns:repeat(2,1fr)}}",
            6000.0,
        ),
        rule("xPseudo1", ".xPseudo1::before{inset:0}", 6000.0),
        rule(
            "xMedia2",
            "@media (min-width: 64rem){.xMedia2{inset:0}}",
            6000.0,
        ),
        rule("xPlain1", ".xPlain1{display:none}", 6000.0),
    ];

    let output1 = process_css(
        &rules,
        ProcessStylexRulesConfig {
            use_layers: UseLayers::Bool(false),
            legacy_disable_layers: true,
            ..Default::default()
        },
    );

    let mut reversed = rules.clone();
    reversed.reverse();
    let output2 = process_css(
        &reversed,
        ProcessStylexRulesConfig {
            use_layers: UseLayers::Bool(false),
            legacy_disable_layers: true,
            ..Default::default()
        },
    );

    let shuffled = vec![
        rules[4].clone(),
        rules[0].clone(),
        rules[3].clone(),
        rules[6].clone(),
        rules[2].clone(),
        rules[5].clone(),
        rules[1].clone(),
    ];
    let output3 = process_css(
        &shuffled,
        ProcessStylexRulesConfig {
            use_layers: UseLayers::Bool(false),
            legacy_disable_layers: true,
            ..Default::default()
        },
    );

    let expected = snapshot(
        r#"
            @container card (min-width: 31.25rem){.xContainer1{display:flex}}
            .xPlain1{display:none}
            @media (min-width: 48rem){.xMedia1{display:none}}
            var(--x10fi87w){.xVar1.xVar1{grid-template-columns:repeat(2,1fr)}}
            .xPseudo1::before{inset:0}
            @media (min-width: 64rem){.xMedia2{inset:0}}
            @starting-style{.xStarting1{opacity:0}}"#,
    );

    assert_eq!(output1, expected);
    assert_eq!(output2, expected);
    assert_eq!(output3, expected);
}

#[test]
fn process_rules_sort_is_deterministic_with_duplicate_rules() {
    let rule_a = rule(
        "xA",
        "@media (min-width: 48rem){.xA{display:flex}}",
        6000.0,
    );
    let rule_b = rule("xB", ".xB::after{inset:0}", 6000.0);
    let rule_c = rule("xC", "@starting-style{.xC{opacity:0}}", 6000.0);

    let output1 = process_css(
        &[rule_a.clone(), rule_b.clone(), rule_b.clone(), rule_c.clone()],
        ProcessStylexRulesConfig {
            use_layers: UseLayers::Bool(false),
            legacy_disable_layers: true,
            ..Default::default()
        },
    );
    let output2 = process_css(
        &[rule_c.clone(), rule_b.clone(), rule_a.clone(), rule_b.clone()],
        ProcessStylexRulesConfig {
            use_layers: UseLayers::Bool(false),
            legacy_disable_layers: true,
            ..Default::default()
        },
    );
    let output3 = process_css(
        &[rule_b.clone(), rule_c.clone(), rule_b.clone(), rule_a.clone()],
        ProcessStylexRulesConfig {
            use_layers: UseLayers::Bool(false),
            legacy_disable_layers: true,
            ..Default::default()
        },
    );

    let expected = snapshot(
        r#"
            @media (min-width: 48rem){.xA{display:flex}}
            .xB::after{inset:0}
            @starting-style{.xC{opacity:0}}"#,
    );

    assert_eq!(output1, expected);
    assert_eq!(output2, expected);
    assert_eq!(output3, expected);
}

#[test]
fn process_rules_with_layers_prefix_before_and_after_matches_snapshot() {
    let options = process_options();
    let (_code, metadata) = transform_compound(FIXTURE, &options);

    assert_eq!(
        process_css(
            &metadata,
            ProcessStylexRulesConfig {
                use_layers: UseLayers::Config(LayersConfig {
                    prefix: "stylex".to_owned(),
                    before: vec!["reset".to_owned(), "typography".to_owned()],
                    after: vec!["xds.theme".to_owned()],
                }),
                enable_ltr_rtl_comments: false,
                ..Default::default()
            }
        ),
        ["".to_owned(), snapshot(
            r#"
                @layer reset, typography, stylex.priority1, stylex.priority2, stylex.priority3, stylex.priority4, xds.theme;
                @property --x-color { syntax: "*"; inherits: false;}
                @keyframes x35atj5-B{0%{box-shadow:1px 2px 3px 4px red;color:yellow;}100%{box-shadow:10px 20px 30px 40px green;color:var(--orange-theme-color);}}
                :root, .xsg933n{--blue-xpqh4lw:blue;--marginTokens-x8nt2k2:10px;--colorTokens-xkxfyv:red;}
                :root, .xbiwvf9{--small-x19twipt:2px;--medium-xypjos2:4px;--large-x1ec7iuc:8px;}
                @media (prefers-color-scheme: dark){:root, .xsg933n{--colorTokens-xkxfyv:lightblue;}}
                @media (min-width: 600px){:root, .xsg933n{--marginTokens-x8nt2k2:20px;}}
                @supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xsg933n{--colorTokens-xkxfyv:oklab(0.7 -0.3 -0.4);}}}
                .x6xqkwy.x6xqkwy, .x6xqkwy.x6xqkwy:root{--blue-xpqh4lw:lightblue;}
                .x57uvma.x57uvma, .x57uvma.x57uvma:root{--large-x1ec7iuc:20px;--medium-xypjos2:10px;--small-x19twipt:5px;}
                .--orange-theme-color-xufgesz{--orange-theme-color:red}
                @layer stylex.priority2{
                .margin-xymmreb{margin:10px 20px}
                .padding-xss17vw{padding:var(--large-x1ec7iuc)}
                }
                @layer stylex.priority3{
                .borderColor-x1bg2uv5{border-color:green}
                @media (max-width: 1000px){.borderColor-x5ugf7c.borderColor-x5ugf7c{border-color:var(--blue-xpqh4lw)}}
                @media (max-width: 500px){@media (max-width: 1000px){.borderColor-xqiy1ys.borderColor-xqiy1ys.borderColor-xqiy1ys{border-color:yellow}}}
                }
                @layer stylex.priority4{
                .animationName-x13ah0pd{animation-name:x35atj5-B}
                .backgroundColor-xrkmrrc{background-color:red}
                .color-x14rh7hd{color:var(--x-color)}
                html:not([dir='rtl']) .float-x1kmio9f{float:left}
                html[dir='rtl'] .float-x1kmio9f{float:right}
                .outlineColor-x184ctg8{outline-color:var(--colorTokens-xkxfyv)}
                .textShadow-x1skrh0i{text-shadow:1px 2px 3px 4px red}
                .backgroundColor-xfy810d.backgroundColor-xfy810d:where(.x-default-marker:focus *){background-color:green}
                .backgroundColor-xbrh7vm:hover{background-color:blue}
                @media (max-width: 1000px){.backgroundColor-xahc4vn.backgroundColor-xahc4vn{background-color:yellow}}
                @media (min-width: 320px){.textShadow-xtj17id.textShadow-xtj17id{text-shadow:10px 20px 30px 40px green}}
                @media (max-width: 1000px){.backgroundColor-x1t4kl4c.backgroundColor-x1t4kl4c.backgroundColor-x1t4kl4c:where(:has(.x-default-marker:focus)){background-color:purple}}
                @media (max-width: 1000px){.backgroundColor-x975j7z.backgroundColor-x975j7z.backgroundColor-x975j7z:where(.x-default-marker:active ~ *, :has(~ .x-default-marker:active)){background-color:orange}}
                }"#,
        )].join("\n")
    );
}

#[test]
fn process_rules_legacy_expand_shorthands_with_logical_styles_polyfill_matches_snapshot() {
    let mut options = process_options();
    options.additional_options.insert(
        "styleResolution".to_owned(),
        serde_json::Value::String("legacy-expand-shorthands".to_owned()),
    );
    options.additional_options.insert(
        "enableLogicalStylesPolyfill".to_owned(),
        serde_json::Value::Bool(true),
    );
    let (code, metadata) = transform_compound(
        r#"
            import * as stylex from '@stylexjs/stylex';
            export const styles = stylex.create({
              container: {
                margin: '10px 20px',
                padding: '5px 15px',
                float: 'inline-start'
              }
            });
        "#,
        &options,
    );

    assert_code_matches_snapshot(
        &code,
        &snapshot(
            r#"
                import * as stylex from '@stylexjs/stylex';
                export const constants = {
                  YELLOW: "yellow",
                  ORANGE: "var(--orange-theme-color)",
                  mediaBig: "@media (max-width: 1000px)",
                  mediaSmall: "@media (max-width: 500px)",
                };
                export const vars = {
                  blue: "var(--blue-xpqh4lw)",
                  marginTokens: "var(--marginTokens-x8nt2k2)",
                  colorTokens: "var(--colorTokens-xkxfyv)",
                  __varGroupHash__: "xsg933n",
                };
                export const spacing = {
                  small: "var(--small-x19twipt)",
                  medium: "var(--medium-xypjos2)",
                  large: "var(--large-x1ec7iuc)",
                  __varGroupHash__: "xbiwvf9",
                };
                export const styles = {
                  container: {
                    "marginTop-keoZOQ": "marginTop-x1anpbxc",
                    "marginInlineEnd-k71WvV": "marginInlineEnd-x3aesyq",
                    "marginBottom-k1K539": "marginBottom-xyorhqc",
                    "marginInlineStart-keTefX": "marginInlineStart-xqsn43r",
                    "paddingTop-kLKAdn": "paddingTop-x123j3cw",
                    "paddingInlineEnd-kwRFfy": "paddingInlineEnd-x1q3ajuy",
                    "paddingBottom-kGO01o": "paddingBottom-xs9asl8",
                    "paddingInlineStart-kZCmMZ": "paddingInlineStart-x1gx403c",
                    "float-kyUFMd": "float-xj87blo",
                    $$css: "src/app/main.js:26",
                  },
                };
            "#,
        ),
        "/src/app/main.js",
    );

    assert_eq!(
        process_css(
            &metadata,
            ProcessStylexRulesConfig {
                use_layers: UseLayers::Bool(false),
                enable_ltr_rtl_comments: true,
                ..Default::default()
            }
        ),
        snapshot(
            r#"
                :root, [dir="ltr"] {
                  --stylex-logical-start: left;
                  --stylex-logical-end: right;
                }
                [dir="rtl"] {
                  --stylex-logical-start: right;
                  --stylex-logical-end: left;
                }
                :root, .xsg933n{--blue-xpqh4lw:blue;--marginTokens-x8nt2k2:10px;--colorTokens-xkxfyv:red;}
                :root, .xbiwvf9{--small-x19twipt:2px;--medium-xypjos2:4px;--large-x1ec7iuc:8px;}
                @media (prefers-color-scheme: dark){:root, .xsg933n{--colorTokens-xkxfyv:lightblue;}}
                @media (min-width: 600px){:root, .xsg933n{--marginTokens-x8nt2k2:20px;}}
                @supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xsg933n{--colorTokens-xkxfyv:oklab(0.7 -0.3 -0.4);}}}
                .float-xj87blo:not(#\#){float:var(--stylex-logical-start)}
                /* @ltr begin */.marginInlineStart-xqsn43r:not(#\#){margin-left:20px}/* @ltr end */
                /* @rtl begin */.marginInlineStart-xqsn43r:not(#\#){margin-right:20px}/* @rtl end */
                /* @ltr begin */.marginInlineEnd-x3aesyq:not(#\#){margin-right:20px}/* @ltr end */
                /* @rtl begin */.marginInlineEnd-x3aesyq:not(#\#){margin-left:20px}/* @rtl end */
                /* @ltr begin */.paddingInlineStart-x1gx403c:not(#\#){padding-left:15px}/* @ltr end */
                /* @rtl begin */.paddingInlineStart-x1gx403c:not(#\#){padding-right:15px}/* @rtl end */
                /* @ltr begin */.paddingInlineEnd-x1q3ajuy:not(#\#){padding-right:15px}/* @ltr end */
                /* @rtl begin */.paddingInlineEnd-x1q3ajuy:not(#\#){padding-left:15px}/* @rtl end */
                .marginBottom-xyorhqc:not(#\#):not(#\#){margin-bottom:10px}
                .marginTop-x1anpbxc:not(#\#):not(#\#){margin-top:10px}
                .paddingBottom-xs9asl8:not(#\#):not(#\#){padding-bottom:5px}
                .paddingTop-x123j3cw:not(#\#):not(#\#){padding-top:5px}"#,
        )
    );
}

#[test]
fn process_rules_legacy_expand_shorthands_duplicate_theme_selectors_matches_snapshot() {
    let mut options = process_options();
    options.additional_options.insert(
        "styleResolution".to_owned(),
        serde_json::Value::String("legacy-expand-shorthands".to_owned()),
    );
    let (_code, metadata) = transform_compound(
        r#"
            import * as stylex from '@stylexjs/stylex';
            export const themeColor = stylex.createTheme(vars, {
              blue: 'lightblue'
            });
            export const themeSpacing = stylex.createTheme(spacing, {
              small: '5px',
              medium: '10px',
              large: '20px'
            });
        "#,
        &options,
    );

    assert_eq!(
        process_css(
            &metadata,
            ProcessStylexRulesConfig {
                use_layers: UseLayers::Bool(false),
                enable_ltr_rtl_comments: false,
                ..Default::default()
            }
        ),
        snapshot(
            r#"
                :root, .xsg933n{--blue-xpqh4lw:blue;--marginTokens-x8nt2k2:10px;--colorTokens-xkxfyv:red;}
                :root, .xbiwvf9{--small-x19twipt:2px;--medium-xypjos2:4px;--large-x1ec7iuc:8px;}
                @media (prefers-color-scheme: dark){:root, .xsg933n{--colorTokens-xkxfyv:lightblue;}}
                @media (min-width: 600px){:root, .xsg933n{--marginTokens-x8nt2k2:20px;}}
                @supports (color: oklab(0 0 0)){@media (prefers-color-scheme: dark){:root, .xsg933n{--colorTokens-xkxfyv:oklab(0.7 -0.3 -0.4);}}}
                .x6xqkwy.x6xqkwy, .x6xqkwy.x6xqkwy:root{--blue-xpqh4lw:lightblue;}
                .x57uvma.x57uvma, .x57uvma.x57uvma:root{--large-x1ec7iuc:20px;--medium-xypjos2:10px;--small-x19twipt:5px;}"#,
        )
    );
}
