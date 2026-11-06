//! The JavaScript host bridge without Node: host services answered by a Rust
//! stand-in, so conversions, owner checks and the compiler-function registry
//! are exercised in the library's own tests (the N-API layer is not built
//! for them).
use super::*;
use crate::utils::host::{
    call_closure, isolated_operation, set_closure_ledger, with_callback, ClosureLedger,
};
use std::sync::Arc;
use stylex_structures::stylex_env::{Adoptable, HostValue as V};

fn values(id: &str, args: &[HostValue]) -> anyhow::Result<HostValue> {
    Ok(match id {
        "@get" => match &args[1] {
            HostValue::String(key) if key == "n" => HostValue::Number(-0.0),
            HostValue::String(key) if key == "u" => HostValue::Utf16(vec![0xd83d]),
            HostValue::String(key) if key == "o" => HostValue::Handle("p7".into()),
            _ => HostValue::Undefined,
        },
        "@materialize" => HostValue::Array(vec![
            HostValue::String("nullObject".into()),
            HostValue::Object(vec![("a".into(), HostValue::Number(f64::NAN))]),
        ]),
        "@call" => args[2].clone(),
        other => anyhow::bail!("unexpected host service {other}"),
    })
}

fn refuse(id: &str, _: &[HostValue]) -> anyhow::Result<HostValue> {
    anyhow::bail!("unexpected host service {id}")
}

#[test]
fn host_values_convert_losslessly_and_stay_with_their_owner() {
    let options = serde_json::json!({ "env": {
        "__stylexHandle": "p0",
        "__stylexEntries": [
            ["f", { "type": "handle", "value": "f1" }],
            ["s", { "type": "utf16", "value": [56320] }],
        ],
    }});
    with_callback(Rc::new(values), || {
        let (environment, entries) = js_host_environment(&options);
        let root = environment.env.clone().unwrap();
        assert_eq!(root.kind, HostKind::PlainObject);
        assert!(
            matches!(entries.get("f"), Some(EnvEntry::Host(V::Ref(r))) if r.kind == HostKind::Function)
        );
        assert!(matches!(entries.get("s"), Some(EnvEntry::Host(V::Utf16(u))) if u[..] == [56320]));
        let host = environment.host.clone();
        assert!(
            matches!(host.get(&root, "n").unwrap(), V::Number(n) if n == 0.0 && n.is_sign_negative())
        );
        assert!(matches!(host.get(&root, "u").unwrap(), V::Utf16(u) if u[..] == [0xd83d]));
        let object = host.get(&root, "o").unwrap();
        assert!(
            matches!(&object, V::Ref(r) if r.kind == HostKind::PlainObject && r.owner == host.owner())
        );
        // One value, one id: identity needs no round trip.
        assert!(host.same_value(&object, &object).unwrap());
        // A reference another host issued is refused, never resolved here.
        let foreign = HostRef {
            owner: host.owner() + 1000,
            id: "p7".into(),
            kind: HostKind::PlainObject,
        };
        assert!(host.get(&foreign, "n").is_err());
        assert!(matches!(
            host.materialize(object.as_ref().unwrap()).unwrap(),
            Some(Materialized::Object { null_prototype: true, entries })
                if matches!(entries[0].1, V::Number(n) if n.is_nan())
        ));
        // Only arrays, plain objects and proxies are traversed.
        let other = HostRef {
            owner: host.owner(),
            id: "o9".into(),
            kind: HostKind::Object,
        };
        assert!(host.materialize(&other).unwrap().is_none());
        // A compiler function is owner-bound and callable by id.
        let V::Closure(function) = host
            .register_function(Rc::new(|_this, args| Ok(args[0].clone())))
            .unwrap()
        else {
            panic!("a registered function is a closure value");
        };
        assert_eq!(function.owner, host.owner());
        let callee = HostRef {
            owner: host.owner(),
            id: "f1".into(),
            kind: HostKind::Function,
        };
        let foreign_function = V::Closure(HostFunctionRef {
            owner: host.owner() + 1,
            id: function.id.clone(),
        });
        assert!(host
            .call(&callee, &V::Undefined, &[foreign_function])
            .is_err());
        assert!(matches!(
            host.call(&callee, &V::Undefined, &[V::Closure(function.clone())]).unwrap(),
            V::Closure(echoed) if echoed.id == function.id && echoed.owner == host.owner()
        ));
        let result = call_closure(
            &function.id,
            &HostValue::Undefined,
            &[HostValue::Utf16(vec![0xdc00])],
        )
        .unwrap();
        assert!(matches!(result, HostValue::Utf16(units) if units == vec![0xdc00]));
    });
}

