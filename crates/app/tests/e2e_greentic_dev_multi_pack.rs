use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use pathdiff::diff_paths;
use tempfile::tempdir;
use which::which;

/// Two packs sharing a component plus isolation check between pack builds.
#[test]
fn greentic_dev_multi_pack_shared_component() -> Result<()> {
    let strict = is_strict();
    let greentic_dev = match which("greentic-dev") {
        Ok(p) => p,
        Err(err) => {
            if strict {
                return Err(err).context("greentic-dev not found in strict mode");
            }
            eprintln!("skipping multi-pack test: greentic-dev not found ({err})");
            return Ok(());
        }
    };

    let tmp = tempdir().context("tempdir")?;
    let work = tmp.path();
    println!("workspace: {}", work.display());
    let envs = prepare_env(work)?;

    // Shared component.
    let comp_dir = work.join("shared-comp");
    if let Err(err) = run_status(
        &greentic_dev,
        &[
            "component",
            "new",
            "--name",
            "shared-comp",
            "--non-interactive",
            "--no-git",
            "--path",
            comp_dir.to_str().unwrap(),
        ],
        work,
        &envs,
        "component new",
        strict,
    ) {
        if !strict {
            eprintln!("skipping multi-pack test: {err:?}");
            return Ok(());
        }
        return Err(err);
    }
    let src = comp_dir.join("src/lib.rs");
    let code = fs::read_to_string(&src)?;
    fs::write(
        &src,
        code.replace(
            "format!(\"demo-comp::{operation} => {}\", input.trim())",
            "format!(\"SHARED::{}\", input.trim().to_ascii_uppercase())",
        ),
    )?;
    run_status(
        &greentic_dev,
        &[
            "component",
            "build",
            "--manifest",
            comp_dir.to_str().unwrap(),
        ],
        work,
        &envs,
        "component build",
        strict,
    )?;
    let wasm_path = comp_dir
        .join("target/wasm32-wasip2/release/shared_comp.wasm")
        .canonicalize()
        .context("locate shared wasm")?;

    // Pack A and B.
    let pack_a = work.join("pack-a");
    let pack_b = work.join("pack-b");
    run_status(
        &greentic_dev,
        &["pack", "new", "--dir", pack_a.to_str().unwrap(), "pack-a"],
        work,
        &envs,
        "pack new A",
        strict,
    )?;
    run_status(
        &greentic_dev,
        &["pack", "new", "--dir", pack_b.to_str().unwrap(), "pack-b"],
        work,
        &envs,
        "pack new B",
        strict,
    )?;

    write_shared_pack(&pack_a, "pack-a.shared", &wasm_path)?;
    write_shared_pack(&pack_b, "pack-b.shared", &wasm_path)?;

    // Build both packs.
    run_status(
        &greentic_dev,
        &["pack", "build", "--in", "."],
        &pack_a,
        &envs,
        "pack build A",
        strict,
    )?;
    let pack_b_yaml_before = fs::read_to_string(pack_b.join("pack.yaml"))?;
    run_status(
        &greentic_dev,
        &["pack", "build", "--in", "."],
        &pack_b,
        &envs,
        "pack build B",
        strict,
    )?;

    // Isolation: mutate Pack A flow, rebuild A, ensure Pack B manifest unchanged.
    let flow_path = pack_a.join("flows/main.ygtc");
    let mut flow_yaml: serde_yaml_bw::Value =
        serde_yaml_bw::from_str(&fs::read_to_string(&flow_path)?)?;
    if let Some(mapping) = flow_yaml.as_mapping_mut() {
        mapping.insert(
            serde_yaml_bw::Value::from("title"),
            serde_yaml_bw::Value::from("Changed only in A"),
        );
    }
    fs::write(&flow_path, serde_yaml_bw::to_string(&flow_yaml)?)?;
    run_status(
        &greentic_dev,
        &["pack", "build", "--in", "."],
        &pack_a,
        &envs,
        "pack build A (modified)",
        strict,
    )?;
    let pack_b_yaml_after = fs::read_to_string(pack_b.join("pack.yaml"))?;
    assert_eq!(
        pack_b_yaml_before, pack_b_yaml_after,
        "Pack B manifest should remain unchanged when Pack A changes"
    );

    Ok(())
}

