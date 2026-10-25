# Glory and Sapid frontend

Codmin's browser application uses the local Glory semantic UI and Sapid widgets:

- Glory: `../../glory-rs/glory/crates/glory`.
- Sapid: `../../glory-rs/sapid`.

The application is a WASM library with a small static loading host in `web/`.
`python scripts/build-web.py --release` runs Cargo and the matching
wasm-bindgen 0.2.129 CLI, then stages the browser bundle in Cargo's reported
`target_directory/web`. Windows defaults and tool checks keep build outputs,
Cargo caches, compiler temporaries and tool binaries on D:.

The existing typed API clients, protocol-owner DTOs, cookies, PKCE checks and
browser settings keys remain in use. Station and Coauth management routes,
filters, cursor pagination, numerical page navigation, CSV export, automatic
refresh, relationship views and destructive operations use semantic widgets.
Global navigation, keyboard chords, cross-tab logout and development warnings
are owned by the mounted application. Theme and language switches reload the
page; complete in-progress forms before switching.

Read resources cancel superseded work and ignore stale replies. Mutations in
refreshed Coauth rows belong to the page and share a keyed busy state. Refresh
and pagination therefore cannot lose a pending token-regeneration response or
start a second operation on the same object. Page retirement still cancels
outstanding work. Confirmation phrases and validated audit reasons gate
sensitive actions, and failures retain the input needed for a retry.

## Validation

- 81 native unit tests pass, including resource cancellation, owner retirement,
  mutation lifetime, large audit cells and same-tick pagination guards.
- Native and WASM application Clippy pass with warnings denied. Two existing
  unused-helper warnings from the separately owned glory-web dependency remain.
- The release WASM bundle builds and loads in Microsoft Edge.
- All 11 migration browser tests pass, including mounting every administration
  route, typed HTTP fixtures, denied access, filtering, pagination, destructive
  actions, token regeneration during refresh, logout, keyboard navigation,
  cross-tab synchronization, mobile navigation and public-page accessibility.
- YAML, Python, JavaScript, shell syntax and translation-key parity checks pass.

Run the browser checks with `npm run test:migration`; set
`CODMIN_E2E_CHANNEL=msedge` or `chrome` to reuse an installed browser.
The suite exercises the actual WASM/DOM against mocked HTTP contracts. It does
not establish live OAuth/provider behavior or backend authorization. Docker
builds and a real service-stack integration run were not performed.

## CI and deployment

CI prepares the same relative dependency layout through
`.ci/checkout-ui-dependencies.sh`. Fresh checkouts pin Glory
`b660dea9fc182270d2eb13bbf00a08b4dab1a49b` and Sapid
`fd64d78edf9f91f9dc63bfa2be6e59ec1f739a58`. Existing local checkouts are preserved.
Local validation used the current D:/Works/glory-rs sources; source fingerprints
and logs are under D:/BuildCache/codmin-glory/logs. Local Glory contains subsequent
rendering work, so local acceptance is distinct from a fresh CI execution.

Docker uses Glory/Sapid named build contexts and the same build script.
The nginx runtime configuration and reverse-proxy contract remain in place.
Release packaging consumes the static web directory and includes dependency
commit provenance and checksums. No frontend framework CLI or git UI dependency
is required.
