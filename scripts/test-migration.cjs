#!/usr/bin/env node
const { join, resolve } = require('node:path');
const { spawnSync } = require('node:child_process');
const env = { ...process.env };
const root = resolve(__dirname, '..');
const layout = process.argv[2] === '--layout';
const result = spawnSync(process.execPath, [join(root, 'node_modules/@playwright/test/cli.js'),
  'test', '-c', layout ? 'tests/e2e/glory-layout.config.ts' : 'tests/e2e/glory-migration.config.ts', ...process.argv.slice(layout ? 3 : 2)], { cwd: root, env, stdio: 'inherit' });
if (result.error) console.error(result.error);
process.exit(result.status ?? 1);
