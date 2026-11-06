//! Tests for [`find_declaration_span`].
//!
//! Cases assert the text a span covers, so they compare directly with Babel's code frame.

use super::*;

use swc_core::{
  common::{DUMMY_SP, FileName, SourceMap, sync::Lrc},
  ecma::{
    ast::EsVersion,
    parser::{Parser, StringInput, Syntax, TsSyntax, lexer::Lexer},
  },
};

fn parse(source: &str) -> Module {
  let source_map: Lrc<SourceMap> = Default::default();
  let source_file = source_map.new_source_file(FileName::Anon.into(), source.to_owned());
  let lexer = Lexer::new(
    Syntax::Typescript(TsSyntax {
      tsx: true,
      ..Default::default()
    }),
    EsVersion::EsNext,
    StringInput::from(&*source_file),
    None,
  );

  match Parser::new_from(lexer).parse_module() {
    Ok(module) => module,
    Err(error) => panic!("failed to parse the fixture: {:?}", error),
  }
}

/// Runs `work` on a stack grown for `levels` of nesting, as the compiler does with `stacker`.
fn on_a_stack_for<R>(levels: usize, work: impl FnOnce() -> R) -> R {
  let claim = levels * 64 * 1024 + 1024 * 1024;
  stacker::maybe_grow(claim, claim, work)
}

/// The source text `name`'s declaration span covers.
/// Each fixture's source map starts at `BytePos(1)`, so a span's offset is `lo - 1`.
#[track_caller]
fn declaration_text(source: &str, name: &str) -> String {
  let span = find_declaration_span(&parse(source), &Atom::from(name));

  assert!(
    !span.is_dummy(),
    "nothing was found declaring `{name}` in:\n{source}"
  );

  let start = span.lo.0 as usize - 1;
  let end = span.hi.0 as usize - 1;

  match source.get(start..end) {
    Some(text) => text.to_owned(),
    None => panic!("the span {span:?} is not a character range of the fixture"),
  }
}

/// The 1-based line `name`'s declaration is framed at.
#[track_caller]
fn declaration_line(source: &str, name: &str) -> usize {
  let span = find_declaration_span(&parse(source), &Atom::from(name));

  assert!(
    !span.is_dummy(),
    "nothing was found declaring `{name}` in:\n{source}"
  );

  source[..span.lo.0 as usize - 1].matches('\n').count() + 1
}

#[track_caller]
fn declares_nothing(source: &str, name: &str) {
  let span = find_declaration_span(&parse(source), &Atom::from(name));

  assert!(
    span.is_dummy(),
    "`{name}` is not declared in the fixture, but a span was found for it:\n{source}"
  );
}

// ── the four shapes upstream's `binding.path` can be ────────────────────────

/// Babel 0.19.0 puts the caret over `c = 'red'`, not over the `const` keyword.
#[test]
fn a_variable_is_framed_at_its_declarator() {
  for keyword in ["const", "let", "var"] {
    let source = format!("{keyword} c = 'red';\n");

    assert_eq!(declaration_text(&source, "c"), "c = 'red'");
  }
}

#[test]
fn a_variable_with_no_initializer_is_framed_at_its_name() {
  assert_eq!(declaration_text("let c;\n", "c"), "c");
}

/// Babel's caret covers the whole `function f() {}`.
#[test]
fn a_function_declaration_is_framed_whole() {
  assert_eq!(
    declaration_text("function f() { return 1; }\n", "f"),
    "function f() { return 1; }"
  );
}

#[test]
fn a_class_declaration_is_framed_whole() {
  assert_eq!(declaration_text("class K {}\n", "K"), "class K {}");
}

/// Babel frames the specifier rather than the whole `import` statement (measured on 0.19.0).
#[test]
fn an_import_is_framed_at_its_specifier() {
  let named = "import { token } from './vars.stylex.js';\n";
  let aliased = "import { token as alias } from './vars.stylex.js';\n";
  let default_import = "import vars from './vars.stylex.js';\n";
  let namespace = "import * as vars from './vars.stylex.js';\n";

  assert_eq!(declaration_text(named, "token"), "token");
  assert_eq!(declaration_text(aliased, "alias"), "token as alias");
  assert_eq!(declaration_text(default_import, "vars"), "vars");
  assert_eq!(declaration_text(namespace, "vars"), "* as vars");
}

#[test]
fn one_import_statement_frames_the_specifier_that_was_refused() {
  let source = "import { first, second, third } from './vars.stylex.js';\n";

  assert_eq!(declaration_text(source, "second"), "second");
}

