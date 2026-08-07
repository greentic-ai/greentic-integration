#!/usr/bin/env bash
set -euo pipefail

# Install one or more Greentic CLIs from crates.io through cargo-binstall.
#
# Why this wrapper exists
# -----------------------
# `cargo binstall` prefers a prebuilt release binary, but when the GitHub
# release lookup exceeds its resolution deadline it silently falls back to
# building the crate from source. A source build re-resolves every dependency
# against the *current* registry, so a CLI released months ago is compiled
# against greentic crates published since — and the 0.4.x lineage no longer
# compiles that way (greentic-flow 0.4.44 against greentic-types 0.4.61 fails
# with E0004/E0063). Nightly run 31147216305 failed exactly this way while the
# sibling jobs, which won the network race, installed the same CLI fine.
#
# Two guards, both needed:
#   * `--locked` makes the source fallback honour the Cargo.lock the release was
#     published with, so a fallback build is reproducible instead of drifting.
#   * a longer resolution deadline plus retries keeps a slow GitHub response
#     from triggering the fallback at all.
#
# Environment:
#   GREENTIC_BINSTALL_ATTEMPTS           attempts per crate (default 3)
#   GREENTIC_BINSTALL_RESOLUTION_TIMEOUT per-resolution deadline in seconds
#                                        (default 60; cargo-binstall default is 15)
#   GREENTIC_BINSTALL_RETRY_DELAY        seconds to wait before a retry (default 10)
#   GITHUB_TOKEN                         optional; authenticates the release
#                                        lookups so they are not rate limited
#
# Usage:
#   ci/install_greentic_cli.sh greentic-pack greentic-runner
#
# Exits non-zero if any crate could not be installed, so callers can treat an
# optional CLI as best-effort with `if ci/install_greentic_cli.sh foo; then`.

max_attempts="${GREENTIC_BINSTALL_ATTEMPTS:-3}"
resolution_timeout="${GREENTIC_BINSTALL_RESOLUTION_TIMEOUT:-60}"
retry_delay="${GREENTIC_BINSTALL_RETRY_DELAY:-10}"

if [[ $# -eq 0 ]]; then
  echo "usage: $(basename "$0") <crate>[@<version>] ..." >&2
  exit 2
fi

install_crate() {
  local crate_spec=$1
  local crate_name="${crate_spec%%@*}"
  local -a version_args=()
  if [[ "$crate_spec" == *@* ]]; then
    version_args=(--version "${crate_spec#*@}")
  fi

  local attempt=1
  while ((attempt <= max_attempts)); do
    echo "==> installing ${crate_spec} (attempt ${attempt}/${max_attempts})"
    # `${arr[@]+…}` keeps an empty array from tripping `set -u` on bash 3.2.
    if cargo binstall "${crate_name}" ${version_args[@]+"${version_args[@]}"} \
      --no-confirm \
      --force \
      --locked \
      --maximum-resolution-timeout "${resolution_timeout}"; then
      return 0
    fi
    echo "WARN: installing ${crate_spec} failed on attempt ${attempt}" >&2
    ((attempt++))
    if ((attempt <= max_attempts)); then
      sleep "${retry_delay}"
    fi
  done

  echo "ERROR: could not install ${crate_spec} after ${max_attempts} attempts" >&2
  return 1
}

failed_crates=()
for crate_spec in "$@"; do
  if ! install_crate "${crate_spec}"; then
    failed_crates+=("${crate_spec}")
  fi
done

if ((${#failed_crates[@]} > 0)); then
  echo "ERROR: failed to install: ${failed_crates[*]}" >&2
  exit 1
fi
