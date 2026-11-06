/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

// The public Rust entry point must not overflow small threads on deep input.

use stylex_swc_plugin::{transform_source, StyleXTransformOptions};

fn run_on_small_thread(prefix: String, stack: usize) -> Result<usize, String> {
    std::thread::Builder::new()
        .stack_size(stack)
        .spawn(move || {
            let source = format!(
                "import * as stylex from '@stylexjs/stylex';\n{prefix}\nexport const s = stylex.create({{a:{{color:'red'}}}});"
            );
            transform_source(&source, "small-stack.js", &StyleXTransformOptions::default())
                .map(|output| output.metadata_stylex.len())
                .map_err(|error| format!("{error:#}"))
        })
        .unwrap()
        .join()
        .expect("the transform must not overflow its caller's stack")
}

#[test]
fn deep_parentheses_compile_on_a_512_kib_thread() {
    let prefix = format!("const x = {}1{};", "(".repeat(1000), ")".repeat(1000));
    assert_eq!(run_on_small_thread(prefix, 512 * 1024), Ok(1));
}

#[test]
fn deep_arrows_compile_on_a_2_mib_thread() {
    let prefix = format!("const x = {}1;", "() => ".repeat(1000));
    assert_eq!(run_on_small_thread(prefix, 2 * 1024 * 1024), Ok(1));
}

/// Deep calls, member chains, patterns and blocks on a default-size test thread.
#[test]
fn deep_calls_members_patterns_and_blocks_compile_on_a_2_mib_thread() {
    let stack = 2 * 1024 * 1024;
    for prefix in [
        format!("const x = a{};", "()".repeat(1000)),
        format!("const x = a{};", ".b".repeat(1000)),
        format!(
            "const {}c{} = theme;",
            "{ outer: ".repeat(500),
            " }".repeat(500)
        ),
        format!("{}const y = 1;{}", "{".repeat(1000), "}".repeat(1000)),
    ] {
        let shape = prefix[..prefix.len().min(24)].to_owned();
        assert_eq!(run_on_small_thread(prefix, stack), Ok(1), "{shape}");
    }
}

#[test]
fn over_limit_input_fails_catchably_on_a_small_thread() {
    let prefix = format!("const x = {}1{};", "(".repeat(20_000), ")".repeat(20_000));
    let result = run_on_small_thread(prefix, 512 * 1024);
    assert!(result.is_err(), "{result:?}");
}
