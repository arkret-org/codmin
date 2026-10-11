# codmin

Arkret administrator web UI for Station and coauth deployments. The app is built with Dioxus and compiled to WebAssembly.

## Pre-commit hook setup

After cloning, enable the project's pre-commit hooks:

```sh
git config core.hooksPath .githooks
```

The hook runs `cargo fmt --all -- --check` and `cargo clippy --no-deps -- -D
warnings` on staged Rust changes. If `.githooks/pre-commit` is missing on
a branch, copy it from
[`arkret-rust-sdk`](https://github.com/arkret-org/arkret-rust-sdk) and
adapt to your local toolchain.

## Scope

- **Deployment-local state (read/write)**: server configuration and diagnostics,
  coauth account workflows, OAuth sessions, upstream providers, registration
  tokens, owner-scoped policy documents, and issuer-local handle lifecycle.
- **Issuer-local credential state (trigger + inspect)**: coauth account and
  session lifecycle operations backed by the Account Authority's durable
  issuer ledger. Session grants are signed credentials with issuer-local
  lifecycle state; they are not Arkret Events and codmin does not draft or
  submit grant Events.
- **Service-attested Event state (trigger + inspect)**: lifecycle operations
  that the protocol explicitly models as Events signed by the authoritative
  service, such as `ak.account.status`.
- **Principal-key state (read-only)**: Realm membership, invites, policies
  and lifecycle, device authorization/revocation, capabilities, and the Realm's
  current governing Station with its authority handoff chain.
- **Dashboard**: server profile, health, storage and conformance status.
- **Actors and devices**: search, detail, sessions, DID/handle lifecycle,
  trust state, key status, grants, delegations and effective permission review.
- **Realms**: read-only protocol projections with membership, invite, policy,
  lifecycle, and authority commit stream context.
- **Federation**: peers, service DIDs, transactions, replay/fork quarantine and verification status.
- **Blob/media**: quota, metadata, retention and anti-enumeration diagnostics.
- **coauth**: accounts, sessions, upstream providers, OAuth2 clients, registration tokens, notification channels and audit logs.

`codmin` authenticates through coauth OAuth + HttpOnly cookies and holds no
Arkret device or service signing key. It does not implement Arkret reducers,
authorization decisions, or principal-key Event authoring. Those writes belong
in a key-holding client, and shared Realm finality belongs to the Realm's
governing Station.

## Architecture

```mermaid
flowchart LR
  Browser["Admin browser"] --> Codmin["codmin Dioxus/WASM"]
  Codmin --> Nginx["nginx static host + proxy"]
  Nginx --> Coland["coland Station admin API"]
  Nginx --> Coauth["coauth auth/account admin API"]
  Coauth --> OAuth["OAuth2 / PKCE session"]
  Coland --> Reducers["Arkret reducers + admin projections"]
  Coland --> Audit["Audit feed"]
```

## Realm vs Space

- **Realm:** security boundary — membership, capability, E2EE, and federation
  policy state is inspected here. URL prefix `/realms/:id/...`.
- **Space:** navigation container — board, list, section, calendar bucket.
  Lives inside a Realm.

The admin pages observe Realm boundary state through `/realms/:id/...`; changes
must be authored by a key-holding client. The **Realm links** page exposes typed
`ak.realm.link` edges between boundaries.

## Protocol review closures

These admin views close out protocol review findings. See
[`CHANGELOG.md`](CHANGELOG.md) `[Unreleased]` and
[`../_codmin_coland_todos.md`](../_codmin_coland_todos.md) for the canonical
wire-breaking list.

- **`ServiceDescribe` detail** — all 17 required fields rendered; the
  combination `development_mode=true` + non-empty `verified_profiles`
  paints red.
- **3PID invite 5-state UI** — `claimed` / `send_failed` /
  `revoked_by_capability_loss` / `revoked_by_inviter_left` /
  `invalidated_by_rate_limit` are all displayed honestly; `send_failed`
  is never masked as success.
- **`deactivation_federation_incomplete` banner** — account-deactivation
  pages must surface this state instead of a silent "fully-deactivated"
  rendering.

## Key-backup surface

One further operator surface — see [`CHANGELOG.md`](CHANGELOG.md)
`[Unreleased]` and [`../arkret-spec`](../arkret-spec) for the
normative source:

- **Key backup** (`/key-backup`) — recovery policies and receipts from
  the root identity recovery API.

## Development

```bash
cargo check
cargo test
dx serve --platform web
```

The UI reads `/config.json` at startup. A typical local config is:

```json
{
  "coauth_public_url": "http://localhost:7080"
}
```

## Build

```bash
cargo build --release
dx build --platform web --release
```

## Container

```bash
docker build -t codmin .
docker run -p 9090:80 \
  -e COLAND_URL=http://coland:8008 \
  -e COAUTH_URL=http://coauth:7080 \
  -e COAUTH_PUBLIC_URL=https://auth.example.com \
  codmin
```

## Deployment

The local image bundles a static Dioxus/WASM build behind nginx. At
container start, `docker-entrypoint.sh` reads the runtime environment
and emits `/usr/share/nginx/html/config.json` plus an nginx vhost
configured with sane security headers.

Required and optional environment variables:

| Variable | Required | Purpose |
| --- | --- | --- |
| `COLAND_URL` | yes | Internal URL of the coland Station reached by the proxy. |
| `COAUTH_URL` | recommended | Internal URL of the coauth admin service. Enables the `/auth/`, `/_arkret/gate/`, `/_coauth/*`, `/authorize`, `/oauth/`, and `/.well-known/` proxy locations. |
| `COAUTH_PUBLIC_URL` | recommended | Browser-facing coauth origin. Written to `/config.json` for the OAuth2 PKCE redirect. |
| `CODMIN_PORT` | no | nginx listen port (default `80`). |

The rendered nginx config enables gzip for static assets and sets a tight
default Content-Security-Policy (`default-src 'self'; script-src 'self'
'wasm-unsafe-eval'; object-src 'none'; upgrade-insecure-requests; ...`),
HSTS, `X-Frame-Options: DENY`, `Referrer-Policy: no-referrer`, and a
`Permissions-Policy` that denies camera/microphone/geolocation/payment.
If the deployment fronts additional origins (e.g. external CDN, third
party auth), edit `connect-src` in `docker-entrypoint.sh` accordingly.

`/config.json` schema:

```json
{
  "coauth_public_url": "https://auth.example.com"
}
```

## API Clients

`coauth-admin-types` is consumed directly via the workspace path dependency,
and coland DTOs remain in the typed facade until a `coland-admin-types` crate
exists. API client code must preserve Arkret error envelopes, reject URL query
credentials, propagate `X-Arkret-Request-Id`, send `Idempotency-Key` for
mutations, and redact sensitive diagnostics.

## Dashboard Discovery

The dashboard reads native Arkret discovery metadata from `/_arkret/describe` and `/_coland/admin/server/info`. Discovery-backed fields currently rendered include service DID, coauth issuer DID, delegated/public DID resolver endpoint, supported profiles, reducer/schema profiles, event-kind registry version, OpenAPI version, health summary, and conformance declarations.

If discovery is unavailable or an older backend omits a field, the UI renders `-` or `Unknown` and does not treat the profile as implemented.

## Repository Layout

```text
src/
  api/          Arkret/coauth admin API clients
  components/   Shared UI components
  pages/        Route pages
  types/        Shared response/request DTOs
  utils/        i18n, storage, config, errors and diagnostics
e2e/            Playwright smoke and stack tests
examples/       Local deployment examples, pending Arkret stack refresh
```

## Current Gaps

See the cross-project [`../_codmin_coland_todos.md`](../_codmin_coland_todos.md). Remaining deferred work includes the docs site/user guide and example-stack cold image verification.

---

<!-- circle-rollout milestone pointer -->
> **Active milestone tracking** (local-only, gitignored): see
> `_codmin_coland_todos.md` in the parent `arkret/` directory for the
> circle-rollout (AKP-0007) work item list and per-stage checkpoints.
