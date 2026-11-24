//! Source maps built while the output is spliced. Owner: emit.
//!
//! Copied text maps token by token to itself (identity). Generated text maps each identifier
//! and string token to the same token inside the span it replaced when there is one, and its
//! first token on every line to the last such anchor. Identifier tokens carry names, like
//! Babel's maps. Original positions use JavaScript line terminators (LF, CRLF, CR, U+2028,
//! U+2029) and UTF-16 columns, as Babel's parser does. Generated lines count LF only.
//! An `inputSourceMap` is composed the way Babel's `@jridgewell/remapping` does it.
//! Everything is one forward pass over the text, so the cost is linear in the file size.

use crate::fx::HashMap;
use std::borrow::Cow;
use std::hash::{BuildHasherDefault, Hasher};

use serde_json::Value;

/// A parsed upstream map: per generated line, segments sorted by column.
pub(crate) struct InputMap {
  value: Value,
  /// (generated column, source index, line, column, name index or u32::MAX). Source index
  /// u32::MAX marks a sourceless segment.
  lines: Vec<Vec<[u32; 5]>>,
}

const NONE: u32 = u32::MAX;

/// FxHash. Names are short identifiers, and SipHash showed up in profiles.
#[derive(Default)]
pub(crate) struct Fx(u64);
impl Hasher for Fx {
  fn write(&mut self, bytes: &[u8]) {
    for chunk in bytes.chunks(8) {
      let mut v = 0u64;
      for (i, b) in chunk.iter().enumerate() {
        v |= (*b as u64) << (8 * i);
      }
      self.0 = (self.0.rotate_left(5) ^ v).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
  }
  fn write_u8(&mut self, i: u8) {
    self.write_u64(i as u64);
  }
  fn write_u32(&mut self, i: u32) {
    self.write_u64(i as u64);
  }
  fn write_u64(&mut self, i: u64) {
    self.0 = (self.0.rotate_left(5) ^ i).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
  }
  fn write_usize(&mut self, i: usize) {
    self.write_u64(i as u64);
  }
  fn finish(&self) -> u64 {
    self.0
  }
}
pub(crate) type FxMap<K, V> = std::collections::HashMap<K, V, BuildHasherDefault<Fx>>;

impl InputMap {
  /// Accepts a flat v3 map as a JSON object or string. Anything else is ignored (None).
  pub(crate) fn parse(v: &Value) -> Option<InputMap> {
    let value = match v {
      Value::String(s) => serde_json::from_str(s).ok()?,
      Value::Object(_) => v.clone(),
      _ => return None,
    };
    let mappings = value.get("mappings")?.as_str()?;
    value.get("sources")?.as_array()?;
    let mut lines = vec![];
    let mut prev = [0i64; 4];
    for line in mappings.split(';') {
      let mut segs: Vec<[u32; 5]> = vec![];
      let mut col = 0i64;
      for seg in line.split(',').filter(|s| !s.is_empty()) {
        let f = decode_vlq(seg)?;
        col += f[0]?;
        let s = if f[1].is_some() {
          for i in 0..3 {
            prev[i] += f[i + 1]?;
          }
          let name = match f[4] {
            Some(d) => {
              prev[3] += d;
              prev[3] as u32
            },
            None => NONE,
          };
          [
            col as u32,
            prev[0] as u32,
            prev[1] as u32,
            prev[2] as u32,
            name,
          ]
        } else {
          [col as u32, NONE, 0, 0, NONE]
        };
        segs.push(s);
      }
      // Unsorted input lines are legal. Lookups need them sorted (stable keeps tie order).
      segs.sort_by_key(|s| s[0]);
      lines.push(segs);
    }
    Some(InputMap { value, lines })
  }

