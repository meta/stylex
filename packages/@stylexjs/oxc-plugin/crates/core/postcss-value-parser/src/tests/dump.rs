//! A node tree flattened to text for single-assertion comparison.
//!
//! Must match `scripts/generate-value-parser-cases.mjs` character for character.

use crate::Node;

/// The canonical quoted form of a string field.
fn dump_string(text: &str) -> String {
  let mut out = String::from("\"");

  for ch in text.chars() {
    match ch {
      '\\' => out.push_str("\\\\"),
      '"' => out.push_str("\\\""),
      '\n' => out.push_str("\\n"),
      '\r' => out.push_str("\\r"),
      '\t' => out.push_str("\\t"),
      _ if (ch as u32) < 0x20 || ch as u32 == 0x7f => {
        out.push_str(&format!("\\u{:04x}", ch as u32));
      },
      _ => out.push(ch),
    }
  }

  out.push('"');
  out
}

fn dump_into(nodes: &[Node], indent: usize, out: &mut Vec<String>) {
  for node in nodes {
    let mut line = format!(
      "{}{} {} {}..{}",
      " ".repeat(indent),
      node.kind,
      dump_string(&node.value),
      node.source_index,
      node.source_end_index
    );

    if let Some(before) = node.before.as_deref() {
      line.push_str(&format!(" before={}", dump_string(before)));
    }
    if let Some(after) = node.after.as_deref() {
      line.push_str(&format!(" after={}", dump_string(after)));
    }
    if let Some(quote) = node.quote {
      line.push_str(&format!(" quote={}", dump_string(&quote.to_string())));
    }
    if node.unclosed {
      line.push_str(" unclosed");
    }
    if let Some(children) = node.nodes.as_deref() {
      line.push_str(&format!(" nodes={}", children.len()));
    }

    out.push(line);

    if let Some(children) = node.nodes.as_deref() {
      dump_into(children, indent + 2, out);
    }
  }
}

/// One line per node, nested nodes indented by two.
pub(super) fn dump(nodes: &[Node]) -> String {
  let mut lines = Vec::new();
  dump_into(nodes, 0, &mut lines);
  lines.join("\n")
}
