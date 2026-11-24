//! flow_blank CLI.
//!
//!   flow_blank blank <file>            print blanked source
//!   flow_blank fastcheck <list>        the oxc fast path vs Flow's parser, byte for byte
//!   flow_blank lower <file> [parse]    print the lowered splice text (or the parse text);
//!                                      env LOWER_ENUMS=1, REACT_19=1 set those options
//!   flow_blank norm <file>             parse a plain JS/JSX file with oxc, print normalized code
//!   flow_blank corpus <list> <outdir>  run the pipeline on every file in <list>; write
//!                                      <outdir>/<n>.blank.norm.js and a JSONL report on stdout
//!   flow_blank bench <list> <iters>    time flow_parser / blank / oxc separately (single thread)

use std::fmt::Write as _;
use std::time::Instant;

use flow_blank::{Options, blank_program, blank_program_with, flow_parse};

fn opts_from_env() -> Options {
    Options { remove_uninitialized_class_fields: std::env::var("BABEL7_FIELDS").is_ok(), ..Options::default() }
}
use oxc_allocator::Allocator;
use oxc_ast::ast::{Program, Statement};
use oxc_ast_visit::{Visit, VisitMut};
use oxc_codegen::{Codegen, CodegenOptions, CommentOptions};
use oxc_parser::{ParseOptions, Parser};
use oxc_span::SourceType;

fn source_type() -> SourceType {
    SourceType::default().with_module(true).with_jsx(true)
}

/// Normalization for comparison: drop empty statements, `/*@__PURE__*/`
/// marks and the parenthesized-IIFE (`pife`) hint (Babel runs with comments off, so it loses them; oxc keeps them as
/// flags and prints extra parens).
struct DropEmpty;
impl<'a> VisitMut<'a> for DropEmpty {
    fn visit_call_expression(&mut self, it: &mut oxc_ast::ast::CallExpression<'a>) {
        it.pure = false;
        oxc_ast_visit::walk_mut::walk_call_expression(self, it);
    }
    fn visit_function(&mut self, it: &mut oxc_ast::ast::Function<'a>, flags: oxc_syntax::scope::ScopeFlags) {
        it.pure = false;
        it.pife = false;
        oxc_ast_visit::walk_mut::walk_function(self, it, flags);
    }
    fn visit_arrow_function_expression(&mut self, it: &mut oxc_ast::ast::ArrowFunctionExpression<'a>) {
        it.pure = false;
        it.pife = false;
        oxc_ast_visit::walk_mut::walk_arrow_function_expression(self, it);
    }
    fn visit_new_expression(&mut self, it: &mut oxc_ast::ast::NewExpression<'a>) {
        it.pure = false;
        oxc_ast_visit::walk_mut::walk_new_expression(self, it);
    }
    fn visit_statements(&mut self, it: &mut oxc_allocator::Vec<'a, Statement<'a>>) {
        it.retain(|s| !matches!(s, Statement::EmptyStatement(_)));
        oxc_ast_visit::walk_mut::walk_statements(self, it);
    }
}

/// Checks that identifier spans in the blanked code point at the same text in the
/// original source (byte offsets preserved).
struct SpanCheck<'s> {
    orig: &'s str,
    checked: usize,
    bad: usize,
}
impl<'a, 's> Visit<'a> for SpanCheck<'s> {
    fn visit_identifier_reference(&mut self, it: &oxc_ast::ast::IdentifierReference<'a>) {
        self.check(it.span, it.name.as_str());
    }
    fn visit_binding_identifier(&mut self, it: &oxc_ast::ast::BindingIdentifier<'a>) {
        self.check(it.span, it.name.as_str());
    }
}
impl SpanCheck<'_> {
    fn check(&mut self, span: oxc_span::Span, name: &str) {
        self.checked += 1;
        let (s, e) = (span.start as usize, span.end as usize);
        if self.orig.get(s..e) != Some(name) {
            // Escaped identifiers (\u0061) are legal but rare; count them as bad too.
            self.bad += 1;
        }
    }
}

