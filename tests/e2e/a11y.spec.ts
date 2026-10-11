import { expect, test, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

const BASE_URL = process.env.CODMIN_E2E_BASE_URL || "";
const ADMIN_EMAIL = process.env.CODMIN_E2E_ADMIN_EMAIL || "";
const ADMIN_PASSWORD = process.env.CODMIN_E2E_ADMIN_PASSWORD || "";
const HAS_ADMIN_CREDENTIALS = Boolean(ADMIN_EMAIL && ADMIN_PASSWORD);

async function login(page: Page) {
  await gotoReady(page, "/login");
  await page.getByLabel(/email/i).fill(ADMIN_EMAIL);
  await page.getByLabel(/password/i).fill(ADMIN_PASSWORD);
  await page.getByRole("button", { name: /sign in|log in|login/i }).click();
  await expect(page.locator("main")).toBeVisible({ timeout: 15_000 });
}

async function gotoReady(page: Page, route: string) {
  const appShell = page.locator("main, [role=main], form").first();
  for (let attempt = 0; attempt < 8; attempt += 1) {
    await page.goto(`${BASE_URL}${route}`, { waitUntil: "domcontentloaded" });
    try {
      await expect(appShell).toBeVisible({ timeout: 15_000 });
      return;
    } catch (error) {
      const rebuilding = await page
        .getByText("Your app is being rebuilt")
        .isVisible()
        .catch(() => false);
      if (!rebuilding || attempt === 7) {
        throw error;
      }
      await page.waitForTimeout(5_000);
    }
  }
}

const authenticatedRoutes = [
  "/",
  "/devices",
  "/federation",
  "/audit",
  "/policy",
  "/server-status",
];
const publicRoutes = ["/login"];
const routes = HAS_ADMIN_CREDENTIALS ? authenticatedRoutes : publicRoutes;

test.describe("axe accessibility audit", () => {
  test.beforeAll(async () => {
    if (!BASE_URL) {
      test.skip(true, "CODMIN_E2E_BASE_URL must be set");
    }
  });

  for (const route of routes) {
    test(`${route} has no critical axe violations`, async ({ page }) => {
      if (HAS_ADMIN_CREDENTIALS) {
        await login(page);
      }
      await gotoReady(page, route);
      if (!HAS_ADMIN_CREDENTIALS) {
        await expect(page.getByRole("button", { name: /sign in/i })).toBeEnabled();
      }

      // `color-contrast` is re-enabled as of Phase 5 — the dark-mode
      // `--muted-foreground` token was bumped to ~78% lightness so it
      // clears WCAG AA against the `--background: 270 22% 10%` token.
      const results = await new AxeBuilder({ page }).analyze();
      const critical = results.violations.filter((violation) =>
        ["critical", "serious"].includes(violation.impact || ""),
      );

      expect(critical).toEqual([]);
    });
  }
});
