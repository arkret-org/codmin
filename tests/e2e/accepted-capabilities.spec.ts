import { test, expect } from '@playwright/test';
import { readFileSync } from 'node:fs';

const fixturePath = process.env.R08_ACCEPTED_BROWSER_FIXTURE;
if (!fixturePath) throw new Error('R08_ACCEPTED_BROWSER_FIXTURE must name the actual PG/HTTP output');
const fixture = JSON.parse(readFileSync(fixturePath, 'utf8'));
if (fixture.capabilities.length !== 5) throw new Error('Non-empty five-row PG/HTTP fixture is required');

async function prepare(page: import('@playwright/test').Page, deny = false) {
  const requests: URL[] = [];
  let legacyCalls = 0;
  await page.addInitScript(() => {
    localStorage.setItem('session_active', JSON.stringify('1'));
    localStorage.setItem('is_admin', JSON.stringify('true'));
    localStorage.setItem('language', JSON.stringify('en'));
  });
  await page.route('**/_coauth/admin/collaboration/capabilities*', async route => {
    legacyCalls++;
    await route.fulfill({status: 500, body: 'retired mirror must never be called'});
  });
  await page.route('**/_coland/admin/viewer', route => route.fulfill({
    json: { principal_id: 'ak:did_core:web:operator.example', state: 'active', is_server_admin: true }
  }));
  await page.route('**/_coland/admin/capabilities*', async route => {
    const url = new URL(route.request().url()); requests.push(url);
    if (deny) {
      await route.fulfill({status: 403, json: {
        type: 'https://arkret.org/problems/capability_denied', title: 'Denied', status: 403,
        detail: 'Station operator permission denied', code: 'capability_denied'
      }}); return;
    }
    let rows = [...fixture.capabilities];
    const actor = url.searchParams.get('filter[subject_actor_id]');
    if (actor) {
      const target = JSON.parse(actor);
      rows = rows.filter(row => row.value.subject.kind === target.kind &&
        (target.kind === 'account'
          ? row.value.subject.account_id.principal_id === target.account_id.principal_id &&
            row.value.subject.account_id.station_id === target.account_id.station_id
          : row.value.subject.service_id === target.service_id));
    }
    const realm = url.searchParams.get('filter[realm_id]');
    if (realm) rows = rows.filter(row => row.source_stream_ref.realm_id === realm);
    const state = url.searchParams.get('filter[state]');
    if (state && state !== 'all') rows = rows.filter(row => row.value.status === state);
    const total = rows.length;
    const cursor = url.searchParams.get('cursor');
    if (cursor) rows = rows.slice(rows.findIndex(row => row.selector.grant_id === cursor) + 1);
    const hasMore = rows.length > 2;
    rows = rows.slice(0, 2);
    await route.fulfill({json: {capabilities: rows, total, has_more: hasMore,
      next_cursor: hasMore ? rows.at(-1).selector.grant_id : null, filters: Object.fromEntries([...url.searchParams].filter(([key]) => key.startsWith("filter[")).map(([key,value]) => [key.slice(7,-1),value]))}});
  });
  return {requests, legacyCalls: () => legacyCalls};
}

for (const path of ['/capabilities', '/coauth/capabilities']) {
  test(`${path}: accepted current rows, cursor and complete account filtering`, async ({page}) => {
    const state = await prepare(page);
    await page.goto(path);
    await expect(page.locator('[data-testid="accepted-capability-filters"]')).toBeVisible();
    await expect(page.locator('table tbody tr')).toHaveCount(2);
    const first = await page.locator('table tbody tr').first().textContent();
    const next = page.getByRole('button', {name: /next|下一页|下一步/i});
    await next.click();
    await expect.poll(() => state.requests.some(url => url.searchParams.has('cursor'))).toBe(true);
    await expect(page.locator('table tbody tr').first()).not.toHaveText(first || '');
    const account = fixture.capabilities.find(row => row.value.subject.kind === 'account').value.subject;
    await page.locator('#capability-actor').fill(JSON.stringify(account));
    await page.getByRole('button', {name: /apply filters|应用筛选/i}).click();
    await expect.poll(() => state.requests.some(url => !!url.searchParams.get('filter[subject_actor_id]'))).toBe(true);
    const applied = state.requests.at(-1)!;
    expect(JSON.parse(applied.searchParams.get('filter[subject_actor_id]')!)).toEqual(account);
    expect(applied.searchParams.has('cursor')).toBe(false);
    await expect(page.locator('table tbody')).toContainText(account.account_id.station_id);
    const count = state.requests.length;
    await page.locator('#capability-actor').fill('ak:did_core:web:holder.example');
    await page.getByRole('button', {name: /apply filters|应用筛选/i}).click();
    await expect(page.getByText(/complete Account or Service ActorId/i)).toBeVisible();
    expect(state.requests.length).toBe(count);
    expect(state.legacyCalls()).toBe(0);
  });
}

test('operator denial stays visible and never falls back to Coauth grants', async ({page}) => {
  const state = await prepare(page, true);
  await page.goto('/coauth/capabilities');
  await expect(page.getByText(/operator permission denied|capability denied/i)).toBeVisible();
  await expect(page.locator('table')).toHaveCount(0);
  expect(state.legacyCalls()).toBe(0);
});

test('Service, condition and terminal rows remain distinct; an empty Realm never falls back', async ({page}) => {
  const state = await prepare(page);
  await page.goto('/coauth/capabilities');
  await expect(page.locator('table tbody tr')).toHaveCount(2);
  const apply = page.getByRole('button', {name: /apply filters|应用筛选/i});
  await page.locator('#capability-state').selectOption('relinquished');
  await apply.click();
  await expect(page.locator('table tbody tr')).toHaveCount(1);
  await expect(page.locator('table tbody')).toContainText('Relinquished');
  const service = fixture.capabilities.find(row => row.value.subject.kind === 'service');
  await page.locator('#capability-actor').fill(JSON.stringify(service.value.subject));
  await page.locator('#capability-realm').fill(service.source_stream_ref.realm_id);
  await page.locator('#capability-state').selectOption('revoked');
  await apply.click();
  await expect(page.locator('table tbody')).toContainText(service.value.subject.service_id);
  await expect(page.locator('table tbody')).toContainText('Revoked');
  expect(JSON.parse(state.requests.at(-1)!.searchParams.get('filter[subject_actor_id]')!))
    .toEqual(service.value.subject);
  await page.locator('#capability-actor').fill('');
  await page.locator('#capability-state').selectOption('active');
  await apply.click();
  await expect(page.locator('table tbody tr')).toHaveCount(2);
  await page.getByRole('button', {name: /next|下一页|下一步/i}).click();
  await expect(page.locator('table tbody')).toContainText('required_claims');
  await expect(page.locator('table tbody')).toContainText('membership');
  const realm = service.source_stream_ref.realm_id;
  const emptyRealm = realm.slice(0, -4) + 'AAAA';
  await page.locator('#capability-realm').fill(emptyRealm);
  await apply.click();
  await expect.poll(() => state.requests.at(-1)?.searchParams.get('filter[realm_id]')).toBe(emptyRealm);
  await expect(page.locator('table tbody')).toContainText(/no capabilities/i);
  expect(state.requests.at(-1)!.searchParams.has('cursor')).toBe(false);
  expect(state.legacyCalls()).toBe(0);
});