fn write_shared_pack(pack_dir: &Path, comp_id: &str, wasm: &Path) -> Result<()> {
    let pack_yaml = pack_dir.join("pack.yaml");
    let mut doc: serde_yaml_bw::Value = serde_yaml_bw::from_str(&fs::read_to_string(&pack_yaml)?)?;
    let mapping = doc.as_mapping_mut().context("pack yaml mapping")?;
    let mut comps = serde_yaml_bw::Sequence::new();
    let comp_dir = pack_dir.join("components");
    fs::create_dir_all(&comp_dir)?;
    let dest_wasm = comp_dir.join("shared_comp.wasm");
    fs::copy(wasm, &dest_wasm)?;
    let wasm_rel = diff_paths(&dest_wasm, pack_dir).unwrap_or(dest_wasm);
    comps.push(serde_yaml_bw::to_value(serde_yaml_bw::Mapping::from_iter(
        [
            (
                serde_yaml_bw::Value::from("id"),
                serde_yaml_bw::Value::from(comp_id),
            ),
            (
                serde_yaml_bw::Value::from("version"),
                serde_yaml_bw::Value::from("0.1.0"),
            ),
            (
                serde_yaml_bw::Value::from("world"),
                serde_yaml_bw::Value::from("greentic:component/stub"),
            ),
            (
                serde_yaml_bw::Value::from("supports"),
                serde_yaml_bw::Value::Sequence({
                    let mut s = serde_yaml_bw::Sequence::new();
                    s.push(serde_yaml_bw::Value::from("messaging"));
                    s
                }),
            ),
            (
                serde_yaml_bw::Value::from("profiles"),
                serde_yaml_bw::to_value(serde_yaml_bw::Mapping::from_iter([
                    (
                        serde_yaml_bw::Value::from("default"),
                        serde_yaml_bw::Value::from("default"),
                    ),
                    (
                        serde_yaml_bw::Value::from("supported"),
                        serde_yaml_bw::Value::Sequence({
                            let mut s = serde_yaml_bw::Sequence::new();
                            s.push(serde_yaml_bw::Value::from("default"));
                            s
                        }),
                    ),
                ]))?,
            ),
            (
                serde_yaml_bw::Value::from("capabilities"),
                serde_yaml_bw::to_value(serde_yaml_bw::Mapping::from_iter([
                    (
                        serde_yaml_bw::Value::from("wasi"),
                        serde_yaml_bw::Value::Mapping(serde_yaml_bw::Mapping::new()),
                    ),
                    (
                        serde_yaml_bw::Value::from("host"),
                        serde_yaml_bw::Value::Mapping(serde_yaml_bw::Mapping::new()),
                    ),
                ]))?,
            ),
            (
                serde_yaml_bw::Value::from("wasm"),
                serde_yaml_bw::Value::from(wasm_rel.to_string_lossy().to_string()),
            ),
        ],
    ))?);
    mapping.insert(
        serde_yaml_bw::Value::from("components"),
        serde_yaml_bw::Value::Sequence(comps),
    );
    fs::write(&pack_yaml, serde_yaml_bw::to_string(&doc)?)?;
    Ok(())
}

fn prepare_env(work: &Path) -> Result<Vec<(String, String)>> {
    let home_dir = work.join("home");
    let xdg_config = work.join(".config");
    let xdg_data = work.join(".local/share");
    let xdg_state = work.join(".local/state");
    let xdg_cache = work.join(".cache");
    for d in [&xdg_config, &xdg_data, &xdg_state, &xdg_cache] {
        fs::create_dir_all(d)?;
    }
    let config_path = xdg_config.join("greentic-dev").join("config.toml");
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let fixtures_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("tests")
        .join("fixtures");
    let profile_tpl = fixtures_root
        .join("greentic-dev")
        .join("profiles")
        .join("default.toml");
    let profile_raw = fs::read_to_string(&profile_tpl).context("read profile template")?;
    let store_path = work.join("store");
    fs::create_dir_all(&store_path)?;
    let config_contents = profile_raw.replace("__STORE_PATH__", store_path.to_str().unwrap());
    fs::write(&config_path, &config_contents)?;
    let home_config = home_dir.join(".config/greentic-dev/config.toml");
    if let Some(parent) = home_config.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&home_config, &config_contents)?;

    Ok(vec![
        ("HOME".into(), home_dir.to_string_lossy().into_owned()),
        (
            "XDG_CONFIG_HOME".into(),
            xdg_config.to_string_lossy().into_owned(),
        ),
        (
            "XDG_DATA_HOME".into(),
            xdg_data.to_string_lossy().into_owned(),
        ),
        (
            "XDG_STATE_HOME".into(),
            xdg_state.to_string_lossy().into_owned(),
        ),
        (
            "XDG_CACHE_HOME".into(),
            xdg_cache.to_string_lossy().into_owned(),
        ),
        ("GREENTIC_DISTRIBUTOR_PROFILE".into(), "default".into()),
        (
            "GREENTIC_CONFIG_FILE".into(),
            config_path.to_string_lossy().into_owned(),
        ),
    ])
}

fn run_status(
    bin: &Path,
    args: &[&str],
    cwd: &Path,
    envs: &[(String, String)],
    label: &str,
    strict: bool,
) -> Result<()> {
    let status = Command::new(bin)
        .args(args)
        .current_dir(cwd)
        .envs(envs.iter().cloned())
        .status()
        .with_context(|| format!("{label} failed to spawn"))?;
    if !status.success() {
        if strict {
            anyhow::bail!("{label} failed in strict mode: {:?}", status.code());
        } else {
            eprintln!("{label} failed (non-strict skip): {:?}", status.code());
            return Err(anyhow::anyhow!("non-strict skip"));
        }
    }
    Ok(())
}

fn is_strict() -> bool {
    std::env::var("GREENTIC_DEV_E2E_STRICT")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
        || std::env::var("CI").is_ok()
}
