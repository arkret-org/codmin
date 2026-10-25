import AxeBuilder from '@axe-core/playwright';
import { expect, test, Page } from '@playwright/test';

const accountId = '01JZ9PK6HKFY0MM7C0TMZ1X8N7';
const deviceId = 'device-1234';
const timestamp = '2026-10-25T00:00:00.000Z';
const principal = 'ak:did_core:web:operator.example';
const device = { id: deviceId, account_id: accountId, display_name: 'Operator laptop', risk_level: 'low', mfa_state: 'verified', registered_at: timestamp, revoked_at: null as string | null };

async function setup(page: Page, admin = true) {
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.addInitScript(() => {
    if (sessionStorage.getItem('glory_fixture_seeded')) return;
    sessionStorage.setItem('glory_fixture_seeded', '1');
    for (const [key, value] of Object.entries({ session_active: '1', is_admin: 'true', language: 'en', user_id: 'operator', coauth_public_url: location.origin })) {
      localStorage.setItem(key, JSON.stringify(value));
    }
  });
  await page.route('**/*', async route => {
    const path = new URL(route.request().url()).pathname;
    if (path === '/config.json') return route.fulfill({ json: { coauth_public_url: new URL(route.request().url()).origin, oauth_client_id: '01GFWR28C4KNE04WG3HKXB7C9R' } });
    if (path === '/_coland/admin/viewer') return route.fulfill({ json: { principal_id: principal, state: 'active', is_server_admin: admin } });
    if (path === '/_coland/admin/server/stats') return route.fulfill({ json: { actor_count: 7, active_actor_count: 6, realm_count: 2, device_count: 3, report_count: 0, federation_peer_count: 1, applet_count: 0, blob_count: 4, blob_total_size: 1024, generated_at: timestamp } });
    if (path === '/_coland/admin/server/info') return route.fulfill({ json: { server_version: 'glory-fixture', protocol_version: '1.0', server_name: 'Migration fixture', uptime: null, service_id: 'ak:did_core:web:station.example', trust_domain: 'station.example', development_mode: false, allow_public_registration: false } });
    if (path === '/_coland/admin/server/status') return route.fulfill({ json: { status: 'ok', service_id: 'ak:did_core:web:station.example', storage: 'postgres', development_mode: false, checked_by: principal, generated_at: timestamp, counts: { accounts: 7, devices: 3, realms: 2 } } });
    if (path.startsWith('/_')) return route.fulfill({ status: 503, json: { type: 'about:blank', title: 'Fixture endpoint unavailable', status: 503, code: 'service_unavailable', detail: 'Fixture endpoint unavailable' } });
    return route.continue();
  });
  return errors;
}

function account(id: string, name: string) {
  return { type: 'accounts', id, attributes: { handle: name.toLowerCase(), status: 'active', display_name: name }, links: { self: `/_coauth/admin/accounts/${id}` } };
}
function accounts(data: unknown[], next: string | null = null) {
  return { data, links: { self: '/_coauth/admin/accounts', next }, meta: { count: 2 } };
}
async function mockAccounts(page: Page) {
  const requests: URL[] = [];
  await page.route('**/_coauth/admin/accounts?*', async route => {
    const url = new URL(route.request().url()); requests.push(url);
    const filtered = !!url.searchParams.get('filter[search]');
    const second = !!url.searchParams.get('page[after]');
    return route.fulfill({ json: accounts(
      [filtered ? account(accountId, 'Filtered operator') : second ? account('second-account', 'Second operator') : account(accountId, 'First operator')],
      !filtered && !second ? '/_coauth/admin/accounts?page%5Bafter%5D=next-page' : null,
    ) });
  });
  return requests;
}

