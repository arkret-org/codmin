import { test, expect, type Page } from "@playwright/test";

const BASE_URL = process.env.CODMIN_E2E_BASE_URL || "";
const ADMIN_EMAIL = process.env.CODMIN_E2E_ADMIN_EMAIL || "";
const ADMIN_PASSWORD = process.env.CODMIN_E2E_ADMIN_PASSWORD || "";

async function login(page: Page) {
  await page.goto(`${BASE_URL}/login`);
  await page.getByLabel(/email/i).fill(ADMIN_EMAIL);
  await page.getByLabel(/password/i).fill(ADMIN_PASSWORD);
  await page.getByRole("button", { name: /sign in|log in|login/i }).click();
  await expect(page.locator("nav")).toBeVisible({ timeout: 15_000 });
}

const pageSmokes = [
  { name: "dashboard", path: "/", heading: /dashboard|仪表盘/i },
  { name: "actors", path: "/actors", heading: /actors|用户|角色/i },
  { name: "spaces", path: "/spaces", heading: /spaces|realm|空间/i },
  { name: "federation", path: "/federation", heading: /federation|联邦/i },
  { name: "audit log", path: "/audit", heading: /audit|审计/i },
  { name: "policy", path: "/policy", heading: /policy|策略/i },
  { name: "server status", path: "/server-status", heading: /server|status|服务器|状态/i },
];

test.describe("high-traffic page smoke", () => {
  test.beforeAll(async () => {
    if (!BASE_URL || !ADMIN_EMAIL || !ADMIN_PASSWORD) {
      test.skip(
        true,
        "CODMIN_E2E_BASE_URL / _ADMIN_EMAIL / _ADMIN_PASSWORD must be set",
      );
    }
  });

  for (const smoke of pageSmokes) {
    test(`${smoke.name} renders authenticated shell`, async ({ page }) => {
      await login(page);
      await page.goto(`${BASE_URL}${smoke.path}`);
      await expect(page.locator("nav")).toBeVisible({ timeout: 10_000 });
      await expect(page.locator("main")).toBeVisible({ timeout: 10_000 });
      await expect(
        page.getByRole("heading", { name: smoke.heading }).first(),
      ).toBeVisible({ timeout: 10_000 });
    });
  }
});
