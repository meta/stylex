//! Blanking keeps byte length and every line break, and reports runtime-only syntax.

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!("{}/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

fn assert_layout(src: &str, out: &str) {
    assert_eq!(src.len(), out.len());
    for (i, (a, b)) in src.bytes().zip(out.bytes()).enumerate() {
        assert_eq!(a == b'\n', b == b'\n', "line break moved at byte {i}");
    }
}

#[test]
fn all_types_blank_cleanly() {
    let src = fixture("all.js");
    let out = flow_blank::blank(&src).unwrap();
    assert_layout(&src, &out.code);
    assert!(out.unsupported.is_empty(), "{:?}", out.unsupported);
    assert!(out.internal.is_empty(), "{:?}", out.internal);
    for gone in ["import type", "import typeof", "opaque", "interface", "declare", "implements", "as string", "as const", "%checks", "<T>"] {
        assert!(!out.code.contains(gone), "`{gone}` left in output");
    }
}

#[test]
fn runtime_syntax_is_reported() {
    let src = fixture("unsupported.js");
    let out = flow_blank::blank(&src).unwrap();
    let kinds: Vec<_> = out.unsupported.iter().map(|(k, _)| *k).collect();
    for k in ["enum", "component", "hook", "match"] {
        assert!(kinds.contains(&k), "{k} not reported: {kinds:?}");
    }
}

#[test]
fn multibyte_positions() {
    let src = "const é: number = 1; const ü = (é: any);\nlet x: Ünicode<'ß'> = 2;\n";
    let out = flow_blank::blank(src).unwrap();
    assert_layout(src, &out.code);
    assert_eq!(out.code, "const é         = 1; const ü = (é     );\nlet x                 = 2;\n");
}
