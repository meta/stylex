//! Lowering of Flow runtime syntax: both texts line up, parse in oxc, keep lines, and map back.

use oxc_allocator::Allocator;
use oxc_ast::ast::IdentifierReference;
use oxc_ast_visit::Visit;
use oxc_parser::Parser;
use oxc_span::SourceType;

fn read(path: &str) -> String {
    std::fs::read_to_string(format!("{}/{path}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

/// Every identifier reference in the lowered parse text that came from the source maps back to
/// the same name in the source.
struct Refs<'s> {
    lowered: &'s str,
    orig: &'s str,
    map: &'s flow_blank::OffsetMap,
    checked: usize,
    bad: Vec<String>,
}

impl<'a, 's> Visit<'a> for Refs<'s> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let (s, e) = (it.span.start, it.span.end);
        let name = &self.lowered[s as usize..e as usize];
        let seg = self.map.segs.iter().rev().find(|g| g.out <= s).unwrap();
        if seg.generated || e > seg.out + seg.len {
            return;
        }
        let o = self.map.to_original(s) as usize;
        self.checked += 1;
        if self.orig.get(o..o + name.len()) != Some(name) {
            self.bad.push(format!("{name} at {s} maps to {o}"));
        }
    }
}

fn check(src: &str) -> flow_blank::Prepared {
    let p = flow_blank::prepare(src).unwrap_or_else(|e| panic!("{e:?}"));
    assert!(p.internal.is_empty(), "{:?}", p.internal);
    let splice = p.splice.clone().unwrap_or_else(|| src.to_string());
    assert_eq!(p.parse.len(), splice.len(), "parse and splice texts differ in length");
    for (i, (a, b)) in p.parse.bytes().zip(splice.bytes()).enumerate() {
        assert_eq!(a == b'\n', b == b'\n', "line break differs at byte {i}");
    }
    assert_eq!(
        p.parse.matches('\n').count(),
        src.matches('\n').count(),
        "lowering changed the line count"
    );
    let alloc = Allocator::default();
    let st = if p.ts_enums { SourceType::tsx() } else { SourceType::jsx().with_module(true) };
    let ret = Parser::new(&alloc, &p.parse, st).parse();
    assert!(!ret.diagnostics.has_errors() && !ret.fatal_error, "{}", p.parse);
    if let Some(map) = &p.map {
        let mut v = Refs { lowered: &p.parse, orig: src, map, checked: 0, bad: vec![] };
        v.visit_program(&ret.program);
        assert!(v.bad.is_empty(), "{:?}", v.bad);
    }
    p
}

#[test]
fn runtime_syntax_fixture_lowers() {
    let src = read("fixtures/runtime_syntax.js.txt");
    let p = check(&src);
    let splice = p.splice.unwrap();
    assert!(splice.contains("function Button({label"), "{splice}");
    assert!(splice.contains("function useThing"));
    assert!(splice.contains("$$gen$m0"));
    // Enums stay in the output, as Babel prints them.
    assert!(splice.contains("enum Status { Active, Off }"));
    assert!(p.parse.contains("enum Status { Active, Off }"));
    assert!(p.parse.contains("enum Num           { One = 1 }"), "{}", p.parse);
}

#[test]
fn corpus_fixtures_lower() {
    for f in ["component.js", "match.js", "enum.js", "types.js", "misc.js", "tokens.stylex.js", "record.js"] {
        check(&read(&format!("../../tests/flow/fixtures/{f}")));
    }
}

#[test]
fn enums_only_need_no_map() {
    let p = check("// @flow\nenum E {A, B}\nexport enum F of string {X = 'x'}\nenum G {A, ...}\nenum Proof of boolean {A = true}\nexport default enum Q {X}\nE.A;\n");
    assert!(p.ts_enums);
    assert!(p.parse.contains("enum Proof            {A = true}"), "{}", p.parse);
    assert!(p.splice.is_none() && p.map.is_none());
}

#[test]
fn forward_ref_goes_before_first_reference() {
    let src = "// @flow\nconst use = () => Foo;\nexport component Foo(ref: R, a: string) { return a; }\n";
    let p = check(src);
    let s = p.splice.unwrap();
    let fr = s.find("const Foo = React.forwardRef(Foo_withRef);").unwrap();
    assert!(fr < s.find("const use").unwrap(), "{s}");
    assert!(s.contains("function Foo_withRef({a") && s.contains("}, ref"), "{s}");
    assert!(s.contains("export {Foo};"), "{s}");
}

