use serde_json::Value;
use stylex_swc_plugin::{
    process_stylex_rules, LayersConfig, ProcessStylexRulesConfig, RuleEntry, RuleFields,
    UseLayers,
};

mod test_utils;

#[test]
fn resolves_constants_preserves_first_class_order_and_wraps_direction_aware_rules() {
    let output = process_stylex_rules(
        &[
            RuleEntry(
                "constColor".to_owned(),
                RuleFields {
                    ltr: String::new(),
                    rtl: None,
                    const_key: Some("constColor".to_owned()),
                    const_val: Some(Value::String("var(--brand-color)".to_owned())),
                },
                0.0,
            ),
            RuleEntry(
                "brand-color".to_owned(),
                RuleFields {
                    ltr: String::new(),
                    rtl: None,
                    const_key: Some("brand-color".to_owned()),
                    const_val: Some(Value::String("#123456".to_owned())),
                },
                0.0,
            ),
            RuleEntry(
                "alpha".to_owned(),
                RuleFields {
                    ltr: ".alpha{color:var(--constColor)}".to_owned(),
                    rtl: Some("@media print{.alpha{color:var(--constColor)}}".to_owned()),
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
            RuleEntry(
                "alpha".to_owned(),
                RuleFields {
                    ltr: ".alpha{color:red}".to_owned(),
                    rtl: Some("@media print{.alpha{color:red}}".to_owned()),
                    const_key: None,
                    const_val: None,
                },
                3000.0,
            ),
        ],
        Some(&ProcessStylexRulesConfig {
            use_layers: UseLayers::Config(LayersConfig {
                before: vec!["reset".to_owned()],
                after: vec!["utilities".to_owned()],
                prefix: "stylex".to_owned(),
            }),
            enable_ltr_rtl_comments: true,
            ..Default::default()
        }),
    );

    assert_eq!(
        output,
        "\n@layer reset, stylex.priority1, utilities;\n@layer stylex.priority1{\n/* @ltr begin */.alpha{color:#123456}/* @ltr end */\n/* @rtl begin */@media print{.alpha{color:#123456}}/* @rtl end */\n}"
    );
}

#[test]
fn injects_logical_float_variables_before_processed_rules() {
    let output = process_stylex_rules(
        &[RuleEntry(
            "logicalFloat".to_owned(),
            RuleFields {
                ltr: ".logicalFloat{float:var(--stylex-logical-start)}".to_owned(),
                rtl: None,
                const_key: None,
                const_val: None,
            },
            3000.0,
        )],
        None,
    );

    assert_eq!(
        output,
        ":root, [dir=\"ltr\"] {\n  --stylex-logical-start: left;\n  --stylex-logical-end: right;\n}\n[dir=\"rtl\"] {\n  --stylex-logical-start: right;\n  --stylex-logical-end: left;\n}\n.logicalFloat{float:var(--stylex-logical-start)}"
    );
}
