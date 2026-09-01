//! Deterministic corpus of generated SimplicityHL programs.
//!
//! Line count does not predict compile time in this compiler: bounded loops
//! and folds blow up the generated Simplicity DAG, while long flat programs
//! stress the front-end. Each shape targets one suspected hotspot, so a
//! regression can be attributed to the code that caused it.
//!
//! Every generator is driven by [`crate::rng::Rng`] with a fixed per-shape
//! seed: the same spec always produces byte-identical source.

use crate::rng::Rng;
use std::fmt;
use std::path::PathBuf;

/// A corpus program: either a single file, or a multi-file package with an
/// entry file (the driver's `crate::` root is the package directory).
#[derive(Debug, Clone)]
pub enum GeneratedProgram {
    Single {
        name: String,
        source: String,
    },
    Package {
        name: String,
        /// Relative paths (e.g. `lib0.simf`) and their contents.
        files: Vec<(String, String)>,
        /// Relative path of the entry file (always `main.simf` today).
        entry: String,
    },
}

impl GeneratedProgram {
    /// The spec-style name of this program (used for paths and bench ids).
    pub fn name(&self) -> &str {
        match self {
            GeneratedProgram::Single { name, .. } => name,
            GeneratedProgram::Package { name, .. } => name,
        }
    }

    /// The entry-file source text.
    pub fn entry_source(&self) -> &str {
        match self {
            GeneratedProgram::Single { source, .. } => source,
            GeneratedProgram::Package { files, entry, .. } => {
                &files
                    .iter()
                    .find(|(path, _)| path == entry)
                    .expect("entry in files")
                    .1
            }
        }
    }
}

/// The shape of a generated program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// Long flat `main`: sequential bindings and asserts.
    /// Stresses lexing, parsing, and analysis.
    Flat(u32),
    /// `depth` nested blocks with `width` bindings each (a block-expression
    /// per level, README-style). Stresses scope handling in analysis and the
    /// selector chains in codegen.
    Deep { depth: u32, width: u32 },
    /// Many small functions chained from `main`. Stresses item tables and
    /// call-site inlining.
    Functions(u32),
    /// `array_fold` over an N-element array. Stresses balanced-fold codegen.
    ArrayFold(u32),
    /// N sequential `for_while` loops with a cheap body (256 iterations each:
    /// the counter is a `u8`). Stresses loop codegen blowup without the cost
    /// of real hash jets. For the jet-heavy variant, profile
    /// `examples/hash_loop.simf` directly.
    ForWhile(u32),
    /// A package of `files` library files plus an entry. Star: the entry
    /// imports from every file. Chain: file i imports from file i+1.
    /// Stresses the driver.
    MultiFile { files: u32, chain: bool },
}

/// A parsed corpus spec, e.g. `flat:256`, `deep:32x8`, `real:p2pkh`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Spec {
    Shape(Shape),
    /// An example program from the repository's `examples/` directory.
    Real(String),
}

impl fmt::Display for Spec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Spec::Shape(Shape::Flat(n)) => write!(f, "flat:{n}"),
            Spec::Shape(Shape::Deep { depth, width }) => write!(f, "deep:{depth}x{width}"),
            Spec::Shape(Shape::Functions(n)) => write!(f, "funcs:{n}"),
            Spec::Shape(Shape::ArrayFold(n)) => write!(f, "array:{n}"),
            Spec::Shape(Shape::ForWhile(n)) => write!(f, "for_while:{n}"),
            Spec::Shape(Shape::MultiFile { files, chain }) => {
                let kind = if *chain { "chain" } else { "multifile" };
                write!(f, "{kind}:{files}")
            }
            Spec::Real(name) => write!(f, "real:{name}"),
        }
    }
}

fn parse_spec(spec: &str) -> Result<Shape, String> {
    let (kind, arg) = spec
        .split_once(':')
        .ok_or(format!("spec '{spec}' is not <shape>:<arg>"))?;
    match kind {
        "flat" => Ok(Shape::Flat(
            arg.parse().map_err(|_| format!("bad size '{arg}'"))?,
        )),
        "deep" => {
            let (depth, width) = arg
                .split_once('x')
                .ok_or(format!("deep spec is depth x width, got '{arg}'"))?;
            Ok(Shape::Deep {
                depth: depth.parse().map_err(|_| format!("bad depth '{depth}'"))?,
                width: width.parse().map_err(|_| format!("bad width '{width}'"))?,
            })
        }
        "funcs" => Ok(Shape::Functions(
            arg.parse().map_err(|_| format!("bad size '{arg}'"))?,
        )),
        "array" => Ok(Shape::ArrayFold(
            arg.parse().map_err(|_| format!("bad size '{arg}'"))?,
        )),
        "for_while" => Ok(Shape::ForWhile(
            arg.parse().map_err(|_| format!("bad size '{arg}'"))?,
        )),
        "multifile" => Ok(Shape::MultiFile {
            files: arg.parse().map_err(|_| format!("bad size '{arg}'"))?,
            chain: false,
        }),
        "chain" => Ok(Shape::MultiFile {
            files: arg.parse().map_err(|_| format!("bad size '{arg}'"))?,
            chain: true,
        }),
        _ => Err(format!("unknown shape '{kind}'")),
    }
}

