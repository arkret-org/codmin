#!/usr/bin/env python3
"""Build the Glory/Sapid browser app and emit a static nginx-ready directory."""
import argparse
import json
import os
import re
from pathlib import Path
import shutil
import subprocess
import tomllib
from ui_dependencies import cargo_metadata, sapid_stylesheet, ui_provenance

ROOT = Path(__file__).resolve().parents[1]

def run(*args, env):
    subprocess.run(args, cwd=ROOT, env=env, check=True, timeout=3600)

def locked_bindgen_version(root=ROOT):
    packages = tomllib.loads((root / "Cargo.lock").read_text(encoding="utf-8")).get("package", [])
    packages = [package for package in packages if package.get("name") == "wasm-bindgen"]
    if len(packages) != 1:
        raise SystemExit("Cargo.lock must contain exactly one wasm-bindgen package")
    version = packages[0].get("version")
    if not isinstance(version, str) or not re.fullmatch(
        r"[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?", version
    ):
        raise SystemExit("Cargo.lock wasm-bindgen version is invalid")
    return version


def find_bindgen(env, version):
    selected = env.get("WASM_BINDGEN")
    candidates = [selected] if selected else []
    if not selected:
        if env.get("CARGO_HOME"):
            executable = "wasm-bindgen.exe" if os.name == "nt" else "wasm-bindgen"
            candidates.append(str(Path(env["CARGO_HOME"]) / "bin" / executable))
        candidates.append(shutil.which("wasm-bindgen", path=env.get("PATH")) or "wasm-bindgen")
    for candidate in candidates:
        try:
            actual = subprocess.check_output(
                [candidate, "--version"], cwd=ROOT, env=env, text=True
            ).strip()
        except (OSError, subprocess.CalledProcessError):
            continue
        if actual == f"wasm-bindgen {version}":
            return candidate
    raise SystemExit(f"Install wasm-bindgen-cli {version} or set WASM_BINDGEN to a matching CLI")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--print-bindgen-version", action="store_true")
    parser.add_argument("--release", action="store_true")
    parser.add_argument("--out-dir", type=Path)
    args = parser.parse_args()
    expected_bindgen = locked_bindgen_version()
    if args.print_bindgen_version:
        print(expected_bindgen)
        return
    env = os.environ.copy()
    bindgen = find_bindgen(env, expected_bindgen)
    metadata = cargo_metadata(ROOT, env)
    target = Path(metadata["target_directory"]).resolve()
    print(f"Cargo target directory: {target}", flush=True)
    command = ["cargo", "build", "--locked", "--lib", "--target", "wasm32-unknown-unknown"]
    if args.release:
        command.append("--release")
    run(*command, env=env)
    profile = "release" if args.release else "debug"
    output = (args.out_dir or target / "web").resolve()
    output.mkdir(parents=True, exist_ok=True)
    run(bindgen, str(target / "wasm32-unknown-unknown" / profile / "codmin.wasm"),
        "--target", "web", "--out-name", "codmin", "--out-dir", str(output / "pkg"), env=env)
    for source in (ROOT / "web").iterdir():
        if source.is_file():
            shutil.copy2(source, output / source.name)
        elif source.is_dir():
            shutil.copytree(source, output / source.name, dirs_exist_ok=True)
    sapid = sapid_stylesheet(metadata)
    shutil.copy2(sapid, output / "sapid.css")
    (output / "ui-build-info.json").write_text(json.dumps(ui_provenance(metadata), indent=2) + "\n", encoding="utf-8")
    print(f"Static web bundle: {output}")

if __name__ == "__main__":
    main()
