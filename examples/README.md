# Codmin examples

The integrated bring-up: postgres + coauth + coland + floria + codmin,
driven by `docker-compose.example-stack.yaml` + `up.sh` / `down.sh` /
`smoke.sh`. Exercised every night in CI via
[`.github/workflows/nightly-example-stack.yaml`](../.github/workflows/nightly-example-stack.yaml).

## Codmin example stack

| Service  | Image base                              | Host port | Probe         |
|----------|-----------------------------------------|-----------|---------------|
| postgres | `postgres:16-alpine`                    | `55432`   | `pg_isready`  |
| coauth   | `gcr.io/distroless/cc-debian12:nonroot` | `57080`   | `/health`     |
| coland   | `debian:bookworm-slim`                  | `58787`   | `/health`     |
| floria   | `gcr.io/distroless/cc-debian13:nonroot` | `55000`   | `/ready`      |
| codmin   | `nginx:alpine`                          | `58200`   | `/healthz`    |

Container ports differ from host ports; see
[`docker-compose.example-stack.yaml`](docker-compose.example-stack.yaml)
for the full mapping.

### Repository layout assumption

The compose file references sibling repos via relative build contexts.
Coauth + codmin both use an **umbrella build context** (`../..` →
`arkret/`) so the workspace `Cargo.toml` path-deps
`../arkret-rust-sdk/...` and `../coauth/crates/admin-types` resolve
inside the build sandbox. Coland and Floria use named SDK build contexts;
Coland additionally receives the Spec and Floria contract contexts.

The SDK's `.dockerignore` filters its named contexts, and the Coauth and
Codmin Dockerfile-specific ignore files filter their umbrella contexts.
Build caches, Git internals and local credentials stay outside build inputs.
The bring-up script builds the remaining service images in sequence so
independent Rust release builds do not compete for the runner's memory.

```
arkret/
  arkret-rust-sdk/                      <-- required (path-dep target)
  arkret-spec/
  coauth/                                 <-- required (path-dep + image)
  coland/
  floria/
  cotest/
  garth/
  codmin/
    examples/
      docker-compose.example-stack.yaml   <-- run from here
      up.sh / down.sh / smoke.sh
      coauth-config.yaml                  <-- generated, gitignored
```

If your checkout differs, override every `*_IMAGE` env var with a
prebuilt tag (see "Image overrides" below) and skip the `--build` step.

### Operator pre-requisites

The compose stack expects:

- Docker Desktop / dockerd reachable (`docker info` returns 0).
- `python3` on PATH (`up.sh` uses Python to rewrite the generated
  `coauth-config.yaml` deterministically — sed-based YAML edits proved
  brittle against the multi-listener default config in C36.5).
