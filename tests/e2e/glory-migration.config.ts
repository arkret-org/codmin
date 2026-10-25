import { defineConfig } from '@playwright/test';

const output = process.env.CODMIN_E2E_OUTPUT_DIR || (process.platform === 'win32'
  ? 'D:/BuildCache/codmin-glory/e2e'
  : `${process.env.RUNNER_TEMP || process.env.TMPDIR || '/tmp'}/codmin-glory-e2e`);

export default defineConfig({
  testDir: '.',
  testMatch: 'glory-migration.spec.ts',
  fullyParallel: false,
  workers: 1,
  retries: 0,
  timeout: 45_000,
  expect: { timeout: 12_000 },
  outputDir: `${output}/results`,
  reporter: [['list'], ['json', { outputFile: `${output}/results.json` }]],
  use: {
    baseURL: process.env.CODMIN_E2E_BASE_URL || 'http://127.0.0.1:4173',
    browserName: 'chromium',
    channel: process.env.CODMIN_E2E_CHANNEL || undefined,
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
    viewport: { width: 1365, height: 1000 },
  },
});
