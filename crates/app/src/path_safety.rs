use std::{
    ffi::OsString,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow};

/// Normalize a user-supplied path and ensure it stays within an allowed root.
/// Reject absolute paths and any that escape via `..`.
pub fn normalize_under_root(root: &Path, candidate: &Path) -> Result<PathBuf> {
    if candidate.is_absolute() {
        anyhow::bail!("absolute paths are not allowed: {}", candidate.display());
    }

    let canon_root = root
        .canonicalize()
        .with_context(|| format!("failed to canonicalize {}", root.display()))?;
    let joined = canon_root.join(candidate);
    match joined.canonicalize() {
        Ok(canon) => {
            if !canon.starts_with(&canon_root) {
                anyhow::bail!(
                    "path escapes root ({}): {}",
                    canon_root.display(),
                    canon.display()
                );
            }
            Ok(canon)
        }
        Err(err) if err.kind() == ErrorKind::NotFound => {
            let mut missing: Vec<OsString> = Vec::new();
            let mut ancestor = joined.as_path();
            while !ancestor.exists() {
                let file_name = ancestor
                    .file_name()
                    .ok_or_else(|| anyhow!("path has no remaining ancestor to normalize"))?;
                missing.push(file_name.to_os_string());
                ancestor = ancestor
                    .parent()
                    .ok_or_else(|| anyhow!("path has no remaining ancestor to normalize"))?;
            }

            let canon = ancestor
                .canonicalize()
                .with_context(|| format!("failed to canonicalize {}", ancestor.display()))?;

            if !canon.starts_with(&canon_root) {
                anyhow::bail!(
                    "path escapes root ({}): {}",
                    canon_root.display(),
                    canon.display()
                );
            }

            let mut rebuilt = canon;
            for part in missing.iter().rev() {
                rebuilt.push(part);
            }
            Ok(rebuilt)
        }
        Err(err) => {
            Err(err).with_context(|| format!("failed to canonicalize {}", joined.display()))
        }
    }
}
