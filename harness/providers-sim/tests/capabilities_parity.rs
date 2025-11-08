use providers_sim::capabilities::{capabilities_path, load_capabilities};

#[test]
fn simulator_matches_reference_capabilities_except_downgrades() {
    let path = capabilities_path();
    let doc = load_capabilities(&path).expect("parse capabilities yaml");

    let reference = doc
        .reference_capabilities()
        .expect("reference provider missing");
    let simulator = doc
        .simulator_capabilities()
        .expect("simulator provider missing");

    let downgrade_caps: std::collections::BTreeSet<String> = doc
        .downgrades
        .iter()
        .map(|d| d.capability.clone())
        .collect();

    // All reference capabilities must either be implemented by the simulator or explicitly downgraded.
    for capability in reference.iter() {
        if simulator.contains(capability) {
            continue;
        }
        assert!(
            downgrade_caps.contains(capability),
            "Simulator missing capability '{capability}' but downgrade rationale not documented"
        );
    }

    // Any documented downgrade must actually represent a missing capability.
    for downgraded in downgrade_caps {
        assert!(
            reference.contains(&downgraded),
            "Downgrade '{downgraded}' not present in reference provider capabilities"
        );
        assert!(
            !simulator.contains(&downgraded),
            "Downgrade '{downgraded}' is documented but simulator already supports it"
        );
    }
}
