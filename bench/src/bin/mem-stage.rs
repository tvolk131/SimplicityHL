//! Per-stage heap profiling with dhat. One stage per process invocation, so
//! dhat's peak attribution covers exactly that stage: setup work (building a
//! template before profiling codegen, say) happens before the profiler
//! starts.
//!
//!     cargo run --manifest-path bench/Cargo.toml --release --bin mem-stage -- for_while:2 codegen
//!     cargo run --manifest-path bench/Cargo.toml --release --bin mem-stage -- flat:4096 parse
//!
//! Stages: lex, parse, analyze, codegen, serialize, e2e.
//! dhat prints its report (total, peak, block counts) to stderr on exit.

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

use simplicityhl::ast::{ElementsJetHinter, Program as AstProgram};
use simplicityhl::driver::DependencyGraph;
use simplicityhl::error::DiagnosticManager;
use simplicityhl::parse::{ParseFromStrWithErrors, Program as ParseProgram};
use simplicityhl::{Arguments, TemplateProgram};
use simplicityhl_bench::corpus;
use simplicityhl_bench::harness;
use simplicityhl_bench::harness::Materialized;
use std::sync::Arc;

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(spec), Some(stage)) = (args.next(), args.next()) else {
        eprintln!("usage: mem-stage <spec> <stage>");
        eprintln!("stages: lex, parse, analyze, codegen, serialize, e2e");
        std::process::exit(2);
    };

    let program = corpus::generate(&spec).unwrap_or_else(|e| {
        eprintln!("bad spec '{spec}': {e}");
        std::process::exit(2);
    });

    // The profiler brackets only the requested stage; setup below is charged
    // to nobody.
    match stage.as_str() {
        "lex" => {
            let source = program.entry_source().to_string();
            let _profiler = dhat::Profiler::builder().build();
            let (tokens, errs) = simplicityhl::lexer::lex(0, &source, 0);
            assert!(errs.is_empty() && tokens.is_some());
            black_box(&tokens);
        }
        "parse" => {
            let source = program.entry_source().to_string();
            let _profiler = dhat::Profiler::builder().build();
            let mut diagnostics = DiagnosticManager::new();
            let ast = ParseProgram::parse_from_str_with_errors(
                0,
                &source,
                &harness::unstable(),
                &mut diagnostics,
            );
            assert!(ast.is_some() && !diagnostics.has_errors());
        }
        "analyze" => {
            let resolved = build_resolved(&program);
            let _profiler = dhat::Profiler::builder().build();
            let analyzed = AstProgram::analyze(&resolved, Box::new(ElementsJetHinter::new()));
            assert!(analyzed.is_ok());
        }
        "codegen" => {
            let template = build_template(&program);
            let _profiler = dhat::Profiler::builder().build();
            let compiled = template.instantiate(Arguments::default(), false);
            assert!(compiled.is_ok());
        }
        "serialize" => {
            let compiled = build_template(&program)
                .instantiate(Arguments::default(), false)
                .expect("codegen");
            let _profiler = dhat::Profiler::builder().build();
            let bytes = compiled.commit().to_vec_without_witness();
            assert!(!bytes.is_empty());
        }
        "e2e" => {
            let _profiler = dhat::Profiler::builder().build();
            let compiled = build_template(&program)
                .instantiate(Arguments::default(), false)
                .expect("codegen");
            let bytes = compiled.commit().to_vec_without_witness();
            assert!(!bytes.is_empty());
        }
        other => {
            eprintln!(
                "unknown stage '{other}' (expected lex, parse, analyze, codegen, serialize, e2e)"
            );
            std::process::exit(2);
        }
    }
}

fn build_resolved(program: &corpus::GeneratedProgram) -> ParseProgram {
    let materialized = Materialized::new(program, "mem").expect("materialize");
    let dependencies = Arc::new(materialized.dependency_map().expect("dependency map"));
    let (resolved, diagnostics) = DependencyGraph::build_program(
        materialized.source_file().expect("source file"),
        dependencies,
        &harness::unstable(),
    );
    assert!(!diagnostics.has_errors());
    resolved.expect("driver resolves")
}

fn build_template(program: &corpus::GeneratedProgram) -> TemplateProgram {
    let materialized = Materialized::new(program, "mem").expect("materialize");
    let dependencies = Arc::new(materialized.dependency_map().expect("dependency map"));
    TemplateProgram::new_with_dep(
        materialized.source_file().expect("source file"),
        &dependencies,
        &harness::unstable(),
        Box::new(ElementsJetHinter::new()),
    )
    .expect("template compiles")
}

#[allow(dead_code)]
fn black_box<T>(value: T) -> T {
    std::hint::black_box(value)
}
