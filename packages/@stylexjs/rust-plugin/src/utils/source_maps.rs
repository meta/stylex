//! Portable, flat Source Map v3 emission and composition.
//!
//! Generated positions count LF only and columns count UTF-16 code units (CR,
//! U+2028 and U+2029 occupy columns). Original parser locations may instead use
//! JavaScript line terminators: `Position::advance` deliberately retains that
//! convention. `IntermediatePositions::JavaScript` explicitly bridges those
//! original locations to an upstream map's LF-based generated locations using
//! the intermediate source text; ordinary V3 composition never guesses.
//!
//! Composition supports one direct source, as produced by a single-file
//! compiler, and flat upstream maps with any number of sources. Indexed maps,
//! ambiguous file identities, malformed VLQs/indices and resource-limit
//! violations are errors. Missing upstream coverage becomes an unmapped segment,
//! never a fabricated intermediate URL or an extrapolated original column.
//! Upstream `sources`, `sourceRoot` and `sourcesContent` are copied verbatim;
//! source-index ignore lists are retained, but unknown generated extensions are
//! not carried over. Limits: 64 MiB mappings/text, 8 million segments, 1 million
//! lines/metadata entries and 30-bit nonnegative coordinates (compatible with
//! legacy source-map's signed VLQ decoder as well as modern trace-mapping).
//! No filesystem, network, SWC or host-specific dependency is required.

use serde_json::{json, Value};
use std::collections::HashMap;
use std::fmt;

/// Source-map 0.6 uses signed >> after adding the VLQ sign bit; 30-bit
/// magnitudes therefore round-trip through both legacy and modern consumers.
const MAX_COORDINATE: i64 = (1 << 30) - 1;
const MAX_MAPPING_BYTES: usize = 64 * 1024 * 1024;
const MAX_SEGMENTS: usize = 8_000_000;
const MAX_LINES: usize = 1_000_000;
const MAX_ENTRIES: usize = 1_000_000;
const MAX_TEXT_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceMapError(pub String);
impl fmt::Display for SourceMapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for SourceMapError {}
type Result<T> = std::result::Result<T, SourceMapError>;
fn error(message: impl Into<String>) -> SourceMapError {
    SourceMapError(message.into())
}

/// Zero-based original JavaScript parser position; not a UTF-8 byte offset.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
    /// Retained across chunks so a split CRLF is one original JS line break.
    pub cr: bool,
}
impl Position {
    /// Advance an original JS location (CRLF, CR, LF, U+2028 and U+2029).
    pub fn advance(&mut self, character: char) {
        match character {
            '\r' => {
                self.line += 1;
                self.column = 0;
                self.cr = true;
            }
            '\n' if self.cr => self.cr = false,
            '\n' | '\u{2028}' | '\u{2029}' => {
                self.line += 1;
                self.column = 0;
                self.cr = false;
            }
            _ => {
                self.column += character.len_utf16();
                self.cr = false;
            }
        }
    }
    /// Advance a generated source-map position: LF only, UTF-16 columns.
    pub fn advance_generated(&mut self, character: char) {
        if character == '\n' {
            self.line += 1;
            self.column = 0;
        } else {
            self.column += character.len_utf16();
        }
        self.cr = false;
    }
}

