import { expect, test, type Page } from "@playwright/test";

const BASE_URL = process.env.CODMIN_E2E_BASE_URL || "";
const ADMIN_EMAIL = process.env.CODMIN_E2E_ADMIN_EMAIL || "";
const ADMIN_PASSWORD = process.env.CODMIN_E2E_ADMIN_PASSWORD || "";

async function gotoReady(page: Page, route: string) {
  await page.goto(`${BASE_URL}${route}`, { waitUntil: "domcontentloaded" });
  await expect(page.getByRole("main").first()).toBeVisible({
    timeout: 15_000,
  });
}

async function login(page: Page) {
  await gotoReady(page, "/login");
  await page.getByLabel(/email/i).fill(ADMIN_EMAIL);
  await page.getByLabel(/password/i).fill(ADMIN_PASSWORD);
  await page.getByRole("button", { name: /sign in|log in|login/i }).click();
  await expect(page.locator("nav")).toBeVisible({ timeout: 15_000 });
}

async function assertNoHorizontalOverflow(page: Page) {
  const overflow = await page.evaluate(() => {
    const root = document.documentElement;
    return Math.max(0, root.scrollWidth - root.clientWidth);
  });
  expect(overflow).toBeLessThanOrEqual(1);
}

test.describe("RTL layout smoke", () => {
  test.beforeAll(async () => {
    if (!BASE_URL) {
      test.skip(true, "CODMIN_E2E_BASE_URL must be set");
    }
  });

  test("login shell does not overflow in RTL", async ({ page }) => {
    await gotoReady(page, "/login");
    await expect(page.getByRole("button", { name: /sign in/i })).toBeEnabled();
    await page.evaluate(() => {
      document.documentElement.setAttribute("dir", "rtl");
      document.documentElement.setAttribute("lang", "ar");
    });
    await assertNoHorizontalOverflow(page);
  });

  test("authenticated dashboard does not overflow in RTL", async ({ page }) => {
    if (!ADMIN_EMAIL || !ADMIN_PASSWORD) {
      test.skip(true, "admin credentials are required for authenticated RTL smoke");
    }
    await login(page);
    await page.evaluate(() => {
      document.documentElement.setAttribute("dir", "rtl");
      document.documentElement.setAttribute("lang", "ar");
    });
    await assertNoHorizontalOverflow(page);
  });
});
