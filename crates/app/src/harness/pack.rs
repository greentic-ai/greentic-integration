use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{Context, Result, bail};
use serde_json::json;

use super::{now_millis, workspace_root};
use crate::fixtures::Fixture;

#[derive(Debug)]
pub struct PackBuildResult {
    pub gtpack: PathBuf,
    pub mode: BuildMode,
}

#[derive(Debug)]
pub enum BuildMode {
    BuiltWith(PathBuf),
    CopiedFixture(PathBuf),
}

#[derive(Debug)]
pub struct PackVerifyResult {
    pub ok: bool,
    pub mode: VerifyMode,
}

#[derive(Debug)]
pub enum VerifyMode {
    VerifiedWith(PathBuf),
    Stubbed,
}

#[derive(Debug)]
pub struct PackInstallResult {
    pub ok: bool,
    pub target: String,
}

pub fn pack_build(
    fixture_root: &Path,
    artifacts_dir: &Path,
    logs_dir: &Path,
) -> Result<PackBuildResult> {
    let out_dir = artifacts_dir.join("pack");
    fs::create_dir_all(&out_dir)
        .with_context(|| format!("failed to create {}", out_dir.display()))?;
    let gtpack_out = out_dir.join("pack.gtpack");

    let builder = find_binary(&["greentic-packc", "packc"]);
    let log_path = logs_dir.join("pack_build.log");
    if let Some(bin) = builder {
        let status = Command::new(&bin)
            .arg("build")
            .arg(fixture_root)
            .arg("--output")
            .arg(&gtpack_out)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .status()
            .with_context(|| format!("failed to run pack builder {}", bin.display()))?;
        fs::write(
            &log_path,
            format!("builder: {}\nstatus: {:?}\n", bin.display(), status),
        )
        .with_context(|| format!("failed to write {}", log_path.display()))?;
        if !status.success() {
            bail!("pack build failed with status {:?}", status.code());
        }
        return Ok(PackBuildResult {
            gtpack: gtpack_out,
            mode: BuildMode::BuiltWith(bin),
        });
    }

    if strict_pack_mode() {
        bail!("pack build binaries not found and strict mode is enabled");
    }

    // Fallback: copy fixture gtpack if present; else serialize pack.json as placeholder.
    let fixture_gtpack = fixture_root.join("hello.gtpack");
    if fixture_gtpack.exists() {
        fs::copy(&fixture_gtpack, &gtpack_out).with_context(|| {
            format!(
                "failed to copy {} -> {}",
                fixture_gtpack.display(),
                gtpack_out.display()
            )
        })?;
    } else {
        let manifest_path = fixture_root.join("pack.json");
        let manifest = Fixture::load_json(manifest_path)
            .with_context(|| format!("failed to load manifest under {}", fixture_root.display()))?;
        fs::write(&gtpack_out, serde_json::to_vec_pretty(&manifest)?)
            .with_context(|| format!("failed to write {}", gtpack_out.display()))?;
    }
    fs::write(
        &log_path,
        format!(
            "builder: fallback copy at {}\ntimestamp: {}\n",
            fixture_gtpack.display(),
            now_millis()
        ),
    )
    .with_context(|| format!("failed to write {}", log_path.display()))?;

    Ok(PackBuildResult {
        gtpack: gtpack_out,
        mode: BuildMode::CopiedFixture(fixture_gtpack),
    })
}