  /// trace-mapping's `traceSegment`: the last segment at or before the column on that line.
  fn trace(&self, line: u32, col: u32) -> Option<&[u32; 5]> {
    let segs = self.lines.get(line as usize)?;
    let i = segs.partition_point(|s| s[0] <= col);
    if i == 0 { None } else { Some(&segs[i - 1]) }
  }
}

fn decode_vlq(seg: &str) -> Option<[Option<i64>; 5]> {
  let mut out = [None; 5];
  let mut n = 0;
  let mut value = 0i64;
  let mut shift = 0;
  for b in seg.bytes() {
    let d = match b {
      b'A'..=b'Z' => b - b'A',
      b'a'..=b'z' => b - b'a' + 26,
      b'0'..=b'9' => b - b'0' + 52,
      b'+' => 62,
      b'/' => 63,
      _ => return None,
    } as i64;
    if shift > 60 {
      return None;
    }
    value |= (d & 31) << shift;
    if d & 32 != 0 {
      shift += 5;
      continue;
    }
    if n == 5 {
      return None;
    }
    out[n] = Some(if value & 1 == 1 {
      -(value >> 1)
    } else {
      value >> 1
    });
    n += 1;
    value = 0;
    shift = 0;
  }
  if shift != 0 || !(n == 1 || n == 4 || n == 5) {
    return None;
  }
  Some(out)
}

fn write_vlq(out: &mut String, v: i64) {
  const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
  let mut v = if v < 0 { ((-v) << 1) | 1 } else { v << 1 } as u64;
  loop {
    let mut d = (v & 31) as usize;
    v >>= 5;
    if v != 0 {
      d |= 32;
    }
    out.push(B64[d] as char);
    if v == 0 {
      break;
    }
  }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
  Space,
  Word,
  Punct,
}

#[inline]
fn class(b: u8) -> Class {
  match b {
    b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c => Class::Space,
    b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'$' | 0x80..=0xff => Class::Word,
    _ => Class::Punct,
  }
}

/// Brackets and separators start a token each. Other punctuation runs (`=>`, `===`) are one.
#[inline]
fn lone_punct(b: u8) -> bool {
  matches!(
    b,
    b'(' | b')' | b'{' | b'}' | b'[' | b']' | b';' | b',' | b'\'' | b'"' | b'`'
  )
}

/// Words that never parse as an Identifier node (contextual keywords like `default` as an
/// object key, `get` or `of` do, and Babel names them).
fn is_keyword(w: &str) -> bool {
  matches!(
    w,
    "true"
      | "false"
      | "null"
      | "this"
      | "function"
      | "return"
      | "typeof"
      | "void"
      | "new"
      | "in"
      | "instanceof"
      | "delete"
      | "if"
      | "else"
      | "var"
      | "const"
      | "class"
      | "super"
      | "import"
      | "export"
      | "throw"
      | "try"
      | "catch"
      | "switch"
      | "case"
      | "break"
      | "continue"
      | "for"
      | "while"
      | "do"
      | "with"
      | "debugger"
      | "extends"
      | "finally"
  )
}

/// Position in the original text. Lines follow JavaScript line terminators.
#[derive(Clone, Copy, Default)]
struct Cursor {
  off: usize,
  line: u32,
  col: u32,
  cr: bool,
}

impl Cursor {
  #[inline]
  fn step(&mut self, code: &[u8]) {
    let b = code[self.off];
    match b {
      b'\n' => {
        if !self.cr {
          self.line += 1;
          self.col = 0;
        }
        self.cr = false;
      },
      b'\r' => {
        self.line += 1;
        self.col = 0;
        self.cr = true;
      },
      0xe2
        if self.off + 2 < code.len()
          && code[self.off + 1] == 0x80
          && (code[self.off + 2] == 0xa8 || code[self.off + 2] == 0xa9) =>
      {
        self.line += 1;
        self.col = 0;
        self.cr = false;
        self.off += 3;
        return;
      },
      _ => {
        self.cr = false;
        if b & 0xc0 != 0x80 {
          self.col += 1 + (b >= 0xf0) as u32;
        }
      },
    }
    self.off += 1;
  }
}

/// Flow lowering: the spliced text is lowered text, and positions map back to the source
/// through the lowering's segments before anything else (an inputSourceMap) sees them.
pub(crate) struct Lowering<'c> {
  original: &'c str,
  segs: Vec<crate::parse::Seg>,
  /// Line starts of the original (JS line terminators).
  starts: Vec<usize>,
}

