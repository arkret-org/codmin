// T1.4 — dev-mode banner smoke.
//
// Verifies that codmin renders the red "DEVELOPMENT MODE" banner when the
// connected coland reports `development_mode == true` on its
// `/_arkret/describe` surface. The banner is the operator's
// safety signal that proof verification is relaxed and admin endpoints
// accept any authenticated session — losing it is a regression we want
// loud feedback on.
//
// Skip behaviour: if the env vars `CODMIN_E2E_BASE_URL`,
// `CODMIN_E2E_ADMIN_EMAIL`, and `CODMIN_E2E_ADMIN_PASSWORD` are unset,
// the test calls `test.skip()` so CI environments that haven't wired a
// dev stack still pass. Run locally with:
//
//   CODMIN_E2E_BASE_URL=http://localhost:8080 \
//   CODMIN_E2E_ADMIN_EMAIL=admin@example.com \
//   CODMIN_E2E_ADMIN_PASSWORD=changeme \
//     npx playwright test -c tests/e2e/playwright.config.ts dev-mode-banner
//
// The local dev stack should be booted with
// `COLAND_DEVELOPMENT_MODE=true` (the default for `cargo run` in the
// coland repo) for the banner to render. Against a production coland
// the banner is intentionally absent and this spec will fail — that's
// the assertion we want.

import { test, expect } from "@playwright/test";

const BASE_URL = process.env.CODMIN_E2E_BASE_URL || "";
const ADMIN_EMAIL = process.env.CODMIN_E2E_ADMIN_EMAIL || "";
const ADMIN_PASSWORD = process.env.CODMIN_E2E_ADMIN_PASSWORD || "";

test.describe("T1.4 dev-mode banner", () => {
  test.beforeAll(async () => {
    if (!BASE_URL || !ADMIN_EMAIL || !ADMIN_PASSWORD) {
      test.skip(
        true,
        "CODMIN_E2E_BASE_URL / _ADMIN_EMAIL / _ADMIN_PASSWORD must be set",
      );
    }
  });

  test("renders the red DEVELOPMENT MODE banner against a dev coland", async ({
    page,
  }) => {
    // 1. Sign the admin in.
    await page.goto(`${BASE_URL}/login`);
    await page.getByLabel(/email/i).fill(ADMIN_EMAIL);
    await page.getByLabel(/password/i).fill(ADMIN_PASSWORD);
    await page.getByRole("button", { name: /sign in|log in|login/i }).click();

    // 2. Wait for the layout to mount.
    await expect(page.locator("nav")).toBeVisible({ timeout: 15_000 });

    // 3. The banner is a role="alert" strip with the literal phrase
    //    "DEVELOPMENT MODE" (or the zh-CN translation). The locale-
    //    independent assertion is on the alert role + the substring.
    const banner = page.getByRole("alert").filter({
      hasText: /DEVELOPMENT MODE|开发模式/,
    });
    await expect(banner.first()).toBeVisible({ timeout: 10_000 });
  });
});