/// Specs stressing the front-end (lex/parse); safe at large sizes because no
/// DAG is built.
pub const FRONT_LADDER: &[&str] = &[
    "flat:64",
    "flat:256",
    "flat:1024",
    "flat:4096",
    "deep:32x8",
    "funcs:64",
];

/// Specs stressing analysis and codegen; kept smaller because DAG blowup
/// (scopes, folds, loops) makes single iterations expensive.
pub const BACK_LADDER: &[&str] = &[
    "flat:64",
    "flat:256",
    "deep:16x8",
    "deep:32x8",
    "funcs:32",
    "array:64",
    "array:128",
    "for_while:1",
    "for_while:2",
    "for_while:4",
];

/// Specs stressing the multi-file driver.
pub const DRIVER_LADDER: &[&str] = &["multifile:8", "multifile:32", "chain:32"];

/// Generate a program from a spec string (see [`Shape`] for the grammar).
///
/// `real:<name>` reads `examples/<name>.simf` from the repository.
pub fn generate(spec: &str) -> Result<GeneratedProgram, String> {
    if let Some(name) = spec.strip_prefix("real:") {
        let path = examples_dir().join(format!("{name}.simf"));
        let source = std::fs::read_to_string(&path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        return Ok(GeneratedProgram::Single {
            name: spec.to_string(),
            source,
        });
    }
    let shape = parse_spec(spec)?;
    Ok(generate_shape(shape))
}

/// The repository's `examples/` directory, resolved relative to this crate.
pub fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples")
}

/// All top-level example programs (the realistic corpus).
pub fn real_examples() -> Vec<GeneratedProgram> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(examples_dir()) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() || path.extension().is_none_or(|ext| ext != "simf") {
            continue;
        }
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let name = format!(
            "real:{}",
            path.file_stem().expect("simf has a stem").to_string_lossy()
        );
        out.push(GeneratedProgram::Single { name, source });
    }
    out.sort_by(|a, b| a.name().cmp(b.name()));
    out
}

/// Example programs that have a matching `.wit` file on disk.
pub fn real_examples_with_witness() -> Vec<(GeneratedProgram, String)> {
    real_examples()
        .into_iter()
        .filter_map(|program| {
            let stem = program.name().trim_start_matches("real:").to_string();
            let wit = examples_dir().join(format!("{stem}.wit"));
            let wit_text = std::fs::read_to_string(wit).ok()?;
            Some((program, wit_text))
        })
        .collect()
}

/// The `examples/<stem>.args` file for a spec name (`real:p2pk` or bare
/// `p2pk`), if the example takes program arguments.
pub fn example_args_text(name: &str) -> Option<String> {
    let stem = name.strip_prefix("real:").unwrap_or(name);
    std::fs::read_to_string(examples_dir().join(format!("{stem}.args"))).ok()
}

/// Generate the program for `shape`.
pub fn generate_shape(shape: Shape) -> GeneratedProgram {
    let name = spec_name(shape);
    match shape {
        Shape::Flat(n) => GeneratedProgram::Single {
            name,
            source: flat(n),
        },
        Shape::Deep { depth, width } => GeneratedProgram::Single {
            name,
            source: deep(depth, width),
        },
        Shape::Functions(n) => GeneratedProgram::Single {
            name,
            source: functions(n),
        },
        Shape::ArrayFold(n) => GeneratedProgram::Single {
            name,
            source: array_fold(n),
        },
        Shape::ForWhile(n) => GeneratedProgram::Single {
            name,
            source: for_while(n),
        },
        Shape::MultiFile { files, chain } => multifile(name, files, chain),
    }
}

fn spec_name(shape: Shape) -> String {
    Spec::Shape(shape).to_string()
}

/// Cheap shape metrics, printed alongside benchmarks so a regression can be
/// normalized (e.g. ns per token) rather than eyeballed.
#[derive(Debug, Clone, Copy)]
pub struct Metrics {
    pub bytes: usize,
    pub lines: usize,
    pub tokens: usize,
}

