//! Tests for UidGenerator across all counter modes (Global, Local, ThreadLocal,
//! ThreadUnique).

use std::sync::atomic::AtomicUsize;

use crate::uid_generator::*;
use rustc_hash::FxHashMap;
use stylex_enums::counter_mode::CounterMode;

#[test]
fn test_global_counter_consistency() {
  let mut gen1 = UidGenerator::new("test_global_counter_consistency", CounterMode::_Global);
  gen1.clear();
  let gen2 = UidGenerator::new("test_global_counter_consistency", CounterMode::_Global);

  assert_eq!(gen1.generate(), "_test_global_counter_consistency");
  assert_eq!(gen2.generate(), "_test_global_counter_consistency2");
  assert_eq!(gen1.generate(), "_test_global_counter_consistency3");
}

#[test]
fn test_thread_unique_identifiers() {
  let generator = UidGenerator::new("test", CounterMode::_ThreadUnique);
  let id1 = generator.generate();
  let id2 = generator.generate();

  assert!(id1.starts_with("_test_"));
  assert!(id2.starts_with("_test_"));
  assert_ne!(id1, id2);
}

#[test]
fn test_clear_local() {
  let mut uid = UidGenerator::new("clr", CounterMode::Local);
  assert_eq!(uid.generate(), "_clr");
  assert_eq!(uid.generate(), "_clr2");
  uid.clear();
  assert_eq!(uid.generate(), "_clr");
}

#[test]
fn test_clear_thread_local() {
  let mut uid = UidGenerator::new("tclr", CounterMode::ThreadLocal);
  assert_eq!(uid.generate(), "_tclr");
  assert_eq!(uid.generate(), "_tclr2");
  uid.clear();
  assert_eq!(uid.generate(), "_tclr");
}

#[test]
fn test_clear_thread_unique() {
  let mut uid = UidGenerator::new("tuclr", CounterMode::_ThreadUnique);
  uid.clear();
}

#[test]
fn test_generate_ident_returns_valid_ident() {
  let uid = UidGenerator::new("id", CounterMode::Local);
  let ident = uid.generate_ident();
  assert_eq!(ident.sym.as_ref(), "_id");
  let ident2 = uid.generate_ident();
  assert_eq!(ident2.sym.as_ref(), "_id2");
}

#[test]
#[should_panic(expected = "Missing global counter for prefix")]
fn test_get_global_counter_or_panic_panics_when_missing() {
  let counters: FxHashMap<String, AtomicUsize> = FxHashMap::default();
  let _ = get_global_counter_or_panic(&counters, "missing");
}
