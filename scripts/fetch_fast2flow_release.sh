#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_ROOT="${GREENTIC_FAST2FLOW_RELEASE_DIR:-${ROOT_DIR}/artifacts/fast2flow-release}"
REPO="${GREENTIC_FAST2FLOW_GH_REPO:-greenticai/greentic-fast2flow}"
GTPACK_OCI_REPO="${GREENTIC_FAST2FLOW_GTPACK_OCI_REPO:-ghcr.io/greenticai/providers/routing-hook/fast2flow.gtpack}"
REQUIRE_GTPACK="${GREENTIC_FAST2FLOW_REQUIRE_GTPACK:-0}"
ALLOW_MISSING_RELEASE="${GREENTIC_FAST2FLOW_ALLOW_MISSING_RELEASE:-1}"

mkdir -p "${OUT_ROOT}"

log() {
  printf '[fetch_fast2flow_release] %s\n' "$*"
}

need() {
  command -v "$1" >/dev/null 2>&1 || {
    log "error: missing required command '$1'"
    exit 1
  }
}

need gh
need tar
need uname
need python3

arch="$(uname -m)"
os="$(uname -s)"

case "${arch}" in
  x86_64|amd64) target_arch="x86_64" ;;
  aarch64|arm64) target_arch="aarch64" ;;
  *)
    log "error: unsupported architecture '${arch}'"
    exit 1
    ;;
esac

case "${os}" in
  Linux) target_os="unknown-linux-gnu"; ext="tar.gz" ;;
  Darwin) target_os="apple-darwin"; ext="tar.gz" ;;
  *)
    log "error: unsupported OS '${os}'"
    exit 1
    ;;
esac

if ! tag="$(gh release view --repo "${REPO}" --json tagName -q .tagName 2>/dev/null)"; then
  if [[ "${ALLOW_MISSING_RELEASE}" == "1" ]]; then
    log "warn: release not found for ${REPO}; leaving fast2flow release artifacts unresolved"
    mkdir -p "${OUT_ROOT}/latest"
    cat > "${OUT_ROOT}/latest/env.sh" <<EOF
export GREENTIC_FAST2FLOW_RELEASE_VERSION=""
EOF
    exit 0
  fi
  log "error: release not found for ${REPO}"
  exit 1
fi

version="${tag#v}"
target="${target_arch}-${target_os}"
release_dir="${OUT_ROOT}/${tag}"
latest_dir="${OUT_ROOT}/latest"
tmp_dir="$(mktemp -d)"
trap 'rm -rf "${tmp_dir}"' EXIT

assets_json="${tmp_dir}/assets.json"
gh release view --repo "${REPO}" --json assets > "${assets_json}"

readarray -t asset_names < <(
  python3 - "${assets_json}" <<'PY'
import json
import sys

with open(sys.argv[1], "r", encoding="utf-8") as fh:
    data = json.load(fh)

for asset in data.get("assets", []):
    name = asset.get("name")
    if name:
        print(name)
PY
)

pick_asset() {
  local kind="$1"
  python3 - "$kind" "$target" "$ext" "${asset_names[@]}" <<'PY'
import sys

kind = sys.argv[1]
target = sys.argv[2]
ext = sys.argv[3]
assets = sys.argv[4:]

def choose(candidates):
    return sorted(candidates)[-1] if candidates else ""

if kind == "cli":
    chosen = choose(
        [
            a for a in assets
            if a.startswith("greentic-fast2flow-")
            and a.endswith("." + ext)
            and target in a
            and "routing-host" not in a
        ]
    )
elif kind == "host":
    chosen = choose(
        [
            a for a in assets
            if a.startswith("greentic-fast2flow-routing-host-")
            and a.endswith("." + ext)
            and target in a
        ]
    )
else:
    chosen = ""

if chosen:
    print(chosen)
PY
}

cli_asset="$(pick_asset cli)"
host_asset="$(pick_asset host)"

if [[ -z "${cli_asset}" ]]; then
  log "error: could not find greentic-fast2flow release asset for target ${target}"
  exit 1
fi

if [[ -z "${host_asset}" ]]; then
  log "error: could not find greentic-fast2flow-routing-host release asset for target ${target}"
  exit 1
fi

# fast2flow.gtpack is published to GHCR via oras, not as a GitHub Release asset.
# Pull it from the OCI registry; require oras only when REQUIRE_GTPACK=1.
gtpack_pulled=""
gtpack_oci_tmp="${tmp_dir}/gtpack-oci"
if command -v oras >/dev/null 2>&1; then
  mkdir -p "${gtpack_oci_tmp}"
  if (cd "${gtpack_oci_tmp}" && oras pull "${GTPACK_OCI_REPO}:${tag}" >/dev/null 2>&1); then
    gtpack_pulled="$(find "${gtpack_oci_tmp}" -type f -name 'fast2flow.gtpack' | head -n 1)"
  fi
elif [[ "${REQUIRE_GTPACK}" == "1" ]]; then
  log "error: 'oras' is required to fetch ${GTPACK_OCI_REPO}:${tag} but was not found in PATH"
  exit 1
fi

if [[ "${REQUIRE_GTPACK}" == "1" && -z "${gtpack_pulled}" ]]; then
  log "error: failed to pull ${GTPACK_OCI_REPO}:${tag} from GHCR (oras pull). Ensure the runner is authenticated with packages:read and the tag exists."
  exit 1
fi

log "repo: ${REPO}"
log "tag: ${tag}"
log "target: ${target}"
log "output: ${release_dir}"

mkdir -p "${release_dir}"

gh release download "${tag}" \
  --repo "${REPO}" \
  --dir "${tmp_dir}" \
  --pattern "${cli_asset}" \
  --pattern "${host_asset}"

tar -xzf "${tmp_dir}/${cli_asset}" -C "${release_dir}"
tar -xzf "${tmp_dir}/${host_asset}" -C "${release_dir}"

mkdir -p "${latest_dir}"
cp "${release_dir}/greentic-fast2flow" "${latest_dir}/greentic-fast2flow"
cp "${release_dir}/greentic-fast2flow-routing-host" "${latest_dir}/greentic-fast2flow-routing-host"
chmod +x "${latest_dir}/greentic-fast2flow" "${latest_dir}/greentic-fast2flow-routing-host"

if [[ -n "${gtpack_pulled}" ]]; then
  cp "${gtpack_pulled}" "${release_dir}/fast2flow.gtpack"
  cp "${gtpack_pulled}" "${latest_dir}/fast2flow.gtpack"
fi

cat > "${latest_dir}/env.sh" <<EOF
export GREENTIC_FAST2FLOW_CLI_BIN="${latest_dir}/greentic-fast2flow"
export GREENTIC_FAST2FLOW_HOST_BIN="${latest_dir}/greentic-fast2flow-routing-host"
export GREENTIC_FAST2FLOW_GTPACK="${latest_dir}/fast2flow.gtpack"
export GREENTIC_FAST2FLOW_RELEASE_VERSION="${version}"
EOF

log "ready: ${latest_dir}/greentic-fast2flow"
log "ready: ${latest_dir}/greentic-fast2flow-routing-host"
if [[ -n "${gtpack_pulled}" ]]; then
  log "ready: ${latest_dir}/fast2flow.gtpack (from ${GTPACK_OCI_REPO}:${tag})"
else
  log "warn: fast2flow gtpack not pulled from ${GTPACK_OCI_REPO}:${tag}"
fi
log "env file: ${latest_dir}/env.sh"
