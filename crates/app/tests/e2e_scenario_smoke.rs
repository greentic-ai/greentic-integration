use greentic_integration::{
    harness::TestEnv,
    scenario::{Scenario, ScenarioRunner, Step},
};

#[tokio::test]
async fn e2e_scenario_smoke() -> anyhow::Result<()> {
    if !greentic_integration::harness::docker_available() {
        eprintln!("skipping e2e_scenario_smoke: docker daemon not available");
        return Ok(());
    }

    unsafe {
        std::env::set_var("E2E_TEST_NAME", "e2e_scenario_smoke");
    }

    let env = TestEnv::up().await?;
    env.healthcheck().await?;

    let scenario = Scenario {
        name: "nats_echo".into(),
        steps: vec![
            Step::NatsPublish {
                subject: "e2e.scenario.smoke".into(),
                payload: serde_json::json!({"msg": "hello"}),
            },
            Step::AwaitNats {
                subject: "e2e.scenario.smoke".into(),
                expected: Some(serde_json::json!({"msg": "hello"})),
                timeout_ms: Some(3_000),
            },
        ],
    };

    let mut runner = ScenarioRunner::new(&env)?;
    runner.run(&scenario).await?;

    env.down().await?;
    Ok(())
}