fn oxc_parse<'a>(alloc: &'a Allocator, code: &'a str) -> Result<Program<'a>, String> {
    let ret = Parser::new(alloc, code, source_type())
        .with_options(ParseOptions { preserve_parens: false, ..ParseOptions::default() })
        .parse();
    if ret.diagnostics.has_errors() || ret.fatal_error {
        let msg = ret
            .diagnostics
            .errors()
            .take(3)
            .map(|d| {
                let at = d.labels.first().map(|l| l.offset() as usize).unwrap_or(0);
                let line = code[..at.min(code.len())].matches('\n').count() + 1;
                format!("{d} (line {line})")
            })
            .collect::<Vec<_>>()
            .join(" | ");
        if msg.contains("Flow is not supported") && code.contains("@flow") {
            // oxc replaces the real diagnostic when the first comment has `@flow`.
            // Re-parse a same-length copy with the pragma defused to get it.
            let defused = alloc.alloc_str(&code.replacen("@flow", "@flaw", 1));
            let ret = Parser::new(alloc, defused, source_type())
                .with_options(ParseOptions { preserve_parens: false, ..ParseOptions::default() })
                .parse();
            let real = ret
                .diagnostics
                .errors()
                .take(3)
                .map(|d| {
                    let at = d.labels.first().map(|l| l.offset() as usize).unwrap_or(0);
                    let line = code[..at.min(code.len())].matches('\n').count() + 1;
                    format!("{d} (line {line})")
                })
                .collect::<Vec<_>>()
                .join(" | ");
            return Err(format!("[@flow masked] {real}"));
        }
        return Err(msg);
    }
    Ok(ret.program)
}

fn normalize(alloc: &Allocator, code: &str) -> Result<String, String> {
    let code = alloc.alloc_str(code);
    let mut program = oxc_parse(alloc, code)?;
    DropEmpty.visit_program(&mut program);
    let opts = CodegenOptions { comments: CommentOptions::disabled(), ..CodegenOptions::default() };
    Ok(Codegen::new().with_options(opts).build(&program).code)
}

