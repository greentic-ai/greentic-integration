use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::Duration,
};

use anyhow::{Context, Result, bail};
use tokio::{
    net::TcpStream,
    time::{Instant, sleep},
};

use super::{now_millis, workspace_root, write_text};

const RUNNER_PORT: u16 = 3333;

#[derive(Debug)]
pub struct ServiceProcess {
    name: String,
    log_path: PathBuf,
    child: Child,
}

impl ServiceProcess {
    pub fn spawn(
        name: &str,
        binary: &Path,
        args: &[&str],
        envs: &[(&str, &str)],
        logs_dir: &Path,
    ) -> Result<Self> {
        let log_path = logs_dir.join(format!("{name}.log"));
        let log_file = File::create(&log_path)
            .with_context(|| format!("failed to create log file {}", log_path.display()))?;
        let log_err = log_file
            .try_clone()
            .with_context(|| format!("failed to clone log file handle {}", log_path.display()))?;

        let mut cmd = Command::new(binary);
        cmd.args(args)
            .envs(envs.iter().map(|(k, v)| (*k, *v)))
            .stdout(Stdio::from(log_file))
            .stderr(Stdio::from(log_err));

        let child = cmd.spawn().with_context(|| {
            format!("failed to start service {name} using {}", binary.display())
        })?;

        Ok(Self {
            name: name.to_string(),
            log_path,
            child,
        })
    }

    pub fn log_path(&self) -> &Path {
        &self.log_path
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn ensure_running(&mut self) -> Result<()> {
        if let Some(status) = self.child.try_wait()? {
            bail!(
                "service {} exited early with status {:?}",
                self.name,
                status.code()
            );
        }
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        if let Some(_status) = self.child.try_wait()? {
            return Ok(());
        }
        self.child
            .kill()
            .with_context(|| format!("failed to kill {}", self.name))?;
        let _ = self.child.wait();
        Ok(())
    }
}

pub struct TestStack {
    runner: ServiceProcess,
}

impl TestStack {
    pub async fn healthcheck(&mut self, logs_dir: &Path) -> Result<()> {
        self.runner.ensure_running()?;
        wait_for_port("runner", RUNNER_PORT, logs_dir, Duration::from_secs(20)).await?;
        Ok(())
    }

    pub async fn down(mut self) -> Result<()> {
        self.runner.stop()?;
        Ok(())
    }
}

pub enum StackError {
    MissingBinary {
        name: &'static str,
        searched: Vec<PathBuf>,
    },
    Startup(anyhow::Error),
}

impl std::fmt::Display for StackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StackError::MissingBinary { name, .. } => write!(f, "missing binary {}", name),
            StackError::Startup(err) => write!(f, "{err}"),
        }
    }
}

impl std::fmt::Debug for StackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StackError::MissingBinary { name, searched } => f
                .debug_struct("MissingBinary")
                .field("name", name)
                .field("searched", searched)
                .finish(),
            StackError::Startup(err) => f.debug_tuple("Startup").field(err).finish(),
        }
    }
}

impl std::error::Error for StackError {}

pub async fn boot_stack(env: &crate::harness::TestEnv) -> Result<TestStack, StackError> {
    let runner_bin = locate_binary("greentic-runner");
    if runner_bin.is_none() {
        return Err(StackError::MissingBinary {
            name: "greentic-runner",
            searched: binary_candidates("greentic-runner"),
        });
    }
    let runner_bin = runner_bin.unwrap();
    if !is_binary_compatible(&runner_bin) {
        return Err(StackError::MissingBinary {
            name: "greentic-runner",
            searched: binary_candidates("greentic-runner"),
        });
    }

    let config_dir = env.root().join("config");
    fs::create_dir_all(&config_dir).map_err(|e| StackError::Startup(e.into()))?;

    let port_str = RUNNER_PORT.to_string();
    let runner_env = [("PORT", port_str.as_str())];
    let runner = ServiceProcess::spawn("runner", &runner_bin, &[], &runner_env, env.logs_dir())
        .map_err(StackError::Startup)?;

    write_text(
        &env.logs_dir().join("stack-info.log"),
        format!(
            "runner binary: {}\nstarted at: {}\n",
            runner_bin.display(),
            now_millis()
        ),
    )
    .map_err(StackError::Startup)?;

    Ok(TestStack { runner })
}

fn locate_binary(name: &str) -> Option<PathBuf> {
    binary_candidates(name)
        .into_iter()
        .find(|candidate| candidate.exists())
}

fn is_binary_compatible(path: &Path) -> bool {
    // Quick compatibility guard: skip Linux-specific test binaries on non-Linux hosts.
    if std::env::consts::OS != "linux"
        && let Some(p) = path.to_str()
        && (p.contains("linux-x86_64") || p.contains("linux"))
    {
        return false;
    }
    // Ensure the binary is executable.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(path)
            && meta.permissions().mode() & 0o111 == 0
        {
            return false;
        }
    }
    true
}

fn binary_candidates(name: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let root = workspace_root();
    paths.push(root.join("tests/bin/linux-x86_64").join(name));
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

async fn wait_for_port(name: &str, port: u16, logs_dir: &Path, timeout_at: Duration) -> Result<()> {
    let start = Instant::now();
    let addr = format!("127.0.0.1:{port}");
    loop {
        match TcpStream::connect(&addr).await {
            Ok(_) => {
                write_probe(logs_dir, name, "port open")?;
                return Ok(());
            }
            Err(err) => {
                if start.elapsed() > timeout_at {
                    bail!("{name} did not open port {addr} in time: {err}");
                }
                sleep(Duration::from_millis(250)).await;
            }
        }
    }
}

fn write_probe(logs_dir: &Path, service: &str, message: &str) -> Result<()> {
    let probe = logs_dir.join(format!("probe-{service}.log"));
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&probe)?;
    writeln!(file, "[{}] {message}", now_millis())?;
    Ok(())
}
