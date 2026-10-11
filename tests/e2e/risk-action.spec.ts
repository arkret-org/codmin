// Happy-path risk-action approval Playwright e2e.
//
// Drives the codmin SPA against a codmin dev instance + a coland dev
// stack. The test is intentionally narrow: it boots the SPA, signs the
// admin in, navigates to the accounts list, opens an account detail
// page, kicks off the risk-action strand, and asserts that an audit row
// appears for the action.
//
// Skip behaviour: if the env vars `CODMIN_E2E_BASE_URL`,
// `CODMIN_E2E_ADMIN_EMAIL`, `CODMIN_E2E_ADMIN_PASSWORD`, or
// `CODMIN_E2E_ACCOUNT_ID` are unset, the test calls `test.skip()` so a
// CI environment that hasn't wired the dev stack still passes. The
// expectation is that an operator runs the local dev stack manually
// (or via `npm run stack:up`), exports the env vars, then invokes
// `npx playwright test -c tests/e2e/playwright.config.ts`.

import { test, expect } from "@playwright/test";

const BASE_URL = process.env.CODMIN_E2E_BASE_URL || "";
const ADMIN_EMAIL = process.env.CODMIN_E2E_ADMIN_EMAIL || "";
const ADMIN_PASSWORD = process.env.CODMIN_E2E_ADMIN_PASSWORD || "";
const ACCOUNT_ID = process.env.CODMIN_E2E_ACCOUNT_ID || "";

test.describe("risk-action happy-path", () => {
  test.beforeAll(async () => {
    if (!BASE_URL || !ADMIN_EMAIL || !ADMIN_PASSWORD || !ACCOUNT_ID) {
      test.skip(
        true,
        "CODMIN_E2E_BASE_URL / _ADMIN_EMAIL / _ADMIN_PASSWORD / _ACCOUNT_ID must be set",
      );
    }
  });

  test("approves a risk-action and surfaces an audit entry", async ({ page }) => {
    const auditNonce = Date.now();
    const riskReason = `e2e-risk-action-reason-${auditNonce}`;
    const riskTicket = `e2e-risk-action-ticket-${auditNonce}`;

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
    await expect(page.getByRole("heading", { name: /accounts/i })).toBeVisible();

    // 5. Open the target account detail page
    await page.goto(`${BASE_URL}/coauth/accounts/${ACCOUNT_ID}`);
    await expect(page.getByText(ACCOUNT_ID)).toBeVisible({ timeout: 10_000 });

    // 6. Trigger a risk-action proposal (approve strand). The button copy
    //    is "Approve risk action" / 批准风控操作 — we match on either.
    const approveBtn = page.getByRole("button", {
      name: /approve risk action|批准风控操作/i,
    });
    await expect(approveBtn).toBeVisible({ timeout: 10_000 });
    await page.getByLabel(/^reason$/i).fill(riskReason);
    await page.getByLabel(/^ticket$/i).fill(riskTicket);
    await approveBtn.click();

    // 7. Confirm the destructive ConfirmDialog
    const confirmBtn = page.getByRole("button", { name: /^confirm$|批准|approve$/i });
    await expect(confirmBtn).toBeVisible({ timeout: 5_000 });
    await confirmBtn.click();

    // 8. Toast surfaces the mutation result. An unwired backend endpoint
    //    is a real failure for this manual high-risk-action regression spec.
    await expect(
      page.getByText(/risk action recorded|approved/i),
    ).toBeVisible({ timeout: 10_000 });

    // 9. Navigate to the audit log and expect the entry bound to this
    //    operation's unique ticket, not a historical risk_action row.
    await page.goto(`${BASE_URL}/coauth/audit-log`);
    await expect(page.getByText(riskTicket).first()).toBeVisible({
      timeout: 15_000,
    });
    await expect(page.getByText(riskReason).first()).toBeVisible({
      timeout: 15_000,
    });
  });
});