// ── every other binding position ────────────────────────────────────────────

#[test]
fn a_destructured_name_is_framed_at_the_declarator_that_binds_it() {
  let object = "const { a, c } = theme;\n";
  let renamed = "const { token: c } = theme;\n";
  let array = "const [first, c] = pair;\n";
  let rest = "const { a, ...c } = theme;\n";
  let nested = "const { outer: { c } } = theme;\n";
  let defaulted = "const { c = 'red' } = theme;\n";

  assert_eq!(declaration_text(object, "c"), "{ a, c } = theme");
  assert_eq!(declaration_text(renamed, "c"), "{ token: c } = theme");
  assert_eq!(declaration_text(array, "c"), "[first, c] = pair");
  assert_eq!(declaration_text(rest, "c"), "{ a, ...c } = theme");
  assert_eq!(declaration_text(nested, "c"), "{ outer: { c } } = theme");
  assert_eq!(declaration_text(defaulted, "c"), "{ c = 'red' } = theme");
}

/// Covers `Pat::Rest` and `Pat::Assign`, which only array patterns and renamed defaults reach.
#[test]
fn an_array_rest_and_a_defaulted_element_are_framed_at_their_declarator() {
  let array_rest = "const [first, ...c] = pair;\n";
  let array_default = "const [c = 'red'] = pair;\n";
  let renamed_default = "const { token: c = 'red' } = theme;\n";

  assert_eq!(declaration_text(array_rest, "c"), "[first, ...c] = pair");
  assert_eq!(declaration_text(array_default, "c"), "[c = 'red'] = pair");
  assert_eq!(
    declaration_text(renamed_default, "c"),
    "{ token: c = 'red' } = theme"
  );
}

#[test]
fn a_parameter_is_framed_at_the_parameter() {
  assert_eq!(declaration_text("const f = (c) => c;\n", "c"), "c");
  assert_eq!(declaration_text("function f(c) { return c; }\n", "c"), "c");
}

#[test]
fn a_catch_binding_is_framed_at_the_binding() {
  assert_eq!(declaration_text("try { f(); } catch (c) {}\n", "c"), "c");
}

#[test]
fn a_loop_binding_is_framed_at_the_binding() {
  assert_eq!(
    declaration_text("for (const c of values) { f(c); }\n", "c"),
    "c"
  );
}

#[test]
fn a_class_expression_is_framed_at_its_declarator() {
  assert_eq!(
    declaration_text("const K = class {};\n", "K"),
    "K = class {}"
  );
}

/// Bundlers emit one-line modules, where the span must cover only the declarator's columns.
#[test]
fn a_single_line_module_frames_the_declarator_it_is_about() {
  let source = "const a='blue';const c='red';const b='green';\n";

  assert_eq!(declaration_text(source, "c"), "c='red'");
  assert_eq!(declaration_text(source, "a"), "a='blue'");
  assert_eq!(declaration_text(source, "b"), "b='green'");
}

/// `strip` lowers an `enum` or value `namespace` into a `var` plus an IIFE.
#[test]
fn a_name_a_value_emitting_typescript_declaration_binds_is_found() {
  assert_eq!(
    declaration_text("enum c { A }\nconst d = c;\n", "c"),
    "enum c { A }"
  );
  assert_eq!(
    declaration_text("namespace c { export const A = 1; }\nconst d = c;\n", "c"),
    "namespace c { export const A = 1; }"
  );
}

// ── what declares nothing ───────────────────────────────────────────────────

#[test]
fn a_name_the_module_only_reads_declares_nothing() {
  declares_nothing("export const styles = create({ x: { color: c } });\n", "c");
}

#[test]
fn an_object_key_is_not_a_declaration() {
  declares_nothing("const theme = { c: 'red' };\n", "c");
}

/// `[holder.c] = pair;` is an assignment expression, so it never reaches the `Pat::Expr` arm.
#[test]
fn a_name_spelled_only_as_an_assignment_target_declares_nothing() {
  declares_nothing("[holder.c] = pair;\n", "c");
}

#[test]
fn a_default_values_own_parameter_does_not_claim_the_declarator() {
  let source = "const { a = (c) => c } = theme;\n";

  assert_eq!(declaration_text(source, "a"), "{ a = (c) => c } = theme");
  assert_eq!(declaration_text(source, "c"), "c");
}

#[test]
fn an_empty_module_declares_nothing() {
  declares_nothing("", "c");
}

#[test]
fn a_module_of_only_comments_declares_nothing() {
  declares_nothing("// c is discussed but never declared\n", "c");
}

