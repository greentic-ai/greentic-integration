#!/usr/bin/env python3
"""Lightweight validator for local pack fixtures.

The script keeps us honest while we build out the real greentic-dev / greentic-pack flows.
It performs schema checks and optionally shells out to the official CLIs when the
`GREENTIC_PACK_VALIDATE=1` environment flag is set and the executables exist on PATH.
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
PACKS_DIR = REPO_ROOT / "packs"
REQUIRED_MANIFEST_FIELDS = {"id", "name", "version", "description", "type", "scenarios"}
REQUIRED_SCENARIO_FIELDS = {"id", "entry", "golden"}


def _load_json(path: Path) -> dict:
    try:
        return json.loads(path.read_text())
    except FileNotFoundError as exc:
        raise AssertionError(f"Missing file: {path}") from exc
    except json.JSONDecodeError as exc:
        raise AssertionError(f"Invalid JSON in {path}: {exc}") from exc


def _validate_manifest(manifest_path: Path) -> list[str]:
    errors: list[str] = []
    manifest = _load_json(manifest_path)

    missing = REQUIRED_MANIFEST_FIELDS - manifest.keys()
    if missing:
        errors.append(f"{manifest_path}: missing fields {sorted(missing)}")

    scenarios = manifest.get("scenarios") or []
    if not isinstance(scenarios, list) or not scenarios:
        errors.append(f"{manifest_path}: 'scenarios' must be a non-empty list")
        return errors

    for scenario in scenarios:
        if not isinstance(scenario, dict):
            errors.append(f"{manifest_path}: scenario entries must be objects")
            continue
        missing_fields = REQUIRED_SCENARIO_FIELDS - scenario.keys()
        if missing_fields:
            errors.append(
                f"{manifest_path}: scenario {scenario.get('id')} missing fields {sorted(missing_fields)}"
            )
            continue

        scenario_id = scenario["id"]
        entry = manifest_path.parent / scenario["entry"]
        golden = manifest_path.parent / scenario["golden"]

        errors.extend(_validate_scenario_file(scenario_id, entry))
        errors.extend(_validate_golden_file(scenario_id, golden))

    readme_path = manifest_path.parent / "README.md"
    if not readme_path.exists():
        errors.append(f"{manifest_path.parent}: README.md is required for contributor context")

    return errors


def _validate_scenario_file(expected_id: str, path: Path) -> list[str]:
    try:
        data = _load_json(path)
    except AssertionError as err:
        return [str(err)]

    errors: list[str] = []
    if data.get("scenario") != expected_id:
        errors.append(f"{path}: scenario id mismatch (expected {expected_id})")
    steps = data.get("steps")
    if not isinstance(steps, list) or not steps:
        errors.append(f"{path}: steps must be a non-empty list")
    return errors


def _validate_golden_file(expected_id: str, path: Path) -> list[str]:
    try:
        data = _load_json(path)
    except AssertionError as err:
        return [str(err)]

    errors: list[str] = []
    if data.get("scenario_id") != expected_id:
        errors.append(f"{path}: scenario_id mismatch (expected {expected_id})")
    transcript = data.get("transcript")
    if not isinstance(transcript, list) or not transcript:
        errors.append(f"{path}: transcript must be a non-empty list")
    return errors


def _maybe_run_cli(manifest_path: Path) -> list[str]:
    if os.environ.get("GREENTIC_PACK_VALIDATE") != "1":
        return []

    cli_errors: list[str] = []
    commands = [
        ["greentic-dev", "pack", "validate", str(manifest_path)],
        ["greentic-pack", "sim", str(manifest_path)],
    ]
    for cmd in commands:
        try:
            subprocess.run(cmd, cwd=REPO_ROOT, check=True, capture_output=False)
        except FileNotFoundError:
            cli_errors.append(f"Skipping {' '.join(cmd)} (binary not found on PATH)")
        except subprocess.CalledProcessError as exc:
            cli_errors.append(f"Command {' '.join(cmd)} failed with exit {exc.returncode}")
    return cli_errors


def main() -> int:
    manifests = sorted(PACKS_DIR.glob("*/pack.json"))
    if not manifests:
        print("No pack manifests found under packs/", file=sys.stderr)
        return 1

    errors: list[str] = []
    warnings: list[str] = []
    for manifest_path in manifests:
        errors.extend(_validate_manifest(manifest_path))
        warnings.extend(_maybe_run_cli(manifest_path))

    if warnings:
        for warning in warnings:
            print(f"[warn] {warning}")

    if errors:
        for error in errors:
            print(f"[error] {error}", file=sys.stderr)
        return 1

    print(f"Validated {len(manifests)} pack(s) successfully.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