test('login loads deployment configuration, exposes a recoverable error, and retries', async ({ page }) => {
  const errors = await setup(page);
  let loads = 0;
  await page.route('**/config.json', route => route.fulfill({ json: ++loads === 1 ? {} : { coauth_public_url: new URL(route.request().url()).origin, oauth_client_id: '01GFWR28C4KNE04WG3HKXB7C9R' } }));
  await page.goto('/login');
  await expect(page.getByRole('status')).toContainText('coauth_public_url');
  await expect(page.getByRole('button', { name: 'Sign In', exact: true })).toHaveCount(0);
  await page.getByRole('button', { name: 'Retry', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Sign In', exact: true })).toBeEnabled();
  expect(loads).toBe(2);
  expect(errors).toEqual([]);
});

test('a server rejection blocks the admin shell even if a cached marker says admin', async ({ page }) => {
  const errors = await setup(page, false);
  await page.goto('/coauth/accounts');
  await expect(page.getByRole('heading', { name: 'This account is not a server administrator' })).toBeVisible();
  await expect(page.getByRole('navigation')).toHaveCount(0);
  await expect(page.getByRole('button', { name: /sign out/i })).toBeEnabled();
  expect(errors).toEqual([]);
});

test('the Glory shell renders typed server data and opens another admin route', async ({ page }) => {
  const errors = await setup(page);
  await mockAccounts(page);
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Dashboard', exact: true })).toBeVisible();
  await expect(page.getByText('glory-fixture', { exact: true })).toBeVisible();
  await page.locator('a[href="/coauth/accounts"]').click();
  await expect(page).toHaveURL(/\/coauth\/accounts$/);
  await expect(page.getByRole('heading', { name: 'coauth Accounts', exact: true })).toBeVisible();
  await expect(page.getByText('First operator', { exact: true }).first()).toBeVisible();
  expect(errors).toEqual([]);
});

test('account search resets the cursor and pagination keeps the selected filter', async ({ page }) => {
  const errors = await setup(page);
  const requests = await mockAccounts(page);
  await page.goto('/coauth/accounts');
  await expect(page.getByText('First operator', { exact: true }).first()).toBeVisible();
  await page.getByRole('button', { name: 'Next', exact: true }).click();
  await expect(page.getByText('Second operator', { exact: true }).first()).toBeVisible();
  expect(requests.at(-1)!.searchParams.get('page[after]')).toBe('next-page');
  await page.getByRole('textbox', { name: 'Search (any field)', exact: true }).fill('operator');
  await page.getByRole('button', { name: 'Search (any field)', exact: true }).click();
  await expect(page.getByText('Filtered operator', { exact: true }).first()).toBeVisible();
  expect(requests.at(-1)!.searchParams.get('filter[search]')).toBe('operator');
  expect(requests.at(-1)!.searchParams.has('page[after]')).toBe(false);
  await expect(page.getByRole('button', { name: 'Next', exact: true })).toBeDisabled();
  await expect(page.getByRole('button', { name: 'Previous', exact: true })).toBeDisabled();
  expect(errors).toEqual([]);
});

test('device revocation requires the phrase and a safe audit reason, prevents duplicate requests and supports retry', async ({ page }) => {
  const errors = await setup(page);
  let revoked = false;
  let mutations = 0;
  let completeFirst: (() => void) | undefined;
  const firstRequest = new Promise<void>(resolve => { completeFirst = resolve; });
  const received: unknown[] = [];
  await page.route(`**/_coauth/admin/accounts/${accountId}/devices`, route => route.fulfill({ json: { data: [{ ...device, revoked_at: revoked ? timestamp : null }] } }));
  await page.route(`**/_coauth/admin/accounts/${accountId}/devices/${deviceId}/revoke`, async route => {
    mutations++; received.push(route.request().postDataJSON());
    expect(route.request().headers()['idempotency-key']).toBeTruthy();
    if (mutations === 1) {
      await firstRequest;
      return route.fulfill({ status: 503, json: { type: 'about:blank', title: 'Retry device revocation', status: 503, code: 'service_unavailable', detail: 'Retry device revocation' } });
    }
    revoked = true;
    return route.fulfill({ json: { device: { ...device, revoked_at: timestamp }, revoked_session_grants: 2 } });
  });
  try {
    await page.goto(`/coauth/accounts/${accountId}/devices`);
    const submit = page.getByRole('button', { name: 'Revoke', exact: true });
    await expect(submit).toBeDisabled();
    await page.getByRole('textbox', { name: 'Confirmation phrase', exact: true }).fill('wrong');
    await page.getByRole('textbox', { name: 'Reason / ticket', exact: true }).fill('SEC-1234 recovery');
    await expect(submit).toBeDisabled();
    await page.getByRole('textbox', { name: 'Confirmation phrase', exact: true }).fill('1234');
    await page.getByRole('textbox', { name: 'Reason / ticket', exact: true }).fill('password=secret');
    await expect(page.getByText('The reason appears to contain credential material. Remove secrets before continuing.')).toBeVisible();
    await expect(submit).toBeDisabled();
    expect(mutations).toBe(0);
    await page.getByRole('textbox', { name: 'Reason / ticket', exact: true }).fill('SEC-1234 recovery');
    await submit.click();
    await expect.poll(() => mutations).toBe(1);
    await expect(submit).toBeDisabled();
    await submit.dispatchEvent('click');
    expect(mutations).toBe(1);
    completeFirst!();
    await expect(page.getByText(/Retry device revocation/).last()).toBeVisible();
    await expect(page.getByRole('textbox', { name: 'Confirmation phrase', exact: true })).toHaveValue('1234');
    await expect(page.getByRole('textbox', { name: 'Reason / ticket', exact: true })).toHaveValue('SEC-1234 recovery');
    await expect(submit).toBeEnabled();
    await submit.click();
    await expect.poll(() => mutations).toBe(2);
    await expect(submit).toHaveCount(0);
    expect(received).toEqual([{ reason: 'SEC-1234 recovery' }, { reason: 'SEC-1234 recovery' }]);
    expect(errors).toEqual([]);
  } finally { completeFirst!(); }
});

test('the mobile menu opens the sidebar and follows an admin link', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  const errors = await setup(page);
  await mockAccounts(page);
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Dashboard', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Menu', exact: true }).click();
  await expect(page.locator('.sapid-sidebar-drawer')).toBeVisible();
  await page.locator('.sapid-sidebar-drawer a[href="/coauth/accounts"]').click();
  await expect(page.getByRole('heading', { name: 'coauth Accounts', exact: true })).toBeVisible();
  expect(errors).toEqual([]);
});

