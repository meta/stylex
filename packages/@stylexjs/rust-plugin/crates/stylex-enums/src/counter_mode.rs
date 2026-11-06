/// Counter mode for UidGenerator
#[derive(Clone, Debug)]
pub enum CounterMode {
  /// Use global counters shared across all instances (default behavior).
  /// This ensures unique identifiers across the entire application.
  _Global,
  /// Use local counters specific to each instance (legacy behavior).
  /// Each UidGenerator instance maintains its own counter, which can lead to
  /// duplicate identifiers across different instances with the same prefix.
  Local,
  /// Use thread-local counters for test isolation.
  /// Each thread gets its own set of counters, perfect for parallel testing.
  ThreadLocal,
  /// Use a combination of thread ID and prefix for maximum uniqueness.
  /// This mode uses thread ID as part of the identifier generation.
  _ThreadUnique,
}