pub fn metrics(source: &str) -> Metrics {
    let (tokens, _) = simplicityhl::lexer::lex(0, source, 0);
    Metrics {
        bytes: source.len(),
        lines: source.lines().count(),
        tokens: tokens.map(|t| t.len()).unwrap_or(0),
    }
}

// Generators. All stick to idioms proven by the programs in `examples/`:
// typed `let` bindings, `jet::max_32` / destructured `jet::add_32`, `assert!`,
// and unit statements only (SimplicityHL statements must be unit-typed).

fn flat(statements: u32) -> String {
    let mut rng = Rng::new(0xF1A7);
    let mut out = String::from("fn main() {\n");
    out.push_str(&format!("    let x0: u32 = {};\n", rng.literal()));
    for i in 1..statements {
        if i % 16 == 0 {
            out.push_str(&format!(
                "    assert!(jet::eq_32(x{}, {}));\n",
                i - 1,
                rng.literal()
            ));
        }
        if i % 2 == 0 {
            out.push_str(&format!(
                "    let (_, x{i}): (bool, u32) = jet::add_32(x{}, {});\n",
                i - 1,
                rng.literal()
            ));
        } else {
            out.push_str(&format!(
                "    let x{i}: u32 = jet::max_32(x{}, {});\n",
                i - 1,
                rng.literal()
            ));
        }
    }
    let last = statements - 1;
    out.push_str(&format!("    assert!(jet::eq_32(x{last}, x{last}));\n"));
    out.push_str("}\n");
    out
}

fn deep(depth: u32, width: u32) -> String {
    let mut rng = Rng::new(0x0DEE_5EED);
    let mut out = String::from("fn main() {\n");
    out.push_str(&format!("    let seed: u32 = {};\n", rng.literal()));
    out.push_str("    let out: u32 = ");
    out.push_str(&deep_block(&mut rng, depth, width, 0, "seed"));
    out.push_str(";\n");
    out.push_str("    assert!(jet::eq_32(out, out));\n");
    out.push_str("}\n");
    out
}

/// `{ let dL0: u32 = ...; ...; <inner block or tail expr> }` — one level of
/// nesting. `prev` is the variable bound before this block.
fn deep_block(rng: &mut Rng, depth: u32, width: u32, level: u32, prev: &str) -> String {
    let mut block = String::from("{\n");
    let mut last = String::new();
    for w in 0..width {
        last = format!("d{level}v{w}");
        if w == 0 {
            block.push_str(&format!(
                "        let {last}: u32 = jet::max_32({prev}, {});\n",
                rng.literal()
            ));
        } else {
            let prev_w = format!("d{level}v{}", w - 1);
            block.push_str(&format!(
                "        let {last}: u32 = jet::max_32({prev_w}, {});\n",
                rng.literal()
            ));
        }
    }
    if level + 1 < depth {
        block.push_str("        let inner: u32 = ");
        block.push_str(&deep_block(rng, depth, width, level + 1, &last));
        block.push_str(";\n");
        block.push_str(&format!("        jet::max_32(inner, {last})\n"));
    } else {
        block.push_str(&format!("        jet::max_32({last}, {})\n", rng.literal()));
    }
    block.push_str("    }");
    block
}

fn functions(count: u32) -> String {
    let mut rng = Rng::new(0x0BAD_C0DE);
    let mut out = String::new();
    for i in 0..count {
        out.push_str(&format!("fn f{i}(a0: u32, a1: u32) -> u32 {{\n"));
        if i % 2 == 0 {
            out.push_str("    jet::max_32(a0, a1)\n");
        } else {
            out.push_str(&format!(
                "    let (_, s): (bool, u32) = jet::add_32(a0, a1);\n    jet::max_32(s, {})\n",
                rng.literal()
            ));
        }
        out.push_str("}\n\n");
    }
    out.push_str("fn main() {\n");
    out.push_str(&format!(
        "    let r0: u32 = f0({}, {});\n",
        rng.literal(),
        rng.literal()
    ));
    for i in 1..count {
        out.push_str(&format!(
            "    let r{i}: u32 = f{i}(r{}, {});\n",
            i - 1,
            rng.literal()
        ));
    }
    let last = count - 1;
    out.push_str(&format!("    assert!(jet::eq_32(r{last}, r{last}));\n"));
    out.push_str("}\n");
    out
}