impl<'c> Lowering<'c> {
  pub(crate) fn new(original: &'c str, segs: &[crate::parse::Seg]) -> Self {
    let b = original.as_bytes();
    let mut starts = vec![0];
    let mut i = 0;
    while i < b.len() {
      match b[i] {
        b'\n' => starts.push(i + 1),
        b'\r' if b.get(i + 1) != Some(&b'\n') => starts.push(i + 1),
        0xe2 if i + 2 < b.len() && b[i + 1] == 0x80 && (b[i + 2] == 0xa8 || b[i + 2] == 0xa9) => {
          starts.push(i + 3);
          i += 2;
        },
        _ => {},
      }
      i += 1;
    }
    Lowering {
      original,
      segs: segs.to_vec(),
      starts,
    }
  }

  /// Original (line, UTF-16 column) of a lowered offset.
  fn pos(&self, lowered: usize) -> (u32, u32) {
    let off =
      (crate::parse::to_original(&self.segs, lowered as u32) as usize).min(self.original.len());
    let line = self.starts.partition_point(|&s| s <= off) - 1;
    let col = self.original[self.starts[line]..off].encode_utf16().count();
    (line as u32, col as u32)
  }
}

/// What the writer needs to know about the original program.
#[derive(Default)]
pub(crate) struct SourceInfo<'c> {
  /// Flow lowering, when the spliced text is not the source.
  pub(crate) lowering: Option<Lowering<'c>>,
  /// Identifier spans (start, end), sorted. Their tokens get names.
  pub(crate) idents: Vec<(u32, u32)>,
  /// Comment spans, sorted. Copied comments get no segments, as in Babel's maps.
  pub(crate) comments: Vec<(u32, u32)>,
}

/// Builds a source map while the output is spliced together.
pub(crate) struct MapWriter<'c> {
  code: &'c str,
  info: SourceInfo<'c>,
  input: Option<InputMap>,
  cur: Cursor,
  /// Cursor at the start of the span being replaced, for anchors behind `cur`.
  mark: Cursor,
  ident_i: usize,
  comment_i: usize,
  out_line: u32,
  out_col: u32,
  mappings: String,
  line_written: u32,
  line_has: bool,
  prev: [i64; 5],
  last: Option<(u32, u32, [u32; 4])>,
  names: Vec<Cow<'c, str>>,
  name_ids: FxMap<Cow<'c, str>, u32>,
  /// Composed sources: input source index -> output source index.
  src_ids: HashMap<u32, u32>,
  sources: Vec<u32>,
  index: SpanIndex<'c>,
}

impl<'c> MapWriter<'c> {
  pub(crate) fn new(code: &'c str, info: SourceInfo<'c>, input: Option<InputMap>) -> Self {
    MapWriter {
      code,
      info,
      input,
      cur: Cursor::default(),
      mark: Cursor::default(),
      ident_i: 0,
      comment_i: 0,
      out_line: 0,
      out_col: 0,
      mappings: String::new(),
      line_written: 0,
      line_has: false,
      prev: [0; 5],
      last: None,
      names: vec![],
      name_ids: FxMap::default(),
      src_ids: HashMap::default(),
      sources: vec![],
      index: SpanIndex::default(),
    }
  }

  /// Moves the original cursor to `off`.
  fn seek(&mut self, off: usize) {
    if off < self.cur.off {
      self.cur = if off >= self.mark.off {
        self.mark
      } else {
        Cursor::default()
      };
    }
    let code = self.code.as_bytes();
    while self.cur.off < off {
      self.cur.step(code);
    }
  }

