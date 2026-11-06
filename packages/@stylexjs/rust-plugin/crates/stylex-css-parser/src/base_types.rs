/*!
Base types and utilities for CSS parsing.

This module provides fundamental types and utilities used throughout the CSS parser,
including string manipulation utilities.
*/

use std::fmt::{self, Display};

/// A string slice utility that provides efficient substring operations.
#[derive(Debug, Clone)]
pub struct SubString {
  pub string: String,
  pub start_index: usize,
  pub end_index: usize,
}

impl SubString {
  /// Create a new SubString from a string
  pub fn new(s: &str) -> Self {
    Self {
      string: s.to_string(),
      start_index: 0,
      end_index: if s.is_empty() { 0 } else { s.len() - 1 },
    }
  }

  /// Check if the substring starts with the given string
  /// Uses a loop to avoid creating a new string
  pub fn starts_with(&self, s: &str) -> bool {
    let chars: Vec<char> = self.string.chars().collect();
    let search_chars: Vec<char> = s.chars().collect();

    for i in 0..search_chars.len() {
      if self.start_index + i > self.end_index
        || self.start_index + i >= chars.len()
        || chars[self.start_index + i] != search_chars[i]
      {
        return false;
      }
    }
    true
  }

  /// Get the first character of the substring
  pub fn first(&self) -> Option<char> {
    if self.start_index > self.end_index {
      return None;
    }
    self.string.chars().nth(self.start_index)
  }

  /// Get a character at a relative index from the start
  pub fn get(&self, relative_index: usize) -> Option<char> {
    let absolute_index = self.start_index + relative_index;
    if absolute_index > self.end_index {
      return None;
    }
    self.string.chars().nth(absolute_index)
  }

  /// Convert the substring to a String
  pub fn into_string(&self) -> String {
    if self.start_index > self.end_index {
      return String::new();
    }

    let chars: Vec<char> = self.string.chars().collect();
    let start = self.start_index.min(chars.len());
    let end = (self.end_index + 1).min(chars.len());

    if start >= end {
      String::new()
    } else {
      chars[start..end].iter().collect()
    }
  }

  /// Check if the substring is empty
  pub fn is_empty(&self) -> bool {
    self.start_index > self.end_index
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for SubString {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.into_string())
  }
}

#[cfg(test)]
#[path = "tests/base_types_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/base_types_test.rs"]
mod base_types_test;

#[cfg(test)]
#[path = "tests/base_types_coverage_test.rs"]
mod base_types_coverage_test;
