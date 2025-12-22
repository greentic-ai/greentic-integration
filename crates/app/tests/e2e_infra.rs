use futures::StreamExt;
use greentic_integration::harness::TestEnv;
use tokio_postgres::NoTls;

#[tokio::test]
async fn e2e_infra_round_trip() -> anyhow::Result<()> {
    if !greentic_integration::harness::docker_available() {
        eprintln!("skipping e2e_infra: docker daemon not available");
        return Ok(());
    }

    unsafe {
        std::env::set_var("E2E_TEST_NAME", "e2e_infra");
    }

    let env = TestEnv::up().await?;
    env.healthcheck().await?;

    // NATS publish/subscribe round-trip
    let nats = async_nats::connect(env.nats_url()).await?;
    let subject = "e2e.test";
    let mut sub = nats.subscribe(subject).await?;
    nats.publish(subject, "hello".into()).await?;
    nats.flush().await?;

    let msg = sub
        .next()
        .await
        .expect("subscription should yield a message");
    assert_eq!(msg.payload, "hello");

    // Postgres connectivity check
    let (client, connection) = tokio_postgres::connect(&env.db_url(), NoTls).await?;
    tokio::spawn(async move {
        let _ = connection.await;
    });
    let row = client
        .query_one("SELECT 1::int", &[])
        .await
        .expect("simple query should succeed");
    let value: i32 = row.get(0);
    assert_eq!(value, 1);

    env.down().await?;
    Ok(())
}