  fn name_id(&mut self, name: Cow<'c, str>) -> u32 {
    if let Some(&i) = self.name_ids.get(name.as_ref()) {
      return i;
    }
    let i = self.names.len() as u32;
    self.names.push(name.clone());
    self.name_ids.insert(name, i);
    i
  }

  /// The identifier name at `off`, if an identifier starts there. Copied text moves forward,
  /// so the search starts at the last hit.
  fn ident_at(&mut self, off: u32) -> Option<&'c str> {
    let ids = &self.info.idents;
    let mut i = self.ident_i;
    if i >= ids.len() || ids[i].0 > off || (i > 0 && ids[i - 1].0 >= off) {
      i = ids.partition_point(|s| s.0 < off);
    } else {
      while i < ids.len() && ids[i].0 < off {
        i += 1;
      }
    }
    self.ident_i = i;
    let &(s, e) = ids.get(i)?;
    if s != off {
      return None;
    }
    let t = &self.code[s as usize..e as usize];
    (!t.contains('\\')).then_some(t)
  }

  /// Adds a segment at the current generated position for original (line, col).
  /// Callers position the cursor on the token first, so `cur.off` is its lowered offset.
  fn segment(&mut self, line: u32, col: u32, name: Option<&'c str>) {
    let (line, col) = match &self.info.lowering {
      Some(lw) => lw.pos(self.cur.off),
      None => (line, col),
    };
    let (mut src, mut line, mut col, mut name_id) = (
      0u32,
      line,
      col,
      name.map(|n| self.name_id(Cow::Borrowed(n))),
    );
    if let Some(input) = &self.input {
      let Some(&[_, s, l, c, n]) = input.trace(line, col) else {
        return; // remapping drops segments the upstream map does not cover
      };
      if s == NONE {
        return;
      }
      let upstream_name = (n != NONE)
        .then(|| input.value["names"].get(n as usize).and_then(Value::as_str))
        .flatten()
        .map(String::from);
      if let Some(un) = upstream_name {
        name_id = Some(self.name_id(Cow::Owned(un)));
      }
      let next = self.sources.len() as u32;
      src = *self.src_ids.entry(s).or_insert(next);
      if src == next {
        self.sources.push(s);
      }
      line = l;
      col = c;
    }
    let key = [src, line, col, name_id.unwrap_or(NONE)];
    if let Some((gl, gc, k)) = self.last {
      // Same generated position, or a redundant repeat of the previous segment on this line.
      if gl == self.out_line && (gc == self.out_col || k == key) {
        return;
      }
    }
    while self.line_written < self.out_line {
      self.mappings.push(';');
      self.line_written += 1;
      self.line_has = false;
      self.prev[0] = 0;
    }
    if self.line_has {
      self.mappings.push(',');
    }
    write_vlq(&mut self.mappings, self.out_col as i64 - self.prev[0]);
    write_vlq(&mut self.mappings, src as i64 - self.prev[1]);
    write_vlq(&mut self.mappings, line as i64 - self.prev[2]);
    write_vlq(&mut self.mappings, col as i64 - self.prev[3]);
    if let Some(n) = name_id {
      write_vlq(&mut self.mappings, n as i64 - self.prev[4]);
      self.prev[4] = n as i64;
    }
    self.prev[0] = self.out_col as i64;
    self.prev[1] = src as i64;
    self.prev[2] = line as i64;
    self.prev[3] = col as i64;
    self.line_has = true;
    self.last = Some((self.out_line, self.out_col, key));
  }

  /// Copies `code[start..end]` to `out` with a segment at every token start.
  pub(crate) fn copied(&mut self, out: &mut String, start: usize, end: usize) {
    if start >= end {
      return;
    }
    self.seek(start);
    let code = self.code.as_bytes();
    let mut prev_class = Class::Space;
    let mut prev_byte = 0u8;
    let mut i = start;
    while i < end {
      let b = code[i];
      let c = class(b);
      if c == Class::Space {
        // Whitespace: only line breaks change more than the columns.
        if b == b'\n' || b == b'\r' {
          self.advance_out(i, i + 1);
        } else {
          self.cur.col += 1;
          self.cur.cr = false;
          self.cur.off += 1;
          self.out_col += 1;
        }
        prev_class = Class::Space;
        i += 1;
        continue;
      }
      if b == b'/' {
        // Comments get no segments.
        let cs = &self.info.comments;
        let mut k = self.comment_i;
        while k < cs.len() && (cs[k].1 as usize) <= i {
          k += 1;
        }
        self.comment_i = k;
        if let Some(&(s, e)) = cs.get(k)
          && s as usize == i
        {
          let stop = (e as usize).min(end);
          self.advance_out(i, stop);
          prev_class = Class::Space;
          i = stop;
          continue;
        }
      }
      let starts = match c {
        Class::Word => prev_class != Class::Word,
        _ => prev_class != Class::Punct || lone_punct(b) || lone_punct(prev_byte),
      };
      if starts {
        let name = if c == Class::Word {
          self.ident_at(i as u32)
        } else {
          None
        };
        let (l, col) = (self.cur.line, self.cur.col);
        self.segment(l, col, name);
      }
      if b == b'"' || b == b'\'' {
        // A string literal is one token. Stop at its quote or at the end of the line.
        let mut j = i + 1;
        while j < end && code[j] != b && code[j] != b'\n' && code[j] != b'\r' {
          j += if code[j] == b'\\' { 2 } else { 1 };
        }
        let stop = (j + 1).min(end);
        self.advance_out(i, stop);
        prev_class = Class::Punct;
        prev_byte = b;
        i = stop;
        continue;
      }
      if c == Class::Word {
        // Whole word run at once (ASCII fast path; other bytes take the careful path).
        let mut j = i;
        while j < end && class(code[j]) == Class::Word {
          j += 1;
        }
        if code[i..j].is_ascii() {
          let n = (j - i) as u32;
          self.cur.col += n;
          self.cur.cr = false;
          self.cur.off = j;
          self.out_col += n;
        } else {
          self.advance_out(i, j);
        }
        prev_class = Class::Word;
        prev_byte = code[j - 1];
        i = j;
        continue;
      }
      // ASCII punctuation (class Punct is ASCII only).
      self.cur.col += 1;
      self.cur.cr = false;
      self.cur.off += 1;
      self.out_col += 1;
      prev_class = c;
      prev_byte = b;
      i += 1;
    }
    out.push_str(&self.code[start..end]);
  }

  /// Advances the original cursor and the generated position over `code[from..to]`.
  #[inline]
  fn advance_out(&mut self, from: usize, to: usize) {
    let code = self.code.as_bytes();
    let to = to.min(code.len());
    let mut i = from;
    while i < to {
      let b = code[i];
      if b == b'\n' {
        self.out_line += 1;
        self.out_col = 0;
      } else if b & 0xc0 != 0x80 {
        self.out_col += 1 + (b >= 0xf0) as u32;
      }
      i += 1;
    }
    while self.cur.off < to {
      self.cur.step(code);
    }
  }

  fn advance_gen(&mut self, text: &str) {
    for &b in text.as_bytes() {
      if b == b'\n' {
        self.out_line += 1;
        self.out_col = 0;
      } else if b & 0xc0 != 0x80 {
        self.out_col += 1 + (b >= 0xf0) as u32;
      }
    }
  }

  /// Appends generated `text` that replaces `code[start..end]` (an insertion when equal).
  pub(crate) fn generated(&mut self, out: &mut String, start: usize, end: usize, text: &str) {
    if text.is_empty() {
      return;
    }
    let start = start.min(self.code.len());
    let end = end.min(self.code.len());
    self.seek(start);
    self.mark = self.cur;
    let mut index = std::mem::take(&mut self.index);
    index.reset(&self.code[start..end]);
    let t = text.as_bytes();
    let mut anchor = start;
    let mut search_from = 0usize;
    let mut line_open = true; // the next token is the first on its generated line
    let mut i = 0;
    let mut chunk_start = 0;
    while i < t.len() {
      let b = t[i];
      if b == b'\n' {
        i += 1;
        line_open = true;
        continue;
      }
      let c = class(b);
      if c == Class::Space {
        i += 1;
        continue;
      }
      // Token extent.
      let tok_end = if b == b'"' || b == b'\'' {
        let mut j = i + 1;
        while j < t.len() && t[j] != b && t[j] != b'\n' {
          j += if t[j] == b'\\' { 2 } else { 1 };
        }
        (j + 1).min(t.len())
      } else if c == Class::Word {
        let mut j = i;
        while j < t.len() && class(t[j]) == Class::Word {
          j += 1;
        }
        j
      } else {
        i + 1
      };
      let tok = &text[i..tok_end];
      let found = match c {
        Class::Word if !b.is_ascii_digit() => index.find_tok(tok, false, search_from),
        Class::Punct if b == b'"' || b == b'\'' => index.find_tok(
          &tok[1..tok.len().saturating_sub(1).max(1)],
          true,
          search_from,
        ),
        _ => None,
      };
      if found.is_some() || line_open {
        // Bring the generated position up to this token.
        out.push_str(&text[chunk_start..i]);
        self.advance_gen(&text[chunk_start..i]);
        chunk_start = i;
        if let Some(k) = found {
          anchor = start + index.toks[k].1;
          search_from = k + 1;
        }
        self.seek(anchor);
        // Replaced spans are not walked for identifiers. An anchored word that is not a
        // keyword is an identifier there (object keys, references in copied functions).
        let name = if found.is_some() && c == Class::Word && !is_keyword(tok) {
          Some(&self.code[anchor..anchor + tok.len()])
        } else {
          None
        };
        let (l, col) = (self.cur.line, self.cur.col);
        self.segment(l, col, name);
        line_open = false;
      }
      i = tok_end;
    }
    out.push_str(&text[chunk_start..]);
    self.advance_gen(&text[chunk_start..]);
    self.index = index;
    self.seek(end);
    self.mark = Cursor::default();
  }

  /// The map JSON, shaped like Babel's (`version`, `names`, `sources`, `sourcesContent`,
  /// `mappings`, `ignoreList`). With an input map, sources and contents come from it.
  pub(crate) fn finish(self, source_name: &str) -> String {
    let mut j = String::with_capacity(self.code.len() + self.mappings.len() + 256);
    j.push_str("{\"version\":3,");
    let names = |j: &mut String| {
      j.push_str("\"names\":[");
      for (i, n) in self.names.iter().enumerate() {
        if i > 0 {
          j.push(',');
        }
        json_str(j, n);
      }
      j.push_str("],");
    };
    match &self.input {
      None => {
        names(&mut j);
        j.push_str("\"sources\":[");
        json_str(&mut j, source_name);
        j.push_str("],\"sourcesContent\":[");
        let content = self
          .info
          .lowering
          .as_ref()
          .map_or(self.code, |l| l.original);
        json_str(&mut j, content);
        j.push_str("],");
      },
      Some(input) => {
        let iv = &input.value;
        if let Some(f) = iv.get("file").filter(|f| f.is_string()) {
          j.push_str("\"file\":");
          j.push_str(&f.to_string());
          j.push(',');
        }
        names(&mut j);
        let pick = |key: &str| -> String {
          let arr = iv.get(key).and_then(Value::as_array);
          let v: Vec<Value> = self
            .sources
            .iter()
            .map(|&s| {
              arr
                .and_then(|a| a.get(s as usize))
                .cloned()
                .unwrap_or(Value::Null)
            })
            .collect();
          Value::from(v).to_string()
        };
        j.push_str(&format!(
          "\"sources\":{},\"sourcesContent\":{},",
          pick("sources"),
          pick("sourcesContent")
        ));
        if let Some(root) = iv.get("sourceRoot").filter(|r| r.is_string()) {
          j.push_str(&format!("\"sourceRoot\":{root},"));
        }
      },
    }
    j.push_str("\"mappings\":\"");
    j.push_str(&self.mappings);
    j.push_str("\",\"ignoreList\":[]}");
    j
  }
}