fn array_fold(elements: u32) -> String {
    let mut rng = Rng::new(0x0000_F01D_11B2);
    let mut literals = Vec::with_capacity(elements as usize);
    let mut total: u64 = 0;
    for _ in 0..elements {
        let lit = rng.u32_in(1, 1000);
        total += u64::from(lit);
        literals.push(format!("{lit}"));
    }
    let total = total.min(u64::from(u32::MAX)) as u32;
    let mut out = String::from(
        "fn sum(elt: u32, acc: u32) -> u32 {\n\
         \x20   let (_, acc): (bool, u32) = jet::add_32(elt, acc);\n\
         \x20   acc\n\
         }\n\n",
    );
    out.push_str("fn main() {\n");
    out.push_str(&format!(
        "    let arr: [u32; {elements}] = [{}];\n",
        literals.join(", ")
    ));
    out.push_str(&format!(
        "    let total: u32 = array_fold::<sum, {elements}>(arr, 0);\n"
    ));
    out.push_str(&format!("    assert!(jet::eq_32(total, {total}));\n"));
    out.push_str("}\n");
    out
}

fn for_while(loops: u32) -> String {
    let mut out = String::from(
        "fn counter_8(acc: u32, unused: (), byte: u8) -> Either<u32, u32> {\n\
         \x20   match jet::all_8(byte) {\n\
         \x20       true => Left(acc),\n\
         \x20       false => Right(acc),\n\
         \x20   }\n\
         }\n\n",
    );
    out.push_str("fn main() {\n");
    out.push_str("    let acc0: u32 = 7;\n");
    for i in 0..loops {
        out.push_str(&format!(
            "    let out{i}: Either<u32, u32> = for_while::<counter_8>(acc{i}, ());\n"
        ));
        out.push_str(&format!(
            "    let acc{}: u32 = unwrap_right::<u32>(out{i});\n",
            i + 1
        ));
    }
    let last = loops;
    out.push_str(&format!("    assert!(jet::eq_32(acc{last}, acc{last}));\n"));
    out.push_str("}\n");
    out
}

fn multifile(name: String, files: u32, chain: bool) -> GeneratedProgram {
    let mut rng = Rng::new(0x11B2_0DD1);
    let mut out = Vec::new();
    for i in 0..files {
        let mut source = String::new();
        if chain && i + 1 < files {
            source.push_str(&format!("use crate::lib{}::g{};\n\n", i + 1, i + 1));
        }
        source.push_str(&format!("pub fn g{i}(a0: u32, a1: u32) -> u32 {{\n"));
        if chain && i + 1 < files {
            source.push_str(&format!("    g{}(a0, a1)\n", i + 1));
        } else if i % 2 == 0 {
            source.push_str("    jet::max_32(a0, a1)\n");
        } else {
            source.push_str(&format!(
                "    let (_, s): (bool, u32) = jet::add_32(a0, a1);\n    jet::max_32(s, {})\n",
                rng.literal()
            ));
        }
        source.push_str("}\n");
        out.push((format!("lib{i}.simf"), source));
    }

    let mut main = String::new();
    if chain {
        main.push_str("use crate::lib0::g0;\n\n");
    } else {
        for i in 0..files {
            main.push_str(&format!("use crate::lib{i}::g{i};\n"));
        }
        main.push('\n');
    }
    main.push_str("fn main() {\n");
    if chain {
        // Only g0 is in scope in the entry; the dependency chain is between
        // the files, which is what stresses the driver.
        main.push_str(&format!(
            "    let r0: u32 = g0({}, {});\n",
            rng.literal(),
            rng.literal()
        ));
        main.push_str("    assert!(jet::eq_32(r0, r0));\n");
    } else {
        main.push_str(&format!(
            "    let r0: u32 = g0({}, {});\n",
            rng.literal(),
            rng.literal()
        ));
        for i in 1..files {
            main.push_str(&format!(
                "    let r{i}: u32 = g{i}(r{}, {});\n",
                i - 1,
                rng.literal()
            ));
        }
        let last = files - 1;
        main.push_str(&format!("    assert!(jet::eq_32(r{last}, r{last}));\n"));
    }
    main.push_str("}\n");
    out.push(("main.simf".to_string(), main));

    GeneratedProgram::Package {
        name,
        files: out,
        entry: "main.simf".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specs_roundtrip() {
        for spec in FRONT_LADDER.iter().chain(BACK_LADDER).chain(DRIVER_LADDER) {
            let spec = *spec;
            let generated = generate(spec).expect("generates");
            assert_eq!(generated.name(), spec);
        }
    }

    #[test]
    fn corpus_is_deterministic() {
        for shape in [
            Shape::Flat(32),
            Shape::Deep { depth: 4, width: 3 },
            Shape::Functions(8),
            Shape::ArrayFold(9),
            Shape::ForWhile(2),
            Shape::MultiFile {
                files: 4,
                chain: false,
            },
            Shape::MultiFile {
                files: 4,
                chain: true,
            },
        ] {
            let a = generate_shape(shape).entry_source().to_string();
            let b = generate_shape(shape).entry_source().to_string();
            assert_eq!(a, b);
        }
    }
}
