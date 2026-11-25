//! `defineVars` with stored functions, as current Babel compiles it.
use super::*;

const FILE: &str = "/tmp/creation-current/stored-functions.stylex.js";

fn settings() -> Value {
    json!({"unstable_moduleResolution":{"type":"haste"}})
}

#[test]
fn stored_callbacks_use_factory_captures_instead_of_replayed_arrow_source() {
    for source in [
        "import * as stylex from '@stylexjs/stylex';const make=color=>({base:'blue',accent:()=>color});export const tokens=stylex.defineVars(make('red'));",
        "import * as stylex from '@stylexjs/stylex';const make=color=>({accent:()=>color});const config=make('red');export const tokens=stylex.defineVars(config);",
        "import * as stylex from '@stylexjs/stylex';const make=value=>({color:()=>stylex.types.color(value)});export const tokens=stylex.defineVars(make('red'));",
    ] {
        differential(source, FILE, settings());
    }
}

#[test]
fn recursive_zero_argument_functions_preserve_conditional_and_css_type_rules() {
    for source in [
        "import * as stylex from '@stylexjs/stylex';const make=color=>({accent:{default:()=>color,'@media (prefers-color-scheme: dark)':()=> 'blue'}});export const tokens=stylex.defineVars(make('red'));",
        "import * as stylex from '@stylexjs/stylex';export const tokens=stylex.defineVars({accent:()=>stylex.types.color('red')});",
    ] {
        differential(source, FILE, settings());
    }
}

#[test]
fn only_executed_self_reads_are_dependencies_and_cached_reads_are_observed() {
    for source in [
        "import * as stylex from '@stylexjs/stylex';export const tokens=stylex.defineVars({base:'blue',accent:()=>false?tokens.missing:tokens.base});",
        "import * as stylex from '@stylexjs/stylex';export const tokens=stylex.defineVars({base:'blue',first:()=>tokens.base,second:()=>tokens.base});",
        "import * as stylex from '@stylexjs/stylex';export const tokens=stylex.defineVars({base:'blue',group:()=>tokens.__varGroupHash__});",
        "import * as stylex from '@stylexjs/stylex';export const tokens=stylex.defineVars({base:'blue',accent:()=>tokens.__IS_PROXY?'red':'blue'});",
    ] {
        differential(source, FILE, settings());
    }
}

#[test]
fn self_proxy_does_not_make_first_argument_reads_escape_tdz() {
    let source = "import * as stylex from '@stylexjs/stylex';export const tokens=stylex.defineVars({accent:tokens.base});";
    let options = settings();
    let native = with_callback(Rc::new(fixture_host), || {
        utils::transform_source(source, FILE, &super::options(options.clone()))
    })
    .unwrap_err();
    let actual = utils::diagnostics::from_error(&native);
    let expected = oracle(&[json!({"source":source,"filename":FILE,"options":options})]).remove(0);
    assert_eq!(actual.name, expected["name"]);
    assert_eq!(actual.message, expected["frame"]["message"]);
    let range = actual
        .range
        .expect("the original call owns the defineVars deopt");
    assert_eq!(
        json!([range.start, range.end]),
        json!([expected["frame"]["start"], expected["frame"]["end"]])
    );
}
