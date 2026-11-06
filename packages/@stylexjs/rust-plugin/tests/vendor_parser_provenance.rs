//! Portable immutable-upstream provenance. No Python, tar, shell, or platform
//! executable is needed on Linux, macOS, or Windows.
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const ARCHIVE: &[u8] = include_bytes!("fixtures/swc_ecma_parser-45.1.6.crate");
const MANIFEST: &str = include_str!("fixtures/swc_ecma_parser-45.1.6.manifest");

fn sha256(bytes: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut state = [
        0x6a09e667u32,
        0xbb67ae85,
        0x3c6ef372,
        0xa54ff53a,
        0x510e527f,
        0x9b05688c,
        0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_length = (bytes.len() as u64).checked_mul(8).unwrap();
    let mut padded = bytes.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_length.to_be_bytes());
    for block in padded.chunks_exact(64) {
        let mut words = [0u32; 64];
        for (word, bytes) in words.iter_mut().zip(block.chunks_exact(4)) {
            *word = u32::from_be_bytes(bytes.try_into().unwrap());
        }
        for index in 16..64 {
            let x = words[index - 15];
            let y = words[index - 2];
            let s0 = x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3);
            let s1 = y.rotate_right(17) ^ y.rotate_right(19) ^ (y >> 10);
            words[index] = words[index - 16]
                .wrapping_add(s0)
                .wrapping_add(words[index - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = state;
        for (constant, word) in K.iter().zip(words) {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choose = (e & f) ^ (!e & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(choose)
                .wrapping_add(*constant)
                .wrapping_add(word);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(majority);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (state, value) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *state = state.wrapping_add(value);
        }
    }
    state.iter().map(|word| format!("{word:08x}")).collect()
}

fn inventory(root: &Path, directory: &Path, files: &mut BTreeSet<PathBuf>) {
    for entry in std::fs::read_dir(directory).unwrap() {
        let entry = entry.unwrap();
        let kind = entry.file_type().unwrap();
        assert!(
            !kind.is_symlink(),
            "vendored archive cannot contain symlinks"
        );
        if kind.is_dir() {
            inventory(root, &entry.path(), files);
        } else {
            assert!(kind.is_file());
            files.insert(entry.path().strip_prefix(root).unwrap().to_owned());
        }
    }
}

fn restore_hunk(actual: &mut Vec<u8>, patched: &[u8], original: &[u8], name: &str) {
    let matches: Vec<_> = actual
        .windows(patched.len())
        .enumerate()
        .filter(|(_, bytes)| *bytes == patched)
        .map(|(index, _)| index)
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "approved vendor hunk must occur exactly once: {name}"
    );
    actual.splice(
        matches[0]..matches[0] + patched.len(),
        original.iter().copied(),
    );
}

fn modification_notice(name: &str) -> Option<&'static str> {
    match name {
        "src/lexer/mod.rs" => Some(
            "// Modified for StyleX: `TokenFlags` is exported (`pub use`); see vendor/README.md.\n",
        ),
        "src/parser/expr.rs" => Some(
            "// Modified for StyleX: a `#name` expression parses through the private-name parser; see vendor/README.md.\n",
        ),
        "src/parser/stmt.rs" => Some(
            "// Modified for StyleX: module-level `for await` is checked against the real async context; see vendor/README.md.\n",
        ),
        _ => None,
    }
}

#[test]
fn sha256_matches_standard_empty_short_and_multiblock_vectors() {
    assert_eq!(
        sha256(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        sha256(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
}

#[test]
fn vendored_parser_diff_is_exactly_the_three_approved_public_hunks() {
    assert_eq!(ARCHIVE.len(), 206906, "immutable published archive length");
    assert_eq!(
        sha256(ARCHIVE),
        "a4af66bb9a4cbd2a600b41c2592e27b7947b7e30aa6918d321f987f9e0ff63f5"
    );
    // This manifest was generated from every file member of that immutable
    // archive. Freeze it separately so changing source and expected file hashes
    // together cannot silently add a second vendor patch.
    assert_eq!(
        sha256(MANIFEST.as_bytes()),
        "ec1ba12ee860af66507685421d3e28ca6292bcdfd915648cfdeb85a2eb9145f8"
    );
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("vendor/swc_ecma_parser");
    let mut expected = BTreeSet::new();
    for line in MANIFEST.lines().filter(|line| !line.starts_with('#')) {
        let mut fields = line.splitn(3, ' ');
        let hash = fields.next().unwrap();
        let length: usize = fields.next().unwrap().parse().unwrap();
        let name = fields.next().unwrap();
        let path = Path::new(name);
        assert!(path
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_))));
        assert!(
            expected.insert(path.to_owned()),
            "duplicate upstream member"
        );
        let mut actual = std::fs::read(vendor.join(path)).unwrap();
        // Each patched file opens with the Apache-2.0 §4(b) modification notice.
        if let Some(notice) = modification_notice(name) {
            assert!(
                actual.starts_with(notice.as_bytes()),
                "patched file must open with its modification notice: {name}"
            );
            actual.drain(..notice.len());
        }
        if name == "src/lexer/mod.rs" {
            let public = b"pub use state::TokenFlags;";
            let original = b"pub(crate) use state::TokenFlags;";
            let positions: Vec<_> = actual
                .windows(public.len())
                .enumerate()
                .filter(|(_, bytes)| *bytes == public)
                .map(|(index, _)| index)
                .collect();
            assert_eq!(
                positions.len(),
                1,
                "visibility bridge must occur exactly once"
            );
            actual.splice(
                positions[0]..positions[0] + public.len(),
                original.iter().copied(),
            );
        } else if name == "src/parser/expr.rs" {
            let patched = b"        } else if cur == Token::Hash {\n            self.parse_private_name().map(Into::into)\n        } else if cur == Token::Ident {";
            let original = b"        } else if cur == Token::Hash {\n            self.bump(); // consume `#`\n            let id = self.parse_ident_name()?;\n            if self.syntax().flow() {\n                self.emit_err(self.span(start), SyntaxError::TS1003);\n            }\n            Ok(PrivateName {\n                span: self.span(start),\n                name: id.sym,\n            }\n            .into())\n        } else if cur == Token::Ident {";
            restore_hunk(&mut actual, patched, original, name);
        } else if name == "src/parser/stmt.rs" {
            let patched = b"                if await_token.is_some() {\n                    let ctx = self.ctx();\n                    if ctx.contains(Context::InStaticBlock)\n                        || (ctx.contains(Context::InFunction) && !ctx.contains(Context::InAsync))\n                    {\n                        self.emit_err(self.span(start), SyntaxError::AwaitInFunction);\n                    } else if !ctx.contains(Context::InAsync) && !ctx.contains(Context::CanBeModule) {\n                        self.emit_err(self.span(start), SyntaxError::TopLevelAwaitInScript);\n                    }\n                }";
            let original = b"                if await_token.is_some()\n                    && self.syntax().flow()\n                    && !self.ctx().contains(Context::InAsync)\n                {\n                    self.emit_err(self.span(start), SyntaxError::AwaitForStmt);\n                }";
            restore_hunk(&mut actual, patched, original, name);
        }
        assert_eq!(actual.len(), length, "vendored length differs for {name}");
        assert_eq!(
            sha256(&actual),
            hash,
            "unexpected vendor modification: {name}"
        );
    }
    let mut actual = BTreeSet::new();
    inventory(&vendor, &vendor, &mut actual);
    assert_eq!(
        actual, expected,
        "vendored file inventory differs from the published archive"
    );
    assert_eq!(actual.len(), 42);
}