/// Single-source direct writer. Identity chunks map every UTF-16 code unit;
/// constant-origin inserted chunks need only one segment per generated line.
/// Calls to `text` may split a CRLF without changing either convention.
pub struct MappingWriter {
    mappings: String,
    generated: Position,
    mapping_line: usize,
    previous_generated_column: usize,
    previous_original: Position,
    last_mapping: Option<(usize, usize, usize)>,
    line_has_segment: bool,
}
impl Default for MappingWriter {
    fn default() -> Self {
        Self::new()
    }
}
impl MappingWriter {
    pub fn new() -> Self {
        Self {
            mappings: String::new(),
            generated: Position::default(),
            mapping_line: 0,
            previous_generated_column: 0,
            previous_original: Position::default(),
            last_mapping: None,
            line_has_segment: false,
        }
    }
    pub fn map(&mut self, original: Position) {
        self.map_at(self.generated, original);
    }
    fn map_at(&mut self, generated: Position, original: Position) {
        let key = (generated.line, original.line, original.column);
        if self.last_mapping == Some(key) {
            return;
        }
        while self.mapping_line < generated.line {
            self.mappings.push(';');
            self.mapping_line += 1;
            self.previous_generated_column = 0;
            self.line_has_segment = false;
        }
        if self.line_has_segment {
            self.mappings.push(',');
        }
        for value in [
            generated.column as i64 - self.previous_generated_column as i64,
            0,
            original.line as i64 - self.previous_original.line as i64,
            original.column as i64 - self.previous_original.column as i64,
        ] {
            write_vlq(value, &mut self.mappings);
        }
        self.previous_generated_column = generated.column;
        self.previous_original = original;
        self.last_mapping = Some(key);
        self.line_has_segment = true;
    }
    pub fn text(&mut self, text: &str, mut original: Position, unchanged: bool) {
        for character in text.chars() {
            // CR is a generated column, so LF must not be suppressed after CR.
            self.map(original);
            if unchanged && character.len_utf16() == 2 {
                let mut generated_half = self.generated;
                generated_half.column += 1;
                let mut original_half = original;
                original_half.column += 1;
                self.map_at(generated_half, original_half);
            }
            self.generated.advance_generated(character);
            if unchanged {
                original.advance(character);
            }
        }
    }
    pub fn finish(mut self) -> String {
        // Include empty trailing generated lines even without an EOF mapping.
        while self.mapping_line < self.generated.line {
            self.mappings.push(';');
            self.mapping_line += 1;
        }
        self.mappings
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IntermediatePositions {
    /// Direct original coordinates already describe upstream generated lines.
    #[default]
    SourceMap,
    /// Direct originals are JS parser coordinates; convert via sourcesContent.
    JavaScript,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct ComposeOptions<'a> {
    pub intermediate_positions: IntermediatePositions,
    /// Optional output text for strict direct generated-range validation.
    pub generated_source: Option<&'a str>,
}

#[derive(Clone, Copy, Debug)]
struct Original {
    source: u32,
    line: u32,
    column: u32,
    name: Option<u32>,
}
#[derive(Clone, Copy, Debug)]
struct Segment {
    column: u32,
    original: Option<Original>,
}
struct FlatMap<'a> {
    sources: &'a [Value],
    names: &'a [Value],
    contents: Option<&'a [Value]>,
    lines: Vec<Vec<Segment>>,
}
impl<'a> FlatMap<'a> {
    fn parse(value: &'a Value) -> Result<Self> {
        let object = value
            .as_object()
            .ok_or_else(|| error("source map must be a JSON object"))?;
        if object.contains_key("sections") {
            return Err(error(
                "indexed source maps (sections) are unsupported; provide a flat V3 map",
            ));
        }
        if value.get("version").and_then(Value::as_u64) != Some(3) {
            return Err(error("source map version must be 3"));
        }
        for field in ["file", "sourceRoot"] {
            if let Some(value) = object.get(field) {
                if !value.is_null() && !value.is_string() {
                    return Err(error(format!("{field} must be a string or null")));
                }
            }
        }
        let strings = |field: &str| -> Result<&'a [Value]> {
            let values = value
                .get(field)
                .and_then(Value::as_array)
                .ok_or_else(|| error(format!("{field} must be an array of strings")))?;
            if values.len() > MAX_ENTRIES || values.iter().any(|v| !v.is_string()) {
                return Err(error(format!("invalid or oversized {field} array")));
            }
            Ok(values)
        };
        let sources = strings("sources")?;
        let names = strings("names")?;
        let contents = match value.get("sourcesContent") {
            None | Some(Value::Null) => None,
            Some(Value::Array(contents))
                if contents.len() == sources.len()
                    && contents.iter().all(|v| v.is_null() || v.is_string()) =>
            {
                Some(contents.as_slice())
            }
            _ => {
                return Err(error(
                    "sourcesContent must contain one string or null per source",
                ))
            }
        };
        for field in ["ignoreList", "x_google_ignoreList"] {
            if let Some(list) = value.get(field) {
                let list = list
                    .as_array()
                    .ok_or_else(|| error(format!("{field} must be an index array")))?;
                if list.len() > MAX_ENTRIES
                    || list
                        .iter()
                        .any(|v| v.as_u64().is_none_or(|i| i >= sources.len() as u64))
                {
                    return Err(error(format!("{field} contains an invalid source index")));
                }
            }
        }
        let mappings = value
            .get("mappings")
            .and_then(Value::as_str)
            .ok_or_else(|| error("mappings must be a VLQ string"))?;
        if mappings.len() > MAX_MAPPING_BYTES {
            return Err(error("source map exceeds 64 MiB mapping limit"));
        }
        let mut lines = Vec::new();
        let mut previous = [0i64; 4]; // source, original line/column, name
        let mut segment_count = 0usize;
        for (line_index, line) in mappings.split(';').enumerate() {
            if line_index >= MAX_LINES {
                return Err(error("source map exceeds generated-line limit"));
            }
            let mut segments = Vec::new();
            let mut column = 0i64;
            if !line.is_empty() {
                for segment in line.split(',') {
                    segment_count += 1;
                    if segment_count > MAX_SEGMENTS {
                        return Err(error("source map exceeds segment limit"));
                    }
                    let (fields, field_count) = decode_vlq(segment)
                        .map_err(|e| error(format!("generated line {}: {e}", line_index + 1)))?;
                    if !matches!(field_count, 1 | 4 | 5) {
                        return Err(error("mapping segment must have 1, 4 or 5 fields"));
                    }
                    if fields[0] < 0 {
                        return Err(error("generated columns must be nondecreasing"));
                    }
                    column = add_delta(column, fields[0], "generated column")?;
                    let original = if field_count == 1 {
                        None
                    } else {
                        for index in 0..3 {
                            previous[index] = add_delta(
                                previous[index],
                                fields[index + 1],
                                "original coordinate/index",
                            )?;
                        }
                        if previous[0] >= sources.len() as i64 {
                            return Err(error("mapping source index is out of range"));
                        }
                        let name = if field_count == 5 {
                            previous[3] = add_delta(previous[3], fields[4], "name index")?;
                            if previous[3] >= names.len() as i64 {
                                return Err(error("mapping name index is out of range"));
                            }
                            Some(previous[3] as u32)
                        } else {
                            None
                        };
                        Some(Original {
                            source: previous[0] as u32,
                            line: previous[1] as u32,
                            column: previous[2] as u32,
                            name,
                        })
                    };
                    let point = Segment {
                        column: column as u32,
                        original,
                    };
                    // GLB ties use the first segment, matching trace-mapping.
                    // Reject conflicting ties rather than pick a consumer-dependent result.
                    if let Some(last) = segments.last() {
                        let last: &Segment = last;
                        if last.column == point.column {
                            if !same_original(last.original, point.original) {
                                return Err(error(
                                    "conflicting mappings at the same generated position",
                                ));
                            }
                            continue;
                        }
                    }
                    segments.push(point);
                }
            }
            lines.push(segments);
        }
        Ok(Self {
            sources,
            names,
            contents,
            lines,
        })
    }
    fn lookup(&self, line: u32, column: u32) -> Option<Original> {
        let segments = self.lines.get(line as usize)?;
        let index = segments.partition_point(|segment| segment.column <= column);
        index.checked_sub(1).and_then(|i| segments[i].original)
    }
}
fn same_original(a: Option<Original>, b: Option<Original>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            (a.source, a.line, a.column, a.name) == (b.source, b.line, b.column, b.name)
        }
        _ => false,
    }
}
fn add_delta(previous: i64, delta: i64, field: &str) -> Result<i64> {
    previous
        .checked_add(delta)
        .filter(|v| (0..=MAX_COORDINATE).contains(v))
        .ok_or_else(|| {
            error(format!(
                "{field} delta is negative or overflows consumer coordinate range"
            ))
        })
}
fn digit(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}
fn decode_vlq(segment: &str) -> Result<([i64; 5], usize)> {
    if segment.is_empty() {
        return Err(error("empty mapping segment"));
    }
    // Fixed fields avoid a heap allocation per segment in dense identity maps.
    let mut fields = [0i64; 5];
    let mut field_count = 0;
    let mut value = 0u64;
    let mut shift = 0u32;
    for byte in segment.bytes() {
        let digit = digit(byte).ok_or_else(|| error("invalid base64 VLQ character"))?;
        if shift > 30 {
            return Err(error("VLQ exceeds consumer integer range"));
        }
        value |= u64::from(digit & 31) << shift;
        if digit & 32 != 0 {
            shift += 5;
        } else {
            let magnitude = (value >> 1) as i64;
            if magnitude > MAX_COORDINATE || value == 1 {
                return Err(error(
                    "VLQ integer is out of range (including reserved negative zero)",
                ));
            }
            if field_count == fields.len() {
                return Err(error("too many VLQ fields"));
            }
            fields[field_count] = if value & 1 == 1 {
                -magnitude
            } else {
                magnitude
            };
            field_count += 1;
            value = 0;
            shift = 0;
        }
    }
    if shift != 0 {
        return Err(error("unterminated VLQ"));
    }
    Ok((fields, field_count))
}
fn write_vlq(value: i64, output: &mut String) {
    const BASE64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut value = (value.unsigned_abs() << 1) | u64::from(value < 0);
    loop {
        let mut digit = (value & 31) as usize;
        value >>= 5;
        if value != 0 {
            digit |= 32;
        }
        output.push(BASE64[digit] as char);
        if value == 0 {
            break;
        }
    }
}
fn encode_lines(lines: &[Vec<Segment>]) -> String {
    let mut output = String::new();
    let mut previous = [0i64; 4];
    for (line_index, line) in lines.iter().enumerate() {
        if line_index > 0 {
            output.push(';');
        }
        let mut column = 0i64;
        for (index, segment) in line.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            write_vlq(i64::from(segment.column) - column, &mut output);
            column = i64::from(segment.column);
            if let Some(original) = segment.original {
                for (index, value) in [original.source, original.line, original.column]
                    .into_iter()
                    .enumerate()
                {
                    write_vlq(i64::from(value) - previous[index], &mut output);
                    previous[index] = i64::from(value);
                }
                if let Some(name) = original.name {
                    write_vlq(i64::from(name) - previous[3], &mut output);
                    previous[3] = i64::from(name);
                }
            }
        }
    }
    output
}

