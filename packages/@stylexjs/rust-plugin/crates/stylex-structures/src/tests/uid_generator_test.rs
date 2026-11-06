//! Tests for UidGenerator across all counter modes (Global, Local, ThreadLocal,
//! ThreadUnique).

use std::sync::atomic::{AtomicUsize, Ordering};

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
fn test_local_counter_isolation() {
  let gen1 = UidGenerator::new("test", CounterMode::Local);
  let gen2 = UidGenerator::new("test", CounterMode::Local);

  assert_eq!(gen1.generate(), "_test");
  assert_eq!(gen2.generate(), "_test"); // Same because local counters are independent
  assert_eq!(gen1.generate(), "_test2");
  assert_eq!(gen2.generate(), "_test2"); // Each maintains its own counter
}

#[test]
fn test_thread_local_counter() {
  let gen1 = UidGenerator::new("test", CounterMode::ThreadLocal);
  let gen2 = UidGenerator::new("test", CounterMode::ThreadLocal);

  assert_eq!(gen1.generate(), "_test");
  assert_eq!(gen2.generate(), "_test2"); // Shared within same thread
  assert_eq!(gen1.generate(), "_test3");
}

#[test]
fn test_thread_unique_identifiers() {
  let generator = UidGenerator::new("test", CounterMode::_ThreadUnique);
  let id1 = generator.generate();
  let id2 = generator.generate();

  // Both should contain thread ID and be unique
  assert!(id1.starts_with("_test_"));
  assert!(id2.starts_with("_test_"));
  assert_ne!(id1, id2);
}

#[test]
fn test_parallel_thread_local_isolation() {
  use std::{
    sync::{Arc, Barrier},
    thread,
  };

  let barrier = Arc::new(Barrier::new(2));
  let mut handles = vec![];

  for thread_num in 0..2 {
    let barrier = Arc::clone(&barrier);
    let handle = thread::spawn(move || {
      let generator = UidGenerator::new("test", CounterMode::ThreadLocal);

      // Wait for both threads to be ready
      barrier.wait();

      // Each thread should get the same sequence independently
      let results = (0..3).map(|_| generator.generate()).collect::<Vec<_>>();
      (thread_num, results)
    });
    handles.push(handle);
  }

  let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

  // Both threads should generate the same sequence because they're isolated
  assert_eq!(results[0].1, vec!["_test", "_test2", "_test3"]);
  assert_eq!(results[1].1, vec!["_test", "_test2", "_test3"]);
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
fn test_clear_global() {
  let mut uid = UidGenerator::new("gclr", CounterMode::_Global);
  let _ = uid.generate();
  uid.clear();
  // After clearing, creating a new generator should start fresh
  let uid2 = UidGenerator::new("gclr", CounterMode::_Global);
  assert_eq!(uid2.generate(), "_gclr");
}

#[test]
fn test_clear_thread_unique() {
  let mut uid = UidGenerator::new("tuclr", CounterMode::_ThreadUnique);
  // _ThreadUnique clear is a no-op, should not panic
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
fn test_get_global_counter_or_panic_returns_existing_counter() {
  let mut counters = FxHashMap::default();
  counters.insert("present".to_string(), AtomicUsize::new(7));

  let counter = get_global_counter_or_panic(&counters, "present");
  assert_eq!(counter.load(Ordering::SeqCst), 7);
}

#[test]
#[should_panic(expected = "Missing global counter for prefix")]
fn test_get_global_counter_or_panic_panics_when_missing() {
  let counters: FxHashMap<String, AtomicUsize> = FxHashMap::default();
  let _ = get_global_counter_or_panic(&counters, "missing");
}
