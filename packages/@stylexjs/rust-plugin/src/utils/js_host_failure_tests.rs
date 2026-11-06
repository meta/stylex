//! Compiler-failure causes: attached by the compiler function that failed,
//! reported only for the operation its own exception made fail.
use super::*;
use crate::utils::host::{call_closure, with_callback, HOST_CALLBACK_THREW};

/// A host whose `@call` lets the exception with `cause` escape (as JavaScript
/// does when user code does not catch it), and whose `@get` fails plainly.
fn escaping(cause: Option<u64>) -> Rc<crate::utils::host::HostCallback> {
    Rc::new(
        move |id: &str, _: &[HostValue]| -> anyhow::Result<HostValue> {
            match id {
                "@call" => Err(anyhow::Error::new(WireFailure {
                    message: HOST_CALLBACK_THREW.to_owned(),
                    cause,
                })),
                _ => Err(anyhow!("plain failure")),
            }
        },
    )
}

fn failing_function(host: &Rc<dyn HostEnv>, cause: bool) -> String {
    let CoreValue::Closure(function) = host
        .register_causal_function(Rc::new(move |_this, _args| {
            Err(HostCallbackFailure {
                error: HostError {
                    message: "evaluation failed".to_owned(),
                },
                cause: cause.then(|| Rc::new(42u32) as HostFailureCause),
            })
        }))
        .unwrap()
    else {
        panic!("a registered function is a closure value");
    };
    function.id.to_string()
}

fn callee(host: &Rc<dyn HostEnv>) -> HostRef {
    HostRef {
        owner: host.owner(),
        id: "f1".into(),
        kind: HostKind::Function,
    }
}

#[test]
fn the_exception_of_a_compiler_failure_carries_its_cause() {
    with_callback(escaping(None), || {
        let (environment, _) = js_host_environment(&serde_json::json!({}));
        let host = environment.host;
        let id = failing_function(&host, true);
        let failure =
            WireFailure::from_error(call_closure(&id, &HostValue::Undefined, &[]).unwrap_err());
        assert_eq!(failure.message, "evaluation failed");
        let cause = failure.cause.expect("a compiler failure has a cause");

        // Its exception escaped the next operation: the cause comes back.
        with_callback(escaping(Some(cause)), || {
            assert!(host
                .call(&callee(&host), &CoreValue::Undefined, &[])
                .is_err());
            let found = host.failure_cause().expect("the escaped failure's cause");
            assert_eq!(found.downcast_ref::<u32>(), Some(&42));
            // The next operation resets it, and a plain failure has none.
            assert!(host.get(&callee(&host), "x").is_err());
            assert!(host.failure_cause().is_none());
        });
        // User code caught it and threw something else: no cause.
        with_callback(escaping(None), || {
            assert!(host
                .call(&callee(&host), &CoreValue::Undefined, &[])
                .is_err());
            assert!(host.failure_cause().is_none());
        });
    });
}

#[test]
fn a_failure_without_a_cause_stays_plain() {
    with_callback(escaping(None), || {
        let (environment, _) = js_host_environment(&serde_json::json!({}));
        let id = failing_function(&environment.host, false);
        let failure =
            WireFailure::from_error(call_closure(&id, &HostValue::Undefined, &[]).unwrap_err());
        assert_eq!(failure.message, "evaluation failed");
        assert!(failure.cause.is_none());
    });
}

#[test]
fn causes_never_cross_hosts() {
    with_callback(escaping(None), || {
        let (first, _) = js_host_environment(&serde_json::json!({}));
        let (second, _) = js_host_environment(&serde_json::json!({}));
        let id = failing_function(&first.host, true);
        let cause = call_closure(&id, &HostValue::Undefined, &[])
            .unwrap_err()
            .downcast::<WireFailure>()
            .unwrap()
            .cause;
        with_callback(escaping(cause), || {
            let host = &second.host;
            assert!(host
                .call(&callee(host), &CoreValue::Undefined, &[])
                .is_err());
            assert!(host.failure_cause().is_none());
        });
    });
}
