use greentic_integration::harness::{StackError, TestEnv};

#[tokio::test]
async fn e2e_stack_boot() -> anyhow::Result<()> {
    if !greentic_integration::harness::docker_available() {
        eprintln!("skipping e2e_stack_boot: docker daemon not available");
        return Ok(());
    }

    unsafe {
        std::env::set_var("E2E_TEST_NAME", "e2e_stack_boot");
    }

    let env = TestEnv::up().await?;

    let mut stack = match env.up_stack().await {
        Ok(stack) => stack,
        Err(StackError::MissingBinary { name, searched }) => {
            if std::env::var("GREENTIC_STACK_STRICT")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false)
            {
                anyhow::bail!("missing binary {} (checked: {:?})", name, searched);
            }
            eprintln!(
                "skipping e2e_stack_boot: missing binary {} (checked: {:?})",
                name, searched
            );
            return Ok(());
        }
        Err(err) => return Err(err.into()),
    };

    stack.healthcheck(env.logs_dir()).await?;
    stack.down().await?;
    Ok(())
}