#[test]
fn adopted_literals_become_live_values_of_their_host() {
    let adopt = |id: &str, args: &[HostValue]| -> anyhow::Result<HostValue> {
        assert_eq!(id, "@adopt");
        match (&args[0], &args[2..]) {
            (HostValue::String(kind), [HostValue::Array(holes)]) if kind == "array" => {
                assert!(matches!(&holes[..], [HostValue::Number(index)] if *index == 0.0));
                Ok(HostValue::Handle("a3".into()))
            }
            (HostValue::String(kind), []) if kind == "nullObject" => {
                Ok(HostValue::Handle("p4".into()))
            }
            _ => anyhow::bail!("unexpected adopt request"),
        }
    };
    with_callback(Rc::new(adopt), || {
        let (environment, _) = js_host_environment(&serde_json::json!({}));
        let array = environment
            .host
            .adopt(Adoptable::Array(vec![None, Some(V::Number(2.0))]))
            .unwrap();
        assert_eq!(array.kind, HostKind::Array);
        assert_eq!(array.owner, environment.host.owner());
        let object = environment
            .host
            .adopt(Adoptable::Object {
                null_prototype: true,
                entries: vec![("x".into(), V::Utf16(vec![0xd800].into()))],
            })
            .unwrap();
        assert_eq!(object.kind, HostKind::PlainObject);
    });
}

#[test]
fn theme_imports_and_debug_labels_come_from_the_host() {
    let host = |id: &str, args: &[HostValue]| -> anyhow::Result<HostValue> {
        let HostValue::String(first) = &args[0] else {
            anyhow::bail!("a path is expected");
        };
        match (id, first.as_str()) {
            ("@resolveImport", "./tokens.stylex") => {
                Ok(HostValue::String("pkg:tokens.stylex.js".into()))
            }
            ("@resolveImport", "./throws") | ("@debugFile", "/fail") => {
                anyhow::bail!("user callback failed")
            }
            ("@resolveImport", _) => Ok(HostValue::Undefined),
            ("@debugFile", path) => Ok(HostValue::String(format!("n:{path}"))),
            _ => anyhow::bail!("unexpected host service {id}"),
        }
    };
    assert!(js_debug_file_namer(&serde_json::json!({})).is_none());
    assert!(js_debug_file_namer(&serde_json::json!({ "debugFilePath": "x" })).is_none());
    let namer =
        js_debug_file_namer(&serde_json::json!({ "debugFilePath": { "__stylexCallback": "0" } }))
            .unwrap();
    with_callback(Rc::new(host), || {
        let resolver = js_import_resolver();
        assert_eq!(
            resolver
                .resolve_theme_import("./tokens.stylex", "/app/a.js")
                .unwrap()
                .as_deref(),
            Some("pkg:tokens.stylex.js")
        );
        assert_eq!(
            resolver
                .resolve_theme_import("./missing", "/app/a.js")
                .unwrap(),
            None
        );
        assert!(resolver
            .resolve_theme_import("./throws", "/app/a.js")
            .unwrap_err()
            .contains("user callback failed"));
        assert_eq!(namer.debug_file_name("/app/a.js").unwrap(), "n:/app/a.js");
        assert!(namer
            .debug_file_name("/fail")
            .unwrap_err()
            .contains("user callback failed"));
    });
}

#[test]
fn released_compiler_functions_are_collected_when_the_next_operation_starts() {
    let ledger = Arc::new(ClosureLedger::default());
    set_closure_ledger(ledger.clone());
    let register = || {
        with_callback(Rc::new(refuse), || {
            register_closure(Rc::new(|_this, _args| Ok(HostValue::Null)))
        })
    };
    // JavaScript wraps (retains) each function it receives before its next
    // operation; one is released later, one never reaches JavaScript.
    let kept = register();
    ledger.retain(&kept);
    let dropped = register();
    ledger.retain(&dropped);
    let never_wrapped = register();
    ledger.release(&dropped);
    with_callback(Rc::new(refuse), || {});
    assert_eq!(ledger.live(), 1);
    assert!(call_closure(&kept, &HostValue::Undefined, &[]).is_ok());
    assert!(call_closure(&dropped, &HostValue::Undefined, &[]).is_err());
    assert!(call_closure(&never_wrapped, &HostValue::Undefined, &[]).is_err());
}

#[test]
fn nested_operations_run_isolated() {
    let mut ran = false;
    isolated_operation(|| ran = true);
    assert!(ran);
}

#[test]
fn the_node_entry_point_compiles_with_or_without_a_host() {
    let output = crate::transform_with_host(
        "export const x = 1;",
        "/app/plain.js",
        &serde_json::json!({}),
        None,
        indexmap::IndexMap::new(),
        None,
        None,
    )
    .unwrap();
    assert!(output.code.contains("x = 1"));
    assert!(output.metadata_stylex.is_empty());
}
