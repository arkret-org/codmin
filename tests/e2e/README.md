# tests/e2e — Playwright suite

This directory holds the Playwright e2e tests for codmin.

Automated CI runs only `npm run test:smoke`, which is limited to the
unauthenticated responsive smoke that needs `CODMIN_E2E_BASE_URL` only.
The authenticated high-risk management specs in this directory are manual
or seeded-stack checks: they require admin credentials plus account, device,
or policy ids and are intentionally not reported as automated CI coverage.

## Happy-path risk-action

`risk-action.spec.ts` drives the full risk-action approval strand:

1. Login at `/login`
2. Navigate to `/coauth/accounts`
3. Open an account detail page
4. Click "Approve risk action"
5. Confirm the ConfirmDialog
6. Assert the audit log surfaces a `risk_action` entry

## Running locally

```bash
# 1. Boot a codmin dev instance + coland + coauth (manual, today)
#    — typically: `npm run stack:up` in the codmin repo root.

# 2. Export env vars pointing at the dev instance
export CODMIN_E2E_BASE_URL=http://localhost:8080
export CODMIN_E2E_ADMIN_EMAIL=admin@example.com
export CODMIN_E2E_ADMIN_PASSWORD=changeme
export CODMIN_E2E_ACCOUNT_ID=01HXY...

# 3. From the codmin repo root, install playwright + browsers if needed
npm install
npx playwright install chromium

# 4. Run the whole suite
npx playwright test -c tests/e2e/playwright.config.ts
```

If any of the four env vars is missing, the spec self-skips for local
operator convenience. CI does not invoke this manual spec unless a workflow
explicitly provisions those variables and seeded backend data.

## Skip behaviour summary

| Condition                                         | Outcome |
|---------------------------------------------------|---------|
| `CODMIN_E2E_BASE_URL` unset                       | skip    |
| `CODMIN_E2E_ADMIN_EMAIL` / `_PASSWORD` unset      | skip    |
| `CODMIN_E2E_ACCOUNT_ID` unset                     | skip    |
| All env vars set + dev stack running              | run     |

## High-traffic page smoke

`high-traffic-pages.spec.ts` adds authenticated route-level smokes for
the server-ops surfaces (dashboard, actors, spaces, federation,
deactivation, audit, policy, server status). Governance pages
(moderation/applets/agents/directory/circles) were removed from codmin
in the ops-level consolidation and now live in inkson, so they are no
longer smoked here. These tests use the shared `CODMIN_E2E_BASE_URL`,
`CODMIN_E2E_ADMIN_EMAIL`, and `CODMIN_E2E_ADMIN_PASSWORD` env vars and
self-skip when the local stack is not configured.

## Happy-path device revoke + cascade audit

`device-revoke.spec.ts` revokes a device on a target account, asserts
the row badge transitions to Revoked, and cross-checks that the
coauth audit feed shows both the device-revoke entry AND a
session-grant cascade entry (proving the issuer-ledger lifecycle mutation
landed on the canonical coauth audit pipeline):

1. Login at `/login`
2. Navigate to `/coauth/accounts`
3. Open the target account detail page
4. Jump to `/coauth/accounts/:id/devices`
5. Click "Revoke" on the target row
6. Confirm the destructive ConfirmDialog
7. Assert the row badge transitions to "Revoked"
8. Navigate to `/coauth/audit-log` and assert both a device-revoke row
   AND a session-grant cascade row are visible

Required env vars (in addition to the shared trio):

- `CODMIN_E2E_ACCOUNT_ID` — the target account
- `CODMIN_E2E_DEVICE_ID` — a revocable (Active or Stale) device on
  that account

If unset, the spec self-skips.
