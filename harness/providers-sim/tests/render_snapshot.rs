use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use providers_sim::{RenderReport, simulate_render};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir parent")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn render_reports_match_golden() {
    let root = repo_root();
    let packs_dir = root.join("packs");
    let manifests = manifest_paths(&packs_dir);
    assert!(
        !manifests.is_empty(),
        "expected pack manifests under {}",
        packs_dir.display()
    );

    let mut reports = Vec::new();
    for manifest in manifests {
        let mut subset = simulate_render(&manifest).expect("simulate_render");
        reports.append(&mut subset);
    }
    reports.sort_by(|a, b| {
        a.pack_id
            .cmp(&b.pack_id)
            .then(a.scenario_id.cmp(&b.scenario_id))
    });

    let golden_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("golden/render_reports.json");
    if env::var("UPDATE_GOLDEN").as_deref() == Ok("1") {
        let json = serde_json::to_string_pretty(&reports).expect("serialize reports");
        fs::create_dir_all(golden_path.parent().unwrap()).expect("create golden dir");
        fs::write(&golden_path, format!("{}\n", json)).expect("write golden");
    }

    let expected: Vec<RenderReport> =
        serde_json::from_str(&fs::read_to_string(&golden_path).expect("missing golden file"))
            .expect("parse golden reports");

    assert_eq!(
        reports, expected,
        "Renderer reports drifted; run UPDATE_GOLDEN=1 make render.snapshot to refresh"
    );
}

fn manifest_paths(packs_dir: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(entries) = fs::read_dir(packs_dir) {
        for entry in entries.flatten() {
            let candidate = entry.path().join("pack.json");
            if candidate.exists() {
                paths.push(candidate);
            }
        }
    }
    paths.sort();
    paths
}