/// Validate structural fields, VLQ arithmetic and indices without composition.
/// If output text is available, also validate LF/UTF-16 generated ranges.
/// Original coordinates are opaque: V3 does not declare a parser convention.
#[cfg(test)]
pub fn validate_source_map(map: &Value, generated_source: Option<&str>) -> Result<()> {
    let map = FlatMap::parse(map)?;
    if let Some(text) = generated_source {
        validate_generated(&map, text)?;
    }
    Ok(())
}
fn lf_lengths(text: &str) -> Result<Vec<usize>> {
    if text.len() > MAX_TEXT_BYTES {
        return Err(error("supplied text exceeds 64 MiB limit"));
    }
    let mut lengths = vec![0usize];
    for character in text.chars() {
        if character == '\n' {
            if lengths.len() >= MAX_LINES {
                return Err(error("supplied text exceeds line limit"));
            }
            lengths.push(0);
        } else {
            let length = lengths.last_mut().unwrap();
            *length += character.len_utf16();
        }
    }
    Ok(lengths)
}
fn validate_generated(map: &FlatMap<'_>, text: &str) -> Result<()> {
    let lengths = lf_lengths(text)?;
    if map.lines.len() > lengths.len() {
        return Err(error("map has more generated lines than the supplied text"));
    }
    for (line, segments) in map.lines.iter().enumerate() {
        if segments
            .last()
            .is_some_and(|segment| segment.column as usize > lengths[line])
        {
            return Err(error("mapping generated column is outside supplied text"));
        }
    }
    Ok(())
}

