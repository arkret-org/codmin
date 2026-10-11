# Capability preservation — authority-commit migration

codmin is the Arkret server administration UI. The authority-commit migration
removed Seal, protocol-level Cell, causal registers and the Notary from the
protocol, so every admin surface that was built on those objects had to be
re-pointed at the surface that now carries the same operational fact: the
authority-signed `RealmCommit` stream of a Realm's single current governing
Station (`sync/authority-commit-log.md`).

This table is the per-entry record required by the migration task: every admin
entry point that existed before the migration, where its capability lives now,
and which test covers it.

## Migrated

| Admin entry point (before) | Implementation now | Test |
| --- | --- | --- |
| `/realms/:realm_id/seal-chain` — "confirmed Seal head" for a Realm | `/realms/:realm_id/authority` — `src/pages/realms/authority.rs`, "Realm commit stream head" card: `stream_position`, head `RealmCommitId`, genesis Event and genesis `RealmCommit`. Data from `src/api/authority.rs` (`POST /_arkret/open/realm-authority/bundle`). | `tests/e2e/a11y.spec.ts`, `tests/e2e/keyboard-strands.spec.ts` and `tests/e2e/high-traffic-pages.spec.ts` cover the navigable admin route set; no route-specific spec existed for the Seal chain page before the migration either. |
| `/realms/:realm_id/notary` — the Realm's fixed notary signer | `/realms/:realm_id/authority`, "Current governing Station" card: `current_service_id`, `current_generation`, `last_handoff_ref`, nonce-bound assertion and its expiry. The Realm's signer is now the current governing Station service key, identified by authority generation. | as above |
| Sidebar section "Seal control" with three Realm deep links | Sidebar section "Realm authority" with one deep link (`src/components/sidebar.rs`). | as above |
| `codmin-smoke` reachability checks for `realms/{id}/notary`, `realms/{id}/seal-chain`, `realms/{id}/bottom` | `src/bin/codmin_smoke.rs` now probes `realms/{id}/links` and `realms/{id}/organizations`, which are live Realm-scoped admin reads. The capability (post-deploy admin reachability + auth probe) is unchanged. | `cargo test --bin codmin-smoke --features smoke` (unit tests in the same file) |

## Newly exposed

| Capability | Implementation | Test |
| --- | --- | --- |
| Continuous old/new double-signed authority handoff chain, genesis → current generation | `src/pages/realms/authority.rs`, `HandoffRow` table: generations, from/to Station, change `RealmCommit`, snapshot ref, and both signature verification methods. A gap or a missing acceptance signature is the operator-visible symptom of an unverifiable Realm authority. | covered by the admin route specs above |

## Removed with justification

| Admin entry point | Why it has no successor |
| --- | --- |
| `/seal/bottom` — "Data conflict diagnostics" (`src/pages/seal_bottom.rs`, `BottomEntry`, `BottomKind`) | The page rendered ⊥ (bottom) values of protocol-level Cells, i.e. conflicting joins of a causal register. Protocol-level Cells, CRDT joins and causal registers are all deleted: typed reducers now run in `RealmCommit` position order on one stream, so a later legal Event overwrites the same target rather than producing a conflicting pair. There is no residual product semantics — the page displayed a protocol object that no longer exists, and the page conferred no permission and drove no workflow. The operational hazard that replaces it, governing-Station equivocation (two differently-signed Commits at the same `(realm_id, stream_ref, authority_generation, stream_position)`), is a *consumer-side* freeze obligation; the Station codmin administers is itself the authority and cannot meaningfully report its own equivocation, and coland exposes no admin surface for it. If such a surface is added later it belongs beside the authority view, not as a revived Bottom page. |

## Unchanged product capability

Nothing else in codmin changed. Actors, Realms, Spaces, media, federation,
devices, capabilities, handles, key backup, audit, invite tokens, policy, server
status, service routes, hardening and the whole coauth admin section keep their
routes, APIs and i18n keys.

## Protocol type ownership

`src/api/authority.rs` and `src/pages/realms/authority.rs` use
`arkret_wire::{AuthorityBundleRequest, RealmAuthorityBundle, RealmAuthorityTransition}`
directly. codmin defines no protocol type of its own; `src/types/seal.rs`, which
re-exported the deleted `coland_contracts::admin::seal` DTOs, is gone.

## Wording and surface corrections

| Change | Why | Verified by |
| --- | --- | --- |
| `src/pages/dashboard.rs` no longer renders a `supported_reducer_profiles` card, and `i18n/{en,zh-CN}.json` no longer declare `dashboard.reducer_profile`. | The discovery response no longer publishes that list as a separate profile axis; the dashboard already shows the schema- and event-kind-registry versions that carry the same fact. | `cargo check --all-features`; the en/zh key sets are byte-identical and no `reducer_profile` reference remains under `src/`. |
| `i18n/{en,zh-CN}.json` say "Event" where they used to say "Control Move" / "control move". | "Control Move" was the pre-migration name for a submitted Event; the protocol has one Event submission path and no Move object. | key-set parity check; no case-insensitive `control move` hit remains in the repo. |

## SDK realignment (second pass)

The SDK regained the types the first pass reported missing, so the whole crate
now compiles and every check runs. Three call sites needed to follow the SDK's
final module and member names:

| Call site | Change | Verified by |
| --- | --- | --- |
| `src/api/coauth/viewer.rs` | `AccountView` now lives in `arkret_models_collaboration::account_operations`, not `account_lifecycle`. | `cargo check --all-features` |
| `src/pages/key_backup.rs` | `RecoveryMethod::kind()` / `as_wire_str()` collapsed into the single SDK accessor `RecoveryMethod::kind_str()`. | `cargo check --all-features`; `render_policy_row` is exercised by the page's own unit tests in the 85-test lib suite |
| `src/pages/realms/organization.rs` | `RealmOrganizationControlScope::DurabilityPolicy` no longer exists in the SDK enum and is absent from the spec's `control_scope` enum, so the match arm is gone. | `cargo check --all-features` |

## Verification

| Command | Result |
| --- | --- |
| `cargo +nightly fmt` | clean |
| `cargo check --all-features` | 0 errors, 0 warnings |
| `cargo check --all-features --all-targets` | 0 errors, 0 warnings |
| `cargo test --all-features --no-fail-fast` | 90 passed, 0 failed (85 lib + 5 `codmin-smoke`) |
| `cargo test --bin codmin-smoke --features smoke` | 5 passed, 0 failed |
| module reachability self-check | 155 `src/**/*.rs` on disk, 155 reachable from `src/main.rs`, 0 orphans |
| `npx playwright test -c tests/e2e/playwright.config.ts` | 63 specs, all self-skipped — see "External blockers" |

## External blockers

- The Playwright e2e suite needs a running coland + coauth stack; its specs
  self-skip when the stack env vars are unset. coauth's `coauth-backend` does
  not currently compile against the SDK (see that repo's
  `CAPABILITY-MIGRATION.md`), and coland must start after coauth because it
  reads its assertion key from coauth's JWKS. No spec was deleted or skipped by
  this change; all 63 remain and will run once the stack builds.
- `RealmOrganizationControlScope::NotaryControl` is still the SDK's variant
  name, while `event-payload.schema.json`'s `control_scope` enum spells the same
  scope `realm_authority`. codmin renders the SDK value verbatim and cannot fix
  the drift locally; it is reported upstream.
