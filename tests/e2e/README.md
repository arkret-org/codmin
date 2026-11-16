# Codmin Playwright checks

## Current WASM behavior and design checks

Build the current source with `python scripts/build-web.py --release`, serve the reported output directory, and run `npm run test:migration`. Set `CODMIN_E2E_BASE_URL` to that server and `CODMIN_E2E_CHANNEL=msedge` or `chrome` to reuse an installed browser. `CODMIN_E2E_OUTPUT_DIR` selects the artifact directory.

The canonical migration configuration collects `glory-migration.spec.ts` (16 cases) and `frontend-design.spec.ts` (11 cases). It covers current typed HTTP projections, form guards, failure handling, navigation, sign-out, accessibility, keyboard and RTL behavior. CI runs the full entrypoint on its freshly built WASM artifact; `test:a11y`, `test:keyboard` and `test:rtl` select existing subsets and should not be counted as independent extra coverage.

The design suite now owns the former pin-policy privacy/read-only obligation and the current risk-approval boundary: opaque pin resources must not reach the DOM or gain mutation controls, and the approval button remains disabled without the required admin signing capability. These use contract-shaped HTTP fixtures, not live authorization or audit evidence.

`npm test` is the broader multi-project entrypoint and also collects the design spec. Avoid running it after the migration suite merely to count the same assertions again. `npm run test:smoke` selects the unauthenticated responsive spec.

## Live device revoke and cascade audit

`device-revoke.spec.ts` retains a separate live obligation: a real admin cookie session revokes the exact seeded device, the response reports at least one revoked SessionGrant, the reloaded device row loses its revoke control, and a newly created device audit row carries the same account, device, reason, timestamp and cascade count. It does not mock any HTTP response.

Prepare a running Codmin + Coauth + Coland deployment and a separate target account with a revocable device and active SessionGrants. Complete the real OAuth flow, then export that browser context with Playwright `context.storageState({ path })`. The state must include the actual HttpOnly cookies and the application localStorage from that login. The test does not synthesize a bearer token or bypass login by inventing a session marker.

Set these environment variables:

- `CODMIN_E2E_BASE_URL`: the deployed Codmin origin.
- `CODMIN_E2E_AUTH_STATE`: the exported Playwright state JSON, kept in an ignored artifact directory.
- `CODMIN_E2E_ACCOUNT_ID`: the seeded target account ULID.
- `CODMIN_E2E_DEVICE_ID`: its revocable device identifier.

Run the destructive fixture once, without retries:

```sh
npx playwright test -c tests/e2e/playwright.config.ts device-revoke.spec.ts --project chromium-desktop --workers 1 --retries 0
```

Missing qualification is reported as an explicit skip. This live check is not part of the mock migration gate or the current static-SPA CI stack. Session state contains credentials and must remain outside Git. On Windows, call the workspace D-drive cache helper in the same PowerShell before validation; keep browser artifacts, TEMP and dependency caches on D.

## Retired local-login specs and remaining live scope

Codmin has no local email/password form. The old `CODMIN_E2E_ADMIN_EMAIL` and `_ADMIN_PASSWORD` contract and the obsolete pin/risk spec files have been removed. They are not a supported alternative login path.

The browser still cannot create the detached Ed25519 approval JWS, so end-to-end browser approval followed by a live audit remains an unimplemented capability, not a passing test. Coauth's existing `test_risk_action_execute_requires_approval_and_locks_account` and `test_risk_action_approve_rejects_forged_approved_by_threshold_bypass` cover the real signed server workflow at their own HTTP/PG boundary; `native_admin_approval_binds_current_rotated_root_without_document_vm` checks native signing evidence. Their presence does not establish that this live browser path was run.

Likewise the pin summary surface remains unavailable: read-only/privacy projection checks do not prove live pin-policy administration. The follow-up receipt records the old-to-current obligation mapping and the exact validation performed.
