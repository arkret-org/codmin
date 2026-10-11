// Happy-path device revoke + cascade-revoke Playwright e2e.
//
// Drives the codmin SPA against a codmin dev instance + a coauth + coland
// dev stack. The test exercises the per-account device admin page:
// boot the SPA, sign the admin in, navigate to the accounts list, open
// an account detail page, jump to the per-account device list, click
// Revoke on a target device, confirm the destructive ConfirmDialog,
// assert the device row badge transitions to Revoked, and assert the
// coauth audit feed surfaces the cascade-revoke session-grant entries
// emitted by the coland side.
//
// Pattern matches the `risk-action.spec.ts` shape — identical
// env-var contract, identical "self-skip when env vars unset" behaviour.
//
// Skip behaviour: if any of `CODMIN_E2E_BASE_URL`,
// `CODMIN_E2E_ADMIN_EMAIL`, `CODMIN_E2E_ADMIN_PASSWORD`,
// `CODMIN_E2E_ACCOUNT_ID`, or `CODMIN_E2E_DEVICE_ID` are unset, the
// test calls `test.skip()` so a CI environment that hasn't wired the
// dev stack still passes. The expectation is that an operator boots a
// local dev stack manually (or via `npm run stack:up`), seeds a
// revocable device on a target account, exports the env vars, then
// invokes `npx playwright test -c tests/e2e/playwright.config.ts`.

import { test, expect } from "@playwright/test";

const BASE_URL = process.env.CODMIN_E2E_BASE_URL || "";
const ADMIN_EMAIL = process.env.CODMIN_E2E_ADMIN_EMAIL || "";
const ADMIN_PASSWORD = process.env.CODMIN_E2E_ADMIN_PASSWORD || "";
const ACCOUNT_ID = process.env.CODMIN_E2E_ACCOUNT_ID || "";
const DEVICE_ID = process.env.CODMIN_E2E_DEVICE_ID || "";

test.describe("device-revoke happy-path", () => {
  test.beforeAll(async () => {
    if (
      !BASE_URL ||
      !ADMIN_EMAIL ||
      !ADMIN_PASSWORD ||
      !ACCOUNT_ID ||
      !DEVICE_ID
    ) {
      test.skip(
        true,
        "CODMIN_E2E_BASE_URL / _ADMIN_EMAIL / _ADMIN_PASSWORD / _ACCOUNT_ID / _DEVICE_ID must be set",
      );
    }
  });

  test("revokes a device, surfaces Revoked badge + cascade audit entries", async ({
    page,
  }) => {
    const revokeReason = `e2e-device-revoke-${Date.now()}`;

    // 1. Navigate to login
    await page.goto(`${BASE_URL}/login`);
    await expect(page).toHaveURL(/\/login/);

    // 2. Sign the admin in
    await page.getByLabel(/email/i).fill(ADMIN_EMAIL);
    await page.getByLabel(/password/i).fill(ADMIN_PASSWORD);
    await page.getByRole("button", { name: /sign in|log in|login/i }).click();

    // 3. Wait until the dashboard / sidebar mounts
    await expect(page.locator("nav")).toBeVisible({ timeout: 15_000 });

    // 4. Navigate to the accounts list
    await page.goto(`${BASE_URL}/coauth/accounts`);
    await expect(
      page.getByRole("heading", { name: /accounts|账号/i }),
    ).toBeVisible();

    // 5. Open the account detail page
    await page.goto(`${BASE_URL}/coauth/accounts/${ACCOUNT_ID}`);
    await expect(page.getByText(ACCOUNT_ID)).toBeVisible({ timeout: 10_000 });

    // 6. Jump to the per-account device list.
    await page.goto(`${BASE_URL}/coauth/accounts/${ACCOUNT_ID}/devices`);
    await expect(
      page.getByRole("heading", { name: /devices|设备/i }),
    ).toBeVisible({ timeout: 10_000 });

    // 7. Confirm the target device row exists.
    const deviceCell = page.getByText(DEVICE_ID).first();
    await expect(deviceCell).toBeVisible({ timeout: 10_000 });

    // 8. Click Revoke on the row. The button copy is "Revoke" / 撤销.
    //    Multiple Revoke buttons may exist for other rows; the spec
    //    expects exactly one revocable target device row to be set up
    //    by the operator, so we click the first match.
    const revokeBtn = page.getByRole("button", { name: /^revoke$|^撤销$/i }).first();
    await expect(revokeBtn).toBeVisible({ timeout: 10_000 });
    await revokeBtn.click();

    // 9. Confirm the destructive ConfirmDialog. The dialog confirm copy
    //    is also "Revoke" / 撤销; we use `.last()` to disambiguate from
    //    the row trigger button.
    const confirmBtn = page
      .getByRole("button", { name: /^revoke$|^撤销$|^confirm$|^确认$/i })
      .last();
    await expect(confirmBtn).toBeVisible({ timeout: 5_000 });
    await page
      .getByLabel(/reason|ticket|原因|工单/i)
      .fill(revokeReason);
    await confirmBtn.click();

    // 10. Toast surfaces the mutation result. An unwired backend endpoint
    //     is a real failure for this manual high-risk-action regression spec.
    await expect(
      page.getByText(/device revoked|cascade-revoke/i),
    ).toBeVisible({ timeout: 10_000 });

    // 11. After the action, the row's status badge should transition
    //     to Revoked / 已撤销. We re-resolve the row and assert it
    //     contains the Revoked label. The data-restart in the page
    //     re-fetches automatically.
    await expect(
      page.getByText(/^revoked$|^已撤销$/i).first(),
    ).toBeVisible({ timeout: 15_000 });

    // 12. Cross-check the coauth audit feed surfaces a device-revoke
    //     entry AND at least one session-grant cascade entry — proves
    //     the coauth issuer-ledger lifecycle mutation landed on the
    //     canonical audit pipeline, not just the local UI mutation.
    await page.goto(`${BASE_URL}/coauth/audit-log`);
    await expect(page.getByText(revokeReason).first()).toBeVisible({
      timeout: 15_000,
    });
    await expect(
      page.getByText(/device.*revoke|revoke.*device/i).first(),
    ).toBeVisible({ timeout: 15_000 });
    await expect(
      page.getByText(/session.*revoke|revoke.*session|session_grant/i).first(),
    ).toBeVisible({ timeout: 15_000 });
  });
});
