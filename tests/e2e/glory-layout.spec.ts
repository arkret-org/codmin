import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { layoutIds, setupLayout } from './layout-fixtures';

async function assertViewport(page:any) {
  const widths=await page.evaluate(()=>({document:document.documentElement.scrollWidth,viewport:document.documentElement.clientWidth}));
  expect.soft(widths.document, page.url()+" document width").toBeLessThanOrEqual(widths.viewport+1);
  const main=page.getByRole('main');
  if(await main.count()){const metrics=await main.evaluate((el:any)=>({scroll:el.scrollWidth,width:el.clientWidth}));expect.soft(metrics.scroll,page.url()+" main width").toBeLessThanOrEqual(metrics.width+1);}
}

async function assertCardLayouts(page:any) {
  const name=page.locator('.account-display-name');
  if(await name.count()){
    const description=name.locator('..').locator('.text-muted').first();
    const titleBox=await name.boundingBox();
    const descriptionBox=await description.boundingBox();
    expect.soft(descriptionBox!.y,'account description follows its title').toBeGreaterThanOrEqual(titleBox!.y+titleBox!.height);
  }
  for(const header of await page.locator('.sapid-card-header:has(> .sapid-badge)').all()){
    const title=await header.locator(':scope > .sapid-card-title').boundingBox();
    const badge=await header.locator(':scope > .sapid-badge').boundingBox();
    expect.soft(badge!.x,'status badge is beside its title').toBeGreaterThanOrEqual(title!.x+title!.width);
  }
  for(const row of await page.locator('.status-card-body').all()){
    const copy=await row.locator('.status-card-copy').boundingBox();
    const badge=await row.locator('.status-badge').boundingBox();
    expect.soft(badge!.x,'notification status is beside its description').toBeGreaterThanOrEqual(copy!.x+copy!.width);
  }
}

test('login matches the previous centered brand and card layout on desktop and mobile', async ({page})=>{
  await setupLayout(page);
  await page.goto('/login');
  await expect(page.getByRole('button',{name:'Sign In',exact:true})).toBeEnabled();
  const content=await page.locator('.auth-content').boundingBox();
  expect(content!.width).toBeCloseTo(448,0);
  expect(content!.x+content!.width/2).toBeCloseTo(1365/2,0);
  expect(await page.getByRole('heading',{name:'Arkret Admin',exact:true}).evaluate((el:any)=>getComputedStyle(el).fontSize)).toBe('24px');
  await assertViewport(page);
  const audit=await new AxeBuilder({page}).analyze();
  expect(audit.violations.filter(v=>['critical','serious'].includes(v.impact||''))).toEqual([]);
  await page.screenshot({path:test.info().outputPath('login-desktop.png'),fullPage:true});
  await page.setViewportSize({width:390,height:844});
  await assertViewport(page);
  await page.screenshot({path:test.info().outputPath('login-mobile.png'),fullPage:true});
});

test('dashboard restores six metrics, grouped navigation, compact header and responsive cards', async({page})=>{
  const errors=await setupLayout(page);
  await page.goto('/');
  await expect(page.locator('.stat-cell')).toHaveCount(6);
  await expect(page.getByRole('heading',{name:'Dashboard',exact:true})).toBeVisible();
  await expect(page.locator('.loading,.resource-error')).toHaveCount(0);
  await expect(page.locator('.stat-value').first()).toHaveCSS('font-size','24px');
  const sidebar=await page.locator('.sapid-sidebar-dock').boundingBox();
  const main=await page.getByRole('main').boundingBox();
  const header=await page.getByRole('banner').boundingBox();
  expect(sidebar!.width).toBe(256);expect(main!.x).toBe(256);expect(header!.height).toBe(56);
  expect(main!.y).toBe(56);
  const cells=await page.locator('.stat-cell').evaluateAll(nodes=>nodes.map(node=>({top:node.getBoundingClientRect().top,width:node.getBoundingClientRect().width})));
  expect(new Set(cells.map(c=>c.top)).size).toBe(1);
  await expect(page.locator('.sidebar-section-label')).toHaveCount(6);
  await expect(page.locator('.server-summary-grid .metadata-value').first()).toHaveCSS('font-size','18px');
  const conformance=await page.locator('.conformance-grid > .metadata-cell').evaluateAll(nodes=>nodes.map(n=>n.getBoundingClientRect().top));
  expect(conformance).toHaveLength(5);
  expect(conformance[0]).toBe(conformance[1]);
  expect(new Set(conformance.slice(2)).size).toBe(1);
  expect(conformance[2]).toBeGreaterThan(conformance[0]);
  await assertViewport(page);
  await page.screenshot({path:test.info().outputPath('dashboard-desktop.png'),fullPage:true});
  await page.getByRole('button',{name:'Collapse navigation',exact:true}).click();
  await expect.poll(async()=> (await page.getByRole('main').boundingBox())!.x).toBe(64);
  await page.screenshot({path:test.info().outputPath('dashboard-collapsed.png')});
  await page.setViewportSize({width:390,height:844});
  await expect.poll(async()=> (await page.getByRole('main').boundingBox())!.x).toBe(0);
  await assertViewport(page);
  const mobileCells=await page.locator('.stat-cell').evaluateAll(nodes=>nodes.map(node=>node.getBoundingClientRect().top));
  expect(new Set(mobileCells).size).toBe(3);
  await page.screenshot({path:test.info().outputPath('dashboard-mobile.png')});
  await page.getByRole('button',{name:'Open navigation',exact:true}).click();
  await expect(page.locator('.sapid-sidebar-drawer')).toBeVisible();
  await page.screenshot({path:test.info().outputPath('navigation-mobile.png')});
  expect(errors).toEqual([]);
});