/// Compose a direct V3 map through an optional upstream `inputSourceMap`.
/// No upstream is a validated identity operation (metadata remains exact).
/// With upstream, direct must have one source; names prefer upstream names,
/// falling back to direct names only where an upstream mapping has no name.
#[cfg(test)]
pub fn compose_source_maps(direct: &Value, input_source_map: Option<&Value>) -> Result<Value> {
    compose_source_maps_with_options(direct, input_source_map, ComposeOptions::default())
}

pub fn compose_source_maps_with_options(
    direct: &Value,
    input_source_map: Option<&Value>,
    options: ComposeOptions<'_>,
) -> Result<Value> {
    let mut direct_map = FlatMap::parse(direct)?;
    if let Some(text) = options.generated_source {
        validate_generated(&direct_map, text)?;
    }
    let Some(input) = input_source_map else {
        return Ok(direct.clone());
    };
    let input_map = FlatMap::parse(input)?;
    if direct_map.sources.len() != 1 {
        return Err(error("composition requires exactly one direct source"));
    }
    let direct_source = direct_map.sources[0].as_str().unwrap();
    if let Some(file) = input
        .get("file")
        .and_then(Value::as_str)
        .filter(|file| !file.is_empty())
    {
        if !same_file(direct_source, file) {
            return Err(error(
                "inputSourceMap file does not match the direct intermediate source",
            ));
        }
    }
    let intermediate = direct_map
        .contents
        .and_then(|contents| contents[0].as_str());
    if let Some(text) = intermediate {
        validate_generated(&input_map, text)?;
    }
    let js_lines = match options.intermediate_positions {
        IntermediatePositions::SourceMap => None,
        IntermediatePositions::JavaScript => {
            Some(js_line_bridge(intermediate.ok_or_else(|| {
                error("JavaScript intermediate positions require direct sourcesContent")
            })?)?)
        }
    };
    let lengths = intermediate.map(lf_lengths).transpose()?;
    let mut names = input_map.names.to_vec();
    let mut name_indices: HashMap<&str, u32> = HashMap::new();
    for (index, name) in input_map.names.iter().enumerate() {
        name_indices
            .entry(name.as_str().unwrap())
            .or_insert(index as u32);
    }
    let mut direct_name_indices = vec![None; direct_map.names.len()];
    for line in &mut direct_map.lines {
        for segment in line {
            let Some(original) = segment.original else {
                continue;
            };
            let (lookup_line, lookup_column) = if let Some(lines) = &js_lines {
                js_to_generated(lines, original.line, original.column)?
            } else {
                (original.line, original.column)
            };
            if let Some(lengths) = &lengths {
                if lengths
                    .get(lookup_line as usize)
                    .is_none_or(|length| lookup_column as usize > *length)
                {
                    return Err(error(
                        "direct original position is outside intermediate sourcesContent",
                    ));
                }
            }
            // GLB is a point lookup, not a linear range. In particular, do not
            // add lookup_column - upstream_segment.column to its original column.
            segment.original = input_map
                .lookup(lookup_line, lookup_column)
                .map(|mut upstream| {
                    if upstream.name.is_none() {
                        if let Some(direct_name) = original.name {
                            let cache = &mut direct_name_indices[direct_name as usize];
                            let index = *cache.get_or_insert_with(|| {
                                let name = direct_map.names[direct_name as usize].as_str().unwrap();
                                let next = names.len() as u32;
                                *name_indices.entry(name).or_insert_with(|| {
                                    names.push(Value::String(name.to_owned()));
                                    next
                                })
                            });
                            upstream.name = Some(index);
                        }
                    }
                    upstream
                });
        }
    }
    if names.len() > MAX_ENTRIES {
        return Err(error("composed names exceed entry limit"));
    }
    // Retain only source metadata whose indices still refer to the unchanged
    // upstream source array. Generated extensions (e.g. upstream debug IDs or
    // offsets) cannot safely describe the newly generated output.
    let mut output = serde_json::Map::new();
    for field in [
        "sources",
        "sourceRoot",
        "sourcesContent",
        "ignoreList",
        "x_google_ignoreList",
    ] {
        if let Some(value) = input.get(field) {
            output.insert(field.into(), value.clone());
        }
    }
    if let Some(file) = direct.get("file") {
        output.insert("file".into(), file.clone());
    }
    output.insert("version".into(), json!(3));
    output.insert("names".into(), Value::Array(names));
    output.insert(
        "mappings".into(),
        Value::String(encode_lines(&direct_map.lines)),
    );
    Ok(Value::Object(output))
}

