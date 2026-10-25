// Build the actual WASM app first; pass the public directory reported by scripts/build-web.py.
// NODE_PATH must include a Playwright installation. No live accounts are used.
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const http = require('node:http');
const path = require('node:path');
const { chromium } = require('playwright');
assert.ok(process.argv[2], 'pass the static web directory reported by scripts/build-web.py');
const assets = path.resolve(process.argv[2]);
let active;
let origin;
const server = http.createServer(async (req, res) => {
  const pathname = new URL(req.url, 'http://localhost').pathname;
  if (pathname === '/_coland/admin/viewer') {
    active.viewerRequests = (active.viewerRequests || 0) + 1;
    res.writeHead(503, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ error: 'isolated unavailable viewer' }));
    return;
  }
  if (pathname === '/config.json' || pathname === '/oauth/token') {
    res.setHeader('Content-Type', 'application/json');
    if (active && pathname === active.endpoint && !active.recovered) {
      const scenario = active;
      scenario.requests++;
      scenario.started ??= Date.now();
      scenario.closed = new Promise(resolve => res.on('close', () => { if (!res.writableFinished) scenario.aborted++; resolve(); }));
      if (scenario.body) { res.writeHead(200); res.write('{'); }
      return;
    }
    res.end(JSON.stringify({ coauth_public_url: origin }));
    return;
  }
  try {
    let filename = path.resolve(assets, `.${decodeURIComponent(pathname)}`);
    if (filename !== assets && !filename.startsWith(assets + path.sep)) throw new Error('invalid path');
    if (!path.extname(filename)) filename = path.join(assets, 'index.html');
    res.setHeader('Content-Type', { '.html': 'text/html', '.js': 'application/javascript', '.wasm': 'application/wasm', '.css': 'text/css' }[path.extname(filename)] || 'application/octet-stream');
    res.end(await fs.readFile(filename));
  } catch { res.writeHead(404).end(); }
});
(async () => {
  await fs.access(path.join(assets, 'index.html'));
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  origin = `http://127.0.0.1:${server.address().port}`;
  const browser = await chromium.launch({ headless: true, executablePath: process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH });
  try {
    for (const flow of ['config', 'exchange', 'refresh']) {
      const endpoint = flow === 'config' ? '/config.json' : '/oauth/token';
      for (const body of [false, true]) {
        active = { endpoint, body, requests: 0, aborted: 0, started: null, recovered: false };
        const page = await browser.newPage({ locale: 'en-US' });
        const crashes = [];
        page.on('pageerror', error => crashes.push(error.message));
        if (flow === 'refresh') {
          await page.addInitScript(() => {
            localStorage.setItem('session_active', JSON.stringify('1'));
            localStorage.setItem('token_expires_at_ms', JSON.stringify('0'));
          });
          await page.goto(origin + '/');
          await page.getByText('Admin check failed', { exact: true }).waitFor({ timeout: 23_000 });
          assert.equal(active.viewerRequests, 1, 'admin request continues after the refresh deadline');
          assert.equal(await page.getByRole('button', { name: 'Retry', exact: true }).isEnabled(), true);
        } else if (flow === 'exchange') {
          await page.addInitScript(() => {
            sessionStorage.setItem('pkce_code_verifier', 'test-verifier');
            sessionStorage.setItem('oauth_state', 'test-state');
          });
          await page.goto(origin + '/oauth/callback?code=test-code&state=test-state');
          await page.waitForURL(url => url.pathname === '/login', { timeout: 23_000 });
        } else {
          await page.goto(origin + '/login');
          await page.getByText(/Failed to (fetch|parse) \/config.json/).waitFor({ timeout: 12_000 });
        }
        assert.ok(active.started, 'the real WASM app sent the target request');
        const elapsed = Date.now() - active.started;
        const limit = endpoint === '/config.json' ? 5000 : 15000;
        assert.ok(elapsed >= limit - 500 && elapsed < limit + 5000, `deadline: ${elapsed}ms`);
        assert.equal(active.requests, 1, 'no automatic replay');
        await Promise.race([active.closed, new Promise((_, reject) => setTimeout(() => reject(new Error('fetch remained open after timeout')), 1000))]);
        assert.ok(active.aborted > 0, 'underlying fetch was aborted');
        active.recovered = true;
        if (flow === 'refresh') {
          await page.evaluate(() => localStorage.clear());
          await page.goto(origin + '/login');
        }
        await page.reload();
        await page.getByRole('button', { name: /sign in/i }).waitFor();
        assert.equal(await page.getByRole('button', { name: /sign in/i }).isEnabled(), true);
        assert.deepEqual(crashes, []);
        console.log(`PASS ${flow} ${body ? 'body' : 'headers'}: ${elapsed}ms, reload recovered`);
        await page.close();
      }
    }
  } finally {
    await browser.close();
    server.closeAllConnections();
    await new Promise(resolve => server.close(resolve));
  }
})().catch(error => { console.error(error); server.closeAllConnections(); server.close(); process.exitCode = 1; });
