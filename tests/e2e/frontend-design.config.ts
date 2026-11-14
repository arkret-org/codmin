import { defineConfig } from '@playwright/test';
import config from './glory-migration.config';

export default defineConfig({ ...config, testMatch: 'frontend-design.spec.ts' });