test('failed cookie logout keeps the session and a retry can finish sign-out', async ({ page }) => {
  const errors = await setup(page);
  await mockAccounts(page);
  let attempts = 0;
  await page.route('**/_arkret/gate/account/logout', route => route.fulfill({ json: { ok: true, revoked: true } }));
  await page.route('**/oauth/revoke', route => route.fulfill({ status: 200, body: '' }));
  await page.route('**/_coauth/account/auth/logout', route => route.fulfill({ json: { status: ++attempts === 1 ? 'unconfirmed' : 'success' } }));
  await page.goto('/coauth/accounts');
  await expect(page.getByRole('heading', { name: 'coauth Accounts', exact: true })).toBeVisible();
  await page.getByRole('button', { name: /sign out/i }).click();
  await expect(page.getByText(/Sign-out was not confirmed and your local session marker was kept/)).toBeVisible();
  expect(await page.evaluate(() => localStorage.getItem('session_active'))).toBe(JSON.stringify('1'));
  await expect(page).toHaveURL(/\/coauth\/accounts$/);
  await page.getByRole('button', { name: /sign out/i }).click();
  await expect(page).toHaveURL(/\/login$/);
  expect(attempts).toBe(2);
  expect(await page.evaluate(() => localStorage.getItem('session_active'))).toBeNull();
  expect(errors).toEqual([]);
});

test('a pending token regeneration survives list refresh and a remounted row cannot regenerate twice', async ({ page }) => {
  const errors = await setup(page);
  const sessionId = 'session-1234';
  const token = 'fixture-one-shot-token';
  const row = {
    type: 'personal-session', id: sessionId,
    attributes: { created_at: timestamp, revoked_at: null, owner_user_id: 'operator', owner_client_id: null, actor_user_id: 'operator', human_name: 'Automation key', scope: 'admin', last_active_at: null, last_active_ip: null, expires_at: null },
    links: { self: `/_coauth/admin/personal-sessions/${sessionId}` },
  };
  let reads = 0;
  let regenerations = 0;
  let complete: (() => void) | undefined;
  const pending = new Promise<void>(resolve => { complete = resolve; });
  await page.route('**/_coauth/admin/personal-sessions?*', route => {
    reads++;
    return route.fulfill({ json: { data: [row], links: { self: '/_coauth/admin/personal-sessions' }, meta: { count: 1 } } });
  });
  await page.route(`**/_coauth/admin/personal-sessions/${sessionId}/regenerate`, async route => {
    regenerations++;
    await pending;
    return route.fulfill({ json: { data: { ...row, attributes: { ...row.attributes, access_token: token } }, links: { self: `/_coauth/admin/personal-sessions/${sessionId}` } } });
  });
  try {
    await page.goto('/coauth/personal-sessions');
    const regenerate = page.getByRole('button', { name: 'Regenerate', exact: true });
    const form = page.locator('.admin-action-form').filter({ has: regenerate });
    await expect(regenerate).toBeDisabled();
    await form.getByRole('textbox', { name: 'Confirmation phrase', exact: true }).fill('1234');
    await regenerate.click();
    await expect.poll(() => regenerations).toBe(1);
    await page.getByRole('button', { name: 'Refresh', exact: true }).click();
    await expect.poll(() => reads).toBe(2);
    await expect(regenerate).toBeVisible();
    await form.getByRole('textbox', { name: 'Confirmation phrase', exact: true }).fill('1234');
    await expect(regenerate).toBeDisabled();
    await regenerate.dispatchEvent('click');
    expect(regenerations).toBe(1);
    complete!();
    await expect(page.locator('.one-shot-token')).toHaveText(token);
    await expect.poll(() => reads).toBe(3);
    await page.getByRole('button', { name: 'Refresh', exact: true }).click();
    await expect.poll(() => reads).toBe(4);
    await expect(page.locator('.one-shot-token')).toHaveText(token);
    await page.getByRole('button', { name: 'Close', exact: true }).click();
    await expect(page.locator('.one-shot-token')).toHaveCount(0);
    expect(regenerations).toBe(1);
    expect(errors).toEqual([]);
  } finally { complete!(); }
});


