# Changelog

All notable changes to `codmin` (Arkret Server admin UI) are documented in
this file. Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
this project is pre-1.0 so wire-breaking changes can land in any release until
the SDK pins down its 1.0 contract.

## Unreleased — authority-commit admin surface

- Replaced the Realm Seal-chain, Notary and Bottom-diagnostics pages with one
  Realm authority view at `/realms/:realm_id/authority`. It renders the
  nonce-bound `RealmAuthorityBundle` from the governing Station: current Station
  and authority generation, the Realm commit stream head, genesis, and the
  continuous old/new double-signed handoff chain.
- The admin UI no longer reads Seal, Notary or causal-register conflict
  surfaces. Shared Realm finality is the authority-signed `RealmCommit` of the
  Realm's single current governing Station; Realm, Circle and Sidecar streams
  advance independently with no global position.

## Unreleased — SessionGrant issuer-ledger sync

- Reclassified SessionGrant create, refresh, revoke, and regeneration as
  Account Authority issuer-ledger operations. SessionGrant lifecycle is no
  longer described as a service-attested Arkret Event.
- Clarified that device revoke atomically terminalizes linked grants in
  coauth; Stations observe that terminal state through fresh
  introspection rather than maintaining a second grant lifecycle truth.
- Preserved the admin key boundary: codmin neither drafts grant Events nor
  obtains principal, device, recovery, or service signing keys.

## Spec sync 2026-05-31 (arkret-spec @ c2848a4)

- Synced protocol-facing names and fixtures to `c2848a4`: event envelope schema naming, `_ids` grant constraints, accountability principal vocabulary, `ak:rtc_participant:` media participants, agent session start fields, and key-backup signature algorithm naming where applicable.

> No version tag, no crates.io / Docker Hub / npm publish — git commit only.

## Spec sync 2026-05-28 (arkret-spec @ cced4b8)

- Pinned to arkret-spec @ cced4b8 (AKP-0011). The `resolve_target` operator diagnostic page is deferred and not yet implemented.

> No version tag, no crates.io / Docker Hub / npm publish — git commit only.
## Spec sync 2026-05-28 (arkret-spec @ b56cab1)

- Renamed roster `identity_state_digest` → `member_display_state_digest`; added roster v2 gated fields + `HandleClaim`/`HandleBindingState` mirrors (`claim_kind` drops `service_handle`).
- New `utils/primary_handle.rs` mirrors SDK §3.2.1 selection; actor/identity views derive the handle via selection (MemberIdentity handle fields removed).
- New `/handles/by-subject` page calling `list_handles_for_subject` with a "Why am I seeing this?" tooltip; "handle changed since" hint on the identity-audit page.
- Claim-set join + accepted_issuers policy + DID-doc holder preference are deferred and not yet implemented.

> No version tag, no crates.io / Docker Hub / npm publish — git commit only.
## Spec sync 2026-05-27 (arkret-spec @ b47ff6ec)

- Agent list status badges (`active` / `paused` / `deactivated`) with pause / resume / deactivate / rotate-key / grants actions wired to `/agents/{id}/deactivate`; draft / action_request / approve / reject lifecycle stubbed.
- Realm settings page `/realms/:realm_id/media-service` for editing `media_service.foci[]` (livekit / mediasoup / janus / arkret_native / moq_relay).
- Key-backup recovery policy and receipt data is rendered through `/key-backup`; handle homograph inline NFC + script-mixed warning on `/actors/create`.
- Server profile chip surface at `/server-status` showing declared / absent state for `accountable_principals.strict_reject.v1`, `media_service_binding.v1` (+ livekit / arkret_native), and `key_backup.memory_hard.v1`.
- Localized en + zh-CN strings for the new errcodes (`pairing_request_expired`, `proof_invalid`, `agent_paused`, `agent_deactivated`, `accountability_grant_missing`, `handle_homograph_forbidden`, `participant_binding_invalid`) rendered via `ErrorBanner`.

> No version tag, no crates.io / Docker Hub / npm publish — git commit only.

## [Unreleased]

### Admin write boundary

- Removed Realm destroy, Realm invite-token mutation, Arkret device revoke,
  Notary reconfiguration, Seal compaction, and multisig partial-signature
  authoring from the SPA. Their protocol projections remain available as
  read-only operational and audit views.
