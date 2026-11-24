//! Flow `Loc` columns are UTF-8 byte offsets. `offset_utils::OffsetTable` reads them as code points.
#[test]
fn loc_columns_are_bytes() {
    let src = "const é: number = 1;";
    let (prog, errs) = flow_blank::flow_parse(src);
    assert!(errs.is_empty());
    // Find the `number` annotation start via the blanker's own output instead of walking the AST.
    let out = flow_blank::blank(src).unwrap();
    let colon = src.find(':').unwrap();
    assert_eq!(&out.code[colon..colon + 1], " ", "blank must start at the colon (byte {colon})");
    let loc = prog.loc.clone();
    let table = flow_parser::offset_utils::OffsetTable::make(src);
    let end_utf8 = table.offset_utf8(loc.end).unwrap();
    // 21 bytes, 20 code points. The parser reports column 21 (bytes)...
    assert_eq!(loc.end.column as usize, src.len());
    // ...and OffsetTable maps it one byte past the end of the source.
    assert_eq!(end_utf8 as usize, src.len() + 1);
}