test('all administrative routes mount their real Glory widgets and show recoverable API errors', async ({ page }) => {
  test.setTimeout(120_000);
  const errors = await setup(page);
  const routes = [
    '/actors', '/actors/actor-1', '/realms', '/realms/realm-1', '/realms/realm-1/authority',
    '/realms/realm-1/links', '/realms/realm-1/media-service', '/realms/realm-1/organization',
    '/spaces', '/spaces/space-1', '/media', '/federation', '/federation/operation-1',
    '/devices', '/capabilities', '/handles', '/handles/handle-1', '/key-backup', '/audit',
    '/invite-tokens', '/policy', '/server-status', '/service-routes', '/service-routes/service-1/station', '/hardening',
    '/coauth/audit-log', '/coauth/accounts', '/coauth/accounts/account-1', '/coauth/accounts/account-1/devices',
    '/coauth/capabilities', '/coauth/oauth2-sessions', '/coauth/personal-sessions',
    '/coauth/upstream-providers', '/coauth/upstream-links', '/coauth/registration-tokens',
    '/coauth/notification-channels', '/coauth/notification-templates', '/coauth/connector-health',
  ];
  for (const route of routes) {
    await page.goto(route);
    await expect(page.getByRole('navigation')).toBeVisible();
    await expect(page.getByRole('main')).toBeVisible();
    await expect(page.getByRole('main')).not.toBeEmpty();
    expect(errors, route).toEqual([]);
  }
});


test('the public sign-in surface passes serious accessibility checks and fits a mobile viewport', async ({ page }) => {
  const errors = await setup(page);
  await page.goto('/login');
  await expect(page.getByRole('button', { name: 'Sign In', exact: true })).toBeEnabled();
  const audit = await new AxeBuilder({ page }).analyze();
  expect(audit.violations.filter(item => ['critical', 'serious'].includes(item.impact || ''))).toEqual([]);
  await page.screenshot({ path: test.info().outputPath('login-desktop.png') });
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(page.getByRole('button', { name: 'Sign In', exact: true })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1)).toBe(true);
  await page.screenshot({ path: test.info().outputPath('login-mobile.png') });
  expect(errors).toEqual([]);
});


test('keyboard navigation ignores form editing and confirmed logout in another tab updates this tab', async ({ page, context }) => {
  const errors = await setup(page);
  await mockAccounts(page);
  await page.goto('/coauth/accounts');
  await expect(page.getByRole('heading', { name: 'coauth Accounts', exact: true })).toBeVisible();
  await page.keyboard.press('?');
  await expect(page.getByRole('heading', { name: 'Keyboard Shortcuts', exact: true })).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('heading', { name: 'Keyboard Shortcuts', exact: true })).toHaveCount(0);
  await page.keyboard.press('g');
  await page.keyboard.press('d');
  await expect(page).toHaveURL(/\/$/);
  await expect(page.getByRole('heading', { name: 'Dashboard', exact: true })).toBeVisible();
  await page.locator('a[href="/coauth/accounts"]').click();
  await page.getByRole('textbox', { name: 'Search (any field)', exact: true }).pressSequentially('gd?');
  await expect(page).toHaveURL(/\/coauth\/accounts$/);
  await expect(page.getByRole('heading', { name: 'Keyboard Shortcuts', exact: true })).toHaveCount(0);
  const second = await context.newPage();
  try {
    await second.goto('/login');
    await expect(second.getByRole('button', { name: 'Sign In', exact: true })).toBeEnabled();
    await second.evaluate(() => localStorage.removeItem('session_active'));
    await expect(page).toHaveURL(/\/login$/);
    await expect(page.getByRole('button', { name: 'Sign In', exact: true })).toBeEnabled();
  } finally { await second.close(); }
  expect(errors).toEqual([]);
});
