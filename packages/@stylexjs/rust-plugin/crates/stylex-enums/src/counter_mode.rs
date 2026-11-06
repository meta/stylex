/// Counter mode for UidGenerator
#[derive(Clone, Debug)]
pub enum CounterMode {
  /// Global counters shared across all instances (default).
  _Global,
  /// Per-instance counters (legacy); can duplicate identifiers across instances.
  Local,
  /// Thread-local counters, for test isolation.
  ThreadLocal,
  /// Thread ID plus prefix.
  _ThreadUnique,
}