pub fn pack_verify(gtpack: &Path, logs_dir: &Path) -> Result<PackVerifyResult> {
    let verifier = find_binary(&["greentic-pack", "greentic-packc", "packc"]);
    let log_path = logs_dir.join("pack_verify.log");

    if let Some(bin) = verifier {
        let output = Command::new(&bin)
            .arg("verify")
            .arg(gtpack)
            .output()
            .with_context(|| format!("failed to run pack verify {}", bin.display()))?;
        fs::write(
            &log_path,
            format!(
                "verifier: {}\nstatus: {:?}\nstdout:\n{}\nstderr:\n{}",
                bin.display(),
                output.status.code(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ),
        )
        .with_context(|| format!("failed to write {}", log_path.display()))?;
        if !output.status.success() {
            bail!("pack verify failed with status {:?}", output.status.code());
        }
        return Ok(PackVerifyResult {
            ok: true,
            mode: VerifyMode::VerifiedWith(bin),
        });
    }

    if strict_pack_mode() {
        bail!("pack verify binaries not found and strict mode is enabled");
    }

    // Stub verification: ensure file parses as JSON.
    let data = fs::read_to_string(gtpack)
        .with_context(|| format!("failed to read {}", gtpack.display()))?;
    let _json: serde_json::Value = serde_json::from_str(&data)
        .with_context(|| format!("failed to parse gtpack {}", gtpack.display()))?;
    fs::write(
        &log_path,
        format!("verifier: stub parse ok\nfile: {}\n", gtpack.display()),
    )
    .with_context(|| format!("failed to write {}", log_path.display()))?;
    Ok(PackVerifyResult {
        ok: true,
        mode: VerifyMode::Stubbed,
    })
}

pub fn pack_install(
    target: &str,
    gtpack: &Path,
    artifacts_dir: &Path,
    logs_dir: &Path,
) -> Result<PackInstallResult> {
    let installer = find_binary(&["greentic-deployer", "greentic-pack"]);
    let log_path = logs_dir.join("pack_install.log");
    let install_out = artifacts_dir.join("pack").join("installed.json");
    fs::create_dir_all(install_out.parent().unwrap())
        .with_context(|| format!("failed to create {}", install_out.display()))?;

    if let Some(bin) = installer {
        let output = Command::new(&bin)
            .arg("install")
            .arg(gtpack)
            .arg("--target")
            .arg(target)
            .output()
            .with_context(|| format!("failed to run installer {}", bin.display()))?;
        fs::write(
            &log_path,
            format!(
                "installer: {}\nstatus: {:?}\nstdout:\n{}\nstderr:\n{}",
                bin.display(),
                output.status.code(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ),
        )
        .with_context(|| format!("failed to write {}", log_path.display()))?;
        if !output.status.success() {
            bail!("pack install failed with status {:?}", output.status.code());
        }
        fs::write(
            &install_out,
            json!({"installed": true, "target": target, "mode": "binary"}).to_string(),
        )
        .with_context(|| format!("failed to write {}", install_out.display()))?;
        return Ok(PackInstallResult {
            ok: true,
            target: target.to_string(),
        });
    }

    if strict_pack_mode() {
        bail!("pack install binaries not found and strict mode is enabled");
    }

    // Stub install: copy gtpack and note target.
    let data = fs::read(gtpack).with_context(|| format!("failed to read {}", gtpack.display()))?;
    fs::write(&install_out, data)
        .with_context(|| format!("failed to write {}", install_out.display()))?;
    fs::write(
        &log_path,
        format!(
            "installer: stub copy\nsource: {}\ntarget: {}",
            gtpack.display(),
            target
        ),
    )
    .with_context(|| format!("failed to write {}", log_path.display()))?;
    Ok(PackInstallResult {
        ok: true,
        target: target.to_string(),
    })
}

fn find_binary(names: &[&str]) -> Option<PathBuf> {
    for name in names {
        for candidate in binary_candidates(name) {
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}

fn binary_candidates(name: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let root = workspace_root();
    paths.push(root.join("tests/bin").join(name));
    paths.push(root.join("target/release").join(name));
    paths.push(root.join("target/debug").join(name));
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in path_var.split(std::path::MAIN_SEPARATOR) {
            if dir.is_empty() {
                continue;
            }
            paths.push(PathBuf::from(dir).join(name));
        }
    }
    paths
}

fn strict_pack_mode() -> bool {
    let vars = [
        "GREENTIC_PACK_STRICT",
        "GREENTIC_PACK_NO_FALLBACK",
        "GREENTIC_INTEGRATION_STRICT",
    ];
    vars.iter().any(|name| {
        std::env::var(name)
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
    })
}
