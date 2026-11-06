# Third-party notices

The native addon (`native/*.node`) is built from the Rust crates below. The
full license and notice texts for every crate in its normal dependency graph
(`cargo tree -p stylex_rust_compiler --features node -e normal --target all`,
which also includes proc-macro and target-specific crates) are in
`RUST_DEPENDENCY_LICENSES.md`; `LICENSE_PROVENANCE.json` records where each
text came from.

- The StyleX compiler core in `crates/` is vendored from
  [stylex-swc-plugin](https://github.com/Dwlad90/stylex-swc-plugin) 0.19.2 under
  the MIT license: see `crates/LICENSE` (license text and copyright),
  `crates/NOTICE.md` (attribution) and `crates/PROVENANCE.md`
  (exact source and every local change).
- `vendor/swc_ecma_parser` is the published `swc_ecma_parser` 45.1.6 crate with
  the patches described in `vendor/README.md` (Apache-2.0).
- On Windows, macOS and 64-bit Linux (GNU) the addon uses mimalloc as its memory
  allocator through `swc_malloc`; `libmimalloc-sys` compiles Microsoft's mimalloc
  C sources (MIT). On 32-bit ARM Linux (GNU), `swc_malloc` selects jemalloc
  through `tikv-jemallocator` (jemalloc's BSD-2-Clause license).

Every crate's license is its own. The table lists the crates of the
host-target graph (`cargo tree -e normal --features node`) with their declared
licenses.

| Crate | Version | License | Source |
|---|---|---|---|
| ahash | 0.8.12 | MIT OR Apache-2.0 | crates.io |
| aho-corasick | 1.1.5 | Unlicense OR MIT | crates.io |
| allocator-api2 | 0.2.21 | MIT OR Apache-2.0 | crates.io |
| ansi_term | 0.12.1 | MIT | crates.io |
| anyhow | 1.0.104 | MIT OR Apache-2.0 | crates.io |
| arrayvec | 0.7.8 | MIT OR Apache-2.0 | crates.io |
| ascii | 1.1.0 | Apache-2.0 OR MIT | crates.io |
| base64 | 0.22.1 | MIT OR Apache-2.0 | crates.io |
| base64-simd | 0.8.0 | MIT | crates.io |
| better_scoped_tls | 1.0.1 | Apache-2.0 | crates.io |
| bit-set | 0.8.0 | Apache-2.0 OR MIT | crates.io |
| bit-vec | 0.8.0 | Apache-2.0 OR MIT | crates.io |
| bitflags | 2.13.2 | MIT OR Apache-2.0 | crates.io |
| bitvec | 1.1.1 | MIT | crates.io |
| block-buffer | 0.10.4 | MIT OR Apache-2.0 | crates.io |
| bstr | 1.12.1 | MIT OR Apache-2.0 | crates.io |
| bumpalo | 3.20.3 | MIT OR Apache-2.0 | crates.io |
| byteorder | 1.5.0 | Unlicense OR MIT | crates.io |
| bytes | 1.12.1 | MIT | crates.io |
| bytes-str | 0.2.9 | Apache-2.0 | crates.io |
| camino | 1.1.11 | MIT OR Apache-2.0 | crates.io |
| cargo-platform | 0.1.9 | MIT OR Apache-2.0 | crates.io |
| cargo_metadata | 0.18.1 | MIT | crates.io |
| castaway | 0.2.4 | MIT | crates.io |
| cbor4ii | 1.2.2 | MIT | crates.io |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 | crates.io |
| colored | 3.1.1 | MPL-2.0 | crates.io |
| compact_str | 0.10.0 | MIT | crates.io |
| compact_str | 0.7.1 | MIT | crates.io |
| concurrent_lru | 0.2.0 | MIT | crates.io |
| convert_case | 0.10.0 | MIT | crates.io |
| convert_case | 0.12.0 | MIT | crates.io |
| cpufeatures | 0.2.17 | MIT OR Apache-2.0 | crates.io |
| crc32fast | 1.5.0 | MIT OR Apache-2.0 | crates.io |
| crossbeam-utils | 0.8.21 | MIT OR Apache-2.0 | crates.io |
| crypto-common | 0.1.6 | MIT OR Apache-2.0 | crates.io |
| cssparser | 0.38.0 | MPL-2.0 | crates.io |
| ctor | 1.0.13 | Apache-2.0 OR MIT | crates.io |
| darling | 0.20.11 | MIT | crates.io |
| darling_core | 0.20.11 | MIT | crates.io |
| dashmap | 6.2.1 | MIT | crates.io |
| data-encoding | 2.11.1 | MIT | crates.io |
| debugid | 0.8.0 | Apache-2.0 | crates.io |
| derive_builder | 0.20.2 | MIT OR Apache-2.0 | crates.io |
| derive_builder_core | 0.20.2 | MIT OR Apache-2.0 | crates.io |
| derive_more | 2.1.1 | MIT | crates.io |
| diff | 0.1.13 | MIT OR Apache-2.0 | crates.io |
| difference | 2.0.0 | MIT | crates.io |
| digest | 0.10.7 | MIT OR Apache-2.0 | crates.io |
| dragonbox_ecma | 0.1.12 | Apache-2.0 WITH LLVM-exception OR BSL-1.0 | crates.io |
| dtoa | 1.0.10 | MIT OR Apache-2.0 | crates.io |
| dtoa-short | 0.3.5 | MPL-2.0 | crates.io |
| either | 1.19.0 | MIT OR Apache-2.0 | crates.io |
| endian-type | 0.2.0 | MIT | crates.io |
| env_logger | 0.10.2 | MIT OR Apache-2.0 | crates.io |
| equivalent | 1.0.3 | Apache-2.0 OR MIT | crates.io |
| fancy-regex | 0.19.2 | MIT | crates.io |
| fast-glob | 1.0.0 | MIT | crates.io |
| fastrand | 2.3.0 | Apache-2.0 OR MIT | crates.io |
| fixedbitset | 0.5.7 | MIT OR Apache-2.0 | crates.io |
| flate2 | 1.1.5 | MIT OR Apache-2.0 | crates.io |
| float-cmp | 0.10.0 | MIT | crates.io |
| fnv | 1.0.7 | Apache-2.0 / MIT | crates.io |
| foldhash | 0.2.0 | Zlib | crates.io |
| form_urlencoded | 1.2.2 | MIT OR Apache-2.0 | crates.io |
| funty | 2.0.0 | MIT | crates.io |
| futures | 0.3.34 | MIT OR Apache-2.0 | crates.io |
| futures-channel | 0.3.34 | MIT OR Apache-2.0 | crates.io |
| futures-core | 0.3.34 | MIT OR Apache-2.0 | crates.io |
| futures-executor | 0.3.34 | MIT OR Apache-2.0 | crates.io |
| futures-io | 0.3.34 | MIT OR Apache-2.0 | crates.io |
| futures-sink | 0.3.34 | MIT OR Apache-2.0 | crates.io |
| futures-task | 0.3.34 | MIT OR Apache-2.0 | crates.io |
| futures-util | 0.3.34 | MIT OR Apache-2.0 | crates.io |
| generic-array | 0.14.9 | MIT | crates.io |
| getrandom | 0.3.4 | MIT OR Apache-2.0 | crates.io |
| glob | 0.3.4 | MIT OR Apache-2.0 | crates.io |
| globset | 0.4.18 | Unlicense OR MIT | crates.io |
| halfbrown | 0.4.0 | Apache-2.0/MIT | crates.io |
| hashbrown | 0.14.5 | MIT OR Apache-2.0 | crates.io |
| hashbrown | 0.16.1 | MIT OR Apache-2.0 | crates.io |
| hashbrown | 0.17.1 | MIT OR Apache-2.0 | crates.io |
| heck | 0.5.0 | MIT OR Apache-2.0 | crates.io |
| hex | 0.4.3 | MIT OR Apache-2.0 | crates.io |
| hstr | 4.0.0 | Apache-2.0 | crates.io |
| humantime | 2.1.0 | MIT/Apache-2.0 | crates.io |
| icu_collator | 2.3.1 | Unicode-3.0 | crates.io |
| icu_collator_data | 2.3.0 | Unicode-3.0 | crates.io |
| icu_collections | 2.3.0 | Unicode-3.0 | crates.io |
| icu_locale_core | 2.4.0 | Unicode-3.0 | crates.io |
| icu_locale_fallback | 2.3.0 | Unicode-3.0 | crates.io |
| icu_locale_fallback_data | 2.3.0 | Unicode-3.0 | crates.io |
| icu_normalizer | 2.3.0 | Unicode-3.0 | crates.io |
| icu_normalizer_data | 2.3.0 | Unicode-3.0 | crates.io |
| icu_properties | 2.3.0 | Unicode-3.0 | crates.io |
| icu_properties_data | 2.3.0 | Unicode-3.0 | crates.io |
| icu_provider | 2.4.0 | Unicode-3.0 | crates.io |
| ident_case | 1.0.1 | MIT/Apache-2.0 | crates.io |
| idna | 1.1.0 | MIT OR Apache-2.0 | crates.io |
| idna_adapter | 1.2.2 | Apache-2.0 OR MIT | crates.io |
| if_chain | 1.0.3 | MIT/Apache-2.0 | crates.io |
| indexmap | 2.14.2 | Apache-2.0 OR MIT | crates.io |
| is-terminal | 0.4.15 | MIT | crates.io |
| itoa | 1.0.18 | MIT OR Apache-2.0 | crates.io |
| json-strip-comments | 3.1.0 | MIT | crates.io |
| lazy_static | 1.5.0 | MIT OR Apache-2.0 | crates.io |
| libc | 0.2.191 | MIT OR Apache-2.0 | crates.io |
| libloading | 0.9.0 | ISC | crates.io |
| libmimalloc-sys | 0.1.49 | MIT | crates.io |
| libz-rs-sys | 0.5.4 | Zlib | crates.io |
| link-section | 0.19.3 | Apache-2.0 OR MIT | crates.io |
| linux-raw-sys | 0.12.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | crates.io |
| litemap | 0.8.3 | Unicode-3.0 | crates.io |
| lock_api | 0.4.14 | MIT OR Apache-2.0 | crates.io |
| log | 0.4.34 | MIT OR Apache-2.0 | crates.io |
| matchers | 0.2.0 | MIT | crates.io |
| memchr | 2.8.3 | Unlicense OR MIT | crates.io |
| miette | 7.6.0 | Apache-2.0 | crates.io |
| mimalloc | 0.1.52 | MIT | crates.io |
| murmur2 | 0.1.0 | Apache-2.0 | crates.io |
| napi | 3.16.1 | MIT | crates.io |
| napi-derive-backend | 7.0.0 | MIT | crates.io |
| napi-sys | 3.4.1 | MIT | crates.io |
| new_debug_unreachable | 1.0.6 | MIT | crates.io |
| nibble_vec | 0.1.0 | MIT | crates.io |
| nodejs-built-in-modules | 1.0.0 | MIT | crates.io |
| nohash-hasher | 0.2.0 | Apache-2.0 OR MIT | crates.io |
| nu-ansi-term | 0.50.1 | MIT | crates.io |
| num-bigint | 0.4.8 | MIT OR Apache-2.0 | crates.io |
| num-integer | 0.1.47 | MIT OR Apache-2.0 | crates.io |
| num-traits | 0.2.19 | MIT OR Apache-2.0 | crates.io |
| num_cpus | 1.17.0 | MIT OR Apache-2.0 | crates.io |
| once_cell | 1.21.4 | MIT OR Apache-2.0 | crates.io |
| outref | 0.5.2 | MIT | crates.io |
| owo-colors | 4.2.2 | MIT | crates.io |
| oxc_resolver | 11.24.3 | MIT | crates.io |
| package-json | 0.5.0 | MIT | crates.io |
| par-core | 2.0.0 | Apache-2.0 | crates.io |
| par-iter | 2.0.0 | Apache-2.0 | crates.io |
| parking_lot | 0.12.5 | MIT OR Apache-2.0 | crates.io |
| parking_lot_core | 0.9.12 | MIT OR Apache-2.0 | crates.io |
| path-clean | 1.0.1 | MIT OR Apache-2.0 | crates.io |
| pathdiff | 0.2.3 | MIT/Apache-2.0 | crates.io |
| percent-encoding | 2.3.2 | MIT OR Apache-2.0 | crates.io |
| petgraph | 0.7.1 | MIT OR Apache-2.0 | crates.io |
| phf | 0.11.3 | MIT | crates.io |
| phf | 0.14.0 | MIT | crates.io |
| phf_generator | 0.11.3 | MIT | crates.io |
| phf_generator | 0.14.0 | MIT | crates.io |
| phf_shared | 0.11.3 | MIT | crates.io |
| phf_shared | 0.14.0 | MIT | crates.io |
| pin-project-lite | 0.2.17 | Apache-2.0 OR MIT | crates.io |
| pnp | 0.12.12 | BSD-2-Clause | crates.io |
| postcss_value_parser | 0.19.2 | MIT | vendored (crates/) |
| potential_utf | 0.1.6 | Unicode-3.0 | crates.io |
| pretty_assertions | 1.4.1 | MIT OR Apache-2.0 | crates.io |
| pretty_env_logger | 0.5.0 | MIT/Apache-2.0 | crates.io |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 | crates.io |
| psm | 0.1.32 | MIT OR Apache-2.0 | crates.io |
| quote | 1.0.47 | MIT OR Apache-2.0 | crates.io |
| radium | 0.7.0 | MIT | crates.io |
| radix_fmt | 1.0.0 | Apache-2.0 | crates.io |
| radix_trie | 0.3.0 | MIT | crates.io |
| rand | 0.8.8 | MIT OR Apache-2.0 | crates.io |
| rand_core | 0.6.4 | MIT OR Apache-2.0 | crates.io |
| ref-cast | 1.0.25 | MIT OR Apache-2.0 | crates.io |
| regex | 1.13.1 | MIT OR Apache-2.0 | crates.io |
| regex-automata | 0.4.18 | MIT OR Apache-2.0 | crates.io |
| regex-syntax | 0.8.11 | MIT OR Apache-2.0 | crates.io |
| regress | 0.11.1 | MIT OR Apache-2.0 | crates.io |
| regress | 0.12.0 | MIT OR Apache-2.0 | crates.io |
| relative-path | 1.9.3 | MIT OR Apache-2.0 | crates.io |
| rustc-hash | 2.1.3 | Apache-2.0 OR MIT | crates.io |
| rustix | 1.1.4 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | crates.io |
| ryu | 1.0.23 | Apache-2.0 OR BSL-1.0 | crates.io |
| ryu-js | 1.0.3 | Apache-2.0 OR BSL-1.0 | crates.io |
| scoped-tls | 1.0.1 | MIT/Apache-2.0 | crates.io |
| scopeguard | 1.2.0 | MIT OR Apache-2.0 | crates.io |
| self_cell | 1.2.1 | Apache-2.0 OR GPL-2.0-only | crates.io |
| semver | 1.0.28 | MIT OR Apache-2.0 | crates.io |
| serde | 1.0.229 | MIT OR Apache-2.0 | crates.io |
| serde_core | 1.0.229 | MIT OR Apache-2.0 | crates.io |
| serde_json | 1.0.154 | MIT OR Apache-2.0 | crates.io |
| sha1 | 0.10.6 | MIT OR Apache-2.0 | crates.io |
| sha2 | 0.10.9 | MIT OR Apache-2.0 | crates.io |
| sharded-slab | 0.1.7 | MIT | crates.io |
| simd-json | 0.18.1 | Apache-2.0 OR MIT | crates.io |
| simdutf8 | 0.1.5 | MIT OR Apache-2.0 | crates.io |
| siphasher | 0.3.11 | MIT/Apache-2.0 | crates.io |
| siphasher | 1.0.4 | MIT OR Apache-2.0 | crates.io |
| slab | 0.4.12 | MIT | crates.io |
| smallvec | 1.16.4 | MIT OR Apache-2.0 | crates.io |
| stable_deref_trait | 1.2.1 | MIT OR Apache-2.0 | crates.io |
| stacker | 0.1.25 | MIT OR Apache-2.0 | crates.io |
| static_assertions | 1.1.0 | MIT OR Apache-2.0 | crates.io |
| strsim | 0.11.1 | MIT | crates.io |
| stylex_ast | 0.19.2 | MIT | vendored (crates/) |
| stylex_atoms | 0.19.2 | MIT | vendored (crates/) |
| stylex_constants | 0.19.2 | MIT | vendored (crates/) |
| stylex_css | 0.19.2 | MIT | vendored (crates/) |
| stylex_css_parser | 0.19.2 | MIT | vendored (crates/) |
| stylex_diagnostics | 0.19.2 | MIT | vendored (crates/) |
| stylex_enums | 0.19.2 | MIT | vendored (crates/) |
| stylex_evaluator | 0.19.2 | MIT | vendored (crates/) |
| stylex_js | 0.19.2 | MIT | vendored (crates/) |
| stylex_macros | 0.19.2 | MIT | vendored (crates/) |
| stylex_nested_config | 0.19.2 | MIT | vendored (crates/) |
| stylex_path_resolver | 0.19.2 | MIT | vendored (crates/) |
| stylex_regex | 0.19.2 | MIT | vendored (crates/) |
| stylex_state | 0.19.2 | MIT | vendored (crates/) |
| stylex_state_index | 0.19.2 | MIT | vendored (crates/) |
| stylex_structures | 0.19.2 | MIT | vendored (crates/) |
| stylex_styleq | 0.19.2 | MIT | vendored (crates/) |
| stylex_transform | 0.19.2 | MIT | vendored (crates/) |
| stylex_types | 0.19.2 | MIT | vendored (crates/) |
| stylex_utils | 0.19.2 | MIT | vendored (crates/) |
| swc_allocator | 5.0.0 | Apache-2.0 | crates.io |
| swc_atoms | 10.0.0 | Apache-2.0 | crates.io |
| swc_common | 26.0.0 | Apache-2.0 | crates.io |
| swc_compiler_base | 65.0.0 | Apache-2.0 | crates.io |
| swc_config | 6.0.0 | Apache-2.0 | crates.io |
| swc_core | 81.0.2 | Apache-2.0 | crates.io |
| swc_ecma_ast | 29.0.2 | Apache-2.0 | crates.io |
| swc_ecma_codegen | 32.0.3 | Apache-2.0 | crates.io |
| swc_ecma_hooks | 1.0.0 | Apache-2.0 | crates.io |
| swc_ecma_minifier | 62.0.1 | Apache-2.0 | crates.io |
| swc_ecma_parser | 45.1.6 | Apache-2.0 | vendored (vendor/) |
| swc_ecma_testing | 27.0.0 | Apache-2.0 | crates.io |
| swc_ecma_transforms_base | 49.0.1 | Apache-2.0 | crates.io |
| swc_ecma_transforms_optimization | 52.0.2 | Apache-2.0 | crates.io |
| swc_ecma_transforms_react | 55.0.1 | Apache-2.0 | crates.io |
| swc_ecma_transforms_testing | 53.0.0 | Apache-2.0 | crates.io |
| swc_ecma_transforms_typescript | 55.0.4 | Apache-2.0 | crates.io |
| swc_ecma_utils | 35.0.2 | Apache-2.0 | crates.io |
| swc_ecma_visit | 29.0.0 | Apache-2.0 | crates.io |
| swc_error_reporters | 28.0.0 | Apache-2.0 | crates.io |
| swc_macros_common | 1.0.1 | Apache-2.0 | crates.io |
| swc_malloc | 1.2.5 | Apache-2.0 | crates.io |
| swc_plugin | 1.0.1 | Apache-2.0 | crates.io |
| swc_plugin_proxy | 30.0.0 | Apache-2.0 | crates.io |
| swc_sourcemap | 10.0.3 | BSD-3-Clause | crates.io |
| swc_transform_common | 20.0.0 | Apache-2.0 | crates.io |
| swc_visit | 2.0.2 | Apache-2.0 | crates.io |
| syn | 2.0.119 | MIT OR Apache-2.0 | crates.io |
| syn | 3.0.9 | MIT OR Apache-2.0 | crates.io |
| synstructure | 0.14.0 | MIT | crates.io |
| tap | 1.0.1 | MIT | crates.io |
| tempfile | 3.21.0 | MIT OR Apache-2.0 | crates.io |
| termcolor | 1.4.1 | Unlicense OR MIT | crates.io |
| testing | 27.0.0 | Apache-2.0 | crates.io |
| textwrap | 0.16.2 | MIT | crates.io |
| thiserror | 1.0.69 | MIT OR Apache-2.0 | crates.io |
| thiserror | 2.0.21 | MIT OR Apache-2.0 | crates.io |
| thread_local | 1.1.9 | MIT OR Apache-2.0 | crates.io |
| tinystr | 0.8.5 | Unicode-3.0 | crates.io |
| tracing | 0.1.44 | MIT | crates.io |
| tracing-core | 0.1.36 | MIT | crates.io |
| tracing-log | 0.2.0 | MIT | crates.io |
| tracing-subscriber | 0.3.20 | MIT | crates.io |
| triomphe | 0.1.17 | MIT OR Apache-2.0 | crates.io |
| typenum | 1.19.0 | MIT OR Apache-2.0 | crates.io |
| unicode-id-start | 1.5.0 | AND Unicode-3.0 | crates.io |
| unicode-ident | 1.0.27 | AND Unicode-3.0 | crates.io |
| unicode-linebreak | 0.1.5 | Apache-2.0 | crates.io |
| unicode-segmentation | 1.13.3 | MIT OR Apache-2.0 | crates.io |
| unicode-width | 0.1.14 | MIT OR Apache-2.0 | crates.io |
| unicode-width | 0.2.3 | MIT OR Apache-2.0 | crates.io |
| unicode-xid | 0.2.6 | MIT OR Apache-2.0 | crates.io |
| url | 2.5.8 | MIT OR Apache-2.0 | crates.io |
| utf16_iter | 1.1.0 | Apache-2.0 OR MIT | crates.io |
| utf8_iter | 1.1.0 | Apache-2.0 OR MIT | crates.io |
| utf_types | 1.0.0 | Apache-2.0 OR MIT | crates.io |
| uuid | 1.29.0 | Apache-2.0 OR MIT | crates.io |
| value-trait | 0.12.1 | Apache-2.0/MIT | crates.io |
| vsimd | 0.8.0 | MIT | crates.io |
| write16 | 1.0.0 | Apache-2.0 OR MIT | crates.io |
| writeable | 0.6.4 | Unicode-3.0 | crates.io |
| wyz | 0.5.1 | MIT | crates.io |
| xxhash-rust | 0.8.18 | BSL-1.0 | crates.io |
| yansi | 1.0.1 | MIT OR Apache-2.0 | crates.io |
| yoke | 0.8.4 | Unicode-3.0 | crates.io |
| zerocopy | 0.8.63 | BSD-2-Clause OR Apache-2.0 OR MIT | crates.io |
| zerofrom | 0.1.8 | Unicode-3.0 | crates.io |
| zerotrie | 0.2.5 | Unicode-3.0 | crates.io |
| zerovec | 0.11.9 | Unicode-3.0 | crates.io |
| zlib-rs | 0.5.4 | Zlib | crates.io |
| zmij | 1.0.23 | MIT | crates.io |
