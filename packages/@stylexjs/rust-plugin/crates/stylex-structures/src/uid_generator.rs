use std::{
  fmt::Write,
  sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
  },
  thread,
};

use rustc_hash::FxHashMap;
use stylex_enums::counter_mode::CounterMode;
use stylex_macros::stylex_panic;
use swc_core::ecma::ast::Ident;

use once_cell::sync::Lazy;

// Global counters map, protected by Mutex for thread safety
static GLOBAL_COUNTERS: Lazy<Mutex<FxHashMap<String, AtomicUsize>>> =
  Lazy::new(|| Mutex::new(FxHashMap::default()));

// Thread-local counters for test isolation
thread_local! {
  static THREAD_LOCAL_COUNTERS: std::cell::RefCell<FxHashMap<String, usize>> =
    std::cell::RefCell::new(FxHashMap::default());
}

pub struct UidGenerator {
  prefix: String,
  mode: CounterMode,
  local_counter: AtomicUsize,
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn lock_global_counters() -> std::sync::MutexGuard<'static, FxHashMap<String, AtomicUsize>> {
  match GLOBAL_COUNTERS.lock() {
    Ok(c) => c,
    Err(e) => stylex_panic!("GLOBAL_COUNTERS mutex poisoned: {}", e),
  }
}

pub(crate) fn get_global_counter_or_panic<'a>(
  counters: &'a FxHashMap<String, AtomicUsize>,
  prefix: &str,
) -> &'a AtomicUsize {
  match counters.get(prefix) {
    Some(c) => c,
    None => stylex_panic!("Missing global counter for prefix '{}'", prefix),
  }
}

impl UidGenerator {
  /// Creates a new UidGenerator with the given prefix and counter mode.
  pub fn new(prefix: &str, mode: CounterMode) -> Self {
    match mode {
      CounterMode::_Global => {
        // Ensure the counter for this prefix exists in global counters
        let mut counters = lock_global_counters();
        counters
          .entry(prefix.to_string())
          .or_insert_with(|| AtomicUsize::new(1));
        drop(counters);
      },
      CounterMode::Local | CounterMode::ThreadLocal | CounterMode::_ThreadUnique => {
        // These modes don't need global counter initialization
      },
    }

    Self {
      prefix: prefix.to_string(),
      mode,
      local_counter: AtomicUsize::new(1),
    }
  }

  pub fn clear(&mut self) {
    match self.mode {
      CounterMode::_Global => {
        let mut counters = lock_global_counters();
        counters.remove(&self.prefix);
      },
      CounterMode::Local => {
        self.local_counter.store(1, Ordering::SeqCst);
      },
      CounterMode::ThreadLocal => {
        THREAD_LOCAL_COUNTERS.with(|counters| {
          counters.borrow_mut().remove(&self.prefix);
        });
      },
      CounterMode::_ThreadUnique => {
        // Thread unique mode doesn't maintain persistent counters
      },
    }
  }

  pub fn generate(&self) -> String {
    match self.mode {
      CounterMode::_Global => {
        let counters = lock_global_counters();
        let counter = get_global_counter_or_panic(&counters, &self.prefix);
        let count = counter.fetch_add(1, Ordering::SeqCst);

        prefixed_count(&self.prefix, count)
      },
      CounterMode::Local => {
        let count = self.local_counter.fetch_add(1, Ordering::SeqCst);

        prefixed_count(&self.prefix, count)
      },
      CounterMode::ThreadLocal => {
        let count = THREAD_LOCAL_COUNTERS.with(|counters| {
          let mut counters = counters.borrow_mut();
          match counters.get_mut(&self.prefix) {
            Some(counter) => {
              let current_count = *counter;
              *counter += 1;
              current_count
            },
            None => {
              counters.insert(self.prefix.clone(), 2);
              1
            },
          }
        });

        prefixed_count(&self.prefix, count)
      },
      CounterMode::_ThreadUnique => {
        let thread_id = thread::current().id();
        let count = self.local_counter.fetch_add(1, Ordering::SeqCst);

        let suffix = count_suffix(count);
        let mut result = String::with_capacity(self.prefix.len() + suffix.len() + 24);
        result.push('_');
        result.push_str(&self.prefix);
        result.push('_');
        let _ = write!(result, "{thread_id:?}");
        result.push_str(&suffix);
        result
      },
    }
  }

  /// Generates a unique identifier.
  pub fn generate_ident(&self) -> Ident {
    let unique_name = self.generate();

    Ident::from(unique_name.as_str())
  }
}

fn count_suffix(count: usize) -> String {
  if count < 2 {
    String::default()
  } else {
    count.to_string()
  }
}

fn prefixed_count(prefix: &str, count: usize) -> String {
  let suffix = count_suffix(count);
  let mut result = String::with_capacity(prefix.len() + suffix.len() + 1);
  result.push('_');
  result.push_str(prefix);
  result.push_str(&suffix);
  result
}

#[cfg(test)]
#[path = "tests/uid_generator_test.rs"]
mod tests;
