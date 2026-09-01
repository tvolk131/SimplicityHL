# SimplicityHL benchmarks and profiling

This crate is **excluded from the root workspace** (see the root `Cargo.toml`):
it keeps its own lockfile and its dev-dependencies (criterion, dhat, chumsky)
never affect the library's MSRV or the root `Cargo.lock`, which CI checks with
`cargo rbmt --lock-file existing`. Always invoke it through
`--manifest-path bench/Cargo.toml` or the `just bench*` recipes.

The goal is to optimize the compiler with data instead of guesses: which stage
(lexing, parsing, analysis, codegen, serialization) and which program shape
consumes the time and memory.

## The layers

1. **Stage timing in `simc`** (`SIMC_TIMING=1 simc program.simf`) — wall-time
   per pipeline stage for any real program, in the binary users actually run.
   Stages nest: `driver:build-graph` contains `lex`/`parse` (one entry per
   file, summed in the report), and `serialize` contains `witness`/`prune`.
2. **Criterion benchmarks** (`just bench`) — statistical per-stage wall-time
   over a deterministic generated corpus, comparable across changes.
3. **Heap profiling** (`just bench-mem <spec> <stage>`) — dhat totals and
   peaks for exactly one stage, one process per stage.
4. **CPU profiling** — `samply`/`cargo flamegraph`/Instruments against
   `simc` (with `SIMC_TIMING` for the stage split) or the materialized corpus.

## The corpus

Line count does not predict compile time in this compiler: bounded loops and
folds blow up the generated Simplicity DAG, while long flat programs stress
the front-end. Each shape targets a hotspot, and every generator is seeded, so
`flat:256` is byte-identical on every machine:

| Spec                | Shape                                                    | Stresses |
|---------------------|----------------------------------------------------------|----------|
| `flat:N`            | N sequential bindings/asserts in `main`                  | lex, parse, analyze |
| `deep:DxW`          | D nested blocks with W bindings each                     | scopes, selector chains in codegen |
| `funcs:N`           | N small functions chained from `main`                    | item tables, call inlining |
| `array:N`           | `array_fold` over an N-element array                     | balanced-fold codegen |
| `for_while:N`       | N `for_while` loops, cheap body, 256 iterations each     | loop codegen blowup |
| `multifile:N`       | package of N libs, entry imports from all (star)         | driver discovery |
| `chain:N`           | package of N libs importing each other in a line         | driver linearization |
| `real:<name>`       | `examples/<name>.simf` from the repository               | realistic mix |

Ladders: `FRONT_LADDER` (large, cheap shapes for lex/parse), `BACK_LADDER`
(smaller, DAG-blowup shapes for analyze/codegen), `DRIVER_LADDER` — see
`src/corpus.rs`. The jet-heavy blowup case is `real:hash_loop` (a `for_while`
over `sha_256` jets; the u16 variant is the known ~10s compile).

After changing the language or the generators, verify the corpus is still
valid SimplicityHL:

```sh
just bench-corpus --check
```

Materialize it to disk for profilers (gitignored):

```sh
just bench-corpus                     # writes bench/corpus/
SIMC_TIMING=1 target/release/simc bench/corpus/for_while_2.simf   # from the root workspace
```

## Running benchmarks

```sh
just bench                             # everything (~minutes)
just bench lex                         # one group
just bench "codegen/for_while:2"       # one benchmark
just bench "parse/flat:1024" --measurement-time 5   # shorter runs while iterating
```

Compare before/after a change with baselines (per machine — never commit them):

```sh
just bench -- --save-baseline before
# ... make the change ...
just bench -- --baseline before
```

Methodology notes:

- Run on a quiet machine (plugged in, browsers closed); Apple Silicon is
  generally stable but thermal throttling will skew long runs.
- The benches assert success, so a corpus regression fails loudly instead of
  benchmarking an error path.
- Correlate regressions with shape metrics (bytes/tokens printed by
  `corpus-gen`): a slower `for_while` compile that produces a proportionally
  larger DAG is a different problem from a pure slowdown.

## Profiling workflows

CPU (macOS): [samply](https://github.com/mstange/samply) needs no sudo and
renders interactive flamegraphs:

```sh
cargo install samply
just bench-corpus                                   # or use any real program
samply ../target/release/simc ../bench/corpus/for_while_4.simf
SIMC_TIMING=1 samply ../target/release/simc ../bench/corpus/deep_32x8.simf
```

(`cargo flamegraph` works too but needs sudo on macOS; Xcode Instruments via
`cargo-instruments` is an alternative.)

Memory: dhat, one stage per process so the peak is attributable:

```sh
just bench-mem for_while:2 codegen
just bench-mem flat:4096 parse
just bench-mem flat:256 serialize
```

dhat prints total allocations, total bytes, and the peak (t-gmax) to stderr.
For a quick whole-process peak RSS: `/usr/bin/time -l target/release/simc program.simf`.

## Extending

- New shapes: add a variant to `Shape` in `src/corpus.rs`, a generator, and a
  ladder entry. Keep generators seeded and valid (`just bench-corpus --check`
  must pass).
- Finer stages: `perf::stage("name", || ...)` in the library, then read it in
  `simc`'s report. Keep names stable; nesting is fine as long as the report
  documents it.
- CI gating: wall-clock on shared CI runners is noisy; if regression gating
  is needed there, prefer deterministic instruction counts (`iai-callgrind`)
  on a Linux runner over committing criterion baselines.

## Known hotspot hypotheses (to be confirmed or killed by the data)

1. Chumsky lex+parse dominates small/medium programs; `parse_only` vs `lex`
   splits the two.
2. `for_while`/folds dominate blowup compiles (`codegen/for_while:*`).
3. Variable access compiles to `take`/`drop` selector chains that grow with
   scope depth, so `deep` shapes may show superlinear codegen.
4. Type finalization over large DAGs (`codegen` includes `finalize_types`).
5. Allocation churn everywhere (`Arc`-heavy AST) — visible in dhat.