#[test]
fn simple_match_becomes_conditional() {
    let p = check("// @flow\nconst x = match (a.b) { 1 => 'one', _ => 'other' };\n");
    let s = p.splice.unwrap();
    assert!(s.contains("(a.b === 1 ? ('one') : ('other'))"), "{s}");
}

#[test]
fn hermes_syntax_errors_are_errors() {
    for src in [
        "// @flow\nconst x = match (a) { -0 => 1, _ => 2 };\n",
        "// @flow\nconst x = match (a) { const b as c => 1 };\n",
        "// @flow\nconst x = match (a) { [const b] | [_, const b] => 1, _ => 2 };\n",
    ] {
        assert!(flow_blank::prepare(src).is_err(), "{src}");
    }
}

#[test]
fn type_imports_keep_their_source() {
    let src = "// @flow\nimport type {A} from 'a';\nimport typeof B from 'b';\nimport {type C} from 'c';\nimport {type D, e} from 'd';\n";
    let p = check(src);
    let lines: Vec<String> = p.parse.lines().map(|l| l.split_whitespace().collect::<Vec<_>>().join(" ")).collect();
    assert_eq!(lines[1..], ["import 'a';", "import 'b';", "import 'c';", "import { e} from 'd';"], "{}", p.parse);
}

#[test]
fn fuzz_regressions() {
    // A sequence argument stays one argument.
    let s = check("// @flow\nconst r = match ((x, y)) { const a => a };\n").splice.unwrap();
    assert!(s.contains("})((x, y))"), "{s}");
    // Rest-only optimization does not apply next to a ref param.
    let s = check("// @flow\ncomponent C(ref: R, ...rest: P) { return null; }\n").splice.unwrap();
    assert!(s.contains("function C_withRef({...rest"), "{s}");
    // Cases after a catch-all are dropped and their nested matches take no generated names.
    let s = check("// @flow\nconst a = match (f()) { _ => 1, 2 => match (g()) { _ => 3 } };\nconst b = match (h()) { const v => v };\n")
        .splice
        .unwrap();
    assert!(s.contains("$$gen$m1") && !s.contains("$$gen$m2"), "{s}");
}

#[test]
fn multiline_match_argument_keeps_later_lines() {
    let src = "// @flow\nconst r = match (\n  f(\n    x,\n  )\n) {\n  const v => v,\n};\nconst after = 1;\n";
    let p = check(src);
    let s = p.splice.unwrap();
    let line_of = |t: &str, needle: &str| t[..t.find(needle).unwrap()].matches('\n').count();
    assert_eq!(line_of(&s, "const after"), line_of(src, "const after"), "{s}");
}

#[test]
fn enum_and_react_options_follow_hermes() {
    let src = "// @flow\nenum A {X, Y}\nenum C of number {One = 1}\nexport default enum G {Q}\n";
    let o = flow_blank::Options { lower_enums: true, ..Default::default() };
    let p = flow_blank::prepare_with(src, o).unwrap();
    let s = p.splice.unwrap();
    assert!(s.contains("const A = require(\"flow-enums-runtime\").Mirrored([\"X\", \"Y\"]);"), "{s}");
    assert!(s.contains("const C = require(\"flow-enums-runtime\")({One: 1});"), "{s}");
    assert!(s.contains("export default G;"), "{s}");
    let src = "// @flow\ncomponent C(ref: R, a: string) { return a; }\n";
    let p = flow_blank::prepare_with(src, flow_blank::Options { react_19: true, ..Default::default() }).unwrap();
    let s = p.splice.unwrap();
    assert!(s.contains("function C({ref") && !s.contains("forwardRef"), "{s}");
}

#[test]
fn records_lower_to_classes() {
    let src = "// @flow\nrecord R {\n  a: number,\n  default: string = 'd',\n  static s: number = 1,\n  m(): number { return this.a; }\n}\nconst r = R {a: 1};\n";
    let s = check(src).splice.unwrap();
    assert!(s.contains("class R { constructor({a, default: $$gen$r0 = "), "{s}");
    assert!(s.contains("this.default = $$gen$r0;"), "{s}");
    assert!(s.contains("static s = 1;"), "{s}");
    assert!(s.contains("new R({a: 1})"), "{s}");
}
