#!/usr/bin/env node
const { mkdirSync } = require('node:fs');
const { join, resolve } = require('node:path');
const { spawnSync } = require('node:child_process');
const env = { ...process.env };
if (process.platform === 'win32') {
  const cache = 'D:/BuildCache/codmin-glory';
  env.TEMP = env.TMP = `${cache}/tmp`;
  env.CODMIN_E2E_OUTPUT_DIR ||= `${cache}/e2e`;
  env.PWTEST_CACHE_DIR ||= `${env.CODMIN_E2E_OUTPUT_DIR}/transform-cache`;
  for (const directory of [env.TEMP, env.CODMIN_E2E_OUTPUT_DIR, env.PWTEST_CACHE_DIR]) mkdirSync(directory, { recursive: true });
}
const root = resolve(__dirname, '..');
const result = spawnSync(process.execPath, [join(root, 'node_modules/@playwright/test/cli.js'),
  'test', '-c', 'tests/e2e/glory-migration.config.ts', ...process.argv.slice(2)], { cwd: root, env, stdio: 'inherit' });
if (result.error) console.error(result.error);
process.exit(result.status ?? 1);
