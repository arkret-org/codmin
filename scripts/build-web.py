#!/usr/bin/env python3
"""Build the Glory/Sapid browser app and emit a static nginx-ready directory."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]

def run(*args, env):
    subprocess.run(args, cwd=ROOT, env=env, check=True, timeout=3600)

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--release", action="store_true")
    parser.add_argument("--out-dir", type=Path)
    args = parser.parse_args()
    env = os.environ.copy()
    if os.name == "nt":
        cache = Path("D:/BuildCache/codmin-glory")
        cache.mkdir(parents=True, exist_ok=True)
        temporary = cache / "tmp"
        temporary.mkdir(exist_ok=True)
        env.setdefault("CARGO_HOME", "D:/BuildCache/cargo")
        env.setdefault("CARGO_TARGET_DIR", str(cache / "target"))
        env["TEMP"] = env["TMP"] = str(temporary)
        tempfile.tempdir = str(temporary)
    if os.name == "nt" and Path(env["CARGO_HOME"]).resolve().drive.lower() != "d:":
        raise SystemExit("CARGO_HOME must be on D:")
    candidate = Path("D:/BuildCache/codmin-glory/tools/bin/wasm-bindgen.exe")
    bindgen = env.get("WASM_BINDGEN") or (str(candidate) if os.name == "nt" and candidate.is_file() else "wasm-bindgen")
    version = subprocess.check_output([bindgen, "--version"], cwd=ROOT, env=env, text=True).strip()
    if version != "wasm-bindgen 0.2.129":
        raise SystemExit(f"Expected wasm-bindgen 0.2.129, got {version}")
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"], cwd=ROOT, env=env, text=True))
    target = Path(metadata["target_directory"]).resolve()
    if os.name == "nt" and target.drive.lower() != "d:":
        raise SystemExit(f"Build output must be on D:, got {target}")
    print(f"Cargo target directory: {target}", flush=True)
    command = ["cargo", "build", "--locked", "--lib", "--target", "wasm32-unknown-unknown"]
    if args.release:
        command.append("--release")
    run(*command, env=env)
    profile = "release" if args.release else "debug"
    output = (args.out_dir or target / "web").resolve()
    if os.name == "nt" and output.drive.lower() != "d:":
        raise SystemExit(f"Web output must be on D:, got {output}")
    output.mkdir(parents=True, exist_ok=True)
    bindgen = env.get("WASM_BINDGEN", "wasm-bindgen")
    run(bindgen, str(target / "wasm32-unknown-unknown" / profile / "codmin.wasm"),
        "--target", "web", "--out-name", "codmin", "--out-dir", str(output / "pkg"), env=env)
    for source in (ROOT / "web").iterdir():
        if source.is_file():
            shutil.copy2(source, output / source.name)
        elif source.is_dir():
            shutil.copytree(source, output / source.name, dirs_exist_ok=True)
    sapid = ROOT.parent.parent / "glory-rs" / "sapid" / "styles" / "sapid.css"
    shutil.copy2(sapid, output / "sapid.css")
    print(f"Static web bundle: {output}")

if __name__ == "__main__":
    main()