// ── which declaration wins ──────────────────────────────────────────────────

#[test]
fn the_first_declaration_in_source_order_wins() {
  let source = "\
const c = 'red';
function inner() {
  const c = 'blue';
  return c;
}
";

  assert_eq!(declaration_line(source, "c"), 1);
}

#[test]
fn a_declaration_below_the_read_is_still_found() {
  let source = "\
export const styles = create({ x: { color: c } });
const c = 'red';
";

  assert_eq!(declaration_line(source, "c"), 2);
}

#[test]
fn a_deeply_nested_declaration_is_found() {
  let source = "\
function outer() {
  if (true) {
    for (;;) {
      switch (1) {
        default: {
          try {
            const c = 'red';
          } finally {
          }
        }
      }
    }
  }
}
";

  assert_eq!(declaration_line(source, "c"), 7);
}

// ── inputs that are unusual rather than wrong ───────────────────────────────

#[test]
fn a_declaration_after_multibyte_characters_keeps_a_character_range() {
  let source = "\
const σ = 'λλλλ';
const c = 'red';
";

  assert_eq!(declaration_text(source, "c"), "c = 'red'");
  assert_eq!(declaration_line(source, "c"), 2);
}

#[test]
fn a_name_spelled_with_non_ascii_characters_is_found() {
  assert_eq!(declaration_text("const λ = 'red';\n", "λ"), "λ = 'red'");
}

#[test]
fn an_escaped_identifier_is_the_name_it_spells() {
  assert_eq!(
    declaration_text("const \\u0063 = 'red';\n", "c"),
    "\\u0063 = 'red'"
  );
}

#[test]
fn a_name_that_only_differs_by_case_is_not_the_declaration() {
  declares_nothing("const C = 'red';\n", "c");
}

#[test]
fn a_longer_name_containing_the_one_asked_for_declares_nothing() {
  declares_nothing("const color = 'red';\n", "col");
}

/// The walk must not grow a stack frame for each statement it passes.
#[test]
fn a_declaration_at_the_end_of_a_long_module_is_found() {
  let mut source = String::new();
  for index in 0..5_000 {
    source.push_str(&format!("const n{index} = {index};\n"));
  }
  source.push_str("const c = 'red';\n");

  assert_eq!(declaration_line(&source, "c"), 5_001);
}

/// The pattern walk recurses, so this pins that a thousand levels do not overflow the stack.
#[test]
fn a_deeply_nested_pattern_is_walked_to_the_bottom() {
  let depth = 500;
  let source = format!(
    "const {}c{} = theme;\n",
    "{ outer: ".repeat(depth),
    " }".repeat(depth)
  );

  assert_eq!(on_a_stack_for(depth, || declaration_line(&source, "c")), 1);
}

#[test]
fn a_type_only_name_declares_no_value() {
  declares_nothing("type c = string;\n", "c");
  declares_nothing("interface c { a: string }\n", "c");
}

#[test]
fn one_statement_of_several_declarators_frames_the_one_asked_about() {
  let source = "const a = 1, c = 2, d = 3;\n";

  assert_eq!(declaration_text(source, "c"), "c = 2");
}

#[test]
fn a_loop_header_declarator_is_framed_at_the_declarator() {
  assert_eq!(
    declaration_text("for (let c = 0; c < 2; c++) {}\n", "c"),
    "c = 0"
  );
}

#[test]
fn a_label_is_not_a_declaration() {
  declares_nothing("c: for (;;) { break c; }\n", "c");
}

#[test]
fn a_method_name_is_not_a_declaration() {
  declares_nothing("const api = { c() { return 1; } };\n", "c");
  declares_nothing("class K { c() { return 1; } }\n", "c");
}

/// This includes `#private` fields.
#[test]
fn a_class_field_is_not_a_declaration() {
  declares_nothing("class K { c = 1; }\n", "c");
}

#[test]
fn a_named_function_expression_binds_its_own_name() {
  assert_eq!(
    declaration_text("const run = function c() { return c; };\n", "c"),
    "function c() { return c; }"
  );
  assert_eq!(
    declaration_text("const held = class c { };\n", "c"),
    "class c { }"
  );
}

#[test]
fn a_declaration_around_a_template_literal_is_found() {
  let interpolated = "const text = `${(() => { const c = 'red'; return c; })()}`;\n";
  let after = "const text = `const c = 'not this one';`;\nconst c = 'red';\n";

  assert_eq!(declaration_text(interpolated, "c"), "c = 'red'");
  assert_eq!(declaration_line(after, "c"), 2);
}

