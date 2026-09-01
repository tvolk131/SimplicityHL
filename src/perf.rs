//! Optional per-stage timing, enabled by setting the `SIMC_TIMING` environment variable.
//!
//! rustc's `-Ztime-passes` in miniature: the pipeline records how long each
//! compilation stage took, and `simc` prints the report to stderr when it is
//! done. Recording is a single boolean check per stage boundary unless the
//! variable is set, so the library can stay instrumented in normal builds.
//!
//! Stages nest: `driver:build-graph` contains the per-file `lex`/`parse`
//! entries, and `serialize` contains `witness`. Entries with the same name
//! (one per parsed file) are summed by [`take_report`].

use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// The recorded stages, in completion order.
#[derive(Debug, Default)]
struct StageLog {
    entries: Vec<(&'static str, Duration)>,
}

impl StageLog {
    fn get() -> &'static Mutex<Self> {
        static LOG: OnceLock<Mutex<StageLog>> = OnceLock::new();
        LOG.get_or_init(|| Mutex::new(StageLog::default()))
    }
}

/// Whether stage recording is enabled: `SIMC_TIMING` set to a non-empty value.
pub fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("SIMC_TIMING").is_some_and(|v| !v.is_empty()))
}

/// Run `f`, recording its duration under `stage` if recording is enabled.
pub fn stage<T>(stage: &'static str, f: impl FnOnce() -> T) -> T {
    if !enabled() {
        return f();
    }
    let start = Instant::now();
    let output = f();
    record(stage, start.elapsed());
    output
}

fn record(stage: &'static str, duration: Duration) {
    if let Ok(mut log) = StageLog::get().lock() {
        log.entries.push((stage, duration));
    }
}

/// Aggregate the recorded entries by stage name and clear the log.
///
/// Stages that run once per file (`lex`, `parse`) are summed into a single
/// entry. First-recorded order is preserved so the report reads in pipeline
/// order.
pub fn take_report() -> Vec<(&'static str, Duration)> {
    let Ok(mut log) = StageLog::get().lock() else {
        return Vec::new();
    };
    let mut report: Vec<(&'static str, Duration)> = Vec::new();
    for (stage, duration) in log.entries.drain(..) {
        match report.iter_mut().find(|(name, _)| *name == stage) {
            Some((_, total)) => *total += duration,
            None => report.push((stage, duration)),
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_aggregates_and_clears() {
        // One test function: the log is global, so parallel tests would race.
        record("lex", Duration::from_millis(1));
        record("parse", Duration::from_millis(2));
        record("lex", Duration::from_millis(3));
        assert_eq!(
            take_report(),
            vec![
                ("lex", Duration::from_millis(4)),
                ("parse", Duration::from_millis(2)),
            ]
        );
        assert!(take_report().is_empty());
    }

    #[test]
    fn stage_returns_output_untouched() {
        let output = stage("test", || 42);
        assert_eq!(output, 42);
    }
}