/// Exact identities, or a basename paired with an unambiguous absolute path.
/// Two distinct explicit paths are never treated as the same file. No URL/path
/// normalization is used to rewrite output sources or sourceRoot metadata.
fn same_file(source: &str, file: &str) -> bool {
    if source == file {
        return true;
    }
    fn basename(path: &str) -> Option<&str> {
        let has_separator = path.contains('/') || path.contains('\\');
        has_separator
            .then(|| path.rsplit(['/', '\\']).next())
            .flatten()
    }
    let bare = |path: &str| !path.contains('/') && !path.contains('\\') && !path.contains(':');
    (bare(source) && basename(file) == Some(source))
        || (bare(file) && basename(source) == Some(file))
}

#[derive(Clone, Copy)]
struct JsLine {
    generated_line: u32,
    generated_column: u32,
    length: u32,
}
/// One entry per JS line, not per character/byte. Thus conversion costs one
/// intermediate text scan and logarithmic map lookups, not repeated prefix scans.
fn js_line_bridge(text: &str) -> Result<Vec<JsLine>> {
    if text.len() > MAX_TEXT_BYTES {
        return Err(error("intermediate text exceeds 64 MiB limit"));
    }
    let mut lines = Vec::new();
    let mut generated = Position::default();
    let mut start = generated;
    let mut length = 0usize;
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        if matches!(character, '\r' | '\n' | '\u{2028}' | '\u{2029}') {
            push_js_line(&mut lines, start, length)?;
            generated.advance_generated(character);
            if character == '\r' && chars.peek() == Some(&'\n') {
                chars.next();
                generated.advance_generated('\n');
            }
            start = generated;
            length = 0;
        } else {
            length += character.len_utf16();
            generated.advance_generated(character);
        }
    }
    push_js_line(&mut lines, start, length)?;
    Ok(lines)
}
fn push_js_line(lines: &mut Vec<JsLine>, start: Position, length: usize) -> Result<()> {
    if lines.len() >= MAX_LINES
        || [start.line, start.column, length]
            .into_iter()
            .any(|v| v > MAX_COORDINATE as usize)
    {
        return Err(error("intermediate JS coordinates exceed limits"));
    }
    lines.push(JsLine {
        generated_line: start.line as u32,
        generated_column: start.column as u32,
        length: length as u32,
    });
    Ok(())
}
fn js_to_generated(lines: &[JsLine], line: u32, column: u32) -> Result<(u32, u32)> {
    let line = lines
        .get(line as usize)
        .ok_or_else(|| error("direct JS original line is outside intermediate text"))?;
    if column > line.length {
        return Err(error(
            "direct JS original column is outside intermediate line",
        ));
    }
    let column = add_delta(
        i64::from(line.generated_column),
        i64::from(column),
        "converted column",
    )?;
    Ok((line.generated_line, column as u32))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn composing_without_an_input_map_is_a_validated_identity() {
        let map = json!({
            "version": 3,
            "sources": ["a.js"],
            "names": [],
            "mappings": "AAAA",
        });
        assert_eq!(compose_source_maps(&map, None).unwrap(), map);
        let javascript = ComposeOptions {
            intermediate_positions: IntermediatePositions::JavaScript,
            generated_source: Some("x"),
        };
        assert_eq!(
            compose_source_maps_with_options(&map, None, javascript).unwrap(),
            map
        );
        validate_source_map(&map, Some("x")).unwrap();
        assert!(validate_source_map(&json!({ "version": 3, "mappings": 1 }), None).is_err());
    }
}
