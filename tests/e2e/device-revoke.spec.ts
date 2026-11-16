import { test, expect } from '@playwright/test';

const BASE_URL = process.env.CODMIN_E2E_BASE_URL || '';
const AUTH_STATE = process.env.CODMIN_E2E_AUTH_STATE || '';
const ACCOUNT_ID = process.env.CODMIN_E2E_ACCOUNT_ID || '';
const DEVICE_ID = process.env.CODMIN_E2E_DEVICE_ID || '';

test.describe('live device revocation with an existing OAuth session', () => {
  test.use({ storageState: AUTH_STATE || undefined });
  test.describe.configure({ retries: 0 });
  test.skip(!BASE_URL || !AUTH_STATE || !ACCOUNT_ID || !DEVICE_ID,
    'Set CODMIN_E2E_BASE_URL, _AUTH_STATE, _ACCOUNT_ID and _DEVICE_ID for the seeded live stack');

  test('revokes the exact device and binds its cascade count to the new audit row', async ({ page }, testInfo) => {
    test.skip(testInfo.project.name !== 'chromium-desktop', 'Run this destructive live fixture once');
    const reason = `e2e-device-revoke-${Date.now()}`;
    const devicesPath = `/_coauth/admin/accounts/${encodeURIComponent(ACCOUNT_ID)}/devices`;
    await page.goto(`${BASE_URL}/coauth/accounts/${encodeURIComponent(ACCOUNT_ID)}/devices`);
    const row = page.getByRole('row').filter({ has: page.getByText(DEVICE_ID, { exact: true }) });
    await expect(row).toHaveCount(1);
    const trigger = row.getByRole('button', { name: /^Revoke$|^撤销$/ }).and(page.locator('[aria-expanded]'));
    await expect(trigger).toBeEnabled();
    await trigger.click();
    const form = page.locator('.admin-compact-action').filter({ has: trigger }).locator('.admin-action-form');
    await expect(form).toContainText(DEVICE_ID);
    await form.getByRole('textbox', { name: /^Confirmation phrase$|^确认短语$/ }).fill([...DEVICE_ID].slice(-4).join(''));
    await form.getByRole('textbox', { name: /^Reason \/ ticket$|^原因 \/ 工单$/ }).fill(reason);
    const posted = page.waitForResponse(response => response.request().method() === 'POST'
      && new URL(response.url()).pathname === `${devicesPath}/${encodeURIComponent(DEVICE_ID)}/revoke`);
    const reloaded = page.waitForResponse(response => response.request().method() === 'GET'
      && new URL(response.url()).pathname === devicesPath);
    await form.getByRole('button', { name: /^Revoke$|^撤销$/ }).click();
    const response = await posted;
    expect(response.status()).toBe(200);
    const outcome = await response.json();
    expect(outcome.device.id).toBe(DEVICE_ID);
    expect(outcome.device.account_id).toBe(ACCOUNT_ID);
    expect(outcome.revoked_session_grants).toBeGreaterThan(0);
    expect((await reloaded).status()).toBe(200);
    await expect(row).toHaveCount(1);
    await expect(row.getByRole('cell').nth(5)).toHaveText(new Date(outcome.device.revoked_at).toISOString());
    await expect(row.getByRole('button', { name: /^Revoke$|^撤销$/ })).toHaveCount(0);

    await page.goto(`${BASE_URL}/coauth/audit-log`);
    const audit = page.getByRole('row').filter({ hasText: reason });
    await expect(audit).toHaveCount(1);
    await expect(audit.getByRole('cell').nth(1)).toHaveText('device.revoke');
    const details = JSON.parse(await audit.locator('.audit-details').innerText());
    expect(details.account_id).toBe(ACCOUNT_ID);
    expect(details.device_id).toBe(DEVICE_ID);
    expect(details.reason).toBe(reason);
    expect(details.revoked_session_grants).toBe(outcome.revoked_session_grants);
    expect(details.revoked_at).toBe(outcome.device.revoked_at);
  });
});
