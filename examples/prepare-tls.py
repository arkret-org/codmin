#!/usr/bin/env python3
"""Create and validate the example Station's private development CA and TLS leaf."""

import pathlib
import subprocess


def openssl(*args: str) -> str:
    try:
        return subprocess.run(
            ["openssl", *args], check=True, capture_output=True, text=True
        ).stdout
    except subprocess.CalledProcessError as error:
        raise SystemExit(f"Example TLS setup/validation failed: {error.stderr.strip()}") from error


def main() -> None:
    directory = pathlib.Path(__file__).resolve().parent / ".local" / "tls"
    directory.mkdir(mode=0o700, parents=True, exist_ok=True)
    directory.chmod(0o700)
    files = {name: directory / name for name in ("ca.key", "ca.crt", "coland.key", "coland.crt")}
    present = [path.exists() for path in files.values()]
    if any(present) and not all(present):
        raise SystemExit(f"Incomplete example TLS material in {directory}; restore its matching files")
    if not any(present):
        openssl(
            "req", "-x509", "-newkey", "rsa:3072", "-nodes", "-sha256", "-days", "365",
            "-subj", "/CN=Arkret Codmin Example CA",
            "-addext", "basicConstraints=critical,CA:TRUE",
            "-addext", "keyUsage=critical,keyCertSign,cRLSign",
            "-keyout", str(files["ca.key"]), "-out", str(files["ca.crt"]),
        )
        request = directory / "coland.csr"
        extensions = directory / "coland.ext"
        extensions.write_text(
            "subjectAltName=DNS:coland.example,IP:127.0.0.1\n"
            "basicConstraints=critical,CA:FALSE\n"
            "keyUsage=critical,digitalSignature,keyEncipherment\n"
            "extendedKeyUsage=serverAuth\n", encoding="utf-8",
        )
        openssl(
            "req", "-new", "-newkey", "rsa:2048", "-nodes", "-sha256",
            "-subj", "/CN=coland.example", "-keyout", str(files["coland.key"]),
            "-out", str(request),
        )
        openssl(
            "x509", "-req", "-in", str(request), "-CA", str(files["ca.crt"]),
            "-CAkey", str(files["ca.key"]), "-CAcreateserial", "-sha256", "-days", "30",
            "-extfile", str(extensions), "-out", str(files["coland.crt"]),
        )
        request.unlink()
        extensions.unlink()
    # Only the leaf key and public certificates are mounted into containers.
    # The host directory stays private; UID 10001 can read its bind-mounted leaf.
    files["ca.key"].chmod(0o600)
    for name in ("ca.crt", "coland.key", "coland.crt"):
        files[name].chmod(0o644)
    for name in ("ca", "coland"):
        certificate_key = openssl("x509", "-in", str(files[f"{name}.crt"]), "-pubkey", "-noout")
        private_key = openssl("pkey", "-in", str(files[f"{name}.key"]), "-pubout")
        if certificate_key != private_key:
            raise SystemExit(f"Example TLS certificate/key mismatch: {name}")
    openssl("verify", "-CAfile", str(files["ca.crt"]), "-verify_hostname", "coland.example", str(files["coland.crt"]))
    openssl("verify", "-CAfile", str(files["ca.crt"]), "-verify_ip", "127.0.0.1", str(files["coland.crt"]))
    openssl("x509", "-in", str(files["coland.crt"]), "-checkend", "86400", "-noout")
    print(f"[example-stack/tls] verified development TLS material in {directory}")


if __name__ == "__main__":
    main()
