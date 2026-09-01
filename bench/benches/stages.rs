//! Per-stage criterion benchmarks over the generated corpus.
//!
//! Run everything:            cargo bench --manifest-path bench/Cargo.toml
//! Run one group:             cargo bench --manifest-path bench/Cargo.toml -- lex
//! Run one benchmark:         cargo bench --manifest-path bench/Cargo.toml -- "codegen/for_while:2"
//! Compare against a baseline (see bench/README.md):
//!     cargo bench -- ... --save-baseline before   # then after a change: --baseline before

use chumsky::input::Input;
use chumsky::Parser;
use criterion::{criterion_group, criterion_main, Criterion};
use simplicityhl::ast::{ElementsJetHinter, Program as AstProgram};
use simplicityhl::driver::DependencyGraph;
use simplicityhl::error::{DiagnosticManager, Span};
use simplicityhl::parse::{ChumskyParse, ParseFromStrWithErrors, Program as ParseProgram};
use simplicityhl::{Arguments, TemplateProgram, UnresolvedValues};
use simplicityhl_bench::corpus::{self, GeneratedProgram};
use simplicityhl_bench::harness;
use simplicityhl_bench::harness::Materialized;
use std::hint::black_box;
use std::sync::Arc;

/// Benchmarks that need the program on disk (everything from the driver on).
fn materialize(program: &GeneratedProgram, tag: &str) -> Materialized {
    Materialized::new(program, tag).expect("materializing generated program")
}

fn unstable() -> simplicityhl::UnstableFeatures {
    harness::unstable()
}

fn bench_lex(c: &mut Criterion) {
    let mut group = c.benchmark_group("lex");
    for spec in corpus::FRONT_LADDER {
        let program = corpus::generate(spec).expect("generates");
        let source = program.entry_source().to_string();
        group.bench_function(spec.to_string(), |b| {
            b.iter(|| {
                let (tokens, errs) = simplicityhl::lexer::lex(0, black_box(&source), 0);
                assert!(!errs.is_empty() || tokens.is_some());
                black_box(tokens);
            })
        });
    }
    group.finish();
}

/// Full front-end per file: directive prescan, lexing, parsing, feature check.
fn bench_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse");
    for spec in corpus::FRONT_LADDER {
        let program = corpus::generate(spec).expect("generates");
        let source = program.entry_source().to_string();
        group.bench_function(spec.to_string(), |b| {
            b.iter(|| {
                let mut diagnostics = DiagnosticManager::new();
                let ast = ParseProgram::parse_from_str_with_errors(
                    0,
                    black_box(&source),
                    &unstable(),
                    &mut diagnostics,
                );
                assert!(
                    ast.is_some() && !diagnostics.has_errors(),
                    "{spec} failed to parse"
                );
            })
        });
    }
    group.finish();
}

/// Token-level parsing only: tokens are lexed once in setup, so this isolates
/// the chumsky grammar from the lexer (mirrors `parse::pipeline::parse_ast`).
fn bench_parse_only(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_only");
    for spec in corpus::FRONT_LADDER {
        let program = corpus::generate(spec).expect("generates");
        let source = program.entry_source().to_string();
        let (tokens, lex_errs) = simplicityhl::lexer::lex(0, &source, 0);
        assert!(
            lex_errs.is_empty() && tokens.is_some(),
            "{spec} failed to lex"
        );
        let tokens = tokens.expect("checked above");
        group.bench_function(spec.to_string(), |b| {
            b.iter(|| {
                let eoi = Span::eof(0, source.len());
                let (ast, errs) = ParseProgram::parser()
                    .parse(black_box(tokens.as_slice()).map(eoi, |(t, s)| (t, s)))
                    .into_output_errors();
                assert!(errs.is_empty() && ast.is_some(), "{spec} failed to parse");
            })
        });
    }
    group.finish();
}

/// Name resolution and type checking on the flattened parse tree.
fn bench_analyze(c: &mut Criterion) {
    let mut group = c.benchmark_group("analyze");
    for spec in corpus::BACK_LADDER {
        let program = corpus::generate(spec).expect("generates");
        let materialized = materialize(&program, "analyze");
        let dependencies = Arc::new(materialized.dependency_map().expect("dependency map"));
        let (resolved, diagnostics) = DependencyGraph::build_program(
            materialized.source_file().expect("source file"),
            dependencies,
            &unstable(),
        );
        let resolved = resolved.expect("driver resolves generated programs");
        assert!(!diagnostics.has_errors());
        group.bench_function(spec.to_string(), |b| {
            b.iter(|| {
                let analyzed =
                    AstProgram::analyze(black_box(&resolved), Box::new(ElementsJetHinter::new()));
                assert!(analyzed.is_ok(), "{spec} failed to analyze");
            })
        });
    }
    group.finish();
}

/// Codegen and type finalization: typed AST to finalized Commit DAG.
fn bench_codegen(c: &mut Criterion) {
    let mut group = c.benchmark_group("codegen");
    for spec in corpus::BACK_LADDER {
        let program = corpus::generate(spec).expect("generates");
        let materialized = materialize(&program, "codegen");
        let dependencies = Arc::new(materialized.dependency_map().expect("dependency map"));
        let template = TemplateProgram::new_with_dep(
            materialized.source_file().expect("source file"),
            &dependencies,
            &unstable(),
            Box::new(ElementsJetHinter::new()),
        )
        .expect("template compiles");
        group.bench_function(spec.to_string(), |b| {
            b.iter(|| {
                let compiled = template.instantiate(Arguments::default(), false);
                assert!(compiled.is_ok(), "{spec} failed to codegen");
            })
        });
    }
    group.finish();
}