- `curl` on PATH (`smoke.sh` probes each `/health` endpoint).
- `openssl` on PATH (`prepare-tls.py` creates and validates a private
  development CA and the Station's TLS certificate).
- All seven sibling repos shown above checked out alongside Codmin.

Before the first `up.sh` invocation on a new machine, confirm Docker is
reachable and the sibling repos exist. `up.sh` and `smoke.sh` perform the
live stack checks directly.

### Manual binary builds (advanced operators)

If you want to dogfood a service without docker, build each binary
locally with the same feature flags the Dockerfiles use:

```bash
# coauth — distroless build features (no native bundling); wired in C36.5
cargo build --bin coauth --no-default-features --features docker,cedar -p coauth

# coland — single-binary release; default features
cargo build --release --bin coland

# floria — single-binary release; default features
cargo build --release --bin floria

# codmin — Dioxus WASM SPA; uses dx, not cargo
cargo install dioxus-cli@0.7.10 --locked
dx build --release
# Query cargo metadata --format-version 1 --no-deps for target_directory.
# Web output is under its dx/codmin/release/web/public directory.
```

Smoke binary (Rust-side): `cargo build --bin codmin-smoke` builds the
post-deploy probe shipped in the codmin repo (`src/bin/codmin_smoke.rs`).

### Quickstart

```bash
# 1. Bring the stack up (builds images + generates config + waits for /health)
./examples/up.sh

# 2. Confirm every service is reachable
./examples/smoke.sh

# 3. Tear down + drop volumes
./examples/down.sh
```

Open [http://localhost:58200](http://localhost:58200) for the codmin SPA
once `up.sh` reports the stack healthy.

### Health and persistent identity

Coauth and Coland use their native `healthcheck` subcommands. Floria uses
its std-only `floria-healthcheck` executable; its distroless image has no
shell or curl. Codmin uses Alpine's wget. Compose waits for every service
to be healthy, then `smoke.sh` independently requires HTTP 200 from all
four published health endpoints. Host ports bind only to loopback.
The explicit Compose project name `arkret-codmin-example` isolates these
services and volumes from other repositories' `examples/` stacks.

Floria explicitly loads the read-only `floria-config.kdl`, which enables its
documented `FLORIA_*` environment overrides. The file registers a real JPush
provider with public development placeholders so the service can start.
Those placeholders cannot authenticate to JPush. This health gate neither
sends nor verifies external push notifications; replace them with your
provider credentials before exercising push delivery.

`up.sh` creates separate Coauth and Coland databases before either service
runs its migrations. An explicit one-shot setup container calls the official
`coland-keystore-keygen --if-missing` helper to create separate, validated
base64-encoded 32-byte wrapping keys for both services. Each encrypted KeyStore
has its own data volume and a separate master-key volume, mounted read-only by
its service. Coland also persists its identity bundle. The setup assigns
ownership to the images' actual users (Coland 10001, Coauth 65532).
Coauth's generated configuration explicitly selects `encrypted_file`, rather
than desktop Secret Service. Only an empty Coauth KeyStore triggers the separate
`coauth-provision` setup service with `--first-provisioning`; it is stopped after
its real healthcheck succeeds. The ordinary Coauth service omits that flag and
loads the same durable key bundle. Existing stores and wrapping keys are retained.
Nightly also runs `check-key-persistence.py`, which restarts the ordinary Coauth
service, waits for its native healthcheck, and requires its actual published
signing keys to match the keys before restart. The four health probes run again.
When upgrading an older example config, use `up.sh --regen-config` to select
the current durable backend.
Development mode permits initial local Station provisioning. The Station's
advertised service identity uses HTTPS, including on this development network.
`prepare-tls.py` generates a private development CA and a 30-day leaf certificate
for `coland.example` and `127.0.0.1`; `up.sh` validates and reuses existing material.
The generated files live under the gitignored and Docker-excluded
`examples/.local/tls/` directory. The CA private key stays on the host. Coland
mounts only the public CA, leaf certificate and leaf key read-only; its native
healthcheck trusts that CA through `SSL_CERT_FILE`. Codmin's HTTPS upstream proxy
enables certificate and hostname verification against the same CA.
`smoke.sh` uses `curl --cacert` and still requires HTTP 200. To probe manually:

```bash
curl --cacert examples/.local/tls/ca.crt https://127.0.0.1:58787/health
```

Signature and DID verification remain mandatory. Expired or mismatched TLS
material fails preflight; renew the development certificate before restarting.
TLS files survive `down.sh` and are independent of the durable identity keys.
Use `down.sh --keep-vol` to retain data and keys; ordinary `down.sh` removes
all example volumes. Do not replace a retained master key or restore the
encrypted data without its original key.

This nightly gate covers real service startup, health and the admin SPA.
It does not establish an Account Authority trust edge or prove authenticated
admin operations. Such a deployment requires the services' documented TLS,
Station peer, trust-domain and shared-credential configuration. The retired
`SERVERX_*` settings do not configure any of those inputs.

### Image overrides

Every `image:` is `${SVC_IMAGE:-svc:dev}`, so you can pin local image tags
instead of rebuilding each service:

```bash
COAUTH_IMAGE=coauth:dev \
COLAND_IMAGE=coland:dev \
FLORIA_IMAGE=floria:dev \
CODMIN_IMAGE=codmin:dev \
    ./examples/up.sh --no-build
```

### Coauth config generation

`up.sh` generates `examples/coauth-config.yaml` on first run by exec'ing
`coauth config generate` inside an ephemeral coauth container. This
gives you a valid config pointing at the durable encrypted KeyStore,
without embedding private keys. Initial provisioning creates the signing and
encryption keys inside that store. Use `--regen-config` to refresh the config;
this retains the stored keys and separate master key.

### Nightly CI

[`.github/workflows/nightly-example-stack.yaml`](../.github/workflows/nightly-example-stack.yaml)
runs `up.sh` + `smoke.sh` on a fresh runner every night at 07:30 UTC
(plus on PRs that touch the example stack files). On failure it uploads
container logs as artifacts (`example-stack-logs-<run_id>`) so a
regression doesn't require local docker reproduction.

### Troubleshooting

| Symptom                                 | Likely cause / fix                                          |
|-----------------------------------------|-------------------------------------------------------------|
| `coauth config generate failed`         | `coauth:dev` image not built — run `up.sh` without `--no-build`, or pre-pull `COAUTH_IMAGE`. |
| `port 55432 already bound`              | Local postgres on host. Stop it or change the host port in the compose file. |
| `coauth` healthcheck flaps              | Postgres still migrating. The `start_period: 20s` should cover it; bump if your machine is slow. |
| `codmin: never returned 200`            | The Dioxus build inside the image is large. Re-run `smoke.sh` with `SMOKE_ATTEMPT_BUDGET=120`. |
| Build context error: `../../coauth` not found | Sibling repos aren't checked out. Either clone them next to `codmin/` or override every `*_IMAGE` env var. |
| `502 Bad Gateway` during apt-get      | Docker Desktop proxy or upstream Debian mirror flake. Floria/coauth wrap apt in a 5-attempt retry loop (added C37.3); just re-run `up.sh` if it surfaces. |
| `cargo chef cook: failed to read /arkret-rust-sdk/...` | Sibling `arkret-rust-sdk` missing from umbrella checkout. Confirm the sibling checkout exists before rerunning. |
| `cargo fetch --locked: ../arkret-rust-sdk not found` | Same as above. The codmin/coauth Dockerfiles need arkret-rust-sdk side-by-side at the umbrella context root. |