const pages=[
  ['/actors','Actors'],
  [`/actors/${encodeURIComponent(layoutIds.actor)}`,'Active actor detail'],
  [`/actors/${encodeURIComponent(layoutIds.secondActor)}`,'Locked actor detail'],
  ['/devices','Devices'],['/realms','Realms'],
  [`/realms/${encodeURIComponent(layoutIds.realm)}`,'Encrypted Realm detail'],
  [`/realms/${encodeURIComponent(layoutIds.secondRealm)}`,'Public Realm detail'],
  ['/spaces','Spaces'],
  [`/spaces/${encodeURIComponent(layoutIds.space)}`,'Parent Space hierarchy'],
  [`/spaces/${encodeURIComponent(layoutIds.secondSpace)}`,'Child Space hierarchy'],
  ['/federation','Federation'],['/federation/ak%3Aoperation%3A0199a3a6-8000-7000-8000-000000000001','Federation operation'],['/media','Media'],['/handles','Handles'],
  ['/handles/handle-1234','Active Handle detail'],['/handles/handle-5678','Revoked Handle detail'],
  ['/invite-tokens','Invite tokens'],['/policy','Policy'],
  ['/capabilities','Capabilities'],['/coauth/capabilities','Authz capability grants'],
  ['/key-backup','Key backup and recovery policies'],
  [`/realms/${encodeURIComponent(layoutIds.realm)}/links`,'Realm links'],
  [`/realms/${encodeURIComponent(layoutIds.realm)}/media-service`,'Realm media service'],
  [`/realms/${encodeURIComponent(layoutIds.realm)}/organization`,'Organization control'],
  ['/realms/_/links','Select Realm links'],['/realms/_/media-service','Select Realm media service'],
  ['/realms/_/organization','Select organization Realm'],['/realms/_/authority','Select Realm authority'],
  ['/service-routes','Service routes'],
  [`/service-routes/${encodeURIComponent(layoutIds.station)}/station`,'Service route detail'],
  ['/server-status','Server status'],['/hardening','Hardening'],['/audit','Audit'],
  ['/coauth/accounts','Accounts'],['/coauth/accounts/01JZ9PK6HKFY0MM7C0TMZ1X8N7','Account detail'],
  ['/coauth/audit-log','Coauth audit log'],
  ['/coauth/accounts/01JZ9PK6HKFY0MM7C0TMZ1X8N7/devices','Account devices'],
  ['/coauth/oauth2-sessions','OAuth sessions'],['/coauth/personal-sessions','Personal sessions'],
  ['/coauth/registration-tokens','Registration tokens'],['/coauth/upstream-providers','Providers'],
  ['/coauth/upstream-links','Links'],['/coauth/notification-channels','Channels'],
  ['/coauth/notification-templates','Templates'],['/coauth/connector-health','Connector health'],
] as const;
for (const dark of [false,true]) {
  test('populated administration pages preserve table/card layout '+(dark?'dark':'light'),async({page})=>{
    test.setTimeout(180_000);
    const errors=await setupLayout(page,{dark});
    for (const [path,name]of pages) {
      await page.setViewportSize({width:1365,height:1000});
      await page.goto(path);
      await expect(page.getByRole('main')).toBeVisible();
      await expect(page.locator('.loading,.page-loading')).toHaveCount(0);
      expect(errors,name+' renderer and fixture errors').toEqual([]);
      await expect(page.locator('.resource-error'),name).toHaveCount(0);
      if(await page.getByRole('main').getByRole('table').count()) {
        expect(await page.getByRole('main').locator('tbody tr').count(),name+' populated rows').toBeGreaterThan(0);
      }
      await assertViewport(page);
      await assertCardLayouts(page);
      for(const pagination of await page.locator('.table-pagination').all()) {
        const table=pagination.locator('..').getByRole('table').first();
        if(await table.count()) {
          const tableBox=await table.boundingBox();
          const paginationBox=await pagination.boundingBox();
          expect.soft(paginationBox!.y,name+' pagination follows its table').toBeGreaterThanOrEqual(tableBox!.y+tableBox!.height);
        }
      }
      const heading=page.getByRole('main').getByRole('heading').first();
      await expect(heading,name).toBeVisible();
      expect(await heading.evaluate(el=>parseFloat(getComputedStyle(el).fontSize)),name).toBeGreaterThanOrEqual(24);
      await page.screenshot({path:test.info().outputPath((path.replaceAll('/','-')||'dashboard')+'-desktop.png')});
      await page.setViewportSize({width:390,height:844});
      await assertViewport(page);
      await assertCardLayouts(page);
      await page.screenshot({path:test.info().outputPath(path.replaceAll('/','-')+'-mobile.png')});
      expect(errors,name).toEqual([]);
    }
  });
}

