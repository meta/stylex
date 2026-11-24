//! The oxc fast path (`fast_prepare`) must give exactly what Flow's parser gives, or decline.
//! tests/flow/fuzz-types.cjs and `flow_blank fastcheck` run the same check over large corpora.

use flow_blank::{Options, fast_prepare, prepare_flow_parser};

fn same_as_flow(src: &str) -> bool {
    let Some(f) = fast_prepare(src) else { return false };
    let s = prepare_flow_parser(src, Options::default()).unwrap_or_else(|e| panic!("Flow rejects what the fast path took: {e:?}\n{src}"));
    assert!(s.splice.is_none(), "{src}");
    assert_eq!(f.parse, s.parse, "blanked text differs for\n{src}");
    assert_eq!(f.type_refs, s.type_refs, "type names differ for\n{src}");
    true
}

#[test]
fn shared_syntax_takes_the_fast_path_and_matches_flow() {
    let cases = [
        "// @flow\nimport type {A, B as C} from './a';\nimport {type D, e} from './d';\nimport F, {type G} from './f';\nexport type {A};\nexport type H<T = number> = {a: T, b?: Array<string>, [k: string]: mixed};\nexport interface I extends J { m(x: number): void }\n",
        "// @flow\nfunction f<T>(this: Foo, x?: T, {a}: {a: string} = {a: ''}, ...rest: Array<T>): T | void { return x; }\nconst g = (x: number)\n  : string => String(x);\nconst h = f<number>(1) as any;\nconst k = [1] as const;\n",
        "// @flow\nclass C<T> extends B<T> implements I, J {\n  x: number;\n  #y: string = '';\n  static z: T;\n  declare w: number;\n  m<U>(u: U): T { return (null: any) ?? u; }\n}\n",
        "// @flow\nconst styles = stylex.create({a: {color: 'red'}});\ntype K = $Keys<typeof styles>;\ntype F = (styles: number, string) => void;\nexport const c: typeof styles = styles;\n",
    ];
    let mut fast = 0;
    for c in cases {
        // Case 2 has a `(x: T)` cast, which only Flow parses.
        if same_as_flow(c) {
            fast += 1;
        }
    }
    assert_eq!(fast, 3, "three of four cases are shared syntax");
}

#[test]
fn flow_only_and_ts_only_syntax_decline() {
    for c in [
        "// @flow\ntype A = ?number;\n",
        "// @flow\ntype A = {|a: 1|};\n",
        "// @flow\nopaque type A = 1;\n",
        "// @flow\nconst x = (y: any);\n",
        "// @flow\ntype A = {...B, c: 1};\n",
        "// @flow\nenum E {A}\n",
        "// @flow\ntype A = keyof B;\n",
        "// @flow\nconst x = y!;\n",
        "// @flow\nclass C { private x: number = 1; }\n",
        "// @flow\nfunction f<T extends U>() {}\n",
        "// @flow\nconst f = cond ? (a): T => a : b;\n",
        "// @flow\nconst f = cond ? (a) : (b) => c;\n",
        "// @flow\nconst x = f < {...A} > (y);\n",
        "// @flow\ntype F = ({a: number}) => void;\n",
        "// @flow\nconst f = ({a}, {a}) => 1;\n",
        "// @flow\ninterface I extends bool {}\n",
        "// @flow\nclass C {\n  x: number\n  [y]: number = 1;\n}\n",
    ] {
        assert!(fast_prepare(c).is_none(), "fast path took\n{c}");
    }
}

#[test]
fn fixtures_match_when_taken() {
    let dir = format!("{}/../../tests/flow/fixtures", env!("CARGO_MANIFEST_DIR"));
    for e in std::fs::read_dir(dir).unwrap() {
        let src = std::fs::read_to_string(e.unwrap().path()).unwrap();
        same_as_flow(&src);
    }
}
