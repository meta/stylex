//! Babel's (jsesc) spelling for generated JavaScript and JSX strings.
pub(crate) fn string_literal(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    let mut units = value.encode_utf16().peekable();
    while let Some(unit) = units.next() {
        match unit {
            0 if !units
                .peek()
                .is_some_and(|next| (b'0' as u16..=b'9' as u16).contains(next)) =>
            {
                output.push_str("\\0")
            }
            8 => output.push_str("\\b"),
            9 => output.push_str("\\t"),
            10 => output.push_str("\\n"),
            12 => output.push_str("\\f"),
            13 => output.push_str("\\r"),
            34 => output.push_str("\\\""),
            92 => output.push_str("\\\\"),
            32..=126 => output.push(unit as u8 as char),
            0..=255 => {
                output.push_str("\\x");
                output.push(HEX[(unit >> 4) as usize] as char);
                output.push(HEX[(unit & 15) as usize] as char);
            }
            _ => {
                output.push_str("\\u");
                for shift in [12, 8, 4, 0] {
                    output.push(HEX[((unit >> shift) & 15) as usize] as char);
                }
            }
        }
    }
    output.push('"');
    output
}
