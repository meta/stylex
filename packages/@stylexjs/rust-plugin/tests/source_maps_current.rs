//! Source-map output, checked with independent JavaScript consumers.
#![allow(dead_code)]
#[path = "../src/utils/source_maps.rs"]
mod source_maps;

use serde_json::{json, Value};
use source_maps::{
    compose_source_maps, compose_source_maps_with_options, validate_source_map, ComposeOptions,
    IntermediatePositions, MappingWriter, Position,
};
use std::io::Write;
use std::process::{Command, Stdio};

const CONSUMERS: &str = r#"
const fs = require('node:fs');
const {TraceMap, originalPositionFor} = require('@jridgewell/trace-mapping');
const {SourceMapConsumer} = require('source-map');
const request = JSON.parse(fs.readFileSync(0, 'utf8'));
const trace = new TraceMap(request.map);
const legacy = new SourceMapConsumer(request.map);
const result = request.queries.map(query => ({
  trace: originalPositionFor(trace, query),
  legacy: legacy.originalPositionFor(query),
}));
if (legacy.destroy) legacy.destroy();
process.stdout.write(JSON.stringify(result));
"#;
fn node(script: &str, input: &Value) -> Value {
    let mut child = Command::new("node")
        .args(["-e", script])
        .current_dir(
            std::env::var_os("STYLEX_SOURCE_MAP_NODE_ROOT")
                .unwrap_or_else(|| env!("CARGO_MANIFEST_DIR").into()),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Node and installed JS map consumers are required");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.to_string().as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn probes(map: &Value, queries: Vec<Value>) -> Vec<Value> {
    node(CONSUMERS, &json!({"map": map, "queries": queries}))
        .as_array()
        .unwrap()
        .clone()
}
fn expected(source: &str, line: usize, column: usize, name: Option<&str>) -> Value {
    json!({"source": source, "line": line + 1, "column": column, "name": name})
}
fn both(probe: &Value, position: &Value) {
    assert_eq!(&probe["trace"], position, "trace-mapping");
    assert_eq!(&probe["legacy"], position, "source-map");
}
fn identity_map(source: &str) -> Value {
    let mut writer = MappingWriter::new();
    writer.text(source, Position::default(), true);
    let mut original = Position::default();
    source.chars().for_each(|c| original.advance(c));
    writer.map(original);
    json!({"version": 3, "file": "intermediate.js", "sources": ["actual/input.js"],
        "sourcesContent": [source], "names": [], "mappings": writer.finish()})
}
fn vlq(value: i64) -> String {
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut number = value.unsigned_abs() * 2 + u64::from(value < 0);
    let mut output = String::new();
    loop {
        let mut digit = (number & 31) as usize;
        number >>= 5;
        if number != 0 {
            digit |= 32;
        }
        output.push(alphabet[digit] as char);
        if number == 0 {
            break;
        }
    }
    output
}
// Absolute generated points -> conventional delta-encoded fixture map.
// (generated line,column, source,original line,column,name), None = unmapped.
type FixturePoint = (usize, usize, Option<(i64, i64, i64, Option<i64>)>);
fn fixture(points: &[FixturePoint], sources: &[&str], names: &[&str]) -> Value {
    let mut mappings = String::new();
    let mut previous = [0i64; 4];
    let mut line = 0;
    let mut column = 0;
    let mut separator = false;
    for &(generated_line, generated_column, original) in points {
        while line < generated_line {
            mappings.push(';');
            line += 1;
            column = 0;
            separator = false;
        }
        if separator {
            mappings.push(',');
        }
        mappings.push_str(&vlq(generated_column as i64 - column));
        column = generated_column as i64;
        if let Some((source, original_line, original_column, name)) = original {
            for (index, value) in [source, original_line, original_column]
                .into_iter()
                .enumerate()
            {
                mappings.push_str(&vlq(value - previous[index]));
                previous[index] = value;
            }
            if let Some(name) = name {
                mappings.push_str(&vlq(name - previous[3]));
                previous[3] = name;
            }
        }
        separator = true;
    }
    json!({"version":3,"sources":sources,"names":names,"mappings":mappings})
}

#[test]
fn generated_lf_queries_after_unicode_separators_keep_js_original_positions() {
    // Review5 p17's raw separator shape, plus U+2029, CRLF and non-BMP.
    let source = "import * as stylex from '@stylexjs/stylex';\n// a😀\u{2028}b\u{2029}c\r\nmarkerZ();\rbare();\nlast();";
    let map = identity_map(source);
    validate_source_map(&map, Some(source)).unwrap();
    let mut generated = Position::default();
    let mut original = Position::default();
    let mut queries = Vec::new();
    let mut positions = Vec::new();
    for character in source.chars() {
        queries.push(json!({"line":generated.line+1,"column":generated.column}));
        positions.push(expected(
            "actual/input.js",
            original.line,
            original.column,
            None,
        ));
        if character.len_utf16() == 2 {
            queries.push(json!({"line":generated.line+1,"column":generated.column+1}));
            positions.push(expected(
                "actual/input.js",
                original.line,
                original.column + 1,
                None,
            ));
        }
        generated.advance_generated(character);
        original.advance(character);
    }
    queries.push(json!({"line":generated.line+1,"column":generated.column}));
    positions.push(expected(
        "actual/input.js",
        original.line,
        original.column,
        None,
    ));
    for (probe, position) in probes(&map, queries).iter().zip(&positions) {
        both(probe, position);
    }
    assert_eq!(map["sourcesContent"][0], source);
}

#[test]
fn split_crlf_chunks_and_non_bmp_preserve_exact_identity() {
    let source = "😀\r\n\u{2028}x\u{2029}y\n";
    let mut split = MappingWriter::new();
    let mut original = Position::default();
    for character in source.chars() {
        split.text(&character.to_string(), original, true);
        original.advance(character);
    }
    split.map(original);
    assert_eq!(split.finish(), identity_map(source)["mappings"]);
}

#[test]
fn inserted_constant_origins_are_compressed_per_lf_line_not_per_character() {
    let source = "😀\u{2028}x\u{2029}y\r\nnext\n";
    let origin = Position {
        line: 4,
        column: 7,
        cr: false,
    };
    let mut writer = MappingWriter::new();
    writer.text(source, origin, false);
    let map = json!({"version":3,"sources":["real.js"],"names":[],"mappings":writer.finish()});
    assert_eq!(map["mappings"].as_str().unwrap().split(',').count(), 1);
    assert_eq!(map["mappings"].as_str().unwrap().split(';').count(), 3);
    let result = probes(
        &map,
        vec![json!({"line":1,"column":7}), json!({"line":2,"column":3})],
    );
    for probe in &result {
        both(probe, &expected("real.js", 4, 7, None));
    }
}

#[test]
fn no_upstream_is_validated_metadata_identity() {
    let mut map = identity_map("x😀\r\n");
    map["sourceRoot"] = json!("../actual/root/");
    map["x_custom"] = json!({"preserve":true});
    assert_eq!(compose_source_maps(&map, None).unwrap(), map);
    map["mappings"] = json!("?");
    assert!(compose_source_maps(&map, None).is_err());
}

#[test]
fn composition_preserves_real_upstream_metadata_and_never_adds_column_offsets() {
    let source = "0123456789😀\nabcdefgh";
    let mut direct = fixture(
        &[
            (0, 0, Some((0, 0, 3, Some(0)))),
            (0, 4, Some((0, 0, 9, Some(0)))),
            (0, 8, Some((0, 1, 4, Some(1)))),
            (1, 0, None),
        ],
        &["intermediate.js"],
        &["localName", "upstreamName"],
    );
    direct["sourcesContent"] = json!([source]);
    direct["file"] = json!("output.js");
    direct["sourceRoot"] = json!("/stale/intermediate/root/");
    let mut upstream = fixture(
        &[
            (0, 0, Some((0, 2, 11, Some(0)))),
            (0, 5, Some((1, 0, 4, None))),
            (1, 0, Some((0, 3, 8, None))),
        ],
        &["src/actual.ts", "../shared/helper.ts"],
        &["upstreamName"],
    );
    upstream["file"] = json!("/build/intermediate.js");
    upstream["sourceRoot"] = json!("https://example.org/repo/");
    upstream["sourcesContent"] = json!([
        "// actual\n\nconst realSource = 1;\nexport const live=1;",
        "const helper = 2;"
    ]);
    upstream["ignoreList"] = json!([1]);
    let output = compose_source_maps(&direct, Some(&upstream)).unwrap();
    for field in ["sources", "sourcesContent", "sourceRoot", "ignoreList"] {
        assert_eq!(output[field], upstream[field]);
    }
    assert_eq!(output["file"], "output.js");
    assert_eq!(output["names"], json!(["upstreamName", "localName"]));
    let result = probes(
        &output,
        vec![
            json!({"line":1,"column":0}),
            json!({"line":1,"column":6}),
            json!({"line":1,"column":8}),
            json!({"line":2,"column":0}),
        ],
    );
    both(
        &result[0],
        &expected(
            "https://example.org/repo/src/actual.ts",
            2,
            11,
            Some("upstreamName"),
        ),
    );
    // direct original column 9 -> upstream GLB col5 -> original col4, NOT col8.
    both(
        &result[1],
        &expected(
            "https://example.org/shared/helper.ts",
            0,
            4,
            Some("localName"),
        ),
    );
    both(
        &result[2],
        &expected(
            "https://example.org/repo/src/actual.ts",
            3,
            8,
            Some("upstreamName"),
        ),
    );
    both(
        &result[3],
        &json!({"source":null,"line":null,"column":null,"name":null}),
    );
}

#[test]
fn composition_uses_same_line_glb_and_retains_unmapped_boundaries() {
    let direct = fixture(
        &[
            (0, 0, Some((0, 0, 1, None))),
            (0, 1, Some((0, 0, 3, None))),
            (0, 2, Some((0, 0, 5, None))),
            (0, 3, Some((0, 0, 8, None))),
            (1, 0, Some((0, 1, 10, None))),
            (2, 0, Some((0, 2, 0, None))),
        ],
        &["intermediate.js"],
        &[],
    );
    let input = fixture(
        &[
            (0, 2, Some((0, 7, 12, None))),
            (0, 5, None),
            (0, 7, Some((0, 8, 4, None))),
            (2, 0, Some((0, 9, 0, None))),
        ],
        &["actual.ts"],
        &[],
    );
    let map = compose_source_maps(&direct, Some(&input)).unwrap();
    let result = probes(
        &map,
        vec![
            json!({"line":1,"column":0}),
            json!({"line":1,"column":1}),
            json!({"line":1,"column":2}),
            json!({"line":1,"column":3}),
            json!({"line":2,"column":0}),
            json!({"line":3,"column":0}),
        ],
    );
    let unmapped = json!({"source":null,"line":null,"column":null,"name":null});
    both(&result[0], &unmapped);
    both(&result[1], &expected("actual.ts", 7, 12, None));
    both(&result[2], &unmapped);
    both(&result[3], &expected("actual.ts", 8, 4, None));
    both(&result[4], &unmapped); // never bleed from line0 into empty line1
    both(&result[5], &expected("actual.ts", 9, 0, None));
}

#[test]
fn js_intermediate_coordinates_are_explicitly_bridged_to_lf_upstream() {
    let intermediate = "/*😀\u{2028}x\u{2029}y*/marker();\r\nend();\rb();";
    let direct = identity_map(intermediate);
    let mut generated = Position::default();
    let mut points = Vec::new();
    for character in intermediate.chars() {
        points.push((
            generated.line,
            generated.column,
            Some((0, generated.line as i64, generated.column as i64, Some(0))),
        ));
        if character.len_utf16() == 2 {
            points.push((
                generated.line,
                generated.column + 1,
                Some((
                    0,
                    generated.line as i64,
                    generated.column as i64 + 1,
                    Some(0),
                )),
            ));
        }
        generated.advance_generated(character);
    }
    points.push((
        generated.line,
        generated.column,
        Some((0, generated.line as i64, generated.column as i64, Some(0))),
    ));
    let mut upstream = fixture(&points, &["actual/upstream.ts"], &["actualMarker"]);
    upstream["sourcesContent"] = json!([intermediate]);
    upstream["file"] = json!("input.js"); // direct source actual/input.js basename alias
    let composed = compose_source_maps_with_options(
        &direct,
        Some(&upstream),
        ComposeOptions {
            intermediate_positions: IntermediatePositions::JavaScript,
            generated_source: Some(intermediate),
        },
    )
    .unwrap();
    let mut queries = Vec::new();
    let mut wanted = Vec::new();
    let mut position = Position::default();
    // Query visible token starts, not the LF in split CRLF (JS has already reset).
    for (offset, character) in intermediate.char_indices() {
        if ["marker", "end", "b()"]
            .iter()
            .any(|token| intermediate[offset..].starts_with(token))
        {
            queries.push(json!({"line":position.line+1,"column":position.column}));
            wanted.push(expected(
                "actual/upstream.ts",
                position.line,
                position.column,
                Some("actualMarker"),
            ));
        }
        position.advance_generated(character);
    }
    for (probe, wanted) in probes(&composed, queries).iter().zip(&wanted) {
        both(probe, wanted);
    }
    // Coordinates past the intermediate text's lines are rejected, not converted.
    assert!(compose_source_maps(&direct, Some(&upstream)).is_err());
}

#[test]
fn conventional_identity_coverage_composes_without_guessing_parser_lines() {
    let source = "a😀\u{2028}b\u{2029}c\r\nd";
    let mut points = Vec::new();
    let mut pos = Position::default();
    for character in source.chars() {
        points.push((
            pos.line,
            pos.column,
            Some((0, pos.line as i64, pos.column as i64, None)),
        ));
        pos.advance_generated(character);
    }
    points.push((
        pos.line,
        pos.column,
        Some((0, pos.line as i64, pos.column as i64, None)),
    ));
    let mut direct = fixture(&points, &["intermediate.js"], &[]);
    direct["sourcesContent"] = json!([source]);
    let mut upstream = fixture(
        &[(0, 0, Some((0, 3, 6, None))), (1, 0, Some((0, 4, 0, None)))],
        &["actual.ts"],
        &[],
    );
    upstream["file"] = json!("intermediate.js");
    let map = compose_source_maps(&direct, Some(&upstream)).unwrap();
    let results = probes(
        &map,
        vec![json!({"line":1,"column":6}), json!({"line":2,"column":0})],
    );
    both(&results[0], &expected("actual.ts", 3, 6, None));
    both(&results[1], &expected("actual.ts", 4, 0, None));
}

#[test]
fn malformed_vlq_deltas_indices_and_indexed_maps_fail_closed() {
    let valid = fixture(&[(0, 0, Some((0, 0, 0, None)))], &["a.js"], &[]);
    for mappings in [
        "?",
        "B",
        "g",
        "ggggggggA",
        ",",
        "AAAA,",
        "AA",
        "AAA",
        "AAAAAA",
        "DAAA",
        "ACAA",
        "AADA",
        "AAAD",
        "AAAAA",
        "AAAA,CAAD",
    ] {
        let mut map = valid.clone();
        map["mappings"] = json!(mappings);
        assert!(
            validate_source_map(&map, None).is_err(),
            "accepted {mappings:?}"
        );
    }
    for patch in [
        json!({"version":2}),
        json!({"sources":[null]}),
        json!({"names":[3]}),
        json!({"sourcesContent":[]}),
        json!({"sourcesContent":[4]}),
        json!({"sections":[]}),
        json!({"sourceRoot":5}),
        json!({"ignoreList":[1]}),
        json!({"x_google_ignoreList":[-1]}),
    ] {
        let mut map = valid.clone();
        map.as_object_mut()
            .unwrap()
            .extend(patch.as_object().unwrap().clone());
        assert!(compose_source_maps(&map, None).is_err(), "accepted {patch}");
        assert!(
            compose_source_maps(&valid, Some(&map)).is_err(),
            "accepted upstream {patch}"
        );
    }
    let mut overflow = valid.clone();
    overflow["mappings"] = json!(format!("{}AAA,CAAA", vlq(i32::MAX as i64)));
    assert!(validate_source_map(&overflow, None).is_err());
}

#[test]
fn range_and_file_identity_validation_does_not_accept_wrong_maps() {
    let mut direct = fixture(
        &[(0, 0, Some((0, 0, 2, None)))],
        &["/build/intermediate.js"],
        &[],
    );
    direct["sourcesContent"] = json!(["x"]);
    let upstream = fixture(&[(0, 0, Some((0, 0, 0, None)))], &["actual.ts"], &[]);
    assert!(compose_source_maps(&direct, Some(&upstream)).is_err()); // direct original beyond content
    direct["sourcesContent"] = json!(["abc"]);
    assert!(validate_source_map(&direct, Some("")).is_ok()); // col0 EOF allowed
    let mut wrong = upstream.clone();
    wrong["file"] = json!("/other/intermediate.js");
    assert!(compose_source_maps(&direct, Some(&wrong)).is_err()); // same basename, distinct explicit paths
    wrong["file"] = json!("different.js");
    assert!(compose_source_maps(&direct, Some(&wrong)).is_err());
    wrong["file"] = json!("intermediate.js");
    assert!(compose_source_maps(&direct, Some(&wrong)).is_ok());
    let mut shifted = upstream.clone();
    shifted["mappings"] = json!("IAAA");
    assert!(compose_source_maps(&direct, Some(&shifted)).is_err()); // upstream generated col4 > text3
    let mut oversized = direct.clone();
    oversized["mappings"] = json!("CAAA");
    assert!(validate_source_map(&oversized, Some("")).is_err());
    oversized["mappings"] = json!(";AAAA");
    assert!(validate_source_map(&oversized, Some("a")).is_err());
    let options = ComposeOptions {
        intermediate_positions: IntermediatePositions::JavaScript,
        generated_source: None,
    };
    direct.as_object_mut().unwrap().remove("sourcesContent");
    assert!(compose_source_maps_with_options(&direct, Some(&upstream), options).is_err());
    direct["sources"] = json!(["a", "b"]);
    assert!(compose_source_maps(&direct, Some(&upstream)).is_err());
}

#[test]
fn identical_duplicate_segments_are_coherent_conflicting_ties_rejected() {
    let mut map = fixture(&[(0, 0, Some((0, 0, 0, None)))], &["a.js"], &[]);
    map["mappings"] = json!("AAAA,AAAA");
    assert!(validate_source_map(&map, None).is_ok());
    map["mappings"] = json!("AAAA,AAAC");
    assert!(validate_source_map(&map, None).is_err());
    map["mappings"] = json!("AAAA,A");
    assert!(validate_source_map(&map, None).is_err());
}

#[test]
fn empty_maps_and_null_contents_do_not_invent_sources() {
    let direct = json!({"version":3,"sources":["intermediate.js"],"names":[],"mappings":""});
    let upstream = json!({"version":3,"sources":["actual.ts"],"names":[],"mappings":"", "sourcesContent":[null],"sourceRoot":"../real/"});
    let output = compose_source_maps(&direct, Some(&upstream)).unwrap();
    assert_eq!(output["sourcesContent"], json!([null]));
    assert_eq!(output["sources"], json!(["actual.ts"]));
    assert_eq!(output["sourceRoot"], "../real/");
    assert_eq!(output["mappings"], "");
    let absent = json!({"version":3,"sources":[],"names":[],"mappings":""});
    let output = compose_source_maps(&direct, Some(&absent)).unwrap();
    assert_eq!(output["sources"], json!([]));
    assert!(output.get("sourcesContent").is_none());
    assert!(output.get("sourceRoot").is_none());
}

#[test]
fn coordinate_limits_round_trip_through_both_consumers_and_reject_overflow() {
    let maximum = (1i64 << 30) - 1;
    let map = fixture(
        &[(0, maximum as usize, Some((0, 0, maximum, None)))],
        &["real.ts"],
        &[],
    );
    validate_source_map(&map, None).unwrap();
    let result = probes(&map, vec![json!({"line":1,"column":maximum})]);
    both(&result[0], &expected("real.ts", 0, maximum as usize, None));
    for mappings in [
        format!("{}AAA,CAAA", vlq(maximum)),
        format!("AA{}A,AACA", vlq(maximum)),
        format!("AAA{},AAAC", vlq(maximum)),
    ] {
        let mut bad = map.clone();
        bad["mappings"] = json!(mappings);
        assert!(validate_source_map(&bad, None).is_err());
    }
    let mut too_many_lines = map.clone();
    too_many_lines["mappings"] = json!(";".repeat(1_000_000));
    assert!(validate_source_map(&too_many_lines, None).is_err());
    let empty = json!({"version":3,"sources":[],"names":[],"mappings":""});
    assert!(validate_source_map(&empty, Some(&"\n".repeat(1_000_000))).is_err());
}

#[test]
fn seeded_composition_matches_two_stage_consumer_lookups() {
    let mut state = 0x51c3_99a7u64;
    let mut random = |range: u64| {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (state >> 32) % range
    };
    let mut cases = Vec::new();
    for _ in 0..200 {
        let mut upstream_points = Vec::new();
        for line in 0..3 {
            let mut column = random(3) as usize;
            for _ in 0..5 {
                let original = if random(4) == 0 {
                    None
                } else {
                    Some((
                        random(2) as i64,
                        random(20) as i64,
                        random(40) as i64,
                        if random(3) == 0 {
                            None
                        } else {
                            Some(random(2) as i64)
                        },
                    ))
                };
                upstream_points.push((line, column, original));
                column += random(5) as usize + 1;
            }
        }
        let upstream = fixture(
            &upstream_points,
            &["actual/a.ts", "actual/b.ts"],
            &["upstreamName", "sharedName"],
        );
        let mut direct_points = Vec::new();
        for line in 0..2 {
            let mut column = random(3) as usize;
            for _ in 0..7 {
                let original = if random(5) == 0 {
                    None
                } else {
                    Some((
                        0,
                        random(4) as i64,
                        random(26) as i64,
                        if random(3) == 0 {
                            None
                        } else {
                            Some(random(2) as i64)
                        },
                    ))
                };
                direct_points.push((line, column, original));
                column += random(4) as usize + 1;
            }
        }
        let direct = fixture(
            &direct_points,
            &["intermediate.js"],
            &["directName", "sharedName"],
        );
        let composed = compose_source_maps(&direct, Some(&upstream)).unwrap();
        cases.push(json!({"direct":direct,"upstream":upstream,"composed":composed}));
    }
    const DIFFERENTIAL: &str = r#"
const fs=require('node:fs');
const assert=require('node:assert/strict');
const {TraceMap,originalPositionFor}=require('@jridgewell/trace-mapping');
const {SourceMapConsumer}=require('source-map');
const cases=JSON.parse(fs.readFileSync(0,'utf8')).cases;
let count=0;
for(const item of cases) {
  const direct=new TraceMap(item.direct), upstream=new TraceMap(item.upstream);
  const trace=new TraceMap(item.composed), legacy=new SourceMapConsumer(item.composed);
  for(let line=1;line<=2;line++) for(let column=0;column<27;column++) {
    const query={line,column};
    const intermediate=originalPositionFor(direct,query);
    let wanted={source:null,line:null,column:null,name:null};
    if(intermediate.source!==null) {
      wanted=originalPositionFor(upstream,{line:intermediate.line,column:intermediate.column});
      if(wanted.source!==null && wanted.name===null) wanted.name=intermediate.name;
    }
    assert.deepEqual(originalPositionFor(trace,query),wanted);
    assert.deepEqual(legacy.originalPositionFor(query),wanted);
    count++;
  }
}
process.stdout.write(JSON.stringify({count}));
"#;
    assert_eq!(node(DIFFERENTIAL, &json!({"cases":cases}))["count"], 10_800);
}

#[test]
fn babel_produced_input_map_traces_to_actual_upstream_source_and_names() {
    const BABEL_UPSTREAM: &str = r#"
const fs = require('node:fs');
const babel = require('@babel/core');
const source = JSON.parse(fs.readFileSync(0,'utf8')).source;
const output = babel.transformSync(source, {
  filename: '/actual/src/upstream.js', sourceFileName: 'src/upstream.js',
  sourceRoot: 'https://example.org/actual/', sourceMaps:true,
  babelrc:false, configFile:false,
  plugins:[() => ({visitor:{Identifier(p){
    if(p.node.name === 'originalName') p.node.name = 'generatedName';
  }}})],
});
const indices = [...source.matchAll(/originalName/g)].map(match=>match.index);
const original = indices.map(index=>{
  const lines=source.slice(0,index).split(/\r\n|\r|\n|\u2028|\u2029/);
  return {source:'https://example.org/actual/src/upstream.js',line:lines.length,
    column:lines.at(-1).length,name:'originalName'};
});
const queries = [...output.code.matchAll(/generatedName/g)].map(match=>{
  const lines=output.code.slice(0,match.index).split('\n');
  return {line:lines.length,column:lines.at(-1).length};
});
process.stdout.write(JSON.stringify({code:output.code,map:output.map,original,queries}));
"#;
    let source = "/* actual 😀\u{2028}next\u{2029}line */\r\nexport function originalName(value) { return value + 1; }\r\noriginalName(2);";
    let upstream = node(BABEL_UPSTREAM, &json!({"source":source}));
    let code = upstream["code"].as_str().unwrap();
    let direct = identity_map(code);
    let output = compose_source_maps_with_options(
        &direct,
        Some(&upstream["map"]),
        ComposeOptions {
            intermediate_positions: IntermediatePositions::JavaScript,
            generated_source: Some(code),
        },
    )
    .unwrap();
    assert_eq!(output["sources"], upstream["map"]["sources"]);
    assert_eq!(output["sourceRoot"], upstream["map"]["sourceRoot"]);
    assert_eq!(output["sourcesContent"], json!([source]));
    assert_eq!(output["names"], upstream["map"]["names"]);
    let queries = upstream["queries"].as_array().unwrap().clone();
    let original = upstream["original"].as_array().unwrap();
    assert_eq!(queries.len(), 2);
    for (probe, wanted) in probes(&output, queries).iter().zip(original) {
        both(probe, wanted);
    }
}