/// Bitcode serialization of the commit DAG (witness-less form).
fn bench_serialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialize");
    let specs: Vec<&str> = corpus::BACK_LADDER
        .iter()
        .copied()
        .take(3)
        .chain(["for_while:2"])
        .collect();
    for spec in specs {
        let program = corpus::generate(spec).expect("generates");
        let materialized = materialize(&program, "serialize");
        let dependencies = Arc::new(materialized.dependency_map().expect("dependency map"));
        let template = TemplateProgram::new_with_dep(
            materialized.source_file().expect("source file"),
            &dependencies,
            &unstable(),
            Box::new(ElementsJetHinter::new()),
        )
        .expect("template compiles");
        let compiled = template
            .instantiate(Arguments::default(), false)
            .expect("codegen");
        group.bench_function(spec.to_string(), |b| {
            b.iter(|| {
                let bytes = compiled.commit().to_vec_without_witness();
                assert!(!bytes.is_empty());
            })
        });
    }
    group.finish();
}

/// Witness parsing, resolution, satisfaction, and serialization with witness
/// data, over the real examples that ship `.wit` files.
fn bench_satisfy(c: &mut Criterion) {
    let mut group = c.benchmark_group("satisfy");
    for (program, wit_text) in corpus::real_examples_with_witness() {
        let name = program.name().to_string();
        let materialized = materialize(&program, "satisfy");
        let dependencies = Arc::new(materialized.dependency_map().expect("dependency map"));
        let template = TemplateProgram::new_with_dep(
            materialized.source_file().expect("source file"),
            &dependencies,
            &unstable(),
            Box::new(ElementsJetHinter::new()),
        )
        .expect("template compiles");
        let arguments = harness::arguments_for(&template, &name).expect("args resolve");
        let compiled = template.instantiate(arguments, false).expect("codegen");
        let unresolved: UnresolvedValues =
            serde_json::from_str(&wit_text).expect("witness json parses");
        let witness: simplicityhl::WitnessValues = unresolved
            .resolve(compiled.witness_types())
            .expect("witness resolves");
        group.bench_function(name.clone(), |b| {
            b.iter(|| {
                let satisfied = compiled.satisfy(witness.clone());
                assert!(satisfied.is_ok(), "{name} failed to satisfy");
                let (program_bytes, _) = satisfied.expect("checked").redeem().to_vec_with_witness();
                assert!(!program_bytes.is_empty());
            })
        });
    }
    group.finish();
}

/// The multi-file driver: file discovery, dependency resolution, parsing of
/// every file, and assembly into one program. File contents are read from the
/// OS page cache, which matches repeated compilation of a warm project.
fn bench_driver(c: &mut Criterion) {
    let mut group = c.benchmark_group("driver");
    for spec in corpus::DRIVER_LADDER {
        let program = corpus::generate(spec).expect("generates");
        let materialized = materialize(&program, "driver");
        let dependencies = Arc::new(materialized.dependency_map().expect("dependency map"));
        let source = materialized.source_file().expect("source file");
        group.bench_function(spec.to_string(), |b| {
            b.iter(|| {
                let (resolved, diagnostics) = DependencyGraph::build_program(
                    source.clone(),
                    Arc::clone(&dependencies),
                    &unstable(),
                );
                assert!(
                    resolved.is_some() && !diagnostics.has_errors(),
                    "{spec} failed"
                );
            })
        });
    }
    group.finish();
}

/// End-to-end: template (parse + driver + analyze), instantiation (codegen),
/// and serialization. What a user of `simc` experiences.
fn bench_end_to_end(c: &mut Criterion) {
    let mut group = c.benchmark_group("end_to_end");
    let specs: Vec<&str> = corpus::BACK_LADDER.to_vec();
    for spec in specs {
        let program = corpus::generate(spec).expect("generates");
        let materialized = materialize(&program, "e2e");
        let dependencies = Arc::new(materialized.dependency_map().expect("dependency map"));
        let source = materialized.source_file().expect("source file");
        group.bench_function(spec.to_string(), |b| {
            b.iter(|| {
                let template = TemplateProgram::new_with_dep(
                    source.clone(),
                    &dependencies,
                    &unstable(),
                    Box::new(ElementsJetHinter::new()),
                )
                .expect("template compiles");
                let compiled = template
                    .instantiate(Arguments::default(), false)
                    .expect("codegen");
                assert!(!compiled.commit().to_vec_without_witness().is_empty());
            })
        });
    }
    for program in corpus::real_examples() {
        let name = program.name().to_string();
        let materialized = materialize(&program, "e2e-real");
        let dependencies = Arc::new(materialized.dependency_map().expect("dependency map"));
        let source = materialized.source_file().expect("source file");
        group.bench_function(name.clone(), |b| {
            b.iter(|| {
                let template = TemplateProgram::new_with_dep(
                    source.clone(),
                    &dependencies,
                    &unstable(),
                    Box::new(ElementsJetHinter::new()),
                )
                .expect("template compiles");
                // Mirrors `simc`: the args file is parsed against the
                // program's parameter types once the template exists.
                let arguments = harness::arguments_for(&template, &name).expect("args resolve");
                let compiled = template.instantiate(arguments, false).expect("codegen");
                assert!(!compiled.commit().to_vec_without_witness().is_empty());
            })
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_lex,
    bench_parse,
    bench_parse_only,
    bench_analyze,
    bench_codegen,
    bench_serialize,
    bench_satisfy,
    bench_driver,
    bench_end_to_end,
);
criterion_main!(benches);
