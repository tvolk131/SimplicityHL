//! Materialize the generated benchmark corpus to disk, for use with external
//! profilers and `simc` directly.
//!
//!     cargo run --manifest-path bench/Cargo.toml --release --bin corpus-gen
//!     cargo run --manifest-path bench/Cargo.toml --release --bin corpus-gen -- --out /tmp/corpus
//!     cargo run --manifest-path bench/Cargo.toml --release --bin corpus-gen -- --check
//!
//! `--check` compiles every generated program end-to-end (in a temp dir, not
//! the output dir) and fails loudly if the corpus stopped being valid
//! SimplicityHL — run it whenever the language or the generators change.

use simplicityhl_bench::corpus::{self, GeneratedProgram};
use simplicityhl_bench::harness;

fn main() {
    let mut args = std::env::args().skip(1).peekable();
    let mut check_only = false;
    let mut out = std::path::PathBuf::from("bench/corpus");
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--check" => check_only = true,
            "--out" => {
                out = std::path::PathBuf::from(args.next().unwrap_or_else(|| {
                    eprintln!("--out requires a directory argument");
                    std::process::exit(2);
                }));
            }
            other => {
                eprintln!("unknown argument '{other}' (expected --check or --out DIR)");
                std::process::exit(2);
            }
        }
    }

    let mut specs: Vec<String> = corpus::FRONT_LADDER
        .iter()
        .chain(corpus::BACK_LADDER)
        .chain(corpus::DRIVER_LADDER)
        .map(|spec| (*spec).to_string())
        .collect();
    // The documented slow case: ~10s to compile. Included so profilers have a
    // real jet-heavy blowup target; pass --out to get it on disk.
    specs.push("real:hash_loop".to_string());

    let mut failures = 0;
    for spec in &specs {
        let program = match corpus::generate(spec) {
            Ok(program) => program,
            Err(e) => {
                eprintln!("FAIL {spec}: {e}");
                failures += 1;
                continue;
            }
        };
        let metrics = corpus::metrics(program.entry_source());

        if check_only {
            match harness::compile_end_to_end(&program) {
                Ok(()) => println!(
                    "OK   {spec:<16} {:>7} bytes {:>5} lines {:>6} tokens",
                    metrics.bytes, metrics.lines, metrics.tokens
                ),
                Err(e) => {
                    eprintln!("FAIL {spec}: compiles end-to-end failed\n{e}");
                    failures += 1;
                }
            }
            continue;
        }

        let dest = out.join(spec.replace(':', "_"));
        let result = write_program(&program, &dest);
        match result {
            Ok(()) => println!(
                "wrote {spec:<16} {:>7} bytes {:>5} lines {:>6} tokens -> {}",
                metrics.bytes,
                metrics.lines,
                metrics.tokens,
                dest.display()
            ),
            Err(e) => {
                eprintln!("FAIL {spec}: {e}");
                failures += 1;
            }
        }
    }

    if failures > 0 {
        eprintln!("{failures} spec(s) failed");
        std::process::exit(1);
    }
}

fn write_program(program: &GeneratedProgram, dest: &std::path::Path) -> Result<(), String> {
    match program {
        GeneratedProgram::Single { source, .. } => {
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
            }
            let file = dest.with_extension("simf");
            std::fs::write(&file, source)
                .map_err(|e| format!("cannot write {}: {e}", file.display()))
        }
        GeneratedProgram::Package { files, .. } => {
            for (rel, content) in files {
                let path = dest.join(rel);
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
                }
                std::fs::write(&path, content)
                    .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
            }
            Ok(())
        }
    }
}
