use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use walkdir::WalkDir;

#[derive(Debug, Deserialize)]
struct RunnerCase {
    name: String,
    runs: Vec<SessionRun>,
}

#[derive(Debug, Deserialize)]
struct SessionRun {
    tenant_id: String,
    session_id: String,
    events: Vec<Event>,
    state_snapshot: Option<StateSnapshot>,
}

#[derive(Debug, Deserialize)]
struct Event {
    sequence: u64,
    kind: String,
    tenant_id: String,
    #[serde(default)]
    trace_id: String,
}

#[derive(Debug, Deserialize)]
struct StateSnapshot {
    writer: String,
    bytes_written: usize,
}

fn main() -> Result<()> {
    let cases_dir = parse_args();
    let cases = load_cases(&cases_dir)?;
    if cases.is_empty() {
        bail!("No runner smoke cases found under {}", cases_dir.display());
    }

    let mut total_runs = 0;
    for case in &cases {
        verify_case(case).with_context(|| format!("case '{}': invariant failed", case.name))?;
        total_runs += case.runs.len();
    }

    println!(
        "runner-smoke: {} case(s), {} session(s) verified (tenant isolation + session continuity + state writes)",
        cases.len(),
        total_runs
    );

    Ok(())
}

fn parse_args() -> PathBuf {
    let mut args = env::args().skip(1);
    match args.next() {
        Some(flag) if flag == "--cases" => {
            let path = args.next().expect("--cases requires a path argument");
            PathBuf::from(path)
        }
        Some(other) => {
            eprintln!(
                "Unexpected argument '{}'. Usage: runner-smoke [--cases <dir>]",
                other
            );
            std::process::exit(2);
        }
        None => PathBuf::from("harness/runner-smoke/cases"),
    }
}

fn load_cases(dir: &Path) -> Result<Vec<RunnerCase>> {
    let mut cases = Vec::new();
    for entry in WalkDir::new(dir).min_depth(1).max_depth(3) {
        let entry = entry?;
        if entry.file_type().is_file()
            && entry.path().extension().and_then(|s| s.to_str()) == Some("json")
        {
            let case: RunnerCase = serde_json::from_slice(&std::fs::read(entry.path())?)
                .with_context(|| format!("failed to parse {}", entry.path().display()))?;
            cases.push(case);
        }
    }
    Ok(cases)
}

fn verify_case(case: &RunnerCase) -> Result<()> {
    if case.runs.is_empty() {
        bail!("case '{}' contains no runs", case.name);
    }

    let mut trace_cache = std::collections::BTreeSet::new();
    for run in &case.runs {
        ensure_tenant_isolation(run)?;
        ensure_session_continuity(run)?;
        ensure_state_write(run)?;
        ensure_once_only_effects(run, &mut trace_cache)?;
    }
    Ok(())
}

fn ensure_tenant_isolation(run: &SessionRun) -> Result<()> {
    for event in &run.events {
        if event.tenant_id != run.tenant_id {
            bail!(
                "session {} leaked tenant boundary: event tenant {} expected {}",
                run.session_id,
                event.tenant_id,
                run.tenant_id
            );
        }
    }
    Ok(())
}

fn ensure_session_continuity(run: &SessionRun) -> Result<()> {
    if run.events.is_empty() {
        bail!("session {} has no events", run.session_id);
    }
    for window in run.events.windows(2) {
        let current = &window[0];
        let next = &window[1];
        if next.sequence != current.sequence + 1 {
            bail!(
                "session {} sequence gap: {} -> {}",
                run.session_id,
                current.sequence,
                next.sequence
            );
        }
    }
    Ok(())
}

fn ensure_state_write(run: &SessionRun) -> Result<()> {
    let snapshot = run
        .state_snapshot
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("session {} missing state snapshot", run.session_id))?;
    if snapshot.bytes_written == 0 {
        bail!("session {} state snapshot was empty", run.session_id);
    }
    if snapshot.writer.trim().is_empty() {
        bail!(
            "session {} invalid state writer metadata (writer missing)",
            run.session_id
        );
    }
    if !run.events.iter().any(|event| event.kind == "state_write") {
        bail!(
            "session {} never emitted a state_write event",
            run.session_id
        );
    }
    Ok(())
}

fn ensure_once_only_effects(
    run: &SessionRun,
    trace_cache: &mut std::collections::BTreeSet<String>,
) -> Result<()> {
    for event in &run.events {
        if event.kind != "state_write" {
            continue;
        }
        if event.trace_id.trim().is_empty() {
            bail!(
                "session {} missing trace_id for state_write event at sequence {}",
                run.session_id,
                event.sequence
            );
        }
        if !trace_cache.insert(event.trace_id.clone()) {
            bail!(
                "trace_id {} seen multiple times; effect log once-only invariant violated",
                event.trace_id
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_trace_id_fails() {
        let run = SessionRun {
            tenant_id: "t".into(),
            session_id: "s".into(),
            events: vec![
                Event {
                    sequence: 1,
                    kind: "state_write".into(),
                    tenant_id: "t".into(),
                    trace_id: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
                },
                Event {
                    sequence: 2,
                    kind: "state_write".into(),
                    tenant_id: "t".into(),
                    trace_id: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
                },
            ],
            state_snapshot: Some(StateSnapshot {
                writer: "runner".into(),
                bytes_written: 1,
            }),
        };

        let mut cache = std::collections::BTreeSet::new();
        let err = ensure_once_only_effects(&run, &mut cache).unwrap_err();
        assert!(
            err.to_string().contains("trace_id"),
            "expected duplicate trace error, got {err}"
        );
    }
}