- Documented the three allowed surfaces: deployment-local writes,
  service-attested account/session actions, and read-only principal/notary-key
  state. codmin continues to hold no Arkret device or notary key.

### AKP-0007 Circle rollout

UI surfaces for the encrypted-sub-boundary primitive shipped by
coland and coauth.

- **Added** Circle management UI: `/circles`, `/circles/new`,
  `/circles/:id`, `/circles/:id/members`, `/circles/:id/scope` Dioxus
  pages plus a `src/api/circles.rs` fetch wrapper. Eight
  `/_coland/self/circles/*` routes added to `build.rs` REQUIRED_COLAND so a
  missing route fails the wasm build at contract-check time.
- **Added** Capability grant dialog gains a quick-select for the
  six `ak.circle.*` actions and an `allowed_circle_ids` CSV input
  that emits the GrantConstraint server-side.
- **Added** Audit log renders the new `effective_scope` field
  with a deep link into `/circles/:id` for Circle-scoped events, and
  the filter row grows an event-kind dropdown covering the seven
  `ak.circle.*` event kinds.
- **Added** Realm classification badges: new
  `RealmClassificationBadge` component renders the Principal
  Control / Collaboration / unknown distinction. The Realm/Space
  create form now requires the immutable classification at create
  time (defaults to `collaboration`).
- **Added** AKP-0007 reason-code i18n: six reducer
  rejection reasons (`circle_realm_mismatch`,
  `circle_member_must_be_realm_member`, `circle_not_active`,
  `circle_already_terminal`, `circle_capability_denied`,
  `circle_scope_rotation_in_progress`) now render with localised
  toast bodies.
- **Added** Playwright e2e `tests/e2e/circle-admin.spec.ts` covering
  create / valid member / subset-rejection / archive / audit fanout
  and a capability grant happy path.
- **Fixed** `.gitignore` tightening — explicit pins for
  `/bundle.js.map`, `/docker-build.log`, and `**/dist/` so root-level
  build artifacts cannot regress past the wildcard rules.
- **Notes** Version unchanged; ships against the existing
  `circle-rollout` branch only. Lighthouse perf budget raised from
  70 → 80 to track industry baseline.

### Protocol review closures (2026-05-20, arkret-spec `2a4d39b..a77b995`)

UI surfaces for the wire changes implemented in coland / coauth.
See [`../_codmin_coland_todos.md`](../_codmin_coland_todos.md) for the workstream context.

- **Added** `ServiceDescribe` v2 admin view — all 17 required fields
  rendered; `development_mode=true` paired with non-empty
  `verified_profiles` raises a red warning row.
- **Added** 3PID invite admin: the 5-terminal-state machine
  (`claimed` / `send_failed` / `revoked_by_capability_loss` /
  `revoked_by_inviter_left` / `invalidated_by_rate_limit`) is fully visible
  and `send_failed` must render honestly (no fake-success states).
- **Added** account-deactivation panel now shows the
  `deactivation_federation_incomplete` banner state (federation fanout
  cannot complete without a peer ack); silent "fully-deactivated"
  rendering removed.

### Added — deactivation and Realm-destroy surfaces (arkret-spec, 2026-05-20)

- **Deactivation 7-domain fanout panel** — new
  `components::deactivation_fanout_panel` renders the local-PS fanout result
  for `ak.self.agent.deactivate` across the seven cascade domains (`session`,
  `device`, `applet`, `keypackage`, `push`, `to_device`, `capability`). Failed
  rows render red with a per-domain Retry button. Wired into a new
  `/deactivations/review` page and reused on the realm destroy page below.
- **Realm destroy confirmation dialog** — new
  `components::realm_destroy_dialog` gates `ak.realm.destroy` behind five
  explicit normative checkboxes (no further ordinary writes, snapshots /
  backfill / GC will run, no successor Realm, erasure receipt + legal hold
  precedence, 30-day federation fanout window) plus a typed-`DESTROY`
  confirmation. Mounted at `/realms/{id}/destroy`, which reuses the fanout
  panel + an erasure-receipt block (local-PS only; the cross-PS erasure
  receipt is not implemented).