fn json_str(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(o, "\\u{:04x}", c as u32);
            }
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn same_layout(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes().zip(b.bytes()).all(|(x, y)| (x == b'\n') == (y == b'\n') && (x == b'\r') == (y == b'\r'))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("lower") => {
            // flow_blank lower <file> [parse]: print the splice text (or the parse text)
            let src = std::fs::read_to_string(&args[2]).unwrap();
            let lopts = Options {
                lower_enums: std::env::var("LOWER_ENUMS").is_ok(),
                react_19: std::env::var("REACT_19").is_ok(),
                ..Options::default()
            };
            match flow_blank::prepare_with(&src, lopts) {
                Ok(p) => {
                    let text = if args.get(3).map(|s| s.as_str()) == Some("parse") {
                        p.parse
                    } else {
                        p.splice.unwrap_or(src)
                    };
                    print!("{text}");
                    for i in &p.internal {
                        eprintln!("internal: {i}");
                    }
                    for (k, at) in &p.unsupported {
                        eprintln!("unsupported: {k} at {at}");
                    }
                }
                Err(e) => {
                    eprintln!("{}", e.join("\n"));
                    std::process::exit(1);
                }
            }
        }
        Some("fastcheck") => {
            // flow_blank fastcheck <list>: the oxc fast path against Flow's parser, file by file.
            let list = std::fs::read_to_string(&args[2]).unwrap();
            let (mut n, mut fast, mut same, mut refs_same, mut slow_err) = (0, 0, 0, 0, 0);
            let mut shown = 0;
            for path in list.lines().filter(|l| !l.is_empty()) {
                let Ok(src) = std::fs::read_to_string(path) else { continue };
                n += 1;
                let Some(f) = flow_blank::fast_prepare(&src) else { continue };
                fast += 1;
                match flow_blank::prepare_flow_parser(&src, Options::default()) {
                    Err(e) => {
                        slow_err += 1;
                        if shown < 10 {
                            shown += 1;
                            println!("SLOW-ERR {path}: {}", e.join(" | "));
                        }
                    }
                    Ok(s) => {
                        let text = s.splice.is_none() && s.parse == f.parse;
                        if text {
                            same += 1;
                        } else if shown < 10 {
                            shown += 1;
                            let at = s.parse.bytes().zip(f.parse.bytes()).position(|(a, b)| a != b).unwrap_or(0);
                            let lo = at.saturating_sub(40);
                            let show = |t: &str| t.get(lo..(at + 40).min(t.len())).unwrap_or("").replace('\n', "⏎");
                            println!("TEXT {path} @{at}\n  flow: {:?}\n  fast: {:?}", show(&s.parse), show(&f.parse));
                        }
                        if s.type_refs == f.type_refs {
                            refs_same += 1;
                        } else if shown < 10 {
                            shown += 1;
                            let a: std::collections::BTreeSet<_> = s.type_refs.iter().collect();
                            let b: std::collections::BTreeSet<_> = f.type_refs.iter().collect();
                            println!("REFS {path}: flow-only {:?} fast-only {:?}", a.difference(&b).collect::<Vec<_>>(), b.difference(&a).collect::<Vec<_>>());
                        }
                    }
                }
            }
            println!("{{\"files\":{n},\"fast\":{fast},\"text_same\":{same},\"refs_same\":{refs_same},\"flow_errors\":{slow_err}}}");
            if same != fast || refs_same != fast || slow_err != 0 {
                std::process::exit(1);
            }
        }
        Some("blank") => {
            let src = std::fs::read_to_string(&args[2]).unwrap();
            match flow_blank::blank_with(&src, opts_from_env()) {
                Ok(out) => {
                    print!("{}", out.code);
                    for (k, off) in &out.unsupported {
                        eprintln!("unsupported {k} at byte {off}");
                    }
                    for m in &out.internal {
                        eprintln!("internal: {m}");
                    }
                }
                Err(errs) => {
                    eprintln!("flow parse error: {errs:?}");
                    std::process::exit(1);
                }
            }
        }
        Some("norm") => {
            let src = std::fs::read_to_string(&args[2]).unwrap();
            let alloc = Allocator::default();
            match normalize(&alloc, &src) {
                Ok(c) => print!("{c}"),
                Err(e) => {
                    eprintln!("oxc error: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some("corpus") => corpus(&args[2], &args[3]),
        Some("bench") => bench(&args[2], args.get(3).map(|s| s.parse().unwrap()).unwrap_or(5)),
        _ => {
            eprintln!("usage: flow_blank blank|norm|corpus|bench ...");
            std::process::exit(2);
        }
    }
}

fn corpus(list: &str, outdir: &str) {
    std::fs::create_dir_all(outdir).unwrap();
    let files: Vec<String> =
        std::fs::read_to_string(list).unwrap().lines().filter(|l| !l.is_empty()).map(String::from).collect();
    for (n, path) in files.iter().enumerate() {
        let src = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                println!("{{\"n\":{n},\"path\":{},\"io\":{}}}", json_str(path), json_str(&e.to_string()));
                continue;
            }
        };
        let mut rec = format!("{{\"n\":{n},\"path\":{},\"bytes\":{}", json_str(path), src.len());
        let parsed = std::panic::catch_unwind(|| flow_parse(&src));
        let (program, errors) = match parsed {
            Ok(p) => p,
            Err(_) => {
                rec.push_str(",\"flow_ok\":false,\"flow_err\":\"panic\"}");
                println!("{rec}");
                continue;
            }
        };
        if !errors.is_empty() {
            let msg: Vec<String> =
                errors.iter().take(3).map(|(l, e)| format!("{}:{}: {:?}", l.start.line, l.start.column, e)).collect();
            let _ = write!(rec, ",\"flow_ok\":false,\"flow_err\":{}}}", json_str(&msg.join(" | ")));
            println!("{rec}");
            continue;
        }
        let out = blank_program_with(&src, &program, opts_from_env());
        let kinds: Vec<String> = out.unsupported.iter().map(|(k, o)| format!("{k}@{o}")).collect();
        let _ = write!(
            rec,
            ",\"flow_ok\":true,\"ranges\":{},\"unsupported\":{},\"internal\":{},\"layout_ok\":{}",
            out.ranges,
            json_str(&kinds.join(",")),
            json_str(&out.internal.join(" | ")),
            same_layout(&src, &out.code)
        );
        std::fs::write(format!("{outdir}/{n}.blank.js"), &out.code).unwrap();
        let alloc = Allocator::default();
        let code = alloc.alloc_str(&out.code);
        match oxc_parse(&alloc, code) {
            Ok(mut program) => {
                let mut sc = SpanCheck { orig: &src, checked: 0, bad: 0 };
                sc.visit_program(&program);
                DropEmpty.visit_program(&mut program);
                let opts = CodegenOptions { comments: CommentOptions::disabled(), ..CodegenOptions::default() };
                let norm = Codegen::new().with_options(opts).build(&program).code;
                std::fs::write(format!("{outdir}/{n}.blank.norm.js"), norm).unwrap();
                let _ = write!(rec, ",\"oxc_ok\":true,\"ids_checked\":{},\"ids_bad\":{}}}", sc.checked, sc.bad);
            }
            Err(e) => {
                let _ = write!(rec, ",\"oxc_ok\":false,\"oxc_err\":{}}}", json_str(&e));
            }
        }
        println!("{rec}");
    }
}

fn bench(list: &str, iters: usize) {
    let files: Vec<String> = std::fs::read_to_string(list)
        .unwrap()
        .lines()
        .filter(|l| !l.is_empty())
        .map(|p| std::fs::read_to_string(p).unwrap())
        .collect();
    // Keep only files that go through the whole pipeline.
    let files: Vec<String> = files
        .into_iter()
        .filter(|s| {
            let (_, errs) = flow_parse(s);
            if !errs.is_empty() {
                return false;
            }
            let b = flow_blank::blank(s).unwrap();
            let alloc = Allocator::default();
            b.unsupported.is_empty() && oxc_parse(&alloc, alloc.alloc_str(&b.code)).is_ok()
        })
        .collect();
    let blanked: Vec<String> = files.iter().map(|s| flow_blank::blank(s).unwrap().code).collect();
    let bytes: usize = files.iter().map(|s| s.len()).sum();
    let mb = bytes as f64 / 1e6;
    let (mut t_flow, mut t_blank, mut t_oxc, mut t_drop) = (f64::MAX, f64::MAX, f64::MAX, f64::MAX);
    for _ in 0..iters {
        let t0 = Instant::now();
        let progs: Vec<_> = files.iter().map(|s| flow_parse(s).0).collect();
        let t1 = Instant::now();
        for (s, p) in files.iter().zip(&progs) {
            std::hint::black_box(blank_program(s, p));
        }
        let t2 = Instant::now();
        drop(progs);
        let t3 = Instant::now();
        t_drop = t_drop.min((t3 - t2).as_secs_f64());
        for b in &blanked {
            let alloc = Allocator::default();
            let ret = Parser::new(&alloc, b, source_type()).parse();
            std::hint::black_box(&ret.program);
        }
        let t4 = Instant::now();
        t_flow = t_flow.min((t1 - t0).as_secs_f64());
        t_blank = t_blank.min((t2 - t1).as_secs_f64());
        t_oxc = t_oxc.min((t4 - t3).as_secs_f64());
    }
    println!("files={} bytes={} iters={} (best of)", files.len(), bytes, iters);
    println!("flow_parser parse : {:8.2} ms  {:7.1} MB/s", t_flow * 1e3, mb / t_flow);
    println!("blank (visit+fill): {:8.2} ms  {:7.1} MB/s", t_blank * 1e3, mb / t_blank);
    println!("oxc parse blanked : {:8.2} ms  {:7.1} MB/s", t_oxc * 1e3, mb / t_oxc);
    println!("drop flow AST     : {:8.2} ms", t_drop * 1e3);
    let tot = t_flow + t_blank + t_oxc + t_drop;
    println!("pipeline total    : {:8.2} ms  {:7.1} MB/s", tot * 1e3, mb / tot);
}
