#!/usr/bin/env python3
"""Set the application version from the GitHub release tag."""

import re
import sys
from pathlib import Path


version = sys.argv[1] if len(sys.argv) == 2 else ""
if not re.fullmatch(r"(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)", version):
    raise SystemExit(f"Invalid release version: {version!r}")

root = Path(__file__).resolve().parents[2]
cargo_toml = root / "src-tauri" / "Cargo.toml"
tauri_config = root / "src-tauri" / "tauri.conf.json"
cargo_lock = root / "src-tauri" / "Cargo.lock"

cargo_text = cargo_toml.read_text(encoding="utf-8")
cargo_text, replacements = re.subn(
    r'(?m)^(version\s*=\s*)"[^"]+"$',
    lambda match: f'{match.group(1)}"{version}"',
    cargo_text,
    count=1,
)
if replacements != 1:
    raise SystemExit("Could not find the package version in src-tauri/Cargo.toml")
cargo_toml.write_text(cargo_text, encoding="utf-8", newline="\n")

lock_text = cargo_lock.read_text(encoding="utf-8")
lock_text, replacements = re.subn(
    r'(?m)(\[\[package\]\]\nname = "irani-idm"\nversion = ")[^"]+("$)',
    lambda match: f'{match.group(1)}{version}{match.group(2)}',
    lock_text,
    count=1,
)
if replacements != 1:
    raise SystemExit("Could not find the irani-idm package version in src-tauri/Cargo.lock")
cargo_lock.write_text(lock_text, encoding="utf-8", newline="\n")

config_text = tauri_config.read_text(encoding="utf-8")
config_text, replacements = re.subn(
    r'(?m)^([ \t]*"version"\s*:\s*)"[^"]+"',
    lambda match: f'{match.group(1)}"{version}"',
    config_text,
    count=1,
)
if replacements != 1:
    raise SystemExit("Could not find the application version in src-tauri/tauri.conf.json")
tauri_config.write_text(config_text, encoding="utf-8", newline="\n")

print(f"Application version set to {version}")
