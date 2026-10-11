// Responsive smoke — verifies codmin's login surface renders without
// horizontal overflow on the mobile and tablet projects defined in
// playwright.config.ts.
//
// The check is intentionally minimal:
//   1. Hit the login page (no auth needed, no admin stack required).
//   2. Wait for the configured OAuth sign-in control to become ready.
//   3. Assert the document does not scroll horizontally — i.e. the
//      mobile / tablet breakpoint actually engaged and nothing
//      overflowed the viewport.
//
// This catches the most common responsive regression (a `min-width` or
// `width: 100vw + N` rule sneaking in via a new component) without
// needing a live coland stack.
//
// Skip behaviour: if `CODMIN_E2E_BASE_URL` is unset, the spec is
// skipped so CI environments that haven't wired a codmin instance
// still pass. Run locally with:
//
//   CODMIN_E2E_BASE_URL=http://localhost:8080 \
//     npx playwright test -c tests/e2e/playwright.config.ts mobile
//
// Runs across the chromium-desktop, mobile (Pixel 5), and tablet
// (iPad) projects — desktop is included as a baseline so a regression
// that breaks all three viewports surfaces clearly.

import { test, expect } from "@playwright/test";

const BASE_URL = process.env.CODMIN_E2E_BASE_URL || "";

test.describe("responsive smoke", () => {
  test.beforeAll(async () => {
    if (!BASE_URL) {
      test.skip(true, "CODMIN_E2E_BASE_URL must be set");
    }
  });

  test("login page fits the viewport without horizontal overflow", async ({
    page,
  }) => {
    await page.goto(`${BASE_URL}/login`);

    const signIn = page.getByRole("button", { name: /sign in/i });
    await expect(signIn).toBeVisible({ timeout: 15_000 });
    await expect(signIn).toBeEnabled();

    // No horizontal scrollbar — scrollWidth must fit within clientWidth.
    // We compare on documentElement (the <html> root) because that's
    // where the browser surfaces overflow caused by any descendant.
    const overflow = await page.evaluate(() => ({
      scrollWidth: document.documentElement.scrollWidth,
      clientWidth: document.documentElement.clientWidth,
    }));
    expect(overflow.scrollWidth).toBeLessThanOrEqual(overflow.clientWidth + 1);
  });
});