/// Appends `s` as a JSON string. Copies unescaped runs in bulk.
fn json_str(out: &mut String, s: &str) {
  out.push('"');
  let b = s.as_bytes();
  let mut run = 0;
  for (i, &c) in b.iter().enumerate() {
    if c >= 0x20 && c != b'"' && c != b'\\' {
      continue;
    }
    out.push_str(&s[run..i]);
    match c {
      b'"' => out.push_str("\\\""),
      b'\\' => out.push_str("\\\\"),
      b'\n' => out.push_str("\\n"),
      b'\r' => out.push_str("\\r"),
      b'\t' => out.push_str("\\t"),
      0x08 => out.push_str("\\b"),
      0x0c => out.push_str("\\f"),
      _ => out.push_str(&format!("\\u{:04x}", c)),
    }
    run = i + 1;
  }
  out.push_str(&s[run..]);
  out.push('"');
}

/// Word and string-literal tokens of a replaced span, for anchoring generated tokens.
/// Generated tokens mostly follow source order, so a short forward scan from the last match
/// usually finds them. Misses fall back to a first-occurrence map built on first use.
/// Buffers are reused across spans.
#[derive(Default)]
struct SpanIndex<'s> {
  /// (token, offset in span, is string). Strings hold their content without quotes.
  toks: Vec<(&'s str, usize, bool)>,
  first: FxMap<(&'s str, bool), usize>,
  first_built: bool,
}

const SCAN_AHEAD: usize = 24;

/// Calls `f(start, end, is_string)` for each word and string-literal token of `src`.
fn tokens(src: &str, mut f: impl FnMut(usize, usize, bool)) {
  let s = src.as_bytes();
  let mut i = 0;
  while i < s.len() {
    let b = s[i];
    if b == b'"' || b == b'\'' {
      let mut j = i + 1;
      while j < s.len() && s[j] != b && s[j] != b'\n' {
        j += if s[j] == b'\\' { 2 } else { 1 };
      }
      if j < s.len() && s[j] == b {
        f(i, j + 1, true);
      }
      i = j + 1;
    } else if class(b) == Class::Word {
      let mut j = i;
      while j < s.len() && class(s[j]) == Class::Word {
        j += 1;
      }
      if !b.is_ascii_digit() {
        f(i, j, false);
      }
      i = j;
    } else {
      i += 1;
    }
  }
}

impl<'s> SpanIndex<'s> {
  fn reset(&mut self, src: &'s str) {
    self.toks.clear();
    self.first.clear();
    self.first_built = false;
    let toks = &mut self.toks;
    tokens(src, |a, b, is_str| {
      if is_str {
        toks.push((&src[a + 1..b - 1], a, true));
      } else {
        toks.push((&src[a..b], a, false));
      }
    });
  }

  /// The token index of `raw` at or after token `from`, else its first occurrence. Most
  /// generated tokens (hashed keys, class names) are not in the source: one map lookup
  /// rejects them.
  fn find_tok(&mut self, raw: &str, string: bool, from: usize) -> Option<usize> {
    if !self.first_built {
      for (k, &(t, _, st)) in self.toks.iter().enumerate() {
        self.first.entry((t, st)).or_insert(k);
      }
      self.first_built = true;
    }
    let k = *self.first.get(&(raw, string))?;
    if k >= from {
      return Some(k);
    }
    let end = (from + SCAN_AHEAD).min(self.toks.len());
    (from..end)
      .find(|&j| self.toks[j].2 == string && self.toks[j].0 == raw)
      .or_else(|| {
        (end..self.toks.len()).find(|&j| self.toks[j].2 == string && self.toks[j].0 == raw)
      })
      .or(Some(k))
  }
}
