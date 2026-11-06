//! What [`find_declaration_span`] answers for each shape a name can be declared
//! in, and for the shapes that declare nothing.
//!
//! Each case asserts the *text* the span covers rather than a byte offset, so a
//! failure reads as "it underlined the wrong thing" and the expectation can be
//! compared against `@stylexjs/babel-plugin`'s own code frame, which is what
//! these positions exist to agree with.

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

/// The source text `name`'s declaration span covers.
///
/// Each fixture is parsed into a source map of its own, whose first byte is
/// `BytePos(1)`, so a span's offset into the fixture is one less than its `lo`.
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

/// The 1-based line `name`'s declaration is framed at, which is what a code
/// frame prints and what the two compilers are compared on.
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

/// Upstream frames the declarator, not the `const` keyword: measured on 0.19.0,
/// a refused `const c = 'red'` carries a caret over `c = 'red'`.
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

/// Upstream's caret covers the whole `function f() {}`, so this does too.
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

/// An import is framed at the specifier rather than at the whole `import`
/// statement, which is what upstream's `binding.path` is: one statement declares
/// several names, and only the specifier says which of them was refused.
/// Measured on 0.19.0 for the named and default shapes.
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

/// A statement importing several names frames the one that was refused, not the
/// first one it declares.
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

/// The two `Pat` arms the case above does not reach. An object pattern's rest
/// and default are `ObjectPatProp::Rest` and `ObjectPatProp::Assign`, which are
/// different enum arms from `Pat::Rest` and `Pat::Assign` -- those are only
/// reachable through an *array* pattern, or through a renamed key whose value is
/// itself a default. Return `false` from either and the case above still passes,
/// while these bindings fall through to the binding-identifier walk and get
/// framed at the bare name instead of at the declarator.
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

/// A class *expression* is not a `ClassDecl`, so the name it binds is the
/// declarator's, and that is what gets framed.
#[test]
fn a_class_expression_is_framed_at_its_declarator() {
  assert_eq!(
    declaration_text("const K = class {};\n", "K"),
    "K = class {}"
  );
}

/// A whole program on one line, which is what a bundler hands a downstream
/// transform and the shape where every declaration shares a line number. The
/// span has to be the declarator's own columns, not the line's.
#[test]
fn a_single_line_module_frames_the_declarator_it_is_about() {
  let source = "const a='blue';const c='red';const b='green';\n";

  assert_eq!(declaration_text(source, "c"), "c='red'");
  assert_eq!(declaration_text(source, "a"), "a='blue'");
  assert_eq!(declaration_text(source, "b"), "b='green'");
}

/// A value-emitting TypeScript declaration, which `strip` lowers into something
/// the walk did not see written. An `enum` becomes a `var` plus an IIFE, and a
/// value `namespace` the same, so the name is still declared -- by a node the
/// author did not spell. This is the case the `is_dummy` fallback in
/// `get_span_from_source_code_impl` exists for, and nothing exercised it: the
/// type-only case next door covers a name that declares *no* value.
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

/// Not a test of the `Pat::Expr` arm, despite appearances: `[holder.c] = pair;`
/// parses as an assignment *expression*, so no declarator is walked at all.
/// `Pat::Expr` and `Pat::Invalid` are unreachable from a parsed declarator, and
/// the arm exists to say so rather than to be exercised.
#[test]
fn a_name_spelled_only_as_an_assignment_target_declares_nothing() {
  declares_nothing("[holder.c] = pair;\n", "c");
}

/// A default value is an expression, and an expression can hold bindings of its
/// own. The declarator binds `a`; the arrow's parameter is not what `c` names,
/// so `c` resolves to the parameter itself rather than to the declarator.
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

/// The chain resolves bindings module-wide with no scope of its own, so a name
/// declared twice has no second binding for it to prefer. The first in source
/// order is the answer, which keeps it the same whichever reference asked.
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

/// Byte offsets are what a span carries, and a multi-byte character makes a
/// count of characters and a count of bytes different numbers. The span has to
/// be a character range of the source or slicing it panics.
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

/// An escaped identifier is the same name to the language, and the parser
/// resolves the escape before this sees it — so the declaration is found, and
/// framed over the text as it is written.
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

/// A name is not a substring match: `color` does not declare `col`.
#[test]
fn a_longer_name_containing_the_one_asked_for_declares_nothing() {
  declares_nothing("const color = 'red';\n", "col");
}

/// The last declaration of a long module is still reached, and reached without
/// the walk growing a stack frame per statement it passes.
#[test]
fn a_declaration_at_the_end_of_a_long_module_is_found() {
  let mut source = String::new();
  for index in 0..5_000 {
    source.push_str(&format!("const n{index} = {index};\n"));
  }
  source.push_str("const c = 'red';\n");

  assert_eq!(declaration_line(&source, "c"), 5_001);
}

/// A pattern nested a thousand deep is recursion this walks rather than
/// iterates, so it is pinned: the answer is the declarator, and getting there
/// does not overflow the stack.
#[test]
fn a_deeply_nested_pattern_is_walked_to_the_bottom() {
  let depth = 500;
  let source = format!(
    "const {}c{} = theme;\n",
    "{ outer: ".repeat(depth),
    " }".repeat(depth)
  );

  assert_eq!(declaration_line(&source, "c"), 1);
}

/// A name declared only inside TypeScript syntax the transform strips is not a
/// value binding. The production module reaching this is already stripped, and
/// a fixture that is not agrees: a type parameter is not a declaration a code
/// frame should be pointed at.
#[test]
fn a_type_only_name_declares_no_value() {
  declares_nothing("type c = string;\n", "c");
  declares_nothing("interface c { a: string }\n", "c");
}

