// Playwright config for the codmin tests/e2e/ suite. The specs talk to
// a codmin dev instance backed by a coauth + coland stack.
//
// Run from the codmin repo root:
//   npx playwright test -c tests/e2e/playwright.config.ts
//
// The specs self-skip if the required env vars are unset, so this is
// safe to invoke in a CI that hasn't yet wired the dev stack.

import { defineConfig, devices } from "@playwright/test";

const BASE_URL = process.env.CODMIN_E2E_BASE_URL || "http://localhost:8080";

export default defineConfig({
  testDir: ".",
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  workers: 1,
  reporter: [["list"]],
  timeout: 60_000,
  expect: { timeout: 10_000 },

  use: {
    baseURL: BASE_URL,
    trace: "on-first-retry",
    screenshot: "only-on-failure",
    video: "retain-on-failure",
  },

  projects: [
    {
      name: "chromium-desktop",
      use: { ...devices["Desktop Chrome"] },
    },
    // Responsive coverage: codmin's admin UI ships a mobile breakpoint
    // and an iPad layout; mobile.spec.ts asserts the breakpoint actually
    // engages on these viewports.
    {
      name: "mobile",
      use: { ...devices["Pixel 5"] },
    },
    {
      name: "tablet",
      use: { ...devices["iPad (gen 7)"], browserName: "chromium" },
    },
  ],
});
