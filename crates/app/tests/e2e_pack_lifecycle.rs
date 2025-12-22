use std::path::PathBuf;

use greentic_integration::harness::pack::{pack_build, pack_install, pack_verify};
use greentic_integration::harness::{
    PackBuildResult, PackInstallResult, PackVerifyResult, TestEnv,
};

#[tokio::test]
async fn e2e_pack_lifecycle() -> anyhow::Result<()> {
    if !greentic_integration::harness::docker_available() {
        eprintln!("skipping e2e_pack_lifecycle: docker daemon not available");
        return Ok(());
    }

    unsafe {
        std::env::set_var("E2E_TEST_NAME", "e2e_pack_lifecycle");
    }

    let env = TestEnv::up().await?;
    env.healthcheck().await?;

    let fixture_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .unwrap()
        .join("fixtures")
        .join("packs")
        .join("hello");

    let PackBuildResult { gtpack, mode } =
        pack_build(&fixture_root, env.artifacts_dir(), env.logs_dir())?;
    assert!(
        gtpack.exists(),
        "gtpack output missing at {}",
        gtpack.display()
    );

    let PackVerifyResult { ok, .. } = pack_verify(&gtpack, env.logs_dir())?;
    assert!(ok, "pack verify should succeed");

    let PackInstallResult { ok, target } =
        pack_install("dev", &gtpack, env.artifacts_dir(), env.logs_dir())?;
    assert!(ok, "pack install should succeed");
    assert_eq!(target, "dev");

    // Record build mode for debugging.
    let build_mode_note = env.artifacts_dir().join("pack").join("build_mode.txt");
    let note = format!("mode: {:?}\n", mode);
    std::fs::write(build_mode_note, note)?;

    env.down().await?;
    Ok(())
}