/// One statement declaring several names frames the one asked about, not the
/// statement.
#[test]
fn one_statement_of_several_declarators_frames_the_one_asked_about() {
  let source = "const a = 1, c = 2, d = 3;\n";

  assert_eq!(declaration_text(source, "c"), "c = 2");
}

/// A `for` header's own binding, which is a declarator inside a statement rather
/// than a statement of its own.
#[test]
fn a_loop_header_declarator_is_framed_at_the_declarator() {
  assert_eq!(
    declaration_text("for (let c = 0; c < 2; c++) {}\n", "c"),
    "c = 0"
  );
}

/// A label shares no namespace with a binding, so it declares nothing.
#[test]
fn a_label_is_not_a_declaration() {
  declares_nothing("c: for (;;) { break c; }\n", "c");
}

/// A method's name is a property, not a binding.
#[test]
fn a_method_name_is_not_a_declaration() {
  declares_nothing("const api = { c() { return 1; } };\n", "c");
  declares_nothing("class K { c() { return 1; } }\n", "c");
}

/// A class field is a property too, `#private` included.
#[test]
fn a_class_field_is_not_a_declaration() {
  declares_nothing("class K { c = 1; }\n", "c");
}

/// A name bound by a named function *expression* is a binding in that
/// expression's own scope, and it is where the name resolves.
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

/// A declaration inside a template literal's interpolation is still a
/// declaration, and one written after a template literal is still found — a
/// template's text is not source the walk can be lost in.
#[test]
fn a_declaration_around_a_template_literal_is_found() {
  let interpolated = "const text = `${(() => { const c = 'red'; return c; })()}`;\n";
  let after = "const text = `const c = 'not this one';`;\nconst c = 'red';\n";

  assert_eq!(declaration_text(interpolated, "c"), "c = 'red'");
  assert_eq!(declaration_line(after, "c"), 2);
}

/// Windows line endings: a line number is a count of `\n`, and a `\r` in front
/// of one must not shift the position it reports.
#[test]
fn a_declaration_in_a_crlf_module_keeps_its_line() {
  let source = "const first = 1;\r\nconst c = 'red';\r\n";

  assert_eq!(declaration_line(source, "c"), 2);
  assert_eq!(declaration_text(source, "c"), "c = 'red'");
}

/// An empty name cannot be declared, and asking for one must answer rather than
/// match the first binding it meets.
#[test]
fn an_empty_name_declares_nothing() {
  declares_nothing("const c = 'red';\n", "");
}

/// A name that is a reserved word cannot be a binding either, and the fixture
/// that spells it is a property access rather than a declaration.
#[test]
fn a_reserved_word_declares_nothing() {
  declares_nothing("const holder = { class: 1 };\n", "class");
}

/// The inner declaration written *first*: the chain resolves bindings
/// module-wide, so the module-level binding is the one a refusal is about, and
/// framing a block-scoped namesake that happens to come earlier in the file
/// would send the reader to a declaration the refusal was never about.
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

/// The module's own binding is preferred whichever shape declares it, so each
/// way a module can bind a name is asked with an inner namesake written above it.
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

/// With nothing at module level, the nested declaration is still the answer —
/// the preference is an ordering, not a filter.
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

/// `export default interface` exports a type and binds no value, so there is
/// nothing to frame even though the name is written right there.
#[test]
fn an_exported_default_interface_declares_no_value() {
  declares_nothing("export default interface c { a: string }\n", "c");
}

/// Of the TypeScript module forms only a non-`declare` `namespace` emits a
/// value. The rest bind nothing at run time, so a read of one has no
/// declaration worth framing.
#[test]
fn a_typescript_module_that_emits_nothing_declares_no_value() {
  declares_nothing("declare namespace c { export const a = 1; }\n", "c");
  declares_nothing("module c { export const a = 1; }\n", "c");
  declares_nothing("declare module 'c' { export const a: number; }\n", "c");
}

/// A `function` written inside another one is still a declaration, and the
/// module-level walk never sees it.
#[test]
fn a_nested_function_declaration_is_framed_whole() {
  let source = "function outer() {\n  function c() { return 1; }\n}\n";

  assert_eq!(declaration_text(source, "c"), "function c() { return 1; }");
}

/// The same for a `class`.
#[test]
fn a_nested_class_declaration_is_framed_whole() {
  let source = "function outer() {\n  class c { m() {} }\n}\n";

  assert_eq!(declaration_text(source, "c"), "class c { m() {} }");
}

/// A named function expression whose name is *not* the one asked about is
/// walked into rather than claimed, so a declaration inside its body is found.
#[test]
fn a_function_expression_of_another_name_is_walked_into() {
  let source = "const run = function outer() {\n  const c = 'red';\n};\n";

  assert_eq!(declaration_text(source, "c"), "c = 'red'");
}

/// The same for a named class expression.
#[test]
fn a_class_expression_of_another_name_is_walked_into() {
  let source = "const run = class Outer {\n  m() {\n    const c = 'red';\n  }\n};\n";

  assert_eq!(declaration_text(source, "c"), "c = 'red'");
}

/// The walk covers the whole module rather than stopping at the first
/// declaration, so every later shape has to recognise that the answer is
/// already settled and leave it alone. The parameter is reached before any of
/// the five shapes that follow it. The last two are written as expression
/// statements, because a declarator that already knows the answer never walks
/// into its own initializer.
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

/// The two pattern shapes that are not bindings at all. Neither is reachable
/// from a parsed declarator -- `[o.x] = pair` is an assignment expression, so
/// nothing walks a pattern for it -- so they are asked directly.
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
