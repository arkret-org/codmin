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

## Current UI checks

`glory-migration.spec.ts` owns route mounting, public and authenticated axe
checks, RTL layout, the development-mode banner, and keyboard navigation to
an actual device-revoke button. These checks drive the current WASM UI with
contract-shaped mocked HTTP data; they require no retired local password form.
Run `npm run test:migration`, or select `test:a11y`, `test:keyboard`, or `test:rtl`.
They do not establish live cross-service revocation or approval behavior.

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

## Glory migration checks

Build with `python scripts/build-web.py --release`, serve the reported web directory,
and run `npm run test:migration`. This separate suite drives the real WASM app
with mocked HTTP contracts. It covers authorization, forms, cursors, token
regeneration during refresh, mobile navigation, sign-out, every admin route
and the public accessibility surface. It does not require seeded backend data
and does not prove live OAuth or backend authorization. Artifacts are saved under artifacts/glory-e2e by default; CODMIN_E2E_OUTPUT_DIR selects an explicit destination.
Set `CODMIN_E2E_CHANNEL=msedge` or `chrome` to reuse an installed browser.


### 当前WASM行为与设计回归

`npm run test:migration`使用同一个canonical配置，收集`glory-migration.spec.ts`和`frontend-design.spec.ts`，当前分别16和9个case；每个case只在该入口执行一次。可用`npm run test:migration -- frontend-design`单独选择设计场景。CI在fresh WASM artifact上运行全集，a11y、keyboard和RTL已在其中，不再重复运行同一子集。

通用`npm test`仍是多项目广域入口并可收集设计spec；需要已配置的live业务规格时再选择它，不应把它与migration全集连续运行并计作两份独立覆盖。mock设计回归验证输入类型、草稿、取消零写、错误处理和响应归属，不能代替真实后台授权/审计验收。
