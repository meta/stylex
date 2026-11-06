/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

// The public Rust entry point must be safe to call from small threads:
// deep inputs the parser accepts must not overflow later passes.

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

#[test]
fn over_limit_input_fails_catchably_on_a_small_thread() {
    let prefix = format!("const x = {}1{};", "(".repeat(20_000), ")".repeat(20_000));
    let result = run_on_small_thread(prefix, 512 * 1024);
    assert!(result.is_err(), "{result:?}");
}