test('account lists retain explicit Dioxus columns instead of serialized field panels',async({page})=>{
  await setupLayout(page);await page.goto('/coauth/accounts');
  await expect(page.getByRole('table')).toHaveCount(1);
  await expect(page.getByRole('columnheader')).toHaveText(['Account ID','Handle','Display Name','Status','Primary principal ID','Actions']);
  await expect(page.getByRole('row')).toHaveCount(3);
  await expect(page.getByRole('main').getByText('Alice', {exact:true})).toBeVisible();
  await expect(page.locator('.admin-action-form')).toHaveCount(0);
});

test('mobile account detail keeps dangerous editors inside the viewport and can close without discarding confirmation',async({page})=>{
  const errors=await setupLayout(page);
  await page.setViewportSize({width:390,height:844});
  await page.goto(`/coauth/accounts/${layoutIds.account}`);
  const binding=page.locator('.coauth-inline-record').filter({has:page.getByText('did:web:alice.example',{exact:true})});
  await expect(binding).toBeVisible();
  const trigger=binding.locator('button[aria-expanded]');
  await expect(trigger).toHaveCount(1);
  await expect(trigger).toHaveAttribute('aria-expanded','false');
  await trigger.click();
  const form=binding.locator('.admin-action-form');
  await expect(form).toBeVisible();
  const confirmation=form.getByRole('textbox',{name:'Confirmation phrase',exact:true});
  await confirmation.fill('mple');
  const box=await form.boundingBox();
  expect(box!.width).toBeLessThanOrEqual(390);
  expect(box!.x).toBeGreaterThanOrEqual(0);
  expect(box!.x+box!.width).toBeLessThanOrEqual(391);
  await assertViewport(page);
  await page.screenshot({path:test.info().outputPath('account-binding-expanded-mobile.png'),fullPage:true});
  await trigger.click();
  await expect(trigger).toHaveAttribute('aria-expanded','false');
  await expect(form).not.toBeVisible();
  await assertViewport(page);
  await trigger.click();
  await expect(confirmation).toHaveValue('mple');
  await expect(form).toBeVisible();
  expect(errors).toEqual([]);
});


test('Chinese administration labels preserve desktop and mobile layout', async ({page})=>{
  test.setTimeout(120_000);
  const errors=await setupLayout(page,{language:'zh-CN'});
  const paths=['/','/actors','/coauth/accounts','/coauth/audit-log','/server-status','/realms/'+encodeURIComponent(layoutIds.realm)+'/organization'];
  for(const path of paths){
    await page.setViewportSize({width:1365,height:1000});
    await page.goto(path);
    await expect(page.getByRole('main')).toBeVisible();
    await expect(page.locator('.loading,.page-loading,.resource-error')).toHaveCount(0);
    await assertViewport(page);
    await page.screenshot({path:test.info().outputPath(path.replaceAll('/','-')+'-zh-desktop.png')});
    await page.setViewportSize({width:390,height:844});
    await assertViewport(page);
    await page.screenshot({path:test.info().outputPath(path.replaceAll('/','-')+'-zh-mobile.png')});
  }
  expect(errors).toEqual([]);
});