#[test]
fn a_declaration_in_a_crlf_module_keeps_its_line() {
  let source = "const first = 1;\r\nconst c = 'red';\r\n";

  assert_eq!(declaration_line(source, "c"), 2);
  assert_eq!(declaration_text(source, "c"), "c = 'red'");
}

#[test]
fn an_empty_name_declares_nothing() {
  declares_nothing("const c = 'red';\n", "");
}

#[test]
fn a_reserved_word_declares_nothing() {
  declares_nothing("const holder = { class: 1 };\n", "class");
}

#[test]
fn a_module_level_declaration_wins_over_an_earlier_block_scoped_namesake() {
  let source = "\
function unrelated() {
  const c = 'blue';
  return c;
}
const c = 'red';
";

  assert_eq!(declaration_line(source, "c"), 5);
}

#[test]
fn every_module_level_shape_wins_over_an_inner_namesake() {
  let inner = "function unrelated() { const c = 'blue'; return c; }\n";

  for (module_level, expected) in [
    ("const c = 'red';\n", "c = 'red'"),
    ("export const c = 'red';\n", "c = 'red'"),
    ("function c() {}\n", "function c() {}"),
    ("export function c() {}\n", "function c() {}"),
    ("class c {}\n", "class c {}"),
    ("export default function c() {}\n", "function c() {}"),
    ("export default class c {}\n", "class c {}"),
    ("import { c } from './vars.stylex.js';\n", "c"),
    ("import c from './vars.stylex.js';\n", "c"),
    ("import * as c from './vars.stylex.js';\n", "* as c"),
    ("const { c } = theme;\n", "{ c } = theme"),
  ] {
    let source = format!("{inner}{module_level}");

    assert_eq!(
      declaration_text(&source, "c"),
      expected,
      "the module-level declaration must win in:\n{source}"
    );
  }
}

#[test]
fn a_nested_declaration_is_still_found_when_the_module_declares_nothing() {
  let source = "\
function unrelated() {
  const c = 'blue';
  return c;
}
";

  assert_eq!(declaration_line(source, "c"), 2);
}

// ── the shapes the module-level walk hands on to the visitor ────────────────

#[test]
fn an_exported_default_interface_declares_no_value() {
  declares_nothing("export default interface c { a: string }\n", "c");
}

/// Only a non-`declare` `namespace` emits a value; the other TypeScript module forms do not.
#[test]
fn a_typescript_module_that_emits_nothing_declares_no_value() {
  declares_nothing("declare namespace c { export const a = 1; }\n", "c");
  declares_nothing("module c { export const a = 1; }\n", "c");
  declares_nothing("declare module 'c' { export const a: number; }\n", "c");
}

#[test]
fn a_nested_function_declaration_is_framed_whole() {
  let source = "function outer() {\n  function c() { return 1; }\n}\n";

  assert_eq!(declaration_text(source, "c"), "function c() { return 1; }");
}

#[test]
fn a_nested_class_declaration_is_framed_whole() {
  let source = "function outer() {\n  class c { m() {} }\n}\n";

  assert_eq!(declaration_text(source, "c"), "class c { m() {} }");
}

#[test]
fn a_function_expression_of_another_name_is_walked_into() {
  let source = "const run = function outer() {\n  const c = 'red';\n};\n";

  assert_eq!(declaration_text(source, "c"), "c = 'red'");
}

#[test]
fn a_class_expression_of_another_name_is_walked_into() {
  let source = "const run = class Outer {\n  m() {\n    const c = 'red';\n  }\n};\n";

  assert_eq!(declaration_text(source, "c"), "c = 'red'");
}

/// Every shape after the first match must leave the settled answer alone.
#[test]
fn the_first_declaration_reached_is_kept_over_every_later_shape() {
  let source = "\
function outer(c) {
  const later = 1;
  function c() {}
  class c {}
  (function c() {});
  (class c {});
}
";

  assert_eq!(declaration_text(source, "c"), "c");
  assert_eq!(declaration_line(source, "c"), 1);
}

/// No parsed declarator reaches these patterns, so they are passed to `binds_name` directly.
#[test]
fn an_assignment_target_and_an_error_node_bind_nothing() {
  let name = Atom::from("c");

  let target = Pat::Expr(Box::new(Expr::Ident(Ident::new_no_ctxt(
    name.clone(),
    DUMMY_SP,
  ))));

  assert!(!binds_name(&target, &name));
  assert!(!binds_name(
    &Pat::Invalid(Invalid { span: DUMMY_SP }),
    &name
  ));
}
